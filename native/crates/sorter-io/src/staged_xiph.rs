//! Shared exact five-field Xiph assignment semantics; container framing is explicit.
use crate::{metadata::{RawFamily, RawValue, project}, staged_tags::TagWriteError};
use sorter_core::{RepairMode, fix_jibrish, fix_plan::{FIX_FIELDS, FieldRepair}, metadata::{LegacyField, LegacyScalar}};
use std::sync::atomic::{AtomicBool, Ordering};
type Result<T> = std::result::Result<T, TagWriteError>;
const MAX_COMMENTS: usize = 4096;
#[derive(Clone, Copy)]
pub(crate) enum CommentKind { Flac, Vorbis, Opus }
fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) { Err(TagWriteError::Cancelled) } else { Ok(()) }
}

pub(crate) fn validate_repairs(repairs: &[FieldRepair], limit: usize, cancel: &AtomicBool) -> Result<()> {
    if repairs.len() > FIX_FIELDS.len() { return Err(TagWriteError::Unsupported("repair count")); }
    let mut previous = None;
    for repair in repairs {
        cancelled(cancel)?;
        let index = FIX_FIELDS.iter().position(|f| *f == repair.field)
            .ok_or(TagWriteError::Unsupported("repair field"))?;
        if previous.is_some_and(|p| p >= index)
            || repair.original.len() > limit || repair.replacement.len() > limit
            || repair.original == repair.replacement
            || fix_jibrish(&repair.original, RepairMode::Hebrew) != repair.replacement {
            return Err(TagWriteError::Unsupported("repair order or nonpolicy assignment"));
        }
        if repair.original.contains('\0') {
            return Err(TagWriteError::Unsupported("embedded NUL in repaired field"));
        }
        previous = Some(index);
    }
    Ok(())
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

fn put_vector(out: &mut Vec<u8>, value: &[u8], limit: usize) -> Result<()> {
    if out.len().checked_add(4).and_then(|n| n.checked_add(value.len())).is_none_or(|n| n > limit) {
        return Err(TagWriteError::Unsupported("rewritten comment exceeds budget"));
    }
    out.extend(u32::try_from(value.len()).map_err(|_| TagWriteError::Unsupported("comment vector length"))?.to_le_bytes());
    out.extend_from_slice(value);
    Ok(())
}

pub(crate) fn rewrite_comments(bytes: &[u8], repairs: &[FieldRepair], kind: CommentKind, limit: usize, flag: &AtomicBool) -> Result<Vec<u8>> {
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
        if name.is_empty() || !name.bytes().all(|b| (0x20..=if matches!(kind, CommentKind::Flac) { 0x7e } else { 0x7d }).contains(&b) && b != b'=') {
            return Err(TagWriteError::Unsupported("invalid comment field name"));
        }
        comments.push((raw, name, value));
    }
    let suffix = &bytes[at..];
    match kind {
        CommentKind::Flac if !suffix.is_empty() => return Err(TagWriteError::Unsupported("trailing FLAC comment bytes")),
        CommentKind::Vorbis if suffix.first().is_none_or(|b| b & 1 == 0) => return Err(TagWriteError::Unsupported("missing Vorbis comment framing bit")),
        _ => {}
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
    put_vector(&mut out, vendor, limit)?;
    if out.len() > limit - 4 {
        return Err(TagWriteError::Unsupported("rewritten comment count exceeds block limit"));
    }
    out.extend((output_count as u32).to_le_bytes());
    for raw in preserved {
        cancelled(flag)?;
        put_vector(&mut out, raw, limit)?;
    }
    // music-tag FlacFile/OggFile + Mutagen VCommentDict assignment removes all matching
    // pairs, then appends one lowercase key. Released field order is observable.
    for repair in repairs {
        cancelled(flag)?;
        let value = format!("{}={}", key(repair.field)?, repair.replacement);
        put_vector(&mut out, value.as_bytes(), limit)?;
    }
    if out.len().checked_add(suffix.len()).is_none_or(|n| n > limit) {
        return Err(TagWriteError::Unsupported("rewritten comment exceeds budget"));
    }
    // Vorbis framing/padding and Opus binary trailer are opaque, retained bytes.
    out.extend_from_slice(suffix);
    Ok(out)
}
