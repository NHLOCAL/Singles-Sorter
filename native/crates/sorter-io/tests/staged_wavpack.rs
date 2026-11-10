use sha2::{Digest, Sha256};
use sorter_core::fix_plan::{FIX_FIELDS, FieldRepair};
use sorter_io::{staged_tags::TagWriteError, staged_wavpack::WavPackStagedPlan};
use std::{
    io::{self, Cursor, Write},
    sync::atomic::{AtomicBool, Ordering},
};
#[path = "../../../tests/staged_wavpack/fixture_support.rs"]
mod media;
use media::*;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn requests(multi: bool) -> Vec<FieldRepair> {
    FIX_FIELDS
        .iter()
        .map(|field| FieldRepair {
            field: *field,
            original: if multi { "à\0á" } else { "à" }.into(),
            replacement: if multi { "א\0ב" } else { "א" }.into(),
        })
        .collect()
}
fn inspect(bytes: &[u8], multi: bool) -> Result<WavPackStagedPlan, TagWriteError> {
    WavPackStagedPlan::inspect(
        &mut Cursor::new(bytes),
        bytes.len() as u64,
        &requests(multi),
        &AtomicBool::new(false),
    )
}
fn stage(bytes: &[u8], multi: bool) -> Vec<u8> {
    let plan = inspect(bytes, multi).unwrap();
    let mut after = vec![];
    let evidence = plan
        .write_staged(
            &mut Cursor::new(bytes),
            &mut after,
            &hash(bytes),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(bytes.len(), after.len());
    assert_eq!(evidence.media_sha256, hash(AUDIO));
    assert_eq!(evidence.media_length, AUDIO.len() as u64);
    assert_eq!(evidence.staged_sha256, hash(&after));
    assert_eq!(&after[..AUDIO.len()], AUDIO);
    after
}
#[test]
fn header_and_footer_only_preserve_unknown_and_binary_items() {
    for header in [false, true] {
        let before = fixture(header, false);
        let after = stage(&before, false);
        assert_eq!(&before[before.len() - 32..], &after[after.len() - 32..]);
        for untouched in [
            item(
                "Cover Art (Front)",
                2,
                b"cover.png\0exact artwork bytes\xff",
            ),
            item("Unknown Binary", 2, b"opaque\0\xff"),
        ] {
            assert!(after.windows(untouched.len()).any(|part| part == untouched));
        }
    }
}
#[test]
fn nul_separated_values_keep_original_scalar_and_length() {
    let before = fixture(true, true);
    let after = stage(&before, true);
    assert!(
        after
            .windows("א\0ב".len())
            .any(|part| part == "א\0ב".as_bytes())
    );
}
#[test]
fn duplicate_case_insensitive_and_reserved_keys_refuse() {
    for extra in [item("artist", 0, b"extra"), item("ID3", 0, b"extra")] {
        let mut items = item("Artist", 0, "à".as_bytes());
        items.extend(extra);
        assert!(inspect(&with_items(&items, 2, true), false).is_err());
    }
}
#[test]
fn binary_readonly_and_invalid_utf8_target_refuse() {
    for (flags, value) in [
        (2, "à".as_bytes()),
        (1, "à".as_bytes()),
        (0, b"\xff".as_slice()),
    ] {
        let before = with_items(&item("Artist", flags, value), 1, true);
        assert!(matches!(
            inspect(&before, false),
            Err(TagWriteError::Unsupported(_))
        ));
    }
}
#[test]
fn malformed_footer_header_and_item_count_refuse() {
    for edit in 0..3 {
        let mut before = fixture(true, false);
        let n = before.len();
        match edit {
            0 => before[n - 24] = 0,
            1 => before[AUDIO.len() + 20] = 1,
            _ => before[n - 16..n - 12].copy_from_slice(&999u32.to_le_bytes()),
        }
        assert!(inspect(&before, false).is_err());
    }
}
#[test]
fn block_extent_cannot_overlap_tags_or_leave_unparsed_bytes() {
    for size in [24u32, 100000] {
        let mut before = fixture(true, false);
        before[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(inspect(&before, false).is_err());
    }
}
#[test]
fn stale_hash_length_and_policy_never_return_evidence() {
    let before = fixture(false, false);
    let plan = inspect(&before, false).unwrap();
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut Vec::new(),
            &"0".repeat(64),
            &AtomicBool::new(false)
        ),
        Err(TagWriteError::Stale)
    );
    for changed in [
        before[..before.len() - 1].to_vec(),
        [before.clone(), vec![1]].concat(),
    ] {
        assert!(
            plan.write_staged(
                &mut Cursor::new(changed),
                &mut Vec::new(),
                &hash(&before),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    let mut requests = requests(false);
    requests[0].replacement = "x".into();
    assert!(
        WavPackStagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &requests,
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
#[test]
fn cancel_and_output_failure_preserve_no_success_evidence() {
    struct Fail;
    impl Write for Fail {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("owned"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let before = fixture(true, false);
    let plan = inspect(&before, false).unwrap();
    let flag = AtomicBool::new(true);
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut Vec::new(),
            &hash(&before),
            &flag
        ),
        Err(TagWriteError::Cancelled)
    );
    flag.store(false, Ordering::Release);
    assert_eq!(
        plan.write_staged(&mut Cursor::new(&before), &mut Fail, &hash(&before), &flag),
        Err(TagWriteError::Io)
    );
}
#[cfg(native_metadata_bridge)]
#[test]
fn actual_native_reader_matches_ape_nul_projection_and_five_fields() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    for multi in [false, true] {
        let before = fixture(true, multi);
        let after = stage(&before, multi);
        for (bytes, value) in [
            (before, if multi { "à\0á" } else { "à" }),
            (after, if multi { "א\0ב" } else { "א" }),
        ] {
            let n = bytes.len() as u64;
            let mut reader = MetadataReader
                .open(ReadInput::seekable(Cursor::new(bytes), n), "owned.wv")
                .unwrap();
            for field in FIX_FIELDS {
                assert_eq!(
                    reader.get(field).unwrap().value.unwrap(),
                    sorter_core::metadata::LegacyScalar::Text(value.into())
                );
            }
        }
    }
}
#[test]
fn actual_ffmpeg_decodes_repaired_wavpack_identically() {
    use std::process::Command;
    let before = fixture(true, true);
    let after = stage(&before, true);
    let tree = tempfile::tempdir().unwrap();
    let a = tree.path().join("before.wv");
    let b = tree.path().join("after.wv");
    std::fs::write(&a, before).unwrap();
    std::fs::write(&b, after).unwrap();
    let decode = |path: &std::path::Path| {
        let result = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(["-map", "0:a:0", "-f", "s16le", "-"])
            .output()
            .expect("FFmpeg required");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!result.stdout.is_empty());
        result.stdout
    };
    assert_eq!(decode(&a), decode(&b));
}
