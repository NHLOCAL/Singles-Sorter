use super::*;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

mod reparse_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/windows_reparse.rs"
    ));
}

const MEDIA_LENGTH: usize = 196_608;
const JOURNAL_LENGTH: usize = 131_072;
fn no_progress(_: WindowsInspectionEvent) -> Result<(), MutationError> {
    Ok(())
}
fn id(path: &Path) -> ObjectIdentity {
    identity(chain(path).unwrap().last().unwrap()).unwrap()
}
fn observed(path: &Path) -> MutationObservation {
    let handles = chain(path).unwrap();
    read_observation(
        handles.last().unwrap(),
        &id(path.parent().unwrap()),
        &AtomicBool::new(false),
        &mut no_progress,
    )
    .unwrap()
}
fn selected(path: &Path) -> WindowsSelectedRootGrant {
    WindowsSelectedRootGrant::new(path.to_path_buf(), id(path)).unwrap()
}
fn target(path: &Path, kind: EntryKind) -> WindowsAlbumInspectionTarget {
    WindowsAlbumInspectionTarget {
        reference: reference_of(path),
        identity: Some(id(path)),
        parent_identity: id(path.parent().unwrap()),
        kind,
    }
}

struct Fixture {
    tree: tempfile::TempDir,
    parent: PathBuf,
    source: PathBuf,
    original: MutationObservation,
    target: PathBuf,
    journals: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let tree = tempfile::tempdir().unwrap();
        let parent = tree.path().join("source parent");
        let source = parent.join("אלבום מקורי 𐐀");
        let target = tree.path().join("target");
        let journals = tree.path().join("journals");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(target.join("artist")).unwrap();
        std::fs::create_dir(&journals).unwrap();
        std::fs::write(parent.join("unselected sibling"), b"retain this sibling").unwrap();
        std::fs::write(target.join("artist/song.mp3"), vec![19; MEDIA_LENGTH]).unwrap();
        std::fs::write(target.join("artist/song.mp3:notes"), vec![23; MEDIA_LENGTH]).unwrap();
        std::fs::write(journals.join("job.album.jsonl"), vec![b'j'; JOURNAL_LENGTH]).unwrap();
        let original = observed(&source);
        std::fs::rename(&source, parent.join("retained original")).unwrap();
        Self {
            tree,
            parent,
            source,
            original,
            target,
            journals,
        }
    }
    fn original(&self) -> WindowsAlbumOriginalLocationInspection {
        let grant =
            WindowsAlbumInspectionParentGrant::new(self.parent.clone(), id(&self.parent)).unwrap();
        WindowsAlbumOriginalLocationInspection::open_observed(
            grant,
            self.original.clone(),
            &AtomicBool::new(false),
            &mut no_progress,
        )
        .unwrap()
    }
    fn inputs(&self) -> WindowsAlbumReadInspectionInputs {
        let root = observed(&self.target);
        WindowsAlbumReadInspectionInputs {
            target: selected(&self.target),
            journal: selected(&self.journals),
            recorded_target: EntryObservation {
                reference: reference_of(&self.target),
                identity: Some(root.identity),
                parent_identity: Some(root.parent_identity),
                name: "target".into(),
                kind: EntryKind::Directory,
                size: None,
                modified: None,
                capabilities: Default::default(),
            },
            journals: vec![WindowsAlbumInspectionJournal {
                name: MutationName::Windows("job.album.jsonl".encode_utf16().collect()),
                identity: id(&self.journals.join("job.album.jsonl")),
                length: JOURNAL_LENGTH as u64,
            }],
            targets: vec![
                target(&self.target.join("artist"), EntryKind::Directory),
                target(&self.target.join("artist/song.mp3"), EntryKind::File),
            ],
        }
    }
    fn open(&self) -> WindowsAlbumReadInspection {
        WindowsAlbumReadInspection::open_observed(
            self.inputs(),
            self.original(),
            &AtomicBool::new(false),
            &mut no_progress,
        )
        .unwrap()
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Node {
    path: PathBuf,
    identity: ObjectIdentity,
    directory: bool,
    attributes: u32,
    length: u64,
    created: u64,
    modified: u64,
    security: RepairSecurity,
    main: Option<Vec<u8>>,
    notes: Option<Vec<u8>>,
}
fn snapshot(root: &Path) -> Vec<Node> {
    let mut pending = vec![root.to_path_buf()];
    let mut nodes = Vec::new();
    while let Some(path) = pending.pop() {
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert_eq!(
            metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0,
            0
        );
        let file = std::fs::OpenOptions::new()
            .access_mode(FILE_GENERIC_READ.0)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
            .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
            .open(&path)
            .unwrap();
        let security = repair_security(&file).unwrap();
        let digest = |path: &Path| std::fs::read(path).map(|bytes| Sha256::digest(bytes).to_vec());
        let main = metadata.is_file().then(|| digest(&path).unwrap());
        let notes = if metadata.is_file() {
            let mut named = path.as_os_str().to_os_string();
            named.push(":notes");
            match digest(Path::new(&named)) {
                Ok(value) => Some(value),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => panic!("named stream snapshot: {error}"),
            }
        } else {
            None
        };
        nodes.push(Node {
            path: path.strip_prefix(root).unwrap().to_path_buf(),
            identity: id(&path),
            directory: metadata.is_dir(),
            attributes: metadata.file_attributes(),
            length: metadata.file_size(),
            created: metadata.creation_time(),
            modified: metadata.last_write_time(),
            security,
            main,
            notes,
        });
        if metadata.is_dir() {
            pending.extend(
                std::fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        }
    }
    nodes.sort_by(|a, b| a.path.cmp(&b.path));
    nodes
}
fn stale_result<T>(result: Result<T, MutationError>) {
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::StalePlan));
}
fn sharing_refusal(error: io::Error) {
    assert!(
        matches!(error.raw_os_error(), Some(32 | 33)),
        "unexpected native refusal: {error}"
    );
}

