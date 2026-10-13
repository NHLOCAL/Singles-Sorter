//! Ordinary read-for-sort representations, separate from tag-write admission.
#![cfg(native_metadata_bridge)]
use sorter_io::{
    metadata::{LegacyField, LegacyScalar, MetadataReader},
    storage::ReadInput,
};
use std::io::Cursor;
#[path = "../../../tests/metadata_read_compatibility/fixture_support.rs"]
mod fixture_support;
use fixture_support::fixture;
#[test]
fn ordinary_id3v22_reads_exact_legacy_text_fields() {
    check(2, 0);
}
#[test]
fn ordinary_id3v23_unsync_decodes_actual_escaped_utf16_boms() {
    check(3, 0x80);
}
#[test]
fn ordinary_id3v23_extended_header_retains_fields() {
    check(3, 0x40);
}
#[test]
fn combined_unsync_extended_remains_refused_until_original_projection_is_proven() {
    let bytes = fixture(3, 0xc0);
    let length = bytes.len() as u64;
    assert!(matches!(
        MetadataReader.open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "ordinary.mp3"
        ),
        Err(sorter_io::metadata::MetadataError::UnsupportedClass)
    ));
}
fn check(version: u8, flags: u8) {
    let bytes = fixture(version, flags);
    let length = bytes.len() as u64;
    let mut fields = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "ordinary.mp3",
        )
        .unwrap_or_else(|e| panic!("ordinary ID3v2.2: {e:?}"));
    for (field, text) in [
        (LegacyField::Artist, "שלום עולם"),
        (LegacyField::AlbumArtist, "זמר אלבום"),
        (LegacyField::Title, "שיר לבדיקה"),
        (LegacyField::Album, "אלבום לדוגמה"),
        (LegacyField::Genre, "Test"),
    ] {
        assert_eq!(
            fields.get(field).unwrap().value.unwrap(),
            LegacyScalar::Text(text.into())
        );
    }
    assert_eq!(
        fields.get(LegacyField::TrackNumber).unwrap().value.unwrap(),
        LegacyScalar::Integer("2".into())
    );
}
#[test]
fn sub_header_mp3_is_an_ordinary_open_failure_not_an_adapter_capability_gap() {
    for name in ["mp3", ".mp3", "empty.flac", "no-suffix"] {
        assert!(
            matches!(
                MetadataReader.open(ReadInput::seekable(Cursor::new(vec![]), 0), name),
                Err(sorter_io::metadata::MetadataError::Parser)
            ),
            "{name}"
        );
    }
    for bytes in [vec![], vec![0], vec![0xff, 0xfb], b"ID3".to_vec()] {
        let length = bytes.len() as u64;
        assert!(
            matches!(MetadataReader.open(ReadInput::seekable(Cursor::new(bytes),length),"short.MP3"),Err(sorter_io::metadata::MetadataError::LegacyOpen{kind,..})if kind=="HeaderNotFoundError")
        );
    }
}
#[test]
fn real_provider_mixed_103_item_plan_preserves_album_and_filename_fallback() {
    use sorter_core::{Settings, job_plan::*, metadata::MetadataObservation};
    #[cfg(windows)]
    use sorter_io::desktop::DesktopStorage;
    #[cfg(target_os = "linux")]
    use sorter_io::desktop::LinuxTestStorage as DesktopStorage;
    use sorter_io::metadata::StorageMetadataProvider;
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    fixture_support::mixed_library(&source);
    use sorter_io::storage::*;
    use std::{cell::RefCell, rc::Rc};
    #[derive(Clone)]
    struct Shared(Rc<RefCell<DesktopStorage>>);
    impl ReadStorage for Shared {
        fn stat(&mut self, r: &StorageRef) -> Result<EntryObservation, StorageError> {
            self.0.borrow_mut().stat(r)
        }
        fn children(&mut self, r: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
            self.0.borrow_mut().children(r)
        }
        fn open_read(&mut self, r: &StorageRef) -> Result<ReadInput, StorageError> {
            self.0.borrow_mut().open_read(r)
        }
    }
    let mut desktop = DesktopStorage::new(&[source.clone(), target.clone()]).unwrap();
    let source_ref = desktop.reference(&source).unwrap();
    let target_ref = desktop.reference(&target).unwrap();
    let mut reads = Shared(Rc::new(RefCell::new(desktop)));
    let mut metadata_reads = reads.clone();
    let request = JobRequestV2 {
        mode: JobMode::Sort,
        source: source_ref,
        target: Some(target_ref),
        settings: Settings {
            copy_mode: true,
            main_folder_only: false,
            singles_folder: false,
            ..Settings::default()
        },
        alias_rows: vec![
            vec!["זמר אלבום".into(), "Mapped Artist".into()],
            vec!["שלום עולם".into(), "Mapped Artist".into()],
            vec!["Fallback Artist".into(), "Mapped Artist".into()],
            vec!["Artist".into(), "Mapped Artist".into()],
        ],
        resource_ids: vec!["real-metadata-fixture".into()],
        adapter_versions: vec!["native-metadata".into()],
        ai_profile: AiProfile::Disabled,
    };
    let mut metadata = StorageMetadataProvider {
        storage: &mut metadata_reads,
        reader: MetadataReader,
    };
    let plan = plan_job(
        &request,
        &mut reads,
        &mut metadata,
        &mut DisabledRecognition,
    )
    .unwrap();
    #[cfg(windows)]
    assert_eq!(plan.blockers, vec!["PHYSICAL_APPLY_UNAVAILABLE"]);
    #[cfg(target_os = "linux")]
    assert_eq!(
        plan.blockers,
        vec!["PHYSICAL_APPLY_UNAVAILABLE", "UNKNOWN_CAPABILITY"]
    );

    assert_eq!(
        plan.operations
            .iter()
            .filter(|o| matches!(o.kind, OperationKind::AlbumTransfer { .. }))
            .count(),
        1
    );
    assert_eq!(
        plan.operations
            .iter()
            .filter(|o| matches!(o.kind, OperationKind::SingleTransfer { .. }))
            .count(),
        23
    );
    assert!(plan.metadata_observations.iter().any(|o|matches!(o,MetadataObservation::Get{source,field:LegacyField::Artist,result:Ok(item),..} if source.name=="metadata-only-v1.mp3" && item.value==Ok(LegacyScalar::Text("ùìåí òåìí".into())))));
    assert!(plan.operations.iter().any(|o|matches!(&o.kind,OperationKind::SingleTransfer{source,artist,..} if source.name=="metadata-only-v1.mp3"&&artist=="Mapped Artist")));
    assert_eq!(plan.predicted_legacy_counters.songs_sorted, 23);
    assert_eq!(plan.predicted_legacy_counters.albums_processed, 1);
    assert!(plan.metadata_observations.iter().any(|o|matches!(o,MetadataObservation::Open{source,result:Err(sorter_io::metadata::MetadataError::LegacyOpen{kind,..}),..}if source.size==Some(0)&&kind=="HeaderNotFoundError")));
    assert!(std::fs::read_dir(&target).unwrap().next().is_none());
}
#[test]
fn ordinary_id3v22_unsync_decodes_actual_escaped_utf16_boms() {
    check(2, 0x80);
}
#[test]
fn actual_three_file_album_vote_keeps_unreadable_file_in_denominator() {
    use sorter_core::{
        album::{FolderObservation, analyze_album},
        discovery::ReadStorage,
    };
    #[cfg(windows)]
    use sorter_io::desktop::DesktopStorage;
    #[cfg(target_os = "linux")]
    use sorter_io::desktop::LinuxTestStorage as DesktopStorage;
    use sorter_io::metadata::StorageMetadataProvider;
    let tree = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        ("a.mp3", fixture(2, 0)),
        ("b.mp3", fixture(3, 0x80)),
        ("c.mp3", vec![]),
    ] {
        std::fs::write(tree.path().join(name), bytes).unwrap();
    }
    let mut reads = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap();
    let root = reads.reference(tree.path()).unwrap();
    let folder = FolderObservation {
        entry: reads.stat(&root).unwrap(),
        direct_children: reads.children(&root).unwrap(),
    };
    let mut metadata = StorageMetadataProvider {
        storage: &mut reads,
        reader: MetadataReader,
    };
    let decision = analyze_album(&folder, &mut metadata);
    assert!(
        !decision.is_album,
        "two of three is below the original 70% threshold: {decision:?}"
    );
    assert!(decision.trace.iter().any(|e|matches!(e,sorter_core::album::AlbumTraceEvent::FileError{kind,..} if kind=="HeaderNotFoundError")));
}
#[test]
fn new_id3_representations_keep_extent_transform_and_structured_frame_guards() {
    use sorter_io::metadata::MetadataError;
    for kind in [
        "compression",
        "length",
        "structured",
        "bad_unsync",
        "extended_crc",
    ] {
        let mut bytes = fixture(
            if kind == "extended_crc" { 3 } else { 2 },
            if kind == "extended_crc" { 0x40 } else { 0 },
        );
        match kind {
            "compression" => bytes[5] = 0x40,
            "length" => bytes[13..16].fill(0xff),
            "structured" => bytes[10..13].copy_from_slice(b"SLT"),
            "bad_unsync" => bytes[5] = 0x80, // Raw UTF-16 FF FE requires escaping.
            "extended_crc" => bytes[14] = 0x80,
            _ => unreachable!(),
        }
        let length = bytes.len() as u64;
        assert!(
            matches!(
                MetadataReader.open(
                    ReadInput::seekable(Cursor::new(bytes), length),
                    "guarded.mp3"
                ),
                Err(MetadataError::UnsupportedClass)
            ),
            "{kind}"
        );
    }
}
#[test]
fn all_unreadable_candidate_album_stays_protected_by_original_policy() {
    use sorter_core::{
        album::{FolderObservation, analyze_album},
        discovery::ReadStorage,
    };
    #[cfg(windows)]
    use sorter_io::desktop::DesktopStorage;
    #[cfg(target_os = "linux")]
    use sorter_io::desktop::LinuxTestStorage as DesktopStorage;
    use sorter_io::metadata::StorageMetadataProvider;
    let tree = tempfile::tempdir().unwrap();
    for name in ["Artist a.mp3", "Artist b.mp3", "Artist c.mp3"] {
        std::fs::write(tree.path().join(name), []).unwrap();
    }
    let mut reads = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap();
    let root = reads.reference(tree.path()).unwrap();
    let folder = FolderObservation {
        entry: reads.stat(&root).unwrap(),
        direct_children: reads.children(&root).unwrap(),
    };
    let mut metadata = StorageMetadataProvider {
        storage: &mut reads,
        reader: MetadataReader,
    };
    let decision = analyze_album(&folder, &mut metadata);
    assert!(decision.is_album);
    assert!(!decision.should_process);
    assert!(decision.album_name.is_none());
    assert!(decision.artist_name.is_none());
}
#[test]
fn ordinary_id3v1_only_reads_released_fields_and_track_padding_rules() {
    for (track, marker, expected) in [
        (1, 0, "1"),
        (32, 0, "32"),
        (32, b' ', "0"),
        (0, 0, "0"),
        (7, b'x', "7"),
    ] {
        let bytes = fixture_support::id3v1(17, track, marker);
        let length = bytes.len() as u64;
        let mut h = MetadataReader
            .open(
                ReadInput::seekable(Cursor::new(bytes), length),
                "ordinary.mp3",
            )
            .unwrap_or_else(|e| panic!("ID3v1: {e:?}"));
        for (f, t) in [
            (LegacyField::Title, "Synthetic title"),
            (LegacyField::Artist, "Artist"),
            (LegacyField::Album, "Album"),
            (LegacyField::AlbumArtist, ""),
            (LegacyField::Genre, "Rock"),
        ] {
            assert_eq!(
                h.get(f).unwrap().value.unwrap(),
                LegacyScalar::Text(t.into())
            );
        }
        assert_eq!(
            h.get(LegacyField::TrackNumber).unwrap().value.unwrap(),
            LegacyScalar::Integer(expected.into())
        );
    }
}
#[test]
fn ordinary_id3v1_merges_only_missing_v2_fields() {
    let mut bytes = fixture(2, 0);
    let v1 = fixture_support::id3v1(17, 7, 0);
    bytes.extend(&v1[v1.len() - 128..]);
    let length = bytes.len() as u64;
    let mut h = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "ordinary.mp3",
        )
        .unwrap_or_else(|e| panic!("ID3v2+v1: {e:?}"));
    assert_eq!(
        h.get(LegacyField::Artist).unwrap().value.unwrap(),
        LegacyScalar::Text("שלום עולם".into())
    );
    assert_eq!(
        h.get(LegacyField::TrackNumber).unwrap().value.unwrap(),
        LegacyScalar::Integer("2".into())
    );
}
#[test]
fn id3v1_nul_ascii_padding_and_latin1_are_projected_without_native_trimming() {
    let mut bytes = fixture_support::id3v1(255, 0, 0);
    let at = bytes.len() - 128 + 33;
    bytes[at..at + 30].fill(0);
    bytes[at..at + 8].copy_from_slice(&[b' ', 0xa0, b'A', 0xa0, b' ', 0, b'X', b'X']);
    let length = bytes.len() as u64;
    let mut h = MetadataReader
        .open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "ordinary.mp3",
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        h.get(LegacyField::Artist).unwrap().value.unwrap(),
        LegacyScalar::Text("\u{a0}A\u{a0}".into())
    );
    assert_eq!(
        h.get(LegacyField::Genre).unwrap().value.unwrap(),
        LegacyScalar::Text("".into())
    );
}
#[test]
fn ambiguous_id3v1_tail_cannot_override_original_ape_marker_discovery() {
    let mut bytes = fixture_support::id3v1(17, 1, 0);
    let at = bytes.len() - 128;
    bytes[at - 3..at].copy_from_slice(b"APE");
    bytes[at + 3..at + 5].copy_from_slice(b"EX");
    let length = bytes.len() as u64;
    assert!(matches!(
        MetadataReader.open(
            ReadInput::seekable(Cursor::new(bytes), length),
            "ambiguous.mp3"
        ),
        Err(sorter_io::metadata::MetadataError::UnsupportedClass)
    ));
}
#[test]
fn all_id3v1_genre_bytes_match_recorded_original_values() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/metadata_read_compatibility/id3v1-genres-original.json"
    ))
    .unwrap();
    assert_eq!(expected["genres"].as_array().unwrap().len(), 256);
    for genre in 0..=255u8 {
        let bytes = fixture_support::id3v1(genre, 1, 0);
        let length = bytes.len() as u64;
        let mut h = MetadataReader
            .open(
                ReadInput::seekable(Cursor::new(bytes), length),
                "ordinary.mp3",
            )
            .unwrap_or_else(|e| panic!("{genre}: {e:?}"));
        assert_eq!(
            h.get(LegacyField::Genre).unwrap().value.unwrap(),
            LegacyScalar::Text(
                expected["genres"][usize::from(genre)]
                    .as_str()
                    .unwrap()
                    .into()
            ),
            "{genre}"
        );
    }
}
#[test]
fn id3v1_fallback_distinguishes_dropped_v2_frame_from_present_empty() {
    for payload in [vec![0], vec![0, 0]] {
        let mut bytes = fixture(3, 0);
        let old_size = u32::from_be_bytes(bytes[14..18].try_into().unwrap()) as usize;
        assert_eq!(&bytes[10..14], b"TPE1");
        bytes.splice(20..20 + old_size, payload.clone());
        bytes[14..18].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        let old_tag = bytes[6..10]
            .iter()
            .fold(0usize, |n, b| (n << 7) | usize::from(*b));
        let size = old_tag - old_size + payload.len();
        for (i, shift) in [21, 14, 7, 0].iter().enumerate() {
            bytes[6 + i] = ((size >> shift) & 127) as u8;
        }
        let v1 = fixture_support::id3v1(17, 1, 0);
        bytes.extend(&v1[v1.len() - 128..]);
        let length = bytes.len() as u64;
        let mut h = MetadataReader
            .open(
                ReadInput::seekable(Cursor::new(bytes), length),
                "ordinary.mp3",
            )
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            h.get(LegacyField::Artist).unwrap().value.unwrap(),
            LegacyScalar::Text(if payload.len() == 1 { "Artist" } else { "" }.into())
        );
    }
}
