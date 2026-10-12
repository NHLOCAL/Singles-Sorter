use super::*;
use std::os::windows::fs::OpenOptionsExt;

fn name(value: &str) -> MutationName {
    MutationName::Windows(value.encode_utf16().collect())
}
fn applied<T>(outcome: MutationOutcome<T>) -> T {
    match outcome {
        MutationOutcome::Applied(value) => value,
        MutationOutcome::NoEffect(error) => panic!("NoEffect: {error:?}"),
        MutationOutcome::PendingUnknown(error) => panic!("PendingUnknown: {error:?}"),
    }
}
fn no_effect<T>(outcome: MutationOutcome<T>) -> MutationError {
    match outcome {
        MutationOutcome::NoEffect(error) => error,
        _ => panic!("expected NoEffect"),
    }
}
fn pending<T>(outcome: MutationOutcome<T>) {
    match outcome {
        MutationOutcome::PendingUnknown(error) => {
            assert_eq!(error.kind, MutationErrorKind::UnknownOutcome)
        }
        _ => panic!("expected PendingUnknown"),
    }
}
fn set_basic(file: &File, access: i64, write: i64, attributes: u32) {
    let basic = FILE_BASIC_INFO {
        LastAccessTime: access,
        LastWriteTime: write,
        FileAttributes: attributes,
        ..Default::default()
    };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileBasicInfo,
            (&basic as *const FILE_BASIC_INFO).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    }
    .unwrap();
}
fn open_metadata(path: &Path) -> File {
    std::fs::OpenOptions::new()
        .access_mode((FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES).0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(path)
        .unwrap()
}
struct Fixture {
    storage: WindowsMutationStorage,
    source_parent: WindowsParent,
    target: WindowsParent,
    expected: ExpectedSource,
    path: PathBuf,
}
impl Fixture {
    fn new(readonly: bool) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_path_buf();
        std::fs::create_dir(path.join("source")).unwrap();
        std::fs::create_dir(path.join("target")).unwrap();
        std::fs::write(path.join("source/song.mp3"), b"owned song bytes").unwrap();
        let file = open_metadata(&path.join("source/song.mp3"));
        set_basic(
            &file,
            132_537_600_000_000_000,
            132_537_600_012_345_678,
            FILE_ATTRIBUTE_HIDDEN.0
                | if readonly {
                    FILE_ATTRIBUTE_READONLY.0
                } else {
                    0
                },
        );
        drop(file);
        let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let source_parent = storage.parent(Path::new("source")).unwrap();
        let target = storage.parent(Path::new("target")).unwrap();
        let expected = storage
            .observe_source(&source_parent, &name("song.mp3"))
            .unwrap();
        Self {
            storage,
            source_parent,
            target,
            expected,
            path,
        }
    }
    fn staged(&mut self) -> (WindowsSourceLease, WindowsOwnedTemp) {
        let mut source = self
            .storage
            .acquire_source(&self.source_parent, &self.expected)
            .unwrap();
        let mut temp = applied(self.storage.create_temp(
            &self.target,
            "metadata",
            &AtomicBool::new(false),
        ));
        source.rewind().unwrap();
        std::io::copy(&mut source, &mut temp).unwrap();
        temp.sync_data().unwrap();
        temp.verify_content(&self.expected.digest).unwrap();
        (source, temp)
    }
}

