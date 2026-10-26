//! Same-length UTF-8 APEv2 value patches after a complete WavPack block sequence.
use crate::{
    metadata::{RawFamily, RawValue, project},
    staged_tags::{StagedTagEvidence, TagWriteError},
};
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
type Result<T> = std::result::Result<T, TagWriteError>;
fn bad(reason: &'static str) -> TagWriteError {
    TagWriteError::Unsupported(reason)
}
fn check(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) {
        Err(TagWriteError::Cancelled)
    } else {
        Ok(())
    }
}
fn read(input: &mut impl Read, bytes: &mut [u8], flag: &AtomicBool) -> Result<()> {
    for chunk in bytes.chunks_mut(65536) {
        check(flag)?;
        let result = input.read_exact(chunk);
        check(flag)?;
        result.map_err(|_| TagWriteError::Io)?;
    }
    Ok(())
}
fn at(
    input: &mut (impl Read + Seek),
    offset: u64,
    bytes: &mut [u8],
    flag: &AtomicBool,
) -> Result<()> {
    check(flag)?;
    let result = input.seek(SeekFrom::Start(offset));
    check(flag)?;
    result.map_err(|_| TagWriteError::Io)?;
    read(input, bytes, flag)
}
fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}
fn key(field: LegacyField) -> Result<&'static str> {
    Ok(match field {
        LegacyField::Artist => "Artist",
        LegacyField::AlbumArtist => "Album Artist",
        LegacyField::Title => "Title",
        LegacyField::Album => "Album",
        LegacyField::Genre => "Genre",
        _ => return Err(bad("APE repair field")),
    })
}
#[derive(Clone, Debug)]
pub struct WavPackStagedPlan {
    length: u64,
    offset: u64,
    original: Vec<u8>,
    replacement: Vec<u8>,
    fields: Vec<LegacyField>,
}
impl WavPackStagedPlan {
    pub fn inspect(
        input: &mut (impl Read + Seek),
        length: u64,
        repairs: &[FieldRepair],
        flag: &AtomicBool,
    ) -> Result<Self> {
        if length < 64 {
            return Err(bad("WavPack and APE footer required"));
        }
        let mut footer = [0u8; 32];
        at(input, length - 32, &mut footer, flag)?;
        if &footer[..8] != b"APETAGEX"
            || u32le(&footer[8..12]) != 2000
            || footer[24..].iter().any(|b| *b != 0)
        {
            return Err(bad("ordinary terminal APEv2 footer required"));
        }
        let size = u64::from(u32le(&footer[12..16]));
        let count = u32le(&footer[16..20]);
        let flags = u32le(&footer[20..24]);
        if !matches!(flags, 0 | 0x80000000)
            || !(32..=16 * 1024 * 1024).contains(&size)
            || size > length
            || count > 4096
        {
            return Err(bad("APE tag budget or flags"));
        }
        let has_header = flags == 0x80000000;
        let start = length - size;
        let offset = if has_header {
            start.checked_sub(32).ok_or(bad("APE header extent"))?
        } else {
            start
        };
        if offset < 32 {
            return Err(bad("WavPack media required"));
        }
        let mut original = vec![0; (length - offset) as usize];
        at(input, offset, &mut original, flag)?;
        if has_header {
            let mut expected = footer;
            expected[20..24].copy_from_slice(&0xa0000000u32.to_le_bytes());
            if original[..32] != expected {
                return Err(bad("APE header and footer mismatch"));
            }
        }
        // Blocks contain their encoded payload inline. The tag cannot overlap any
        // block or be reached through an external sample/chunk pointer.
        let mut position = 0u64;
        let mut blocks = 0usize;
        while position < offset {
            check(flag)?;
            blocks += 1;
            if blocks > 1_000_000 || offset - position < 32 {
                return Err(bad("WavPack block budget or truncated header"));
            }
            let mut header = [0; 32];
            at(input, position, &mut header, flag)?;
            let size = u64::from(u32le(&header[4..8])) + 8;
            let version = u16::from_le_bytes(header[8..10].try_into().unwrap());
            if &header[..4] != b"wvpk"
                || !(0x402..=0x410).contains(&version)
                || size < 32
                || size > offset - position
            {
                return Err(bad("WavPack block extent or version"));
            }
            position += size;
        }
        let mut replacement = original.clone();
        let mut entries = Vec::new();
        let mut seen = HashSet::new();
        let mut cursor = if has_header { 32 } else { 0 };
        let end = original.len() - 32;
        for _ in 0..count {
            check(flag)?;
            if end - cursor < 8 {
                return Err(bad("truncated APE item"));
            }
            let n = u32le(&original[cursor..cursor + 4]) as usize;
            let flags = u32le(&original[cursor + 4..cursor + 8]);
            cursor += 8;
            if flags & !7 != 0 || (flags >> 1) & 3 == 3 {
                return Err(bad("APE item flags"));
            }
            let name_end = original[cursor..end]
                .iter()
                .position(|b| *b == 0)
                .filter(|n| (2..=255).contains(n))
                .ok_or(bad("APE key length or terminator"))?;
            let bytes = &original[cursor..cursor + name_end];
            if !bytes.iter().all(|b| (0x20..=0x7e).contains(b)) {
                return Err(bad("APE key encoding"));
            }
            let name = std::str::from_utf8(bytes).map_err(|_| bad("APE key"))?;
            if matches!(name, "OggS" | "TAG" | "ID3" | "MP+")
                || !seen.insert(name.to_ascii_lowercase())
            {
                return Err(bad("duplicate or reserved APE key"));
            }
            cursor += name_end + 1;
            let value_end = cursor
                .checked_add(n)
                .filter(|n| *n <= end)
                .ok_or(bad("APE value extent"))?;
            entries.push((name.to_string(), flags, cursor, value_end));
            cursor = value_end;
        }
        if cursor != end {
            return Err(bad("APE item count or trailing bytes"));
        }
        let mut previous = None;
        for repair in repairs {
            check(flag)?;
            let order = FIX_FIELDS
                .iter()
                .position(|field| *field == repair.field)
                .ok_or(bad("APE field"))?;
            if previous.is_some_and(|n| n >= order)
                || repair.original == repair.replacement
                || sorter_core::fix_jibrish(&repair.original, sorter_core::RepairMode::Hebrew)
                    != repair.replacement
                || repair.original.len() != repair.replacement.len()
            {
                return Err(bad("APE exact-width original repair policy"));
            }
            previous = Some(order);
            let expected = key(repair.field)?;
            let (_, flags, a, b) = entries
                .iter()
                .find(|(name, _, _, _)| name.eq_ignore_ascii_case(expected))
                .ok_or(TagWriteError::Stale)?;
            if *flags != 0 {
                return Err(bad("APE repaired field must be writable UTF-8 text"));
            }
            let text = std::str::from_utf8(&original[*a..*b]).map_err(|_| bad("APE text UTF-8"))?;
            if text.split('\0').count() > 4096 {
                return Err(bad("APE value count"));
            }
            let raw = text
                .split('\0')
                .map(|value| RawValue::Text(value.into()))
                .collect();
            if project(RawFamily::Ape, repair.field, raw)
                .map_err(|_| bad("APE projection"))?
                .value
                != Ok(LegacyScalar::Text(repair.original.clone()))
            {
                return Err(TagWriteError::Stale);
            }
            replacement[*a..*b].copy_from_slice(repair.replacement.as_bytes());
        }
        Ok(Self {
            length,
            offset,
            original,
            replacement,
            fields: repairs.iter().map(|r| r.field).collect(),
        })
    }
    pub fn changes_bytes(&self) -> bool {
        self.original != self.replacement
    }
    pub fn staged_length(&self) -> u64 {
        self.length
    }
    pub fn write_staged(
        &self,
        input: &mut (impl Read + Seek),
        output: &mut impl Write,
        expected: &str,
        flag: &AtomicBool,
    ) -> Result<StagedTagEvidence> {
        check(flag)?;
        let result = input.seek(SeekFrom::Start(0));
        check(flag)?;
        result.map_err(|_| TagWriteError::Io)?;
        let mut source = Sha256::new();
        let mut staged = Sha256::new();
        let mut media = Sha256::new();
        let mut buffer = [0; 65536];
        let mut position = 0u64;
        while position < self.length {
            let n = (self.length - position).min(buffer.len() as u64) as usize;
            read(input, &mut buffer[..n], flag)?;
            let bytes = &mut buffer[..n];
            source.update(&*bytes);
            let media_end = (position + n as u64).min(self.offset);
            if position < media_end {
                media.update(&bytes[..(media_end - position) as usize]);
            }
            let left = position.max(self.offset);
            let right = position + n as u64;
            if left < right {
                let a = (left - position) as usize;
                let x = (left - self.offset) as usize;
                let y = (right - self.offset) as usize;
                if bytes[a..] != self.original[x..y] {
                    return Err(TagWriteError::Stale);
                }
                bytes[a..].copy_from_slice(&self.replacement[x..y]);
            }
            staged.update(&*bytes);
            check(flag)?;
            let result = output.write_all(bytes);
            check(flag)?;
            result.map_err(|_| TagWriteError::Io)?;
            position += n as u64;
        }
        let mut extra = [0];
        check(flag)?;
        let result = input.read(&mut extra);
        check(flag)?;
        if result.map_err(|_| TagWriteError::Io)? != 0 {
            return Err(TagWriteError::Stale);
        }
        let source_sha256 = format!("{:x}", source.finalize());
        if source_sha256 != expected {
            return Err(TagWriteError::Stale);
        }
        Ok(StagedTagEvidence {
            source_sha256,
            staged_sha256: format!("{:x}", staged.finalize()),
            media_sha256: format!("{:x}", media.finalize()),
            media_length: self.offset,
            staged_length: self.length,
            changed_fields: self.fields.clone(),
        })
    }
}
