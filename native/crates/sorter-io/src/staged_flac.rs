//! Stream-only native FLAC FixNames repair; never opens a path or publishes data.
//! All blocks except VORBIS_COMMENT, all unassigned comments, and the complete
//! encoded media tail are copied exactly. Metadata size may change; see
//! `native/docs/staged-flac-repair.md` for the offset and compatibility contract.
use crate::{
    staged_xiph::{CommentKind, rewrite_comments, validate_repairs},
    staged_tags::{StagedTagEvidence, TagWriteError},
};
use sha2::{Digest, Sha256};
use sorter_core::{
    fix_plan::FieldRepair,
    metadata::LegacyField,
};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_METADATA: usize = 64 * 1024 * 1024;
const MAX_BLOCK: usize = 0x00ff_ffff;
const MAX_BLOCKS: usize = 4096;
const CHUNK: usize = 64 * 1024;
type Result<T> = std::result::Result<T, TagWriteError>;

#[derive(Clone, Debug)]
pub struct FlacStagedPlan {
    original_header: Vec<u8>,
    repaired_header: Vec<u8>,
    source_length: u64,
    output_length: u64,
    changed_fields: Vec<LegacyField>,
}

fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) {
        Err(TagWriteError::Cancelled)
    } else {
        Ok(())
    }
}

fn read_checked(input: &mut impl Read, bytes: &mut [u8], flag: &AtomicBool) -> Result<()> {
    for chunk in bytes.chunks_mut(CHUNK) {
        cancelled(flag)?;
        input.read_exact(chunk).map_err(|_| TagWriteError::Io)?;
    }
    cancelled(flag)
}


impl FlacStagedPlan {
    /// Inspect native, offset-zero `fLaC` and validate every comment before any
    /// output exists. Repairs must be the ordered subset of FIX_FIELDS produced
    /// by the released planner; original values must match exact Xiph projection.
    pub fn inspect(
        input: &mut (impl Read + Seek),
        source_length: u64,
        repairs: &[FieldRepair],
        cancel: &AtomicBool,
    ) -> Result<Self> {
        cancelled(cancel)?;
        if input.seek(SeekFrom::End(0)).map_err(|_| TagWriteError::Io)? != source_length {
            return Err(TagWriteError::Stale);
        }
        input.seek(SeekFrom::Start(0)).map_err(|_| TagWriteError::Io)?;
        let mut magic = [0; 4];
        read_checked(input, &mut magic, cancel)?;
        if &magic != b"fLaC" {
            return Err(TagWriteError::Unsupported("only offset-zero native FLAC is supported"));
        }
        validate_repairs(repairs, MAX_BLOCK, cancel)?;
        let mut original_header = magic.to_vec();
        let mut repaired_header = magic.to_vec();
        let mut comment_seen = false;
        let mut seektable_seen = false;
        for index in 0..MAX_BLOCKS {
            let mut header = [0; 4];
            read_checked(input, &mut header, cancel)?;
            let kind = header[0] & 0x7f;
            let size = (usize::from(header[1]) << 16) | (usize::from(header[2]) << 8) | usize::from(header[3]);
            if kind == 127 || (index == 0 && kind != 0) || (kind == 0 && (index != 0 || size != 34)) {
                return Err(TagWriteError::Unsupported("invalid FLAC metadata block sequence"));
            }
            let end = original_header.len().checked_add(4).and_then(|n| n.checked_add(size))
                .ok_or(TagWriteError::Unsupported("metadata length overflow"))?;
            if end > MAX_METADATA || end as u64 > source_length {
                return Err(TagWriteError::Unsupported("metadata budget or source length"));
            }
            if kind == 4 && comment_seen {
                return Err(TagWriteError::Unsupported("duplicate Vorbis comment block"));
            }
            let mut body = vec![0; size];
            read_checked(input, &mut body, cancel)?;
            match kind {
                0 => {
                    let min = u16::from_be_bytes([body[0], body[1]]);
                    let max = u16::from_be_bytes([body[2], body[3]]);
                    let rate = (u32::from(body[10]) << 12) | (u32::from(body[11]) << 4) | (u32::from(body[12]) >> 4);
                    let bits = ((body[12] & 1) << 4) | (body[13] >> 4);
                    if min < 16 || max < min || rate == 0 || bits < 3 {
                        return Err(TagWriteError::Unsupported("invalid audio STREAMINFO"));
                    }
                }
                1 if body.iter().any(|b| *b != 0) => return Err(TagWriteError::Unsupported("nonzero FLAC padding")),
                2 if size < 4 => return Err(TagWriteError::Unsupported("truncated APPLICATION identifier")),
                3 => {
                    if seektable_seen || !size.is_multiple_of(18) {
                        return Err(TagWriteError::Unsupported("invalid or duplicate SEEKTABLE"));
                    }
                    seektable_seen = true;
                }
                _ => {}
            }
            let replacement = if kind == 4 {
                comment_seen = true;
                Some(rewrite_comments(&body, repairs, CommentKind::Flac, MAX_BLOCK, cancel)?)
            } else {
                None
            };
            original_header.extend(header);
            original_header.extend_from_slice(&body);
            let new_body = replacement.as_deref().unwrap_or(&body);
            if repaired_header.len().checked_add(4).and_then(|n| n.checked_add(new_body.len())).is_none_or(|n| n > MAX_METADATA) {
                return Err(TagWriteError::Unsupported("rewritten metadata budget"));
            }
            let length = new_body.len();
            repaired_header.extend([header[0], (length >> 16) as u8, (length >> 8) as u8, length as u8]);
            repaired_header.extend_from_slice(new_body);
            if header[0] & 0x80 != 0 {
                break;
            }
            if index + 1 == MAX_BLOCKS {
                return Err(TagWriteError::Unsupported("metadata block count budget"));
            }
        }
        if !comment_seen && !repairs.is_empty() {
            return Err(TagWriteError::Stale);
        }
        let media_length = source_length - original_header.len() as u64;
        if media_length < 4 {
            return Err(TagWriteError::Unsupported("missing FLAC audio"));
        }
        let mut frame = [0; 4];
        read_checked(input, &mut frame, cancel)?;
        if frame[0] != 0xff || frame[1] & 0xfe != 0xf8 || frame[2] >> 4 == 0
            || frame[2] & 15 == 15 || frame[3] >> 4 > 10 || frame[3] & 1 != 0 || (frame[3] >> 1) & 7 == 3
        {
            return Err(TagWriteError::Unsupported("FLAC frame header required"));
        }
        // Reject known nonnative tag mirrors instead of changing only one copy.
        for (distance, signature) in [(128, &b"TAG"[..]), (32, &b"APETAGEX"[..]), (160, &b"APETAGEX"[..])] {
            if source_length >= distance {
                input.seek(SeekFrom::Start(source_length - distance)).map_err(|_| TagWriteError::Io)?;
                let mut bytes = vec![0; signature.len()];
                read_checked(input, &mut bytes, cancel)?;
                if bytes == signature {
                    return Err(TagWriteError::Unsupported("nonnative trailing tag mirror"));
                }
            }
        }
        let output_length = media_length.checked_add(repaired_header.len() as u64)
            .ok_or(TagWriteError::Unsupported("staged length overflow"))?;
        cancelled(cancel)?;
        Ok(Self {
            original_header,
            repaired_header,
            source_length,
            output_length,
            changed_fields: repairs.iter().map(|r| r.field).collect(),
        })
    }