#[test]
fn file_metadata_matches_held_source_and_keeps_target_creation_attributes_and_acl() {
    for readonly in [false, true] {
        let mut f = Fixture::new(readonly);
        let cancel = AtomicBool::new(false);
        let (source, mut temp) = f.staged();
        let before = repair_basic(temp.file.as_ref().unwrap()).unwrap();
        let security = repair_security(temp.file.as_ref().unwrap()).unwrap();
        let evidence = applied(
            f.storage
                .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
        );
        let from = repair_basic(source.file.as_ref().unwrap()).unwrap();
        assert_eq!(evidence.source_identity, f.expected.identity);
        assert_eq!(evidence.destination_identity, temp.identity);
        assert_eq!(evidence.access_time, from.LastAccessTime);
        assert_eq!(evidence.write_time, from.LastWriteTime);
        assert_eq!(evidence.readonly, readonly);
        assert_eq!(evidence.retained_creation_time, before.CreationTime);
        assert_eq!(
            evidence.retained_attributes,
            retained_attributes(before.FileAttributes)
        );
        assert_eq!(
            repair_security(temp.file.as_ref().unwrap()).unwrap(),
            security
        );
        assert_eq!(temp.copy_metadata_evidence().unwrap(), &evidence);
        assert!(temp.write_all(b"must not write").is_err());
        no_effect(
            f.storage
                .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
        );
        temp.verify_content(&f.expected.digest).unwrap();
        let mut publication =
            applied(
                f.storage
                    .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
            );
        assert_eq!(publication.copy_metadata_evidence().unwrap(), &evidence);
        let _ = f
            .storage
            .sync_published_file(&mut publication, &f.expected.digest, &cancel)
            .unwrap();
        assert_eq!(publication.copy_metadata_evidence().unwrap(), &evidence);
        drop(publication);
        // This is an actual post-close metadata check; it intentionally does not
        // claim source atime remains equal after the source's later close.
        let reopened = open_metadata(&f.path.join("target/song.mp3"));
        check_copy_metadata(&reopened, &evidence).unwrap();
        drop(reopened);
        drop(source);
        assert_eq!(
            std::fs::read(f.path.join("source/song.mp3")).unwrap(),
            b"owned song bytes"
        );
        assert_eq!(
            std::fs::read(f.path.join("target/song.mp3")).unwrap(),
            b"owned song bytes"
        );
    }
}

#[test]
fn retained_destination_nonreadonly_attributes_are_not_replaced_with_source_flags() {
    let mut f = Fixture::new(false);
    let (source, mut temp) = f.staged();
    let file = temp.file.as_ref().unwrap();
    let before = repair_basic(file).unwrap();
    set_basic(
        file,
        0,
        0,
        FILE_ATTRIBUTE_SYSTEM.0 | FILE_ATTRIBUTE_ARCHIVE.0,
    );
    let evidence = applied(f.storage.finalize_selected_copy_metadata(
        &source,
        &mut temp,
        &AtomicBool::new(false),
    ));
    assert_eq!(
        evidence.retained_attributes,
        FILE_ATTRIBUTE_SYSTEM.0 | FILE_ATTRIBUTE_ARCHIVE.0
    );
    assert_eq!(evidence.retained_creation_time, before.CreationTime);
    assert_eq!(
        repair_basic(temp.file.as_ref().unwrap())
            .unwrap()
            .FileAttributes
            & FILE_ATTRIBUTE_HIDDEN.0,
        0
    );
}

#[test]
fn metadata_failure_boundaries_poison_temp_and_never_replay_or_publish() {
    for boundary in [
        CopyMetadataCheckpoint::AfterMetadata,
        CopyMetadataCheckpoint::AfterFlush,
        CopyMetadataCheckpoint::AfterVerification,
    ] {
        let mut f = Fixture::new(false);
        let (source, mut temp) = f.staged();
        let cancel = AtomicBool::new(false);
        pending(f.storage.finalize_selected_copy_metadata_with(
            &source,
            &mut temp,
            &cancel,
            |point| {
                if point == boundary {
                    return Err(err(MutationErrorKind::Io, "lost metadata response"));
                }
                Ok(())
            },
        ));
        assert!(temp.write_all(b"bad").is_err());
        assert!(temp.copy_metadata_evidence().is_err());
        no_effect(
            f.storage
                .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
        );
        no_effect(
            f.storage
                .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
        );
        assert!(!f.path.join("target/song.mp3").exists());
        drop(source);
        assert_eq!(
            std::fs::read(f.path.join("source/song.mp3")).unwrap(),
            b"owned song bytes"
        );
    }
}

