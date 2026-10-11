use sha2::{Digest, Sha256};
use sorter_core::{RepairMode, fix_jibrish, fix_plan::FieldRepair, metadata::LegacyField};
use sorter_io::staged_tags::{Id3StagedPlan, TagWriteError};
use std::{
    io::Cursor,
    sync::atomic::{AtomicBool, Ordering},
};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn sync(size: usize) -> [u8; 4] {
    [
        ((size >> 21) & 127) as u8,
        ((size >> 14) & 127) as u8,
        ((size >> 7) & 127) as u8,
        (size & 127) as u8,
    ]
}
fn frame(id: &[u8; 4], payload: &[u8], version: u8) -> Vec<u8> {
    let mut b = id.to_vec();
    b.extend(if version == 4 {
        sync(payload.len())
    } else {
        (payload.len() as u32).to_be_bytes()
    });
    b.extend([0, 0]);
    b.extend(payload);
    b
}
fn media() -> Vec<u8> {
    let original = include_bytes!("../../../tests/fixtures/media/tagged.mp3");
    let size = original[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    original[10 + size..].to_vec()
}
fn fixture(version: u8, payload: &[u8]) -> Vec<u8> {
    let mut body = frame(b"TPE1", payload, version);
    body.extend(frame(
        b"APIC",
        b"\x03image/png\0\x03\0EXACT-ARTWORK",
        version,
    ));
    body.extend(frame(b"XABC", b"opaque unknown bytes\xff\0", version));
    body.extend([0; 37]);
    let mut b = vec![b'I', b'D', b'3', version, 0, 0];
    b.extend(sync(body.len()));
    b.extend(body);
    b.extend(media());
    b
}
fn repair(original: &str) -> FieldRepair {
    FieldRepair {
        field: LegacyField::Artist,
        original: original.into(),
        replacement: fix_jibrish(original, RepairMode::Hebrew),
    }
}
#[test]
fn repair_preserves_audio_artwork_unknown_frames_and_padding() {
    for version in [3, 4] {
        let original = "àáøäí";
        let mut payload = vec![0];
        payload.extend(original.chars().map(|c| c as u8));
        let before = fixture(version, &payload);
        let request = repair(original);
        assert_ne!(request.original, request.replacement);
        let plan = Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[request],
            &AtomicBool::new(false),
        )
        .unwrap();
        let mut after = vec![];
        let evidence = plan
            .write_staged(
                &mut Cursor::new(&before),
                &mut after,
                &hash(&before),
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(evidence.media_sha256, hash(&media()));
        assert_eq!(evidence.staged_sha256, hash(&after));
        assert_eq!(&after[after.len() - media().len()..], media());
        for untouched in [
            frame(b"APIC", b"\x03image/png\0\x03\0EXACT-ARTWORK", version),
            frame(b"XABC", b"opaque unknown bytes\xff\0", version),
        ] {
            assert!(after.windows(untouched.len()).any(|w| w == untouched));
        }
        assert_eq!(after[3], version);
        assert_eq!(after.len(), before.len());
    }
}
#[test]
fn supported_utf16_endianness_and_multivalue_projection_match_legacy() {
    for (encoding, big) in [(1, false), (1, true), (2, true)] {
        let mut payload = vec![encoding];
        if encoding == 1 {
            payload.extend(if big { [0xfe, 0xff] } else { [0xff, 0xfe] });
        }
        for unit in "à\0á".encode_utf16() {
            payload.extend(if big {
                unit.to_be_bytes()
            } else {
                unit.to_le_bytes()
            });
        }
        let before = fixture(4, &payload);
        let request = repair("à, á");
        let plan = Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[request],
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(plan.changes_bytes());
        let mut after = vec![];
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut after,
            &hash(&before),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(after[20], encoding);
        if encoding == 1 {
            assert_eq!(
                &after[21..23],
                if big { &[0xfe, 0xff] } else { &[0xff, 0xfe] }
            );
        }
    }
}
#[test]
fn unchanged_tags_are_byte_preserving_noop() {
    let before = fixture(4, b"\x03unchanged");
    let plan = Id3StagedPlan::inspect(
        &mut Cursor::new(&before),
        before.len() as u64,
        &[],
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(!plan.changes_bytes());
    let mut after = vec![];
    let evidence = plan
        .write_staged(
            &mut Cursor::new(&before),
            &mut after,
            &hash(&before),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(after, before);
    assert!(evidence.changed_fields.is_empty());
}
#[test]
fn all_unsupported_header_and_frame_flags_reject_before_output() {
    let before = fixture(4, b"\0\xe0");
    for (index, value) in [
        (3, 2),
        (4, 1),
        (5, 0x80),
        (5, 0x40),
        (5, 0x10),
        (18, 1),
        (19, 1),
        (6, 0x80),
    ] {
        let mut bad = before.clone();
        bad[index] = value;
        assert!(matches!(
            Id3StagedPlan::inspect(
                &mut Cursor::new(&bad),
                bad.len() as u64,
                &[repair("à")],
                &AtomicBool::new(false)
            ),
            Err(TagWriteError::Unsupported(_))
        ));
    }
}
#[test]
fn stale_values_unknown_fields_and_nonpolicy_replacement_are_rejected() {
    let before = fixture(4, b"\0\xe0");
    assert!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[repair("á")],
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut wrong = repair("à");
    wrong.replacement = "arbitrary edit".into();
    assert!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[wrong],
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut wrong = repair("à");
    wrong.field = LegacyField::TrackNumber;
    assert!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[wrong],
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
#[test]
fn source_digest_and_cancellation_gate_staged_results() {
    let before = fixture(4, b"\0\xe0");
    let flag = AtomicBool::new(false);
    let plan = Id3StagedPlan::inspect(
        &mut Cursor::new(&before),
        before.len() as u64,
        &[repair("à")],
        &flag,
    )
    .unwrap();
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut vec![],
            &hash(b"other"),
            &flag
        ),
        Err(TagWriteError::Stale)
    );
    flag.store(true, Ordering::Release);
    let mut output = vec![];
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut output,
            &hash(&before),
            &flag
        ),
        Err(TagWriteError::Cancelled)
    );
    assert!(output.is_empty());
}
#[test]
fn legacy_id3v1_mirror_and_nonzero_padding_are_not_silently_rewritten() {
    let before = fixture(4, b"\0\xe0");
    let mut legacy = before.clone();
    legacy.extend(b"TAG");
    legacy.extend([0; 125]);
    assert!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&legacy),
            legacy.len() as u64,
            &[repair("à")],
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let mut bad = before.clone();
    let size = bad[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    bad[10 + size - 1] = 1;
    assert!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&bad),
            bad.len() as u64,
            &[repair("à")],
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
#[cfg(native_metadata_bridge)]
#[test]
fn real_native_reader_observes_repaired_field_and_preserved_other_fields() {
    use sorter_io::{
        metadata::{LegacyScalar, MetadataReader},
        storage::ReadInput,
    };
    let before = fixture(4, b"\0\xe0\xe1\xf8\xe4\xed");
    let request = repair("àáøäí");
    let plan = Id3StagedPlan::inspect(
        &mut Cursor::new(&before),
        before.len() as u64,
        &[request.clone()],
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut after = vec![];
    plan.write_staged(
        &mut Cursor::new(&before),
        &mut after,
        &hash(&before),
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut reader = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(after.clone()), after.len() as u64),
            "fixture.mp3",
        )
        .unwrap();
    assert_eq!(
        reader.get(LegacyField::Artist).unwrap().value.unwrap(),
        LegacyScalar::Text(request.replacement)
    );
    assert_eq!(
        reader.get(LegacyField::Title).unwrap().value.unwrap(),
        LegacyScalar::Text(String::new())
    );
}

#[test]
fn insufficient_padding_rejects_instead_of_moving_media_offsets() {
    let mut before = fixture(3, b"\0\xe0");
    let size = before[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    before.drain(10 + size - 37..10 + size);
    before[6..10].copy_from_slice(&sync(size - 37));
    assert!(matches!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[repair("à")],
            &AtomicBool::new(false)
        ),
        Err(TagWriteError::Unsupported(_))
    ));
}

#[test]
fn ape_mirror_is_rejected_without_changing_either_tag() {
    let mut before = fixture(4, b"\0\xe0");
    before.extend(b"APETAGEX");
    before.extend([0; 24]);
    assert!(matches!(
        Id3StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &[repair("à")],
            &AtomicBool::new(false)
        ),
        Err(TagWriteError::Unsupported(_))
    ));
}