    pub fn changes_bytes(&self) -> bool {
        self.original_header != self.repaired_header
    }

    pub fn staged_length(&self) -> u64 {
        self.output_length
    }

    /// Publication cannot preserve unknown private absolute offsets when the
    /// metadata prefix moves. The stream-only writer still preserves those bytes;
    /// product callers must apply this narrower boundary before creating output.
    pub(crate) fn check_product_preservation(&self) -> Result<()> {
        if self.output_length == self.source_length {
            return Ok(());
        }
        let mut at = 4;
        while at < self.original_header.len() {
            let kind = self.original_header[at] & 0x7f;
            if kind == 2 || kind >= 7 {
                return Err(TagWriteError::Unsupported(
                    "opaque FLAC metadata cannot be moved during product repair",
                ));
            }
            let size = (usize::from(self.original_header[at + 1]) << 16)
                | (usize::from(self.original_header[at + 2]) << 8)
                | usize::from(self.original_header[at + 3]);
            at += 4 + size;
        }
        Ok(())
    }

    /// Write only to an exclusively owned, empty staging stream. An error may
    /// leave partial/untrusted output: discard it, never publish it. The caller
    /// must independently flush, re-read/hash, validate, and publish that stage.
    /// Hashes attest consumed/emitted bytes, not filesystem authority/durability.
    pub fn write_staged(
        &self,
        input: &mut (impl Read + Seek),
        output: &mut impl Write,
        expected_source_sha256: &str,
        cancel: &AtomicBool,
    ) -> Result<StagedTagEvidence> {
        cancelled(cancel)?;
        input.seek(SeekFrom::Start(0)).map_err(|_| TagWriteError::Io)?;
        let mut original = vec![0; self.original_header.len()];
        read_checked(input, &mut original, cancel)?;
        if original != self.original_header {
            return Err(TagWriteError::Stale);
        }
        let mut source = Sha256::new();
        source.update(&original);
        let mut staged = Sha256::new();
        let mut media = Sha256::new();
        for chunk in self.repaired_header.chunks(CHUNK) {
            cancelled(cancel)?;
            output.write_all(chunk).map_err(|_| TagWriteError::Io)?;
            staged.update(chunk);
        }
        let media_length = self.source_length - self.original_header.len() as u64;
        let mut remaining = media_length;
        let mut buffer = [0; CHUNK];
        while remaining > 0 {
            cancelled(cancel)?;
            let amount = remaining.min(buffer.len() as u64) as usize;
            read_checked(input, &mut buffer[..amount], cancel)?;
            source.update(&buffer[..amount]);
            media.update(&buffer[..amount]);
            output.write_all(&buffer[..amount]).map_err(|_| TagWriteError::Io)?;
            staged.update(&buffer[..amount]);
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
            staged_length: self.output_length,
            changed_fields: self.changed_fields.clone(),
        })
    }
}