#[test]
fn cancellation_before_dispatch_is_no_effect_and_after_dispatch_is_pending() {
    for boundary in [
        CopyMetadataCheckpoint::BeforeDispatch,
        CopyMetadataCheckpoint::AfterMetadata,
        CopyMetadataCheckpoint::AfterFlush,
        CopyMetadataCheckpoint::AfterVerification,
    ] {
        let mut f = Fixture::new(false);
        let (source, mut temp) = f.staged();
        let cancel = AtomicBool::new(false);
        let result =
            f.storage
                .finalize_selected_copy_metadata_with(&source, &mut temp, &cancel, |point| {
                    if point == boundary {
                        cancel.store(true, Ordering::Release);
                    }
                    Ok(())
                });
        if boundary == CopyMetadataCheckpoint::BeforeDispatch {
            assert_eq!(no_effect(result).kind, MutationErrorKind::Cancelled);
            assert!(temp.copy_metadata.is_none());
        } else {
            pending(result);
            assert!(temp.copy_metadata.as_ref().unwrap().poisoned);
        }
        assert!(!f.path.join("target/song.mp3").exists());
    }
}

#[test]
fn foreign_temp_and_wrong_content_fail_before_metadata_dispatch() {
    let mut f = Fixture::new(false);
    let mut foreign = Fixture::new(false);
    let (source, mut temp) = f.staged();
    let (_, mut other) = foreign.staged();
    no_effect(f.storage.finalize_selected_copy_metadata(
        &source,
        &mut other,
        &AtomicBool::new(false),
    ));
    assert!(other.copy_metadata.is_none());
    temp.write_all(b"altered").unwrap();
    no_effect(f.storage.finalize_selected_copy_metadata(
        &source,
        &mut temp,
        &AtomicBool::new(false),
    ));
    assert!(temp.copy_metadata.is_none());
}

#[test]
fn tampered_temp_metadata_is_detected_before_publication_and_source_is_retained() {
    let mut f = Fixture::new(false);
    let (source, mut temp) = f.staged();
    let cancel = AtomicBool::new(false);
    let evidence = applied(
        f.storage
            .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
    );
    set_basic(temp.file.as_ref().unwrap(), 0, evidence.write_time + 10, 0);
    no_effect(
        f.storage
            .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
    );
    assert!(temp.file.is_some());
    assert!(!f.path.join("target/song.mp3").exists());
}

#[test]
fn tampered_published_metadata_is_detected_by_content_verification() {
    let mut f = Fixture::new(false);
    let (source, mut temp) = f.staged();
    let cancel = AtomicBool::new(false);
    let evidence = applied(
        f.storage
            .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
    );
    let publication =
        applied(
            f.storage
                .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
        );
    set_basic(&publication.file, evidence.access_time + 10, 0, 0);
    assert!(f.storage.verify(&publication, &f.expected.digest).is_err());
}

#[test]
fn existing_destination_is_preserved_and_finalized_temp_does_not_become_writable() {
    let mut f = Fixture::new(false);
    let (source, mut temp) = f.staged();
    let cancel = AtomicBool::new(false);
    let _ = applied(
        f.storage
            .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
    );
    std::fs::write(f.path.join("target/song.mp3"), b"foreign").unwrap();
    no_effect(
        f.storage
            .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
    );
    assert!(temp.write_all(b"bad").is_err());
    assert_eq!(
        std::fs::read(f.path.join("target/song.mp3")).unwrap(),
        b"foreign"
    );
}

#[test]
fn ordinary_unfinalized_temps_keep_existing_behavior_and_have_no_metadata_receipt() {
    let mut f = Fixture::new(false);
    let (_, mut temp) = f.staged();
    assert!(temp.copy_metadata_evidence().is_err());
    temp.write_all(b"more").unwrap();
    assert!(temp.copy_metadata.is_none());
}

