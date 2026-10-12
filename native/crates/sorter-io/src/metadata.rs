//! Read-only metadata observations. No writes and no v1 protocol coercion.
//! Linux bridge validation is fixture-scoped; Windows/Android native acceptance is pending.
use crate::storage::{
    EntryKind, EntryObservation, ReadInput, ReadStorage, StorageError, StorageErrorKind,
};
use std::io::{Read, Seek, SeekFrom};
#[path = "decimal15.rs"]
mod decimal15;

pub use sorter_core::metadata::{
    ErrorStage, FieldError, LegacyField, LegacyItem, LegacyScalar, MetadataError,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawFamily {
    Id3,
    Xiph,
    Ape,
    Mp4,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RawValue {
    Text(String),
    Integer(String),
    Null,
}

fn python_integer(text: &str) -> Result<String, FieldError> {
    let error = || FieldError::get("ValueError", format!("invalid base-10 integer: {text:?}"));
    // CPython int() does not accept the ASCII information separators, although
    // str.isspace() does. Use the pinned Unicode-15 predicate otherwise.
    let trimmed = text
        .trim_matches(|c| sorter_core::unicode15::is_space(c) && !matches!(c, '\u{1c}'..='\u{1f}'));
    let (negative, body) = if let Some(s) = trimmed.strip_prefix('-') {
        (true, s)
    } else {
        (false, trimmed.strip_prefix('+').unwrap_or(trimmed))
    };
    let mut digits = String::new();
    let mut previous_digit = false;
    for c in body.chars() {
        if c == '_' && previous_digit {
            previous_digit = false;
            continue;
        }
        let Some(zero) = decimal15::ZEROES
            .iter()
            .find(|&&z| (z..z + 10).contains(&(c as u32)))
        else {
            return Err(error());
        };
        digits.push(char::from(b'0' + (c as u32 - zero) as u8));
        previous_digit = true;
    }
    if !previous_digit {
        return Err(error());
    }
    // Reference CPython defaults to 4300 significant conversion digits, including
    // leading zeroes. Underscores/sign/whitespace do not count.
    if digits.len() > 4300 {
        return Err(FieldError::get(
            "ValueError",
            "Exceeds the limit (4300 digits) for integer string conversion",
        ));
    }
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        Ok("0".into())
    } else {
        Ok(format!("{}{normalized}", if negative { "-" } else { "" }))
    }
}
fn scalar_text(v: &RawValue) -> String {
    match v {
        RawValue::Text(s) | RawValue::Integer(s) => s.clone(),
        RawValue::Null => "None".into(),
    }
}
/// Music-tag projection stays separate from native parsing. Get and value errors
/// are distinct, so callers preserve sequential policy exception boundaries.
pub fn project(
    family: RawFamily,
    field: LegacyField,
    mut raw: Vec<RawValue>,
) -> Result<LegacyItem, FieldError> {
    let numeric = field == LegacyField::TrackNumber;
    if !numeric && family == RawFamily::Ape && !raw.is_empty() {
        raw = vec![RawValue::Text(
            raw.iter().map(scalar_text).collect::<Vec<_>>().join("\0"),
        )];
    }
    if numeric && matches!(family, RawFamily::Id3 | RawFamily::Ape) && !raw.is_empty() {
        let joined = raw.iter().map(scalar_text).collect::<Vec<_>>().join("\0");
        let part = joined.split('/').next().unwrap_or("");
        raw = if part == "None" {
            vec![]
        } else {
            vec![RawValue::Text(part.into())]
        };
    }
    let values = raw
        .into_iter()
        .map(|v| match v {
            RawValue::Null => Ok(LegacyScalar::Null),
            RawValue::Text(s) | RawValue::Integer(s) if numeric => {
                python_integer(&s).map(LegacyScalar::Integer)
            }
            other => Ok(LegacyScalar::Text(scalar_text(&other))),
        })
        .collect::<Result<Vec<_>, FieldError>>()?;
    let truthy = values.iter().any(|v| match v {
        LegacyScalar::Text(s) => !s.is_empty(),
        LegacyScalar::Integer(s) => s != "0",
        LegacyScalar::Null => false,
    });
    let value = if numeric {
        match values.as_slice() {
            [] => Ok(LegacyScalar::Integer("0".into())),
            [LegacyScalar::Null] => Ok(LegacyScalar::Null),
            [v] => Ok(v.clone()),
            _ => Err(FieldError {
                stage: ErrorStage::Value,
                kind: "ValueError".into(),
                detail: "Metadata must have 1 value to cast to int".into(),
            }),
        }
    } else {
        Ok(LegacyScalar::Text(
            values
                .iter()
                .map(|v| match v {
                    LegacyScalar::Text(s) | LegacyScalar::Integer(s) => s.as_str(),
                    LegacyScalar::Null => "None",
                })
                .collect::<Vec<_>>()
                .join(", "),
        ))
    };
    Ok(LegacyItem {
        values,
        truthy,
        value,
    })
}

pub(crate) fn id3_text(payload: &[u8]) -> Result<Vec<RawValue>, FieldError> {
    let invalid = || FieldError::get("UnsupportedClass", "invalid or unproven ID3 text encoding");
    let (&encoding, data) = payload.split_first().ok_or_else(invalid)?;
    if encoding > 3 {
        return Err(invalid());
    }
    let mut strings = Vec::new();
    if encoding == 0 || encoding == 3 {
        let body = data.strip_suffix(&[0]).unwrap_or(data);
        for part in body.split(|&b| b == 0) {
            if strings.len() >= 4096 {
                return Err(invalid());
            }
            strings.push(if encoding == 0 {
                part.iter().map(|&b| char::from(b)).collect()
            } else {
                std::str::from_utf8(part).map_err(|_| invalid())?.to_owned()
            });
        }
    } else {
        if data.len() % 2 != 0 {
            return Err(invalid());
        }
        let mut start = 0;
        let mut little = encoding == 1;
        while start < data.len() {
            if strings.len() >= 4096 {
                return Err(invalid());
            }
            let mut end = start;
            while end + 1 < data.len() && (data[end] != 0 || data[end + 1] != 0) {
                end += 2;
            }
            let mut part = &data[start..end];
            if part.starts_with(&[255, 254]) {
                little = true;
                part = &part[2..];
            } else if part.starts_with(&[254, 255]) {
                little = false;
                part = &part[2..];
            } else if start == 0 && encoding == 1 && !part.is_empty() {
                return Err(invalid());
            }
            let units = part
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| {
                    if little {
                        u16::from_le_bytes([c[0], c[1]])
                    } else {
                        u16::from_be_bytes([c[0], c[1]])
                    }
                })
                .collect::<Vec<_>>();
            strings.push(String::from_utf16(&units).map_err(|_| invalid())?);
            start = end + 2;
        }
    }
    if strings.len() > 4096 {
        return Err(invalid());
    }
    Ok(strings.into_iter().map(RawValue::Text).collect())
}

