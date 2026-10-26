use super::*;
/// Exact FILE_ID_EXTD_DIR_INFO fields; offsets checked against SDK bindings on Windows.
#[derive(Debug)]
pub(super) struct DirectoryRecord {
    pub name: Vec<u16>,
    pub attributes: u32,
    pub file_id: [u8; 16],
}
pub(super) fn validate_child_units(name: &[u16]) -> Result<(), StorageError> {
    if name.is_empty()
        || name == [46]
        || name == [46, 46]
        || name.iter().any(|u| matches!(*u, 0 | 47 | 92 | 58))
        || name.len() > (u16::MAX as usize / 2)
    {
        return Err(error(
            StorageErrorKind::InvalidReference,
            "expected one lossless child name without separators, dot traversal, NUL or alternate streams",
        ));
    }
    Ok(())
}
pub(super) fn parse_directory_records(bytes: &[u8]) -> Result<Vec<DirectoryRecord>, StorageError> {
    let malformed = || {
        error(
            StorageErrorKind::Io,
            "malformed Windows directory record buffer",
        )
    };
    let mut offset = 0usize;
    let mut result = vec![];
    loop {
        let header = bytes
            .get(offset..offset.checked_add(88).ok_or_else(malformed)?)
            .ok_or_else(malformed)?;
        let u32_at =
            |at| u32::from_le_bytes(header[at..at + 4].try_into().expect("fixed header field"));
        let next = u32_at(0) as usize;
        let name_len = u32_at(60) as usize;
        if name_len == 0 || !name_len.is_multiple_of(2) {
            return Err(malformed());
        }
        let end = offset
            .checked_add(88)
            .and_then(|v| v.checked_add(name_len))
            .ok_or_else(malformed)?;
        let name = bytes
            .get(offset + 88..end)
            .ok_or_else(malformed)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect::<Vec<_>>();
        if name != [46] && name != [46, 46] {
            validate_child_units(&name)?;
            result.push(DirectoryRecord {
                name,
                attributes: u32_at(56),
                file_id: header[72..88].try_into().expect("fixed ID field"),
            });
        }
        if next == 0 {
            break;
        }
        if !next.is_multiple_of(8) || next < 88 + name_len {
            return Err(malformed());
        }
        offset = offset.checked_add(next).ok_or_else(malformed)?;
        if offset >= bytes.len() {
            return Err(malformed());
        }
    }
    Ok(result)
}