#[test]
fn created_directories_finalize_postorder_and_preserve_own_creation_and_attributes() {
    let f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    std::fs::create_dir(f.path.join("source/disc")).unwrap();
    let metadata = open_metadata(&f.path.join("source/disc"));
    set_basic(
        &metadata,
        132_537_601_000_000_000,
        132_537_601_111_111_111,
        FILE_ATTRIBUTE_DIRECTORY.0 | FILE_ATTRIBUTE_READONLY.0,
    );
    drop(metadata);
    let source = f.storage.parent(Path::new("source/disc")).unwrap();
    let mut root = applied(f.storage.create_selected_copy_directory(
        &f.target,
        &name("album"),
        &cancel,
    ));
    let mut child = applied(f.storage.create_selected_copy_directory(
        root.parent(),
        &name("disc"),
        &cancel,
    ));
    let before = repair_basic(root.directory.handle()).unwrap();
    let child_evidence = applied(
        f.storage
            .finalize_selected_copy_directory_metadata(&source, &mut child, &cancel),
    );
    let root_evidence = applied(f.storage.finalize_selected_copy_directory_metadata(
        &f.source_parent,
        &mut root,
        &cancel,
    ));
    assert_eq!(root_evidence.retained_creation_time, before.CreationTime);
    assert_eq!(
        f.storage
            .verify_selected_copy_directory_metadata(&child)
            .unwrap(),
        &child_evidence
    );
    assert_eq!(
        f.storage
            .verify_selected_copy_directory_metadata(&root)
            .unwrap(),
        &root_evidence
    );
    no_effect(
        f.storage
            .finalize_selected_copy_directory_metadata(&source, &mut child, &cancel),
    );
    drop(child);
    drop(root);
    check_copy_metadata(
        &open_metadata(&f.path.join("target/album/disc")),
        &child_evidence,
    )
    .unwrap();
    check_copy_metadata(&open_metadata(&f.path.join("target/album")), &root_evidence).unwrap();
}

#[test]
fn directory_creation_never_adopts_existing_foreign_file_or_directory() {
    for directory in [false, true] {
        let f = Fixture::new(false);
        if directory {
            std::fs::create_dir(f.path.join("target/album")).unwrap();
        } else {
            std::fs::write(f.path.join("target/album"), b"foreign").unwrap();
        }
        no_effect(f.storage.create_selected_copy_directory(
            &f.target,
            &name("album"),
            &AtomicBool::new(false),
        ));
        if !directory {
            assert_eq!(
                std::fs::read(f.path.join("target/album")).unwrap(),
                b"foreign"
            );
        }
    }
}

#[test]
fn directory_metadata_uncertainty_keeps_created_tree_and_disables_replay() {
    for boundary in [
        CopyMetadataCheckpoint::AfterMetadata,
        CopyMetadataCheckpoint::AfterVerification,
    ] {
        let f = Fixture::new(false);
        let cancel = AtomicBool::new(false);
        let mut directory = applied(f.storage.create_selected_copy_directory(
            &f.target,
            &name("album"),
            &cancel,
        ));
        std::fs::write(f.path.join("target/album/retained.txt"), b"retain").unwrap();
        pending(f.storage.finalize_selected_copy_directory_metadata_with(
            &f.source_parent,
            &mut directory,
            &cancel,
            |point| {
                if point == boundary {
                    return Err(err(
                        MutationErrorKind::Io,
                        "lost directory metadata response",
                    ));
                }
                Ok(())
            },
        ));
        assert!(
            f.storage
                .verify_selected_copy_directory_metadata(&directory)
                .is_err()
        );
        no_effect(f.storage.finalize_selected_copy_directory_metadata(
            &f.source_parent,
            &mut directory,
            &cancel,
        ));
        assert_eq!(
            std::fs::read(f.path.join("target/album/retained.txt")).unwrap(),
            b"retain"
        );
    }
}