/// Strict decoder for the owned ABI's little-endian length-prefixed raw values.
/// Never trusts native sizes, discriminants, UTF-8, or trailing bytes.
pub fn decode_raw(bytes: &[u8]) -> Result<(RawFamily, Vec<RawValue>), FieldError> {
    let invalid = || FieldError::get("InvalidBridgeResult", "malformed native result");
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut at = 0usize;
    fn word(bytes: &[u8], at: &mut usize) -> Option<u32> {
        let end = at.checked_add(4)?;
        let v = u32::from_le_bytes(bytes.get(*at..end)?.try_into().ok()?);
        *at = end;
        Some(v)
    }
    let family = match word(bytes, &mut at) {
        Some(1) => RawFamily::Id3,
        Some(2) => RawFamily::Xiph,
        Some(3) => RawFamily::Ape,
        Some(4) => RawFamily::Mp4,
        _ => return Err(invalid()),
    };
    let count = word(bytes, &mut at).ok_or_else(invalid)? as usize;
    if count > 4096 {
        return Err(invalid());
    }
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let kind = word(bytes, &mut at).ok_or_else(invalid)?;
        let len = word(bytes, &mut at).ok_or_else(invalid)? as usize;
        let end = at.checked_add(len).ok_or_else(invalid)?;
        let payload = bytes.get(at..end).ok_or_else(invalid)?;
        at = end;
        if kind == 10 && family == RawFamily::Id3 {
            values.extend(id3_text(payload)?);
            if values.len() > 4096 {
                return Err(invalid());
            }
            continue;
        }
        let s = std::str::from_utf8(payload)
            .map_err(|_| invalid())?
            .to_owned();
        values.push(match kind {
            1 => RawValue::Text(s),
            2 => RawValue::Integer(s),
            3 if s.is_empty() => RawValue::Null,
            _ => return Err(invalid()),
        });
    }
    if at != bytes.len() {
        return Err(invalid());
    }
    Ok((family, values))
}

