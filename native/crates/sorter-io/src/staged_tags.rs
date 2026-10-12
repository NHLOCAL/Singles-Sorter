//! Bounded staged MP3 text-frame repair. Never opens paths or writes live source
//! files. All non-target frames and the complete media tail remain byte-identical;
//! the media start offset and total file length are unchanged.
use crate::metadata::{RawFamily, id3_text, project};
use sha2::{Digest, Sha256};
use sorter_core::{
    fix_plan::{FIX_FIELDS, FieldRepair},
    metadata::{LegacyField, LegacyScalar},
};
use std::{
    collections::HashSet,
    io::{Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};
const MAX_TAG: usize = 16 * 1024 * 1024;
const MAX_FRAMES: usize = 4096;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagWriteError {
    Unsupported(&'static str),
    Stale,
    Cancelled,
    Io,
}
impl std::fmt::Display for TagWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TagWriteError {}
type Result<T> = std::result::Result<T, TagWriteError>;
#[derive(Clone, Debug)]
pub struct Id3StagedPlan {
    original_header: Vec<u8>,
    repaired_header: Vec<u8>,
    source_length: u64,
    changed_fields: Vec<LegacyField>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedTagEvidence {
    pub source_sha256: String,
    pub staged_sha256: String,
    pub media_sha256: String,
    pub media_length: u64,
    pub staged_length: u64,
    pub changed_fields: Vec<LegacyField>,
}
fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) {
        Err(TagWriteError::Cancelled)
    } else {
        Ok(())
    }
}
fn sync_size(bytes: &[u8]) -> Result<usize> {
    if bytes.len() != 4 || bytes.iter().any(|b| b & 128 != 0) {
        return Err(TagWriteError::Unsupported("invalid synchsafe size"));
    }
    Ok(bytes.iter().fold(0usize, |n, b| (n << 7) | usize::from(*b)))
}
fn write_sync_size(size: usize) -> Result<[u8; 4]> {
    if size > 0x0fff_ffff {
        return Err(TagWriteError::Unsupported("tag size overflow"));
    }
    Ok([
        ((size >> 21) & 127) as u8,
        ((size >> 14) & 127) as u8,
        ((size >> 7) & 127) as u8,
        (size & 127) as u8,
    ])
}
fn frame_id(field: LegacyField) -> Result<[u8; 4]> {
    Ok(match field {
        LegacyField::Artist => *b"TPE1",
        LegacyField::AlbumArtist => *b"TPE2",
        LegacyField::Title => *b"TIT2",
        LegacyField::Album => *b"TALB",
        LegacyField::Genre => *b"TCON",
        _ => {
            return Err(TagWriteError::Unsupported(
                "field is outside released repair policy",
            ));
        }
    })
}
fn encode_text(original: &[u8], replacement: &str, version: u8) -> Result<Vec<u8>> {
    if replacement.contains('\0') {
        return Err(TagWriteError::Unsupported("embedded NUL text"));
    }
    let encoding = *original
        .first()
        .ok_or(TagWriteError::Unsupported("empty text frame"))?;
    // Latin-1 cannot represent repaired Hebrew. Upgrade only this frame to the
    // Unicode representation allowed by its existing ID3 version. Codec bytes
    // and the ID3 major version are unchanged.
    let encoding = if encoding == 0 && replacement.chars().any(|c| u32::from(c) > 255) {
        if version == 3 { 1 } else { 3 }
    } else {
        encoding
    };
    let mut bytes = vec![encoding];
    match encoding {
        0 => bytes.extend(replacement.chars().map(|c| c as u8)),
        1 => {
            let big = original.get(1..3) == Some(&[0xfe, 0xff]);
            bytes.extend(if big { [0xfe, 0xff] } else { [0xff, 0xfe] });
            for unit in replacement.encode_utf16() {
                bytes.extend(if big {
                    unit.to_be_bytes()
                } else {
                    unit.to_le_bytes()
                });
            }
        }
        2 if version == 4 => {
            for unit in replacement.encode_utf16() {
                bytes.extend(unit.to_be_bytes());
            }
        }
        3 if version == 4 => bytes.extend(replacement.as_bytes()),
        _ => {
            return Err(TagWriteError::Unsupported(
                "unsupported text encoding for ID3 version",
            ));
        }
    }
    Ok(bytes)
}
impl Id3StagedPlan {
    /// Validate every representation and original field value before producing
    /// any output. `repairs` must be the original planner's five-field sequence.
    pub fn inspect(
        input: &mut (impl Read + Seek),
        source_length: u64,
        repairs: &[FieldRepair],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        input
            .seek(SeekFrom::Start(0))
            .map_err(|_| TagWriteError::Io)?;
        let mut header = [0u8; 10];
        input
            .read_exact(&mut header)
            .map_err(|_| TagWriteError::Io)?;
        if &header[..3] != b"ID3" || !matches!(header[3], 3 | 4) || header[4] != 0 || header[5] != 0
        {
            return Err(TagWriteError::Unsupported(
                "only unflagged ID3v2.3/v2.4; no unsync, footer or extended header",
            ));
        }
        let size = sync_size(&header[6..10])?;
        if size > MAX_TAG || size as u64 + 10 > source_length {
            return Err(TagWriteError::Unsupported("tag budget or source length"));
        }
        let mut body = vec![0; size];
        input.read_exact(&mut body).map_err(|_| TagWriteError::Io)?;
        // A leading ID3 tag alone does not establish MP3 coverage.
        let mut mpeg = [0u8; 4];
        input
            .read_exact(&mut mpeg)
            .map_err(|_| TagWriteError::Unsupported("missing MPEG audio"))?;
        if mpeg[0] != 0xff
            || mpeg[1] & 0xe0 != 0xe0
            || (mpeg[1] >> 3) & 3 == 1
            || (mpeg[1] >> 1) & 3 != 1
            || matches!(mpeg[2] >> 4, 0 | 15)
            || (mpeg[2] >> 2) & 3 == 3
        {
            return Err(TagWriteError::Unsupported("MP3 frame header required"));
        }
        // A legacy ID3v1 mirror has different encoding/update semantics. Do not
        // repair only one representation and leave a contradictory mirror.
        if source_length >= 128 {
            input
                .seek(SeekFrom::Start(source_length - 128))
                .map_err(|_| TagWriteError::Io)?;
            let mut legacy = [0u8; 3];
            input
                .read_exact(&mut legacy)
                .map_err(|_| TagWriteError::Io)?;
            if &legacy == b"TAG" {
                return Err(TagWriteError::Unsupported(
                    "ID3v1 mirror is outside initial repair coverage",
                ));
            }
        }
        if source_length >= 32 {
            input
                .seek(SeekFrom::Start(source_length - 32))
                .map_err(|_| TagWriteError::Io)?;
            let mut ape = [0u8; 8];
            input.read_exact(&mut ape).map_err(|_| TagWriteError::Io)?;
            if &ape == b"APETAGEX" {
                return Err(TagWriteError::Unsupported(
                    "APE mirror is outside initial repair coverage",
                ));
            }
        }
        let mut original_header = header.to_vec();
        original_header.extend(body);
        let repaired_header = rewrite_id3_block(&original_header, repairs, cancel)?;
        Ok(Self {
            original_header,
            repaired_header,
            source_length,
            changed_fields: repairs.iter().map(|r| r.field).collect(),
        })
    }
    pub fn changes_bytes(&self) -> bool {
        self.original_header != self.repaired_header
    }
    pub fn staged_length(&self) -> u64 {
        self.source_length - self.original_header.len() as u64 + self.repaired_header.len() as u64
    }
    /// Caller supplies an exclusively owned, empty staging stream. This generic
    /// byte transform is not a native path authority or publication operation.
    pub fn write_staged(
        &self,
        input: &mut (impl Read + Seek),
        output: &mut impl Write,
        expected_source_sha256: &str,
        cancel: &AtomicBool,
    ) -> Result<StagedTagEvidence> {
        cancelled(cancel)?;
        input
            .seek(SeekFrom::Start(0))
            .map_err(|_| TagWriteError::Io)?;
        let mut original = vec![0; self.original_header.len()];
        input
            .read_exact(&mut original)
            .map_err(|_| TagWriteError::Io)?;
        if original != self.original_header {
            return Err(TagWriteError::Stale);
        }
        let mut source = Sha256::new();
        source.update(&original);
        let mut staged = Sha256::new();
        staged.update(&self.repaired_header);
        let mut media = Sha256::new();
        output
            .write_all(&self.repaired_header)
            .map_err(|_| TagWriteError::Io)?;
        let mut remaining = self.source_length - self.original_header.len() as u64;
        let media_length = remaining;
        let mut buffer = [0u8; 64 * 1024];
        while remaining > 0 {
            cancelled(cancel)?;
            let amount = remaining.min(buffer.len() as u64) as usize;
            input
                .read_exact(&mut buffer[..amount])
                .map_err(|_| TagWriteError::Io)?;
            source.update(&buffer[..amount]);
            staged.update(&buffer[..amount]);
            media.update(&buffer[..amount]);
            output
                .write_all(&buffer[..amount])
                .map_err(|_| TagWriteError::Io)?;
            remaining -= amount as u64;
        }
        let mut extra = [0; 1];
        if input.read(&mut extra).map_err(|_| TagWriteError::Io)? != 0 {
            return Err(TagWriteError::Stale);
        }
        let source_sha256 = format!("{:x}", source.finalize());
        if source_sha256 != expected_source_sha256 {
            return Err(TagWriteError::Stale);
        }
        cancelled(cancel)?;
        Ok(StagedTagEvidence {
            source_sha256,
            staged_sha256: format!("{:x}", staged.finalize()),
            media_sha256: format!("{:x}", media.finalize()),
            media_length,
            staged_length: self.staged_length(),
            changed_fields: self.changed_fields.clone(),
        })
    }
}

/// Shared exact-extent ID3 text transform. The container caller establishes its
/// own media/chunk layout; this function never infers an MP3 from ID3 bytes.
pub(crate) fn rewrite_id3_block(original: &[u8], repairs: &[FieldRepair], cancel: &AtomicBool) -> Result<Vec<u8>> {
    cancelled(cancel)?;
    if original.len() < 10 { return Err(TagWriteError::Unsupported("truncated ID3 header")); }
    let mut header: [u8;10] = original[..10].try_into().unwrap();
    if &header[..3] != b"ID3" || !matches!(header[3],3|4) || header[4] != 0 || header[5] != 0 {
        return Err(TagWriteError::Unsupported("only unflagged ID3v2.3/v2.4; no unsync, footer or extended header"));
    }
    let size = sync_size(&header[6..10])?;
    if size > MAX_TAG || size.checked_add(10) != Some(original.len()) {
        return Err(TagWriteError::Unsupported("ID3 block extent or budget"));
    }
    let body = &original[10..];
        let mut previous = None;
        let mut repairs_by_id = Vec::new();
        for repair in repairs {
            let index = FIX_FIELDS
                .iter()
                .position(|f| *f == repair.field)
                .ok_or(TagWriteError::Unsupported("repair field"))?;
            if previous.is_some_and(|p| p >= index)
                || repair.original == repair.replacement
                || sorter_core::fix_jibrish(&repair.original, sorter_core::RepairMode::Hebrew)
                    != repair.replacement
            {
                return Err(TagWriteError::Unsupported(
                    "repair order or unchanged assignment",
                ));
            }
            previous = Some(index);
            repairs_by_id.push((frame_id(repair.field)?, repair));
        }
        let mut rebuilt = vec![];
        let mut seen = HashSet::new();
        let mut at = 0;
        let mut count = 0;
        while at < body.len() && body[at] != 0 {
            cancelled(cancel)?;
            count += 1;
            if count > MAX_FRAMES || body.len() - at < 10 {
                return Err(TagWriteError::Unsupported(
                    "frame count or truncated header",
                ));
            }
            let id: [u8; 4] = body[at..at + 4]
                .try_into()
                .map_err(|_| TagWriteError::Unsupported("frame ID"))?;
            if !id
                .iter()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
                || body[at + 8] != 0
                || body[at + 9] != 0
            {
                return Err(TagWriteError::Unsupported("frame ID or flags"));
            }
            let length = if header[3] == 4 {
                sync_size(&body[at + 4..at + 8])?
            } else {
                u32::from_be_bytes(
                    body[at + 4..at + 8]
                        .try_into()
                        .map_err(|_| TagWriteError::Unsupported("frame size"))?,
                ) as usize
            };
            if length > body.len() - at - 10 {
                return Err(TagWriteError::Unsupported("truncated frame"));
            }
            let payload = &body[at + 10..at + 10 + length];
            if let Some((_, repair)) = repairs_by_id.iter().find(|(key, _)| *key == id) {
                if !seen.insert(id) {
                    return Err(TagWriteError::Unsupported("duplicate repaired frame"));
                }
                let values = id3_text(payload)
                    .map_err(|_| TagWriteError::Unsupported("text representation"))?;
                let projected = project(RawFamily::Id3, repair.field, values)
                    .map_err(|_| TagWriteError::Unsupported("field projection"))?;
                if projected.value != Ok(LegacyScalar::Text(repair.original.clone())) {
                    return Err(TagWriteError::Stale);
                }
                let new_payload = encode_text(payload, &repair.replacement, header[3])?;
                rebuilt.extend(id);
                rebuilt.extend(if header[3] == 4 {
                    write_sync_size(new_payload.len())?
                } else {
                    u32::try_from(new_payload.len())
                        .map_err(|_| TagWriteError::Unsupported("frame size"))?
                        .to_be_bytes()
                });
                rebuilt.extend([0, 0]);
                rebuilt.extend(new_payload);
            } else {
                rebuilt.extend(&body[at..at + 10 + length]);
            }
            if rebuilt.len() > MAX_TAG {
                return Err(TagWriteError::Unsupported("rewritten tag budget"));
            }
            at += 10 + length;
        }
        if body[at..].iter().any(|b| *b != 0) {
            return Err(TagWriteError::Unsupported("nonzero padding"));
        }
        if seen.len() != repairs.len() {
            return Err(TagWriteError::Stale);
        }
        // Keep the media start byte-for-byte at its original offset. Unknown
        // preserved frames can contain absolute offsets; growing the tag would
        // silently invalidate those references. Existing zero padding is space
        // for larger Unicode text; insufficient padding is explicit unsupported.
        if rebuilt.len() > body.len() {
            return Err(TagWriteError::Unsupported(
                "insufficient existing padding for offset-preserving repair",
            ));
        }
        rebuilt.resize(body.len(), 0);
        header[6..10].copy_from_slice(&write_sync_size(rebuilt.len())?);
        let mut repaired_header = header.to_vec();
        repaired_header.extend(rebuilt);
        Ok(repaired_header)
}