#[test]
fn lost_directory_create_response_is_pending_and_never_replays_create() {
    let f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    pending(f.storage.create_selected_copy_directory_with(
        &f.target,
        &name("album"),
        &cancel,
        || Err(err(MutationErrorKind::Io, "lost create response")),
    ));
    assert!(f.path.join("target/album").is_dir());
    no_effect(
        f.storage
            .create_selected_copy_directory(&f.target, &name("album"), &cancel),
    );
}

#[test]
fn late_directory_metadata_change_is_observed_as_verification_failure() {
    let f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    let mut directory = applied(f.storage.create_selected_copy_directory(
        &f.target,
        &name("album"),
        &cancel,
    ));
    let evidence = applied(f.storage.finalize_selected_copy_directory_metadata(
        &f.source_parent,
        &mut directory,
        &cancel,
    ));
    set_basic(directory.directory.handle(), 0, evidence.write_time + 10, 0);
    assert!(
        f.storage
            .verify_selected_copy_directory_metadata(&directory)
            .is_err()
    );
}

#[test]
fn foreign_directory_token_is_rejected() {
    let f = Fixture::new(false);
    let foreign = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    let mut directory = applied(foreign.storage.create_selected_copy_directory(
        &foreign.target,
        &name("album"),
        &cancel,
    ));
    no_effect(f.storage.finalize_selected_copy_directory_metadata(
        &f.source_parent,
        &mut directory,
        &cancel,
    ));
    assert!(directory.metadata.is_none());
}

#[test]
fn explicit_selected_root_copy_preserves_readonly_source_and_reports_metadata_receipt() {
    let tree = tempfile::tempdir().unwrap();
    let source_path = tree.path().join("source");
    let target_path = tree.path().join("target");
    let journal_path = tree.path().join("journal");
    for path in [&source_path, &target_path, &journal_path] {
        std::fs::create_dir(path).unwrap();
    }
    std::fs::write(source_path.join("song.mp3"), b"selected fixture").unwrap();
    let file = open_metadata(&source_path.join("song.mp3"));
    set_basic(
        &file,
        132_537_600_000_000_000,
        132_537_600_012_345_678,
        FILE_ATTRIBUTE_READONLY.0,
    );
    drop(file);
    let grant = |path: &Path| {
        WindowsSelectedRootGrant::new(
            path.to_path_buf(),
            identity(chain(path).unwrap().last().unwrap()).unwrap(),
        )
        .unwrap()
    };
    let mut storage = WindowsMutationStorage::for_selected_roots(
        grant(&source_path),
        grant(&target_path),
        grant(&journal_path),
    )
    .unwrap();
    let source_parent = storage.selected_source_root().unwrap();
    let target = storage.selected_target_root().unwrap();
    let expected = storage
        .observe_source(&source_parent, &name("song.mp3"))
        .unwrap();
    let mut source = storage.acquire_source(&source_parent, &expected).unwrap();
    let cancel = AtomicBool::new(false);
    let mut temp = applied(storage.create_temp(&target, "selected-metadata", &cancel));
    source.rewind().unwrap();
    std::io::copy(&mut source, &mut temp).unwrap();
    temp.sync_data().unwrap();
    let evidence = applied(storage.finalize_selected_copy_metadata(&source, &mut temp, &cancel));
    let publication =
        applied(storage.publish_no_replace(&mut temp, &target, &name("song.mp3"), &cancel));
    assert_eq!(publication.copy_metadata_evidence().unwrap(), &evidence);
    assert!(evidence.readonly);
    assert_eq!(storage.capabilities().durable_flush, Capability::Unknown);
    drop(publication);
    drop(source);
    assert_eq!(
        std::fs::read(source_path.join("song.mp3")).unwrap(),
        b"selected fixture"
    );
    assert!(
        std::fs::metadata(source_path.join("song.mp3"))
            .unwrap()
            .permissions()
            .readonly()
    );
}