#[derive(Default)]
pub struct MetadataReader;
pub trait MetadataProvider {
    fn open(&mut self, entry: &EntryObservation) -> Result<MetadataHandle, MetadataError>;
}
pub struct StorageMetadataProvider<'a> {
    pub storage: &'a mut dyn ReadStorage,
    pub reader: MetadataReader,
}
impl MetadataProvider for StorageMetadataProvider<'_> {
    fn open(&mut self, e: &EntryObservation) -> Result<MetadataHandle, MetadataError> {
        if e.kind != EntryKind::File {
            return Err(MetadataError::Storage(StorageError::new(
                StorageErrorKind::NotFile,
                "metadata observation is not a regular file",
            )));
        }
        let input = self
            .storage
            .open_read(&e.reference)
            .map_err(MetadataError::Storage)?;
        let unverified = || {
            MetadataError::Storage(StorageError::new(
                StorageErrorKind::UnsupportedCapability,
                "metadata requires verified read-handle identity, size and modification evidence",
            ))
        };
        let binding = input.binding().ok_or_else(unverified)?;
        let (Some(identity), Some(size), Some(modified)) = (&e.identity, e.size, &e.modified)
        else {
            return Err(unverified());
        };
        if identity != &binding.identity
            || size != binding.size
            || modified != &binding.modified
            || input.bound() != binding.size
        {
            return Err(MetadataError::Storage(StorageError::new(
                StorageErrorKind::Changed,
                "metadata source changed since the supplied observation",
            )));
        }
        // Validate before even the header probe; a separate stat-before-open
        // check would leave a replacement race and could mislabel new metadata.
        self.reader.open(input, &e.name)
    }
}
#[cfg(native_metadata_bridge)]
struct Context {
    input: ReadInput,
    error: Option<StorageError>,
}
pub struct MetadataHandle {
    #[cfg(native_metadata_bridge)]
    mp4_duplicates: crate::staged_mp4::DuplicatePreviewFields,
    #[cfg(native_metadata_bridge)]
    handle: std::ptr::NonNull<std::ffi::c_void>,
    #[cfg(native_metadata_bridge)]
    _context: Box<Context>,
}
impl MetadataReader {
    /// Whether this build linked the native bridge; not a platform acceptance claim.
    pub const fn is_available() -> bool {
        cfg!(native_metadata_bridge)
    }

