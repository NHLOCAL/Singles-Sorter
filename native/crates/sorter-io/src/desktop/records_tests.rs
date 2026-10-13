use super::*;
fn record(name: &[u16], next: u32) -> Vec<u8> {
    let len = (88 + name.len() * 2).next_multiple_of(8);
    let mut data = vec![0; len];
    data[0..4].copy_from_slice(&next.to_le_bytes());
    data[56..60].copy_from_slice(&0x20u32.to_le_bytes());
    data[72..88].copy_from_slice(&[7; 16]);
    data[60..64].copy_from_slice(&(name.len() as u32 * 2).to_le_bytes());
    for (at, unit) in name.iter().enumerate() {
        data[88 + at * 2..90 + at * 2].copy_from_slice(&unit.to_le_bytes());
    }
    data
}
#[test]
fn directory_records_preserve_order_and_lossless_names() {
    let first: Vec<u16> = "שיר𐐀.mp3".encode_utf16().collect();
    let first_len = record(&first, 0).len();
    let mut bytes = record(&first, first_len as u32);
    bytes.extend(record(&[0xd800, 65], 0));
    let result = parse_directory_records(&bytes).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, first);
    assert_eq!(result[1].name, vec![0xd800, 65]);
    assert_eq!(result[0].file_id, [7; 16]);
    assert_eq!(result[0].attributes, 0x20);
}
#[test]
fn directory_record_parser_rejects_truncated_overlapping_and_invalid_lengths() {
    assert!(parse_directory_records(&[0; 60]).is_err());
    for (field, value) in [
        (0, 8u32),
        (0, 113),
        (0, u32::MAX),
        (60, 3),
        (60, u32::MAX),
        (60, 0),
    ] {
        let mut bytes = record(&[65, 66], 0);
        bytes[field..field + 4].copy_from_slice(&value.to_le_bytes());
        assert!(
            parse_directory_records(&bytes).is_err(),
            "field={field} value={value}"
        );
    }
}
#[test]
fn single_component_validation_rejects_escapes_without_rewriting_names() {
    for text in ["", ".", "..", "a/b", "a\\b", "C:x", "song:ads", "a\0b"] {
        assert!(
            validate_child_units(&text.encode_utf16().collect::<Vec<_>>()).is_err(),
            "{text:?}"
        );
    }
    for name in ["שיר𐐀.mp3", "A.mp3", "a.mp3", "name. "] {
        assert!(validate_child_units(&name.encode_utf16().collect::<Vec<_>>()).is_ok());
    }
    assert!(validate_child_units(&[0xd800, 65]).is_ok());
}

#[test]
fn directory_records_skip_only_dot_entries_and_reject_embedded_paths() {
    let mut bytes = record(&[46], record(&[46], 0).len() as u32);
    bytes.extend(record(&[46, 46], record(&[46, 46], 0).len() as u32));
    bytes.extend(record(&[65], 0));
    let result = parse_directory_records(&bytes).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, vec![65]);
    assert!(parse_directory_records(&record(&[65, 92, 66], 0)).is_err());
    assert!(validate_child_units(&vec![65; 32_768]).is_err());
}