#[test]
fn absent_source_owner_reads_only_bound_journal_and_target_evidence_without_writes() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    let owner = fixture.open();
    let flag = AtomicBool::new(false);
    let mut bytes = 0usize;
    let mut progress = |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            bytes = bytes.checked_add(count).unwrap();
        }
        Ok(())
    };
    assert_eq!(
        owner
            .read_journal_observed(0, &flag, &mut progress)
            .unwrap(),
        vec![b'j'; JOURNAL_LENGTH]
    );
    let pin = owner
        .pin_directory_observed(0, &flag, &mut progress)
        .unwrap();
    assert_eq!(
        pin.observation().identity,
        id(&fixture.target.join("artist"))
    );
    assert!(
        owner
            .inspect_metadata_observed(1, &flag, &mut progress)
            .unwrap()
            .is_some()
    );
    let (_, main) = owner
        .inspect_main_observed(1, &flag, &mut progress)
        .unwrap();
    let expected_main: [u8; 32] = Sha256::digest(vec![19; MEDIA_LENGTH]).into();
    assert_eq!(main.sha256, expected_main);
    let streams = owner
        .inspect_streams_observed(1, &flag, &mut progress)
        .unwrap();
    assert_eq!(streams.streams.len(), 2);
    assert_eq!(
        streams
            .streams
            .iter()
            .map(|stream| stream.length)
            .sum::<u64>(),
        (MEDIA_LENGTH * 2) as u64
    );
    pin.revalidate_observed(&flag, &mut progress).unwrap();
    assert_eq!(
        owner
            .inspect_original_observed(&flag, &mut progress)
            .unwrap(),
        WindowsAlbumOriginalLocation::Absent
    );
    assert_eq!(bytes, JOURNAL_LENGTH + MEDIA_LENGTH * 3);
    assert_eq!(snapshot(fixture.tree.path()), before);
    drop(pin);
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn present_source_is_refused_and_reappeared_original_or_foreign_leaf_stays_distinct() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    std::fs::create_dir(&fixture.source).unwrap();
    let before = snapshot(fixture.tree.path());
    assert!(matches!(
        owner
            .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
            .unwrap(),
        WindowsAlbumOriginalLocation::ForeignPresent(_)
    ));
    stale_result(WindowsAlbumReadInspection::open_observed(
        fixture.inputs(),
        fixture.original(),
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    assert_eq!(snapshot(fixture.tree.path()), before);
    std::fs::rename(&fixture.source, fixture.parent.join("retained foreign")).unwrap();
    std::fs::rename(fixture.parent.join("retained original"), &fixture.source).unwrap();
    let before = snapshot(fixture.tree.path());
    assert_eq!(
        owner
            .inspect_original_observed(&AtomicBool::new(false), &mut no_progress)
            .unwrap(),
        WindowsAlbumOriginalLocation::OriginalPresent(fixture.original.clone())
    );
    stale_result(WindowsAlbumReadInspection::open_observed(
        fixture.inputs(),
        fixture.original(),
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn target_ledger_rejects_escape_unknown_parent_duplicates_and_unbound_lookup() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    for change in 0..4 {
        let mut inputs = fixture.inputs();
        match change {
            0 => {
                inputs.targets[1].reference =
                    reference_of(&fixture.parent.join("unselected sibling"))
            }
            1 => inputs.targets[1].parent_identity = id(&fixture.journals),
            2 => inputs.targets.push(inputs.targets[1].clone()),
            _ => inputs.targets[0].identity = None,
        }
        stale_result(WindowsAlbumReadInspection::open_observed(
            inputs,
            fixture.original(),
            &AtomicBool::new(false),
            &mut no_progress,
        ));
    }
    let owner = fixture.open();
    stale_result(owner.read_journal_observed(1, &AtomicBool::new(false), &mut no_progress));
    stale_result(owner.inspect_metadata_observed(2, &AtomicBool::new(false), &mut no_progress));
    stale_result(owner.inspect_main_observed(2, &AtomicBool::new(false), &mut no_progress));
    stale_result(owner.inspect_streams_observed(2, &AtomicBool::new(false), &mut no_progress));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn journal_identity_length_and_exact_name_must_match_retained_admission() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    for change in 0..3 {
        let mut inputs = fixture.inputs();
        match change {
            0 => inputs.journals[0].length -= 1,
            1 => inputs.journals[0].identity = id(&fixture.target.join("artist/song.mp3")),
            _ => {
                inputs.journals[0].name =
                    MutationName::Windows("JOB.album.jsonl".encode_utf16().collect())
            }
        }
        let owner = WindowsAlbumReadInspection::open_observed(
            inputs,
            fixture.original(),
            &AtomicBool::new(false),
            &mut no_progress,
        )
        .unwrap();
        stale_result(owner.read_journal_observed(0, &AtomicBool::new(false), &mut no_progress));
    }
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn changed_journal_object_and_outside_journal_name_are_refused() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let journal = fixture.journals.join("job.album.jsonl");
    std::fs::rename(&journal, fixture.journals.join("retained old.jsonl")).unwrap();
    std::fs::write(&journal, vec![b'j'; JOURNAL_LENGTH]).unwrap();
    let before = snapshot(fixture.tree.path());
    stale_result(owner.read_journal_observed(0, &AtomicBool::new(false), &mut no_progress));
    let mut inputs = fixture.inputs();
    inputs.journals[0].name = MutationName::Windows("../outside.jsonl".encode_utf16().collect());
    assert!(
        WindowsAlbumReadInspection::open_observed(
            inputs,
            fixture.original(),
            &AtomicBool::new(false),
            &mut no_progress
        )
        .is_err()
    );
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn journal_cancellation_uses_actual_chunks_and_fresh_reinspection_reads_all_bytes() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    let owner = fixture.open();
    let cancel = AtomicBool::new(false);
    let mut bytes = 0usize;
    let result = owner.read_journal_observed(0, &cancel, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            bytes += count;
            if count > 0 {
                cancel.store(true, Ordering::Release);
            }
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(bytes, 65_536);
    drop(owner);
    let owner = fixture.open();
    bytes = 0;
    owner
        .read_journal_observed(0, &AtomicBool::new(false), &mut |event| {
            if let WindowsInspectionEvent::BytesRead(count) = event {
                bytes += count;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(bytes, JOURNAL_LENGTH);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn journal_growth_is_refused_by_native_lease_or_bounded_eof_check() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let before = snapshot(fixture.tree.path());
    let mut growth = None;
    let result = owner.read_journal_observed(0, &AtomicBool::new(false), &mut |event| {
        if matches!(event, WindowsInspectionEvent::BytesRead(count) if count > 0)
            && growth.is_none()
        {
            growth = Some(
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(fixture.journals.join("job.album.jsonl"))
                    .and_then(|mut file| file.write_all(b"growth")),
            );
        }
        Ok(())
    });
    match growth.expect("must reach actual journal read") {
        Ok(()) => stale_result(result),
        Err(error) => {
            sharing_refusal(error);
            assert_eq!(result.unwrap().len(), JOURNAL_LENGTH);
            assert_eq!(snapshot(fixture.tree.path()), before);
        }
    }
}

#[test]
fn main_stream_cancellation_does_not_cache_a_partial_hash() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let before = snapshot(fixture.tree.path());
    let cancel = AtomicBool::new(false);
    let mut bytes = 0;
    let result = owner.inspect_main_observed(1, &cancel, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            bytes += count;
            if count > 0 {
                cancel.store(true, Ordering::Release);
            }
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(bytes, 65_536);
    drop(owner);
    let owner = fixture.open();
    bytes = 0;
    owner
        .inspect_main_observed(1, &AtomicBool::new(false), &mut |event| {
            if let WindowsInspectionEvent::BytesRead(count) = event {
                bytes += count;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(bytes, MEDIA_LENGTH);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn named_stream_cancellation_accounts_bytes_after_the_main_stream() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let before = snapshot(fixture.tree.path());
    let cancel = AtomicBool::new(false);
    let mut bytes = 0;
    let result = owner.inspect_streams_observed(1, &cancel, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            bytes += count;
            if bytes > MEDIA_LENGTH {
                cancel.store(true, Ordering::Release);
            }
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(bytes, MEDIA_LENGTH + 65_536);
    let mut probes = 0;
    let result = owner.inspect_streams_observed(1, &AtomicBool::new(false), &mut |event| {
        if event == WindowsInspectionEvent::Probe {
            probes += 1;
            return Err(err(
                MutationErrorKind::PermissionDenied,
                "owned injected probe failure",
            ));
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::PermissionDenied));
    assert_eq!(probes, 1);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn replaced_target_parent_cannot_be_hidden_by_preserving_the_file_object() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let artist = fixture.target.join("artist");
    let retired = fixture.target.join("retained artist");
    std::fs::rename(&artist, &retired).unwrap();
    std::fs::create_dir(&artist).unwrap();
    std::fs::rename(retired.join("song.mp3"), artist.join("song.mp3")).unwrap();
    let before = snapshot(fixture.tree.path());
    stale_result(owner.inspect_metadata_observed(1, &AtomicBool::new(false), &mut no_progress));
    stale_result(owner.inspect_main_observed(1, &AtomicBool::new(false), &mut no_progress));
    stale_result(owner.inspect_streams_observed(1, &AtomicBool::new(false), &mut no_progress));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn target_parent_relocation_during_content_is_refused_or_stale() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let original = fixture.target.join("artist");
    let moved = fixture.target.join("relocated artist");
    let original_id = id(&original);
    let before = snapshot(fixture.tree.path());
    let mut relocation = None;
    let result = owner.inspect_main_observed(1, &AtomicBool::new(false), &mut |event| {
        if matches!(event, WindowsInspectionEvent::BytesRead(count) if count > 0)
            && relocation.is_none()
        {
            relocation = Some(std::fs::rename(
                fixture.target.join("artist"),
                fixture.target.join("relocated artist"),
            ));
        }
        Ok(())
    });
    match relocation.expect("must reach content") {
        Ok(()) => stale_result(result),
        Err(error) => {
            assert!(
                matches!(error.raw_os_error(), Some(5 | 32 | 33)),
                "unexpected relocation refusal: {error}"
            );
            assert!(result.is_ok());
            assert_eq!(snapshot(fixture.tree.path()), before);
            assert!(!moved.exists());
            drop(owner);
            std::fs::rename(&original, &moved).unwrap();
            assert_eq!(id(&moved), original_id);
        }
    }
}

#[test]
fn retained_directory_pin_revalidates_after_file_handles_close() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let flag = AtomicBool::new(false);
    let pin = owner
        .pin_directory_observed(0, &flag, &mut no_progress)
        .unwrap();
    owner
        .inspect_main_observed(1, &flag, &mut no_progress)
        .unwrap();
    let before = snapshot(fixture.tree.path());
    match std::fs::rename(
        fixture.target.join("artist"),
        fixture.target.join("relocated held artist"),
    ) {
        Ok(()) => stale_result(pin.revalidate_observed(&flag, &mut no_progress)),
        Err(error) => {
            sharing_refusal(error);
            pin.revalidate_observed(&flag, &mut no_progress).unwrap();
            assert_eq!(snapshot(fixture.tree.path()), before);
        }
    }
}

#[test]
fn journal_root_relocation_during_read_never_returns_rebound_bytes() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let original = fixture.journals.clone();
    let moved = fixture.tree.path().join("relocated journals");
    let original_id = id(&original);
    let before = snapshot(fixture.tree.path());
    let mut relocation = None;
    let result = owner.read_journal_observed(0, &AtomicBool::new(false), &mut |event| {
        if matches!(event, WindowsInspectionEvent::BytesRead(count) if count > 0)
            && relocation.is_none()
        {
            relocation = Some(std::fs::rename(
                &fixture.journals,
                fixture.tree.path().join("relocated journals"),
            ));
        }
        Ok(())
    });
    match relocation.expect("must reach journal read") {
        Ok(()) => stale_result(result),
        Err(error) => {
            assert!(
                matches!(error.raw_os_error(), Some(5 | 32 | 33)),
                "unexpected relocation refusal: {error}"
            );
            assert!(result.is_ok());
            assert_eq!(snapshot(fixture.tree.path()), before);
            assert!(!moved.exists());
            drop(owner);
            std::fs::rename(&original, &moved).unwrap();
            assert_eq!(id(&moved), original_id);
        }
    }
}

#[test]
fn reparse_target_parent_is_refused_without_following_its_destination() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let outside = fixture.tree.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("song.mp3"), b"foreign content").unwrap();
    let outside_before = snapshot(&outside);
    std::fs::rename(
        fixture.target.join("artist"),
        fixture.target.join("retained artist"),
    )
    .unwrap();
    std::fs::create_dir(fixture.target.join("artist")).unwrap();
    reparse_fixture::set_junction(&fixture.target.join("artist"), &outside);
    let result = owner.inspect_metadata_observed(1, &AtomicBool::new(false), &mut no_progress);
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::UnsafeReparse));
    assert_eq!(snapshot(&outside), outside_before);
    drop(owner);
    std::fs::remove_dir(fixture.target.join("artist")).unwrap();
}

#[test]
fn missing_target_and_cancelled_or_failed_probe_never_share_an_outcome() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    std::fs::rename(
        fixture.target.join("artist/song.mp3"),
        fixture.target.join("artist/retained song.mp3"),
    )
    .unwrap();
    let before = snapshot(fixture.tree.path());
    assert_eq!(
        owner
            .inspect_metadata_observed(1, &AtomicBool::new(false), &mut no_progress)
            .unwrap(),
        None
    );
    let flag = AtomicBool::new(false);
    let cancelled_result = owner.inspect_metadata_observed(1, &flag, &mut |_| {
        flag.store(true, Ordering::Release);
        Ok(())
    });
    assert!(matches!(cancelled_result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    let failed = owner.inspect_metadata_observed(1, &AtomicBool::new(false), &mut |_| {
        Err(err(
            MutationErrorKind::PermissionDenied,
            "owned injected denial",
        ))
    });
    assert!(matches!(failed, Err(error) if error.kind == MutationErrorKind::PermissionDenied));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn stale_fresh_target_or_journal_selection_and_overlapping_roots_are_refused() {
    for role in [0, 1] {
        let fixture = Fixture::new();
        let inputs = fixture.inputs();
        let path = if role == 0 {
            &fixture.target
        } else {
            &fixture.journals
        };
        std::fs::rename(path, fixture.tree.path().join("retained old root")).unwrap();
        std::fs::create_dir(path).unwrap();
        let before = snapshot(fixture.tree.path());
        stale_result(WindowsAlbumReadInspection::open_observed(
            inputs,
            fixture.original(),
            &AtomicBool::new(false),
            &mut no_progress,
        ));
        assert_eq!(snapshot(fixture.tree.path()), before);
    }
    let fixture = Fixture::new();
    let mut inputs = fixture.inputs();
    inputs.journal = selected(&fixture.target);
    let before = snapshot(fixture.tree.path());
    stale_result(WindowsAlbumReadInspection::open_observed(
        inputs,
        fixture.original(),
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn construction_cancel_and_probe_failure_release_roots_without_changing_them() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    let flag = AtomicBool::new(false);
    let result = WindowsAlbumReadInspection::open_observed(
        fixture.inputs(),
        fixture.original(),
        &flag,
        &mut |_| {
            flag.store(true, Ordering::Release);
            Ok(())
        },
    );
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    let failed = WindowsAlbumReadInspection::open_observed(
        fixture.inputs(),
        fixture.original(),
        &AtomicBool::new(false),
        &mut |_| {
            Err(err(
                MutationErrorKind::PermissionDenied,
                "owned construction probe failure",
            ))
        },
    );
    assert!(matches!(failed, Err(error) if error.kind == MutationErrorKind::PermissionDenied));
    assert_eq!(snapshot(fixture.tree.path()), before);
    std::fs::rename(&fixture.target, fixture.tree.path().join("released target")).unwrap();
    std::fs::rename(
        &fixture.journals,
        fixture.tree.path().join("released journals"),
    )
    .unwrap();
}

#[test]
fn cancellation_at_journal_eof_does_not_publish_complete_bytes() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let before = snapshot(fixture.tree.path());
    let flag = AtomicBool::new(false);
    let mut consumed = 0;
    let result = owner.read_journal_observed(0, &flag, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            consumed += count;
            if count == 0 {
                flag.store(true, Ordering::Release);
            }
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(consumed, JOURNAL_LENGTH);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

fn unbound(fixture: &Fixture) -> WindowsAlbumJournalInspection {
    let WindowsAlbumReadInspectionInputs {
        target,
        journal,
        recorded_target,
        journals,
        ..
    } = fixture.inputs();
    WindowsAlbumJournalInspection::open_observed(
        WindowsAlbumJournalInspectionInputs {
            target,
            journal,
            recorded_target,
            journals,
        },
        fixture.original(),
        &AtomicBool::new(false),
        &mut no_progress,
    )
    .unwrap()
}

#[test]
fn consuming_ledger_binding_rereads_exact_journal_bytes_before_target_reads() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    let mut reader = unbound(&fixture);
    let flag = AtomicBool::new(false);
    let mut consumed = 0;
    let mut progress = |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            consumed += count;
        }
        Ok(())
    };
    let bytes = reader.read_journals_observed(&flag, &mut progress).unwrap();
    assert_eq!(bytes, &[vec![b'j'; JOURNAL_LENGTH]]);
    let owner = reader
        .bind_targets_observed(fixture.inputs().targets, &flag, &mut progress)
        .unwrap();
    assert_eq!(consumed, JOURNAL_LENGTH * 2);
    owner
        .inspect_main_observed(1, &flag, &mut no_progress)
        .unwrap();
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn unread_or_repeated_journal_state_cannot_mint_a_target_ledger() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    stale_result(unbound(&fixture).bind_targets_observed(
        fixture.inputs().targets,
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    let mut reader = unbound(&fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    stale_result(reader.read_journals_observed(&AtomicBool::new(false), &mut no_progress));
    // Even an empty ledger returns the bound type, which exposes no bind method.
    let owner = reader
        .bind_targets_observed(vec![], &AtomicBool::new(false), &mut no_progress)
        .unwrap();
    stale_result(owner.inspect_metadata_observed(0, &AtomicBool::new(false), &mut no_progress));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn same_identity_and_length_do_not_hide_journal_content_changes_after_replay() {
    let fixture = Fixture::new();
    let path = fixture.journals.join("job.album.jsonl");
    let identity = id(&path);
    let mut reader = unbound(&fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    std::fs::write(&path, vec![b'k'; JOURNAL_LENGTH]).unwrap();
    assert_eq!(id(&path), identity);
    let before = snapshot(fixture.tree.path());
    stale_result(reader.bind_targets_observed(
        fixture.inputs().targets,
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn original_root_reappearing_during_binding_is_never_reported_absent() {
    let fixture = Fixture::new();
    let mut reader = unbound(&fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    let journals_before = snapshot(&fixture.journals);
    let mut inserted = false;
    let mut baseline = None;
    let result = reader.bind_targets_observed(
        fixture.inputs().targets,
        &AtomicBool::new(false),
        &mut |event| {
            if matches!(event, WindowsInspectionEvent::BytesRead(count) if count > 0) && !inserted {
                std::fs::create_dir(&fixture.source).unwrap();
                inserted = true;
                // The journal is leased here. Its full baseline was captured before
                // binding; the deliberately changed media tree gets a new baseline.
                baseline = Some((snapshot(&fixture.parent), snapshot(&fixture.target)));
            }
            Ok(())
        },
    );
    assert!(inserted);
    stale_result(result);
    assert_eq!(
        Some((snapshot(&fixture.parent), snapshot(&fixture.target))),
        baseline
    );
    assert!(fixture.source.is_dir());
    assert_eq!(snapshot(&fixture.journals), journals_before);
}

#[test]
fn binding_reread_chunk_cancellation_releases_owner_and_fresh_reads_start_again() {
    let fixture = Fixture::new();
    let before = snapshot(fixture.tree.path());
    let mut reader = unbound(&fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    let flag = AtomicBool::new(false);
    let mut consumed = 0;
    let result = reader.bind_targets_observed(fixture.inputs().targets, &flag, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            consumed += count;
            if count > 0 {
                flag.store(true, Ordering::Release);
            }
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(consumed, 65_536);
    let mut reader = unbound(&fixture);
    consumed = 0;
    let flag = AtomicBool::new(false);
    let mut progress = |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            consumed += count;
        }
        Ok(())
    };
    reader.read_journals_observed(&flag, &mut progress).unwrap();
    let owner = reader
        .bind_targets_observed(fixture.inputs().targets, &flag, &mut progress)
        .unwrap();
    assert_eq!(consumed, JOURNAL_LENGTH * 2);
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn every_retained_journal_participates_in_replay_to_target_binding() {
    let fixture = Fixture::new();
    let second = fixture.journals.join("job.transfers.jsonl");
    std::fs::write(&second, b"second recorded journal").unwrap();
    let WindowsAlbumReadInspectionInputs {
        target,
        journal,
        recorded_target,
        mut journals,
        targets,
    } = fixture.inputs();
    journals.push(WindowsAlbumInspectionJournal {
        name: MutationName::Windows("job.transfers.jsonl".encode_utf16().collect()),
        identity: id(&second),
        length: 23,
    });
    let mut reader = WindowsAlbumJournalInspection::open_observed(
        WindowsAlbumJournalInspectionInputs {
            target,
            journal,
            recorded_target,
            journals,
        },
        fixture.original(),
        &AtomicBool::new(false),
        &mut no_progress,
    )
    .unwrap();
    assert_eq!(
        reader
            .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
            .unwrap()
            .len(),
        2
    );
    std::fs::write(&second, b"second modified journal").unwrap();
    let before = snapshot(fixture.tree.path());
    stale_result(reader.bind_targets_observed(targets, &AtomicBool::new(false), &mut no_progress));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

fn present_fixture() -> Fixture {
    let fixture = Fixture::new();
    std::fs::rename(fixture.parent.join("retained original"), &fixture.source).unwrap();
    std::fs::create_dir(fixture.source.join("artist")).unwrap();
    std::fs::write(
        fixture.source.join("artist/song.mp3"),
        vec![31; MEDIA_LENGTH],
    )
    .unwrap();
    std::fs::write(
        fixture.source.join("artist/song.mp3:notes"),
        vec![37; MEDIA_LENGTH],
    )
    .unwrap();
    fixture
}
fn present_inputs(fixture: &Fixture) -> WindowsAlbumPresentJournalInspectionInputs {
    let original = observed(&fixture.source);
    let WindowsAlbumReadInspectionInputs {
        target,
        journal,
        recorded_target,
        journals,
        ..
    } = fixture.inputs();
    WindowsAlbumPresentJournalInspectionInputs {
        source: selected(&fixture.source),
        recorded_source: EntryObservation {
            reference: reference_of(&fixture.source),
            identity: Some(original.identity),
            parent_identity: Some(original.parent_identity),
            name: "אלבום מקורי 𐐀".into(),
            kind: EntryKind::Directory,
            size: None,
            modified: None,
            capabilities: Default::default(),
        },
        destination: WindowsAlbumJournalInspectionInputs {
            target,
            journal,
            recorded_target,
            journals,
        },
    }
}
fn source_ledger(fixture: &Fixture) -> Vec<WindowsAlbumInspectionTarget> {
    vec![
        target(&fixture.source.join("artist"), EntryKind::Directory),
        target(&fixture.source.join("artist/song.mp3"), EntryKind::File),
    ]
}
fn present_unbound(fixture: &Fixture) -> WindowsAlbumJournalInspection {
    WindowsAlbumJournalInspection::open_present_observed(
        present_inputs(fixture),
        &AtomicBool::new(false),
        &mut no_progress,
    )
    .unwrap()
}
fn present_bound(fixture: &Fixture) -> WindowsAlbumReadInspection {
    let mut reader = present_unbound(fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    reader
        .bind_present_ledgers_observed(
            source_ledger(fixture),
            fixture.inputs().targets,
            &AtomicBool::new(false),
            &mut no_progress,
        )
        .unwrap()
}

#[test]
fn present_source_and_target_indices_remain_separate_and_streams_are_read_only() {
    let fixture = present_fixture();
    let before = snapshot(fixture.tree.path());
    let owner = present_bound(&fixture);
    let cancel = AtomicBool::new(false);
    let (source, source_hash) = owner
        .inspect_source_main_observed(1, &cancel, &mut no_progress)
        .unwrap();
    let (destination, destination_hash) = owner
        .inspect_main_observed(1, &cancel, &mut no_progress)
        .unwrap();
    assert_ne!(source.identity, destination.identity);
    assert_ne!(source_hash, destination_hash);
    let source_streams = owner
        .inspect_source_streams_observed(1, &cancel, &mut no_progress)
        .unwrap();
    let target_streams = owner
        .inspect_streams_observed(1, &cancel, &mut no_progress)
        .unwrap();
    assert_ne!(source_streams, target_streams);
    assert!(
        matches!(owner.inspect_original_observed(&cancel, &mut no_progress).unwrap(), WindowsAlbumOriginalLocation::OriginalPresent(value) if value.identity == fixture.original.identity)
    );
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn present_root_requires_exact_recorded_reference_identity_parent_and_kind() {
    for change in 0..5 {
        let fixture = present_fixture();
        let mut inputs = present_inputs(&fixture);
        match change {
            0 => inputs.recorded_source.identity = Some(id(&fixture.target)),
            1 => inputs.recorded_source.parent_identity = Some(id(&fixture.target)),
            2 => inputs.recorded_source.reference = reference_of(&fixture.target),
            3 => inputs.recorded_source.kind = EntryKind::File,
            _ => inputs.recorded_source.size = Some(0),
        }
        let before = snapshot(fixture.tree.path());
        stale_result(WindowsAlbumJournalInspection::open_present_observed(
            inputs,
            &AtomicBool::new(false),
            &mut no_progress,
        ));
        assert_eq!(snapshot(fixture.tree.path()), before);
    }
}

#[test]
fn present_root_overlap_is_refused_before_any_journal_read() {
    for use_target in [false, true] {
        let fixture = present_fixture();
        let mut inputs = present_inputs(&fixture);
        if use_target {
            inputs.destination.target = selected(&fixture.source);
            inputs.destination.recorded_target = inputs.recorded_source.clone();
        } else {
            inputs.destination.journal = selected(&fixture.source);
        }
        let before = snapshot(fixture.tree.path());
        let mut bytes = 0;
        stale_result(WindowsAlbumJournalInspection::open_present_observed(
            inputs,
            &AtomicBool::new(false),
            &mut |event| {
                if let WindowsInspectionEvent::BytesRead(count) = event {
                    bytes += count;
                }
                Ok(())
            },
        ));
        assert_eq!(bytes, 0);
        assert_eq!(snapshot(fixture.tree.path()), before);
    }
}

#[test]
fn source_state_and_cross_role_ledgers_cannot_be_substituted() {
    let fixture = present_fixture();
    let before = snapshot(fixture.tree.path());
    stale_result(present_unbound(&fixture).bind_targets_observed(
        fixture.inputs().targets,
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    let mut reader = present_unbound(&fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    stale_result(reader.bind_present_ledgers_observed(
        fixture.inputs().targets,
        source_ledger(&fixture),
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    let owner = present_bound(&fixture);
    stale_result(owner.inspect_source_main_observed(2, &AtomicBool::new(false), &mut no_progress));
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
    let absent = Fixture::new();
    stale_result(unbound(&absent).bind_present_ledgers_observed(
        vec![],
        absent.inputs().targets,
        &AtomicBool::new(false),
        &mut no_progress,
    ));
}

#[test]
fn present_journal_bytes_must_remain_exact_through_consuming_binding() {
    let fixture = present_fixture();
    let mut reader = present_unbound(&fixture);
    reader
        .read_journals_observed(&AtomicBool::new(false), &mut no_progress)
        .unwrap();
    std::fs::write(
        fixture.journals.join("job.album.jsonl"),
        vec![b'x'; JOURNAL_LENGTH],
    )
    .unwrap();
    let before = snapshot(fixture.tree.path());
    stale_result(reader.bind_present_ledgers_observed(
        source_ledger(&fixture),
        fixture.inputs().targets,
        &AtomicBool::new(false),
        &mut no_progress,
    ));
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn present_source_main_read_cancels_on_actual_chunk_and_rechecks_fresh_bytes() {
    let fixture = present_fixture();
    let before = snapshot(fixture.tree.path());
    let owner = present_bound(&fixture);
    let cancel = AtomicBool::new(false);
    let mut bytes = 0;
    let result = owner.inspect_source_main_observed(1, &cancel, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            bytes += count;
            if count > 0 {
                cancel.store(true, Ordering::Release);
            }
        }
        Ok(())
    });
    assert!(matches!(result, Err(error) if error.kind == MutationErrorKind::Cancelled));
    assert_eq!(bytes, 65_536);
    cancel.store(false, Ordering::Release);
    owner
        .inspect_source_main_observed(1, &cancel, &mut no_progress)
        .unwrap();
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn present_source_foreign_equal_content_file_is_not_adopted() {
    let fixture = present_fixture();
    let owner = present_bound(&fixture);
    let file = fixture.source.join("artist/song.mp3");
    std::fs::rename(&file, fixture.source.join("artist/retained.mp3")).unwrap();
    std::fs::write(&file, vec![31; MEDIA_LENGTH]).unwrap();
    let before = snapshot(fixture.tree.path());
    stale_result(owner.inspect_source_main_observed(1, &AtomicBool::new(false), &mut no_progress));
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn present_source_missing_bound_descendant_parent_proves_absence_only_under_held_root() {
    let fixture = present_fixture();
    let owner = present_bound(&fixture);
    std::fs::rename(
        fixture.source.join("artist"),
        fixture.source.join("retained artist"),
    )
    .unwrap();
    let before = snapshot(fixture.tree.path());
    assert!(
        owner
            .inspect_source_entry_observed(1, &AtomicBool::new(false), &mut no_progress)
            .unwrap()
            .is_none()
    );
    std::fs::create_dir(fixture.source.join("artist")).unwrap();
    let changed = snapshot(fixture.tree.path());
    stale_result(owner.inspect_source_entry_observed(1, &AtomicBool::new(false), &mut no_progress));
    drop(owner);
    assert_ne!(changed, before);
    assert_eq!(snapshot(fixture.tree.path()), changed);
}

#[test]
fn present_source_root_relocation_cannot_be_reinterpreted_as_current_source() {
    let fixture = present_fixture();
    let owner = present_bound(&fixture);
    let relocated = fixture.parent.join("relocated source");
    match std::fs::rename(&fixture.source, &relocated) {
        Ok(()) => {
            std::fs::create_dir(&fixture.source).unwrap();
            let before = snapshot(fixture.tree.path());
            stale_result(
                owner.inspect_original_observed(&AtomicBool::new(false), &mut no_progress),
            );
            stale_result(owner.inspect_source_entry_observed(
                1,
                &AtomicBool::new(false),
                &mut no_progress,
            ));
            drop(owner);
            assert_eq!(snapshot(fixture.tree.path()), before);
        }
        Err(error) => sharing_refusal(error),
    }
}

#[test]
fn native_target_reference_verification_binds_both_reader_states_without_writes() {
    let fixture = Fixture::new();
    let expected = observed(&fixture.target).reference;
    let wrong = reference_of(&fixture.target.join("wrong"));
    let before = snapshot(fixture.tree.path());
    let flag = AtomicBool::new(false);
    let reader = unbound(&fixture);
    reader
        .verify_target_reference_observed(&expected, &flag, &mut no_progress)
        .unwrap();
    stale_result(reader.verify_target_reference_observed(&wrong, &flag, &mut no_progress));
    drop(reader);
    let owner = fixture.open();
    owner
        .verify_target_reference_observed(&expected, &flag, &mut no_progress)
        .unwrap();
    stale_result(owner.verify_target_reference_observed(&wrong, &flag, &mut no_progress));
    flag.store(true, std::sync::atomic::Ordering::Release);
    assert_eq!(
        owner
            .verify_target_reference_observed(&expected, &flag, &mut no_progress)
            .unwrap_err()
            .kind,
        MutationErrorKind::Cancelled
    );
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn native_target_reference_verification_rechecks_held_location() {
    let fixture = Fixture::new();
    let expected = observed(&fixture.target).reference;
    let original_id = id(&fixture.target);
    let moved = fixture.tree.path().join("moved target");
    let before = snapshot(fixture.tree.path());
    let owner = fixture.open();
    let mut relocation = None;
    let result =
        owner.verify_target_reference_observed(&expected, &AtomicBool::new(false), &mut |_| {
            if relocation.is_none() {
                relocation = Some(std::fs::rename(&fixture.target, &moved));
            }
            Ok(())
        });
    match relocation.expect("probe callback reached") {
        Ok(()) => {
            stale_result(result);
            assert_eq!(id(&moved), original_id);
            assert!(!fixture.target.exists());
        }
        Err(error) => {
            assert!(matches!(error.raw_os_error(), Some(5 | 32 | 33)), "{error}");
            result.unwrap();
            assert_eq!(snapshot(fixture.tree.path()), before);
            assert!(!moved.exists());
            drop(owner);
            std::fs::rename(&fixture.target, &moved).unwrap();
            assert_eq!(id(&moved), original_id);
        }
    }
}

#[test]
fn pinned_membership_is_fresh_bounded_and_cancellable_without_writes() {
    let fixture = Fixture::new();
    let owner = fixture.open();
    let flag = AtomicBool::new(false);
    let pin = owner
        .pin_directory_observed(0, &flag, &mut no_progress)
        .unwrap();
    let first = pin.members_observed(&flag, &mut no_progress).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(
        first[0].identity,
        id(&fixture.target.join("artist/song.mp3"))
    );
    assert_eq!(first[0].kind, EntryKind::File);
    assert_eq!(
        pin.members_observed(&flag, &mut no_progress).unwrap(),
        first
    );
    std::fs::write(fixture.target.join("artist/late.txt"), b"late child").unwrap();
    let before = snapshot(fixture.tree.path());
    assert_eq!(
        pin.members_observed(&flag, &mut no_progress).unwrap().len(),
        2
    );
    let result = pin.members_observed(&flag, &mut |_| {
        flag.store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    });
    assert_eq!(result.unwrap_err().kind, MutationErrorKind::Cancelled);
    drop(pin);
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
}

#[test]
fn source_root_membership_requires_present_grant_and_keeps_independent_cursor() {
    let fixture = present_fixture();
    let before = snapshot(fixture.tree.path());
    let owner = present_bound(&fixture);
    let flag = AtomicBool::new(false);
    let first = owner
        .source_root_members_observed(&flag, &mut no_progress)
        .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].identity, id(&fixture.source.join("artist")));
    assert_eq!(first[0].kind, EntryKind::Directory);
    assert_eq!(
        owner
            .source_root_members_observed(&flag, &mut no_progress)
            .unwrap(),
        first
    );
    let result = owner.source_root_members_observed(&flag, &mut |_| {
        flag.store(true, std::sync::atomic::Ordering::Release);
        Ok(())
    });
    assert_eq!(result.unwrap_err().kind, MutationErrorKind::Cancelled);
    drop(owner);
    assert_eq!(snapshot(fixture.tree.path()), before);
    let absent = Fixture::new();
    stale_result(
        absent
            .open()
            .source_root_members_observed(&AtomicBool::new(false), &mut no_progress),
    );
}