    pub fn open(
        &self,
        mut input: ReadInput,
        _name_hint: &str,
    ) -> Result<MetadataHandle, MetadataError> {
        if !input.is_seekable() {
            return Err(MetadataError::SeekRequired);
        }
        let length = input.bound();
        if length > i64::MAX as u64 {
            return Err(MetadataError::InvalidInput);
        }
        // Detect the actual header, not the suffix. No new TagLib success is allowed
        // for the legacy ASF and ADTS AAC open-failure classes.
        let mut magic = [0u8; 16];
        let n = length.min(magic.len() as u64) as usize;
        input
            .read_exact(&mut magic[..n])
            .map_err(|e| MetadataError::Storage(e.into()))?;
        input
            .seek(SeekFrom::Start(0))
            .map_err(|e| MetadataError::Storage(e.into()))?;
        if n >= 8 && magic[..8] == [0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11] {
            return Err(MetadataError::LegacyOpen {
                kind: "NotImplementedError".into(),
                detail: "ASF format not implemented".into(),
            });
        }
        if n >= 2 && magic[0] == 255 && magic[1] & 0xf6 == 0xf0 {
            return Err(MetadataError::LegacyOpen {
                kind: "AACError".into(),
                detail: "doesn't support tags".into(),
            });
        }
        #[cfg(native_metadata_bridge)]
        {
            let mp4_duplicates = if n >= 8 && &magic[4..8] == b"ftyp" {
                let fields = crate::staged_mp4::duplicate_preview_fields(&mut input, length)
                    .map_err(|error| match error {
                        crate::staged_tags::TagWriteError::Io => MetadataError::InvalidInput,
                        _ => MetadataError::LegacyOpen {
                            kind: "UnsupportedMp4Projection".into(),
                            detail: "MP4 metadata exceeds the bounded raw duplicate-field representation".into(),
                        },
                    })?;
                input
                    .seek(SeekFrom::Start(0))
                    .map_err(|e| MetadataError::Storage(e.into()))?;
                fields
            } else {
                Vec::new()
            };
            let mut context = Box::new(Context { input, error: None });
            let mut handle = std::ptr::null_mut();
            let abi = ffi::Input {
                version: 1,
                struct_size: std::mem::size_of::<ffi::Input>() as u32,
                context: (&mut *context as *mut Context).cast(),
                read_at: ffi::read_at,
                length,
            };
            // SAFETY: the boxed context remains stable until after native handle drop.
            let status = unsafe { ffi::sm_open(&abi, &mut handle) };
            if status != 0 {
                return Err(context
                    .error
                    .take()
                    .map(MetadataError::Storage)
                    .unwrap_or_else(|| match status {
                        1 => MetadataError::InvalidInput,
                        2 => MetadataError::Parser,
                        3 => MetadataError::UnsupportedClass,
                        4 => MetadataError::InputBudget,
                        _ => MetadataError::NativeException,
                    }));
            }
            let handle = std::ptr::NonNull::new(handle).ok_or(MetadataError::InvalidInput)?;
            Ok(MetadataHandle {
                mp4_duplicates,
                handle,
                _context: context,
            })
        }
        #[cfg(not(native_metadata_bridge))]
        {
            Err(MetadataError::UnsupportedPlatform)
        }
    }
}
impl MetadataHandle {
    pub fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
        #[cfg(native_metadata_bridge)]
        {
            if let Some((_, values)) = self.mp4_duplicates.iter().find(|(key, _)| *key == field) {
                return project(RawFamily::Mp4, field, values.clone()?);
            }
            let mut result = ffi::ResultBuffer {
                data: std::ptr::null_mut(),
                len: 0,
            };
            // SAFETY: uniquely borrowed live handle, valid result out pointer.
            let status = unsafe { ffi::sm_get(self.handle.as_ptr(), field as u32, &mut result) };
            struct Owned(ffi::ResultBuffer);
            impl Drop for Owned {
                fn drop(&mut self) {
                    unsafe { ffi::sm_result_free(&mut self.0) }
                }
            }
            let owned = Owned(result);
            if status != 0 {
                return Err(FieldError::get(
                    if status == 3 {
                        "UnsupportedClass"
                    } else {
                        "NativeReadError"
                    },
                    format!("native field status {status}"),
                ));
            }
            if owned.0.data.is_null() || owned.0.len > 16 * 1024 * 1024 {
                return Err(FieldError::get(
                    "InvalidBridgeResult",
                    "invalid native allocation",
                ));
            }
            // SAFETY: C ABI owns this allocation until Owned::drop, length bounded above.
            let bytes = unsafe { std::slice::from_raw_parts(owned.0.data, owned.0.len) };
            let (family, raw) = decode_raw(bytes)?;
            project(family, field, raw)
        }
        #[cfg(not(native_metadata_bridge))]
        {
            let _ = field;
            Err(FieldError::get(
                "UnsupportedPlatform",
                "native bridge not built",
            ))
        }
    }
}
#[cfg(native_metadata_bridge)]
impl Drop for MetadataHandle {
    fn drop(&mut self) {
        unsafe { ffi::sm_close(self.handle.as_ptr()) }
    }
}
#[cfg(native_metadata_bridge)]
mod ffi {
    use super::*;
    use std::ffi::c_void;
    #[repr(C)]
    pub struct Input {
        pub version: u32,
        pub struct_size: u32,
        pub context: *mut c_void,
        pub read_at: unsafe extern "C" fn(*mut c_void, u64, *mut u8, usize, *mut usize) -> i32,
        pub length: u64,
    }
    #[repr(C)]
    pub struct ResultBuffer {
        pub data: *mut u8,
        pub len: usize,
    }
    unsafe extern "C" {
        pub fn sm_open(input: *const Input, out: *mut *mut c_void) -> i32;
        pub fn sm_get(handle: *mut c_void, field: u32, out: *mut ResultBuffer) -> i32;
        pub fn sm_result_free(result: *mut ResultBuffer);
        pub fn sm_close(handle: *mut c_void);
    }
    pub unsafe extern "C" fn read_at(
        context: *mut c_void,
        offset: u64,
        data: *mut u8,
        len: usize,
        got: *mut usize,
    ) -> i32 {
        if context.is_null() || data.is_null() || got.is_null() || len > 16 * 1024 * 1024 {
            return 1;
        }
        // No Rust panic may unwind through C++; handles are used on one thread only.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let context = unsafe { &mut *context.cast::<Context>() };
            unsafe { *got = 0 };
            let result = (|| {
                context.input.seek(SeekFrom::Start(offset))?;
                context
                    .input
                    .read_exact(unsafe { std::slice::from_raw_parts_mut(data, len) })
            })();
            match result {
                Ok(()) => {
                    unsafe { *got = len };
                    0
                }
                Err(e) => {
                    context.error = Some(e.into());
                    1
                }
            }
        }))
        .unwrap_or(1)
    }
}

impl sorter_core::metadata::MetadataFields for MetadataHandle {
    fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
        MetadataHandle::get(self, field)
    }
}
impl sorter_core::metadata::MetadataProvider for StorageMetadataProvider<'_> {
    fn open(
        &mut self,
        entry: &EntryObservation,
    ) -> Result<Box<dyn sorter_core::metadata::MetadataFields>, MetadataError> {
        MetadataProvider::open(self, entry)
            .map(|h| Box::new(h) as Box<dyn sorter_core::metadata::MetadataFields>)
    }
}