#[test]
fn selected_target_cannot_be_used_as_source_for_directory_copystat() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    for path in [&source, &target, &journal] {
        std::fs::create_dir(path).unwrap();
    }
    let grant = |path: &Path| {
        WindowsSelectedRootGrant::new(
            path.to_path_buf(),
            identity(chain(path).unwrap().last().unwrap()).unwrap(),
        )
        .unwrap()
    };
    let storage =
        WindowsMutationStorage::for_selected_roots(grant(&source), grant(&target), grant(&journal))
            .unwrap();
    let target = storage.selected_target_root().unwrap();
    let cancel = AtomicBool::new(false);
    let mut created =
        applied(storage.create_selected_copy_directory(&target, &name("album"), &cancel));
    no_effect(storage.finalize_selected_copy_directory_metadata(&target, &mut created, &cancel));
    assert!(created.metadata.is_none());
}

#[test]
fn copied_file_metadata_probe_matches_live_receipt_after_publication_guard_release() {
    let mut f = Fixture::new(true);
    let cancel = AtomicBool::new(false);
    let (source, mut temp) = f.staged();
    let evidence = applied(
        f.storage
            .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
    );
    let publication =
        applied(
            f.storage
                .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
        );
    assert!(
        f.storage
            .inspect_selected_copy_metadata(
                &f.target,
                &name("song.mp3"),
                &evidence.destination_identity,
                EntryKind::File
            )
            .is_err()
    );
    drop(publication);
    let observed = f
        .storage
        .inspect_selected_copy_metadata(
            &f.target,
            &name("song.mp3"),
            &evidence.destination_identity,
            EntryKind::File,
        )
        .unwrap()
        .unwrap();
    assert_eq!(observed.observation.identity, evidence.destination_identity);
    assert_eq!(observed.kind, EntryKind::File);
    assert_eq!(observed.creation_time, evidence.retained_creation_time);
    assert_eq!(observed.access_time, evidence.access_time);
    assert_eq!(observed.write_time, evidence.write_time);
    assert_eq!(observed.readonly, evidence.readonly);
    assert_eq!(observed.retained_attributes, evidence.retained_attributes);
}

#[test]
fn created_directory_metadata_probe_checks_same_leaf_kind_and_observed_fields() {
    let f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    let mut created = applied(f.storage.create_selected_copy_directory(
        &f.target,
        &name("album"),
        &cancel,
    ));
    let evidence = applied(f.storage.finalize_selected_copy_directory_metadata(
        &f.source_parent,
        &mut created,
        &cancel,
    ));
    let observed = f
        .storage
        .inspect_selected_copy_metadata(
            &f.target,
            &name("album"),
            &evidence.destination_identity,
            EntryKind::Directory,
        )
        .unwrap()
        .unwrap();
    assert_eq!(observed.creation_time, evidence.retained_creation_time);
    assert_eq!(observed.access_time, evidence.access_time);
    assert_eq!(observed.write_time, evidence.write_time);
    assert_eq!(observed.readonly, evidence.readonly);
    assert_eq!(observed.retained_attributes, evidence.retained_attributes);
    let mismatch = f
        .storage
        .inspect_selected_copy_metadata(
            &f.target,
            &name("album"),
            &evidence.destination_identity,
            EntryKind::File,
        )
        .unwrap_err();
    assert_eq!(mismatch.kind, MutationErrorKind::StalePlan);
    assert_eq!(
        mismatch.observed.unwrap().identity,
        evidence.destination_identity
    );
}

