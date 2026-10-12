use sha2::{Digest, Sha256};
use sorter_core::{
    RepairMode, fix_jibrish,
    fix_plan::{FIX_FIELDS, FieldRepair},
    metadata::LegacyField,
};
use sorter_io::{staged_mp4::Mp4StagedPlan, staged_tags::TagWriteError};
use std::{
    io::{self, Cursor, Write},
    sync::atomic::{AtomicBool, Ordering},
};
#[path = "../../../tests/staged_mp4/fixture_support.rs"]
mod fixture;
use fixture::*;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn repair(field: LegacyField, value: &str) -> FieldRepair {
    FieldRepair {
        field,
        original: value.into(),
        replacement: fix_jibrish(value, RepairMode::Hebrew),
    }
}
fn inspect(bytes: &[u8], repairs: &[FieldRepair]) -> Result<Mp4StagedPlan, TagWriteError> {
    Mp4StagedPlan::inspect(
        &mut Cursor::new(bytes),
        bytes.len() as u64,
        repairs,
        &AtomicBool::new(false),
    )
}
fn stage(bytes: &[u8], repairs: &[FieldRepair]) -> Vec<u8> {
    let plan = inspect(bytes, repairs).unwrap();
    let mut output = vec![];
    let evidence = plan
        .write_staged(
            &mut Cursor::new(bytes),
            &mut output,
            &hash(bytes),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(output.len(), bytes.len());
    assert_eq!(plan.staged_length(), bytes.len() as u64);
    assert_eq!(evidence.staged_sha256, hash(&output));
    assert_eq!(evidence.source_sha256, hash(bytes));
    assert_eq!(evidence.media_sha256, hash(&media(bytes)));
    assert_eq!(media(bytes), media(&output));
    for key in [b"zzzz", b"covr"] {
        assert_eq!(metadata_item(bytes, key), metadata_item(&output, key));
    }
    let old = children(bytes);
    let new = children(&output);
    assert_eq!(old.len(), new.len());
    for ((kind, before), (new_kind, after)) in old.iter().zip(&new) {
        assert_eq!(kind, new_kind);
        assert_eq!(before.len(), after.len());
        if kind != b"moov" {
            assert_eq!(before, after);
        }
    }
    output
}
fn rich(alac: bool) -> Vec<u8> {
    fixture(
        alac,
        &[
            (b"\xa9ART", vec!["à"]),
            (b"aART", vec!["á"]),
            (b"\xa9nam", vec!["â"]),
            (b"\xa9alb", vec!["ã"]),
            (b"\xa9gen", vec!["ä"]),
        ],
    )
}
fn requests() -> Vec<FieldRepair> {
    FIX_FIELDS
        .iter()
        .zip(["à", "á", "â", "ã", "ä"])
        .map(|(field, value)| repair(*field, value))
        .collect()
}
#[test]
fn aac_five_fields_preserve_unknown_artwork_media_and_extent() {
    let before = rich(false);
    let after = stage(&before, &requests());
    assert_ne!(before, after);
}
#[test]
fn alac_five_fields_preserve_unknown_artwork_media_and_extent() {
    let before = rich(true);
    let after = stage(&before, &requests());
    assert_ne!(before, after);
}
#[test]
fn fast_start_and_chunk_offsets_are_unchanged() {
    let before = fast_start(&rich(false));
    let after = stage(&before, &requests());
    assert_eq!(children(&after)[1].0, *b"moov");
    let old = children(&before)[1].1;
    let new = children(&after)[1].1;
    let old_track = children(&old[8..])
        .into_iter()
        .find(|(kind, _)| kind == b"trak")
        .unwrap()
        .1;
    let new_track = children(&new[8..])
        .into_iter()
        .find(|(kind, _)| kind == b"trak")
        .unwrap()
        .1;
    assert_eq!(old_track, new_track);
}
#[test]
fn multi_value_text_collapses_into_valid_adjacent_padding() {
    let before = fixture(false, &[(b"\xa9ART", vec!["à", "á", "â"])]);
    let after = stage(&before, &[repair(LegacyField::Artist, "à, á, â")]);
    assert_eq!(
        metadata_item(&after, b"\xa9ART"),
        vec![text(b"\xa9ART", &["א, ב, ג"])]
    );
    assert!(ilst(&after).len() < ilst(&before).len());
}
#[test]
fn all_frozen_mapping_letters_preserve_utf8_width() {
    let original = "àáâãäåæçèéëìîðñòôö÷øùúêíïóõ";
    let replacement = fix_jibrish(original, RepairMode::Hebrew);
    assert_eq!(original.chars().count(), 27);
    for (a, b) in original.chars().zip(replacement.chars()) {
        assert_eq!(a.len_utf8(), 2);
        assert_eq!(b.len_utf8(), 2);
    }
    let before = fixture(false, &[(b"\xa9ART", vec![original])]);
    stage(&before, &[repair(LegacyField::Artist, original)]);
}
#[test]
fn absent_stale_or_noncanonical_assignment_refuses() {
    let duplicate = fixture(
        false,
        &[
            (b"\xa9ART", vec!["שלום"]),
            (b"\xa9ART", vec!["à"]),
            (b"\xa9nam", vec!["à"]),
        ],
    );
    assert!(inspect(&duplicate, &[repair(LegacyField::Title, "à")]).is_ok());
    let before = rich(false);
    assert!(matches!(
        inspect(&before, &[repair(LegacyField::Artist, "á")]),
        Err(TagWriteError::Stale)
    ));
    let mut r = repair(LegacyField::Artist, "à");
    r.replacement = "new text".into();
    assert!(matches!(
        inspect(&before, &[r]),
        Err(TagWriteError::Unsupported(_))
    ));
    assert!(
        inspect(
            &before,
            &[
                repair(LegacyField::Title, "â"),
                repair(LegacyField::Artist, "à")
            ]
        )
        .is_err()
    );
}
#[test]
fn unsupported_text_encoding_refuses_before_staging() {
    let mut before = rich(false);
    let at = before
        .windows(8)
        .position(|x| x == b"\0\0\0\x12data")
        .unwrap();
    before[at + 11] = 2;
    assert!(matches!(
        inspect(&before, &requests()),
        Err(TagWriteError::Unsupported(_))
    ));
}
#[test]
fn malformed_atom_extent_and_private_structures_refuse() {
    let mut grouped = rich(false);
    let group = grouped.windows(4).position(|x| x == b"sgpd").unwrap();
    grouped[group + 8..group + 12].copy_from_slice(b"seig");
    assert!(inspect(&grouped, &requests()).is_err());
    let base = rich(false);
    let mut short = 1u32.to_be_bytes().to_vec();
    short.extend(b"ftyp");
    short.extend(16u64.to_be_bytes());
    let first = children(&base)[0].1.len();
    short.extend_from_slice(&base[first..]);
    assert!(inspect(&short, &requests()).is_err());
    let before = rich(false);
    let mut bad = before.clone();
    bad[..4].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(inspect(&bad, &requests()).is_err());
    let mut bad = before;
    let at = bad.windows(4).position(|x| x == b"mvhd").unwrap();
    bad[at..at + 4].copy_from_slice(b"uuid");
    assert!(matches!(
        inspect(&bad, &requests()),
        Err(TagWriteError::Unsupported(_))
    ));
}
#[test]
fn no_op_is_byte_identical() {
    let before = rich(false);
    let plan = inspect(&before, &[]).unwrap();
    assert!(!plan.changes_bytes());
    assert_eq!(stage(&before, &[]), before);
}
#[test]
fn source_hash_growth_and_truncation_never_return_verified_output() {
    let before = rich(false);
    let plan = inspect(&before, &requests()).unwrap();
    for bytes in [
        before[..before.len() - 1].to_vec(),
        [before.clone(), vec![1]].concat(),
    ] {
        assert!(
            plan.write_staged(
                &mut Cursor::new(bytes),
                &mut vec![],
                &hash(&before),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut vec![],
            &"0".repeat(64),
            &AtomicBool::new(false)
        ),
        Err(TagWriteError::Stale)
    );
}
#[test]
fn cancellation_and_output_failure_are_explicit() {
    struct Stop<'a>(&'a AtomicBool);
    impl Write for Stop<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.store(true, Ordering::Release);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    struct Fail;
    impl Write for Fail {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("owned failure"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let before = rich(false);
    let flag = AtomicBool::new(true);
    assert_eq!(
        Mp4StagedPlan::inspect(
            &mut Cursor::new(&before),
            before.len() as u64,
            &requests(),
            &flag
        )
        .unwrap_err(),
        TagWriteError::Cancelled
    );
    flag.store(false, Ordering::Release);
    let plan = inspect(&before, &requests()).unwrap();
    assert_eq!(
        plan.write_staged(
            &mut Cursor::new(&before),
            &mut Stop(&flag),
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
fn actual_native_reader_matches_five_assignments_for_aac_and_alac() {
    use sorter_io::{
        metadata::{LegacyScalar, MetadataReader},
        storage::ReadInput,
    };
    for alac in [false, true] {
        let before = rich(alac);
        let after = stage(&before, &requests());
        let length = after.len() as u64;
        let mut reader = MetadataReader
            .open(
                ReadInput::seekable(Cursor::new(after), length),
                if alac { "owned.alac" } else { "owned.m4a" },
            )
            .unwrap();
        for request in requests() {
            assert_eq!(
                reader.get(request.field).unwrap().value.unwrap(),
                LegacyScalar::Text(request.replacement)
            );
        }
    }
}

#[test]
fn media_offsets_into_metadata_or_past_payload_refuse() {
    for into_metadata in [true, false] {
        let mut before = rich(false);
        let at = before.windows(4).position(|x| x == b"stco").unwrap();
        let bad = if into_metadata {
            before.windows(4).position(|x| x == b"ilst").unwrap() as u32
        } else {
            (before.len() + 100) as u32
        };
        before[at + 12..at + 16].copy_from_slice(&bad.to_be_bytes());
        assert!(inspect(&before, &requests()).is_err());
    }
}
#[test]
fn sample_extent_crossing_media_end_and_description_index_refuse() {
    for alac in [false, true] {
        let mut before = rich(alac);
        let at = before.windows(4).position(|x| x == b"stsz").unwrap();
        let size_at = if alac { at + 16 } else { at + 8 };
        before[size_at..size_at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            inspect(&before, &requests()).unwrap_err(),
            TagWriteError::Unsupported("MP4 sample bytes outside local media")
        );
    }
    let mut before = rich(false);
    let at = before.windows(4).position(|x| x == b"stsc").unwrap();
    before[at + 20..at + 24].copy_from_slice(&2u32.to_be_bytes());
    assert!(inspect(&before, &requests()).is_err());
}
#[test]
fn truncated_counts_and_offset_arithmetic_overflow_refuse() {
    let mut before = rich(false);
    let at = before.windows(4).position(|x| x == b"stco").unwrap();
    before[at + 8..at + 12].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(inspect(&before, &requests()).is_err());
    let mut body = vec![0, 0, 0, 0, 0, 0, 0, 1];
    body.extend((u64::MAX - 1).to_be_bytes());
    let before = replace_leaf(&rich(false), b"stco", &atom(b"co64", &body));
    assert!(inspect(&before, &requests()).is_err());
}
#[test]
fn missing_required_track_tables_and_nested_zero_size_refuse() {
    let before = replace_leaf(&rich(false), b"stsd", &atom(b"free", &[]));
    assert!(inspect(&before, &requests()).is_err());
    let mut before = rich(false);
    let at = before.windows(4).position(|x| x == b"ilst").unwrap();
    before[at - 4..at].fill(0);
    assert!(inspect(&before, &requests()).is_err());
}
#[cfg(native_metadata_bridge)]
#[test]
fn native_multi_value_and_duplicate_outer_preview_match_writer() {
    use sorter_io::{
        metadata::{LegacyScalar, MetadataReader},
        storage::ReadInput,
    };
    for duplicate in [false, true] {
        let before = if duplicate {
            fixture(
                false,
                &[(b"\xa9ART", vec!["à", "á"]), (b"\xa9ART", vec!["â"])],
            )
        } else {
            fixture(false, &[(b"\xa9ART", vec!["à", "á", "â"])])
        };
        let mut reader = MetadataReader
            .open(
                ReadInput::seekable(Cursor::new(before.clone()), before.len() as u64),
                "owned.m4a",
            )
            .unwrap();
        let original = reader.get(LegacyField::Artist).unwrap().value.unwrap();
        assert_eq!(original, LegacyScalar::Text("à, á, â".into()));
        let LegacyScalar::Text(value) = original else {
            panic!("text")
        };
        let request = repair(LegacyField::Artist, &value);
        {
            let after = stage(&before, &[request]);
            let mut reader = MetadataReader
                .open(
                    ReadInput::seekable(Cursor::new(after.clone()), after.len() as u64),
                    "owned.m4a",
                )
                .unwrap();
            assert_eq!(
                reader.get(LegacyField::Artist).unwrap().value.unwrap(),
                LegacyScalar::Text("א, ב, ג".into())
            );
        }
    }
}
#[test]
fn ffmpeg_decodes_actual_staged_aac_and_alac_with_fast_start_unchanged() {
    use std::ffi::OsStr;
    use std::path::Path;
    use std::process::Command;
    fn run(args: &[&OsStr]) -> Vec<u8> {
        let output = Command::new("ffmpeg")
            .args(["-v", "error"])
            .args(args)
            .output()
            .expect("FFmpeg required for MP4 acceptance");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    fn decode(path: &Path) -> Vec<u8> {
        run(&[
            "-i".as_ref(),
            path.as_os_str(),
            "-map".as_ref(),
            "0:a:0".as_ref(),
            "-f".as_ref(),
            "s16le".as_ref(),
            "-".as_ref(),
        ])
    }
    let dir = tempfile::tempdir().unwrap();
    for codec in ["aac", "alac"] {
        for front in [false, true] {
            let input = dir.path().join(format!("{codec}-{front}.m4a"));
            let mut args: Vec<&OsStr> = vec![
                "-f".as_ref(),
                "lavfi".as_ref(),
                "-i".as_ref(),
                "sine=frequency=440:duration=1.2".as_ref(),
                "-c:a".as_ref(),
                codec.as_ref(),
                "-metadata".as_ref(),
                "artist=à".as_ref(),
            ];
            if front {
                args.extend(["-movflags".as_ref(), "+faststart".as_ref()]);
            }
            args.extend(["-y".as_ref(), input.as_os_str()]);
            run(&args);
            let before = std::fs::read(&input).unwrap();
            let after = stage(&before, &[repair(LegacyField::Artist, "à")]);
            let output = dir.path().join(format!("{codec}-{front}-after.m4a"));
            std::fs::write(&output, &after).unwrap();
            let pcm = decode(&input);
            assert!(!pcm.is_empty());
            assert_eq!(pcm, decode(&output));
        }
    }
}

#[cfg(native_metadata_bridge)]
#[test]
fn duplicate_outer_preview_finds_repairs_only_in_later_values() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    for key in [b"\xa9ART", b"aART", b"\xa9nam", b"\xa9alb", b"\xa9gen"] {
        let before = fixture(false, &[(key, vec!["שלום"]), (key, vec!["à"])]);
        let mut reader = MetadataReader
            .open(
                ReadInput::seekable(Cursor::new(before.clone()), before.len() as u64),
                "owned.m4a",
            )
            .unwrap();
        let field = match key {
            b"\xa9ART" => LegacyField::Artist,
            b"aART" => LegacyField::AlbumArtist,
            b"\xa9nam" => LegacyField::Title,
            b"\xa9alb" => LegacyField::Album,
            _ => LegacyField::Genre,
        };
        let observed = reader.get(field).unwrap();
        assert_eq!(
            observed.value.unwrap(),
            sorter_core::metadata::LegacyScalar::Text("שלום, à".into())
        );
        assert_eq!(observed.values.len(), 2);
        assert_eq!(repair(field, "שלום, à").replacement, "שלום, א");
        let after = stage(&before, &[repair(field, "שלום, à")]);
        assert_eq!(metadata_item(&after, key), vec![text(key, &["שלום, א"])]);
    }
}
#[cfg(native_metadata_bridge)]
#[test]
fn duplicate_outer_unsupported_text_is_an_explicit_field_error() {
    use sorter_io::{metadata::MetadataReader, storage::ReadInput};
    let mut before = fixture(
        false,
        &[(b"\xa9ART", vec!["שלום"]), (b"\xa9ART", vec!["à"])],
    );
    let positions: Vec<_> = before
        .windows(4)
        .enumerate()
        .filter_map(|(at, v)| (v == b"data").then_some(at))
        .collect();
    let at = positions[1];
    before[at + 4..at + 8].copy_from_slice(&2u32.to_be_bytes());
    let mut reader = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(before.clone()), before.len() as u64),
            "owned.m4a",
        )
        .unwrap();
    let error = reader.get(LegacyField::Artist).unwrap_err();
    assert_eq!(error.kind, "UnsupportedMp4TextRepresentation");
}
