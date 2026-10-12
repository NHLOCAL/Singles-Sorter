//! Stream-only native FLAC FixNames repair; never opens a path or publishes data.
//! All blocks except VORBIS_COMMENT, all unassigned comments, and the complete
//! encoded media tail are copied exactly. Metadata size may change; see
//! `native/docs/staged-flac-repair.md` for the offset and compatibility contract.
use crate::{
    metadata::{RawFamily, RawValue, project},
    staged_tags::{StagedTagEvidence, TagWriteError},
};
use sha2::{Digest, Sha256};
use sorter_core::{
    RepairMode, fix_jibrish,
    fix_plan::{FIX_FIELDS, FieldRepair},
    metadata::{LegacyField, LegacyScalar},
};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    sync::atomic::{AtomicBool, Ordering},
};

const MAX_METADATA: usize = 64 * 1024 * 1024;
const MAX_BLOCK: usize = 0x00ff_ffff;
const MAX_BLOCKS: usize = 4096;
const MAX_COMMENTS: usize = 4096;
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

fn key(field: LegacyField) -> Result<&'static str> {
    match field {
        LegacyField::Artist => Ok("artist"),
        LegacyField::AlbumArtist => Ok("albumartist"),
        LegacyField::Title => Ok("title"),
        LegacyField::Album => Ok("album"),
        LegacyField::Genre => Ok("genre"),
        _ => Err(TagWriteError::Unsupported("field is outside released repair policy")),
    }
}

fn word(bytes: &[u8], at: &mut usize) -> Result<usize> {
    let end = at.checked_add(4).ok_or(TagWriteError::Unsupported("comment length overflow"))?;
    let value = bytes.get(*at..end).ok_or(TagWriteError::Unsupported("truncated comment length"))?;
    *at = end;
    Ok(u32::from_le_bytes(value.try_into().map_err(|_| TagWriteError::Unsupported("comment length"))?) as usize)
}

fn vector<'a>(bytes: &'a [u8], at: &mut usize) -> Result<&'a [u8]> {
    let size = word(bytes, at)?;
    let end = at.checked_add(size).ok_or(TagWriteError::Unsupported("comment vector overflow"))?;
    let value = bytes.get(*at..end).ok_or(TagWriteError::Unsupported("truncated comment vector"))?;
    *at = end;
    Ok(value)
}

fn put_vector(out: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    if out.len().checked_add(4).and_then(|n| n.checked_add(value.len())).is_none_or(|n| n > MAX_BLOCK) {
        return Err(TagWriteError::Unsupported("rewritten comment exceeds FLAC block limit"));
    }
    out.extend(u32::try_from(value.len()).map_err(|_| TagWriteError::Unsupported("comment vector length"))?.to_le_bytes());
    out.extend_from_slice(value);
    Ok(())
}

fn rewrite_comments(bytes: &[u8], repairs: &[FieldRepair], flag: &AtomicBool) -> Result<Vec<u8>> {
    let mut at = 0;
    let vendor = vector(bytes, &mut at)?;
    std::str::from_utf8(vendor).map_err(|_| TagWriteError::Unsupported("vendor is not UTF-8"))?;
    let count = word(bytes, &mut at)?;
    if count > MAX_COMMENTS {
        return Err(TagWriteError::Unsupported("comment count budget"));
    }
    let mut comments = Vec::with_capacity(count);
    for _ in 0..count {
        cancelled(flag)?;
        let raw = vector(bytes, &mut at)?;
        let text = std::str::from_utf8(raw).map_err(|_| TagWriteError::Unsupported("comment is not UTF-8"))?;
        let (name, value) = text.split_once('=').ok_or(TagWriteError::Unsupported("comment has no separator"))?;
        if name.is_empty() || !name.bytes().all(|b| (0x20..=0x7e).contains(&b) && b != b'=') {
            return Err(TagWriteError::Unsupported("invalid comment field name"));
        }
        comments.push((raw, name, value));
    }
    // FLAC has neither Vorbis's packet prefix nor its framing bit. Reject a
    // trailer instead of normalizing it or interpreting it as more comments.
    if at != bytes.len() {
        return Err(TagWriteError::Unsupported("trailing comment bytes"));
    }
    for repair in repairs {
        cancelled(flag)?;
        let name = key(repair.field)?;
        let values = comments.iter().filter(|(_, k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, _, v)| RawValue::Text((*v).to_owned())).collect::<Vec<_>>();
        if values.is_empty() {
            return Err(TagWriteError::Stale);
        }
        // Use raw values, including empty duplicates, not TagLib's addField
        // normalization. A lossy native-reader plan must fail this comparison.
        let projected = project(RawFamily::Xiph, repair.field, values)
            .map_err(|_| TagWriteError::Unsupported("field projection"))?;
        if projected.value != Ok(LegacyScalar::Text(repair.original.clone())) {
            return Err(TagWriteError::Stale);
        }
    }
    let mut preserved = Vec::with_capacity(count);
    for (raw, name, _) in comments {
        if !repairs.iter().any(|r| key(r.field).is_ok_and(|k| name.eq_ignore_ascii_case(k))) {
            preserved.push(raw);
        }
    }
    let output_count = preserved.len() + repairs.len();
    let mut out = Vec::new();
    put_vector(&mut out, vendor)?;
    if out.len() > MAX_BLOCK - 4 {
        return Err(TagWriteError::Unsupported("rewritten comment count exceeds block limit"));
    }
    out.extend((output_count as u32).to_le_bytes());
    for raw in preserved {
        cancelled(flag)?;
        put_vector(&mut out, raw)?;
    }
    // music-tag FlacFile + Mutagen VCommentDict assignment removes all matching
    // pairs, then appends one lowercase key. Released field order is observable.
    for repair in repairs {
        cancelled(flag)?;
        let value = format!("{}={}", key(repair.field)?, repair.replacement);
        put_vector(&mut out, value.as_bytes())?;
    }
    Ok(out)
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
        if repairs.len() > FIX_FIELDS.len() {
            return Err(TagWriteError::Unsupported("repair count"));
        }
        let mut previous = None;
        for repair in repairs {
            cancelled(cancel)?;
            let index = FIX_FIELDS.iter().position(|f| *f == repair.field)
                .ok_or(TagWriteError::Unsupported("repair field"))?;
            if previous.is_some_and(|p| p >= index)
                || repair.original.len() > MAX_BLOCK || repair.replacement.len() > MAX_BLOCK
                || repair.original == repair.replacement
                || fix_jibrish(&repair.original, RepairMode::Hebrew) != repair.replacement
            {
                return Err(TagWriteError::Unsupported("repair order or nonpolicy assignment"));
            }
            // Native UTF-8/NUL interoperability is not established for repaired
            // values. Unknown/unassigned values containing NUL are preserved.
            if repair.original.contains('\0') {
                return Err(TagWriteError::Unsupported("embedded NUL in repaired field"));
            }
            previous = Some(index);
        }
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
                Some(rewrite_comments(&body, repairs, cancel)?)
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