#[test]
fn copied_metadata_probe_reports_absence_or_foreign_identity_without_adoption() {
    let mut f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    let (source, mut temp) = f.staged();
    let evidence = applied(
        f.storage
            .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
    );
    assert_eq!(
        f.storage
            .inspect_selected_copy_metadata(
                &f.target,
                &name("missing"),
                &evidence.destination_identity,
                EntryKind::File
            )
            .unwrap(),
        None
    );
    let publication =
        applied(
            f.storage
                .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
        );
    drop(publication);
    std::fs::rename(
        f.path.join("target/song.mp3"),
        f.path.join("target/retained.mp3"),
    )
    .unwrap();
    std::fs::write(f.path.join("target/song.mp3"), b"foreign").unwrap();
    let error = f
        .storage
        .inspect_selected_copy_metadata(
            &f.target,
            &name("song.mp3"),
            &evidence.destination_identity,
            EntryKind::File,
        )
        .unwrap_err();
    assert_eq!(error.kind, MutationErrorKind::StalePlan);
    assert_ne!(
        error.observed.unwrap().identity,
        evidence.destination_identity
    );
    assert_eq!(
        std::fs::read(f.path.join("target/song.mp3")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        std::fs::read(f.path.join("target/retained.mp3")).unwrap(),
        b"owned song bytes"
    );
}

#[test]
fn copied_metadata_probe_refuses_reparse_target_without_reading_outside_tree() {
    mod reparse_fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/windows_reparse.rs"
        ));
    }
    let f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    let created = applied(f.storage.create_selected_copy_directory(
        &f.target,
        &name("album"),
        &cancel,
    ));
    let expected = created.observation().identity.clone();
    drop(created);
    std::fs::create_dir(f.path.join("outside")).unwrap();
    std::fs::write(f.path.join("outside/sentinel"), b"outside").unwrap();
    reparse_fixture::set_junction(&f.path.join("target/album"), &f.path.join("outside"));
    assert_eq!(
        f.storage
            .inspect_selected_copy_metadata(
                &f.target,
                &name("album"),
                &expected,
                EntryKind::Directory
            )
            .unwrap_err()
            .kind,
        MutationErrorKind::UnsafeReparse
    );
    assert_eq!(
        std::fs::read(f.path.join("outside/sentinel")).unwrap(),
        b"outside"
    );
    std::fs::remove_dir(f.path.join("target/album")).unwrap();
}

#[test]
fn copied_metadata_probe_rejects_foreign_parent_and_selected_source_role() {
    let f = Fixture::new(false);
    let foreign = Fixture::new(false);
    assert!(
        f.storage
            .inspect_selected_copy_metadata(
                &foreign.target,
                &name("missing"),
                &f.expected.identity,
                EntryKind::File
            )
            .is_err()
    );
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    for path in [&source, &target, &journal] {
        std::fs::create_dir(path).unwrap();
    }
    let grant = |path: &Path| {
        WindowsSelectedRootGrant::new(
            path.to_path_buf(),
            identity(chain(path).unwrap().last().unwrap()).unwrap(),
        )
        .unwrap()
    };
    let storage =
        WindowsMutationStorage::for_selected_roots(grant(&source), grant(&target), grant(&journal))
            .unwrap();
    let source_parent = storage.selected_source_root().unwrap();
    assert!(
        storage
            .inspect_selected_copy_metadata(
                &source_parent,
                &name("missing"),
                &f.expected.identity,
                EntryKind::File
            )
            .is_err()
    );
}

#[test]
fn copied_metadata_probe_requires_exact_leaf_spelling_and_rejects_stream_paths() {
    let mut f = Fixture::new(false);
    let cancel = AtomicBool::new(false);
    let (source, mut temp) = f.staged();
    let evidence = applied(
        f.storage
            .finalize_selected_copy_metadata(&source, &mut temp, &cancel),
    );
    let publication =
        applied(
            f.storage
                .publish_no_replace(&mut temp, &f.target, &name("song.mp3"), &cancel),
        );
    drop(publication);
    let error = f
        .storage
        .inspect_selected_copy_metadata(
            &f.target,
            &name("SONG.MP3"),
            &evidence.destination_identity,
            EntryKind::File,
        )
        .unwrap_err();
    assert_eq!(error.kind, MutationErrorKind::StalePlan);
    assert_eq!(error.observed.unwrap().actual_name, name("song.mp3"));
    assert_eq!(
        f.storage
            .inspect_selected_copy_metadata(
                &f.target,
                &name("song.mp3:notes"),
                &evidence.destination_identity,
                EntryKind::File
            )
            .unwrap_err()
            .kind,
        MutationErrorKind::InvalidName
    );
}
