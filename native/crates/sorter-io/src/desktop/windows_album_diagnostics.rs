//! Owned-NTFS diagnostics only; never compiled into product code.

use super::*;
use std::{cell::Cell, ffi::OsString, os::windows::fs::OpenOptionsExt};
use windows::Wdk::Storage::FileSystem::{FILE_FULL_EA_INFORMATION, NtSetEaFile};

mod reparse_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/windows_reparse.rs"
    ));
}

fn name(value: &str) -> MutationName {
    MutationName::Windows(value.encode_utf16().collect())
}
fn directory_identity(path: &Path) -> ObjectIdentity {
    let handles = chain(path).unwrap();
    identity(handles.last().unwrap()).unwrap()
}
fn grant(path: &Path) -> WindowsSelectedRootGrant {
    WindowsSelectedRootGrant::new(path.to_path_buf(), directory_identity(path)).unwrap()
}
fn fixture() -> (
    WindowsMutationStorage,
    WindowsParent,
    ObjectIdentity,
    PathBuf,
) {
    let tree = tempfile::tempdir().unwrap();
    let path = tree.path().to_owned();
    std::fs::create_dir(path.join("album")).unwrap();
    let expected = directory_identity(&path.join("album"));
    let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    (storage, parent, expected, path)
}
fn applied<T>(outcome: MutationOutcome<T>) -> T {
    match outcome {
        MutationOutcome::Applied(value) => value,
        MutationOutcome::NoEffect(error) => panic!("expected Applied, got NoEffect: {error:?}"),
        MutationOutcome::PendingUnknown(error) => {
            panic!("expected Applied, got PendingUnknown: {error:?}")
        }
    }
}
fn no_effect<T>(outcome: MutationOutcome<T>) -> MutationError {
    match outcome {
        MutationOutcome::NoEffect(error) => error,
        _ => panic!("expected NoEffect"),
    }
}
fn pending<T>(outcome: MutationOutcome<T>, expected: &ObjectIdentity) {
    match outcome {
        MutationOutcome::PendingUnknown(error) => {
            assert_eq!(error.kind, MutationErrorKind::UnknownOutcome);
            assert_eq!(&error.observed.unwrap().identity, expected);
        }
        _ => panic!("dispatched cleanup uncertainty must stay PendingUnknown"),
    }
}
fn diagnostic_remove(
    storage: &WindowsMutationStorage,
    parent: &WindowsParent,
    name: &MutationName,
    expected: &ObjectIdentity,
    cancel: &AtomicBool,
) -> MutationOutcome<MutationObservation> {
    storage.remove_empty_source_directory_with(
        parent,
        name,
        expected,
        cancel,
        |_| Ok(()),
        close_directory_once,
    )
}
fn injected() -> MutationError {
    err(MutationErrorKind::Io, "owned fixture lost response")
}
fn set_ea(path: &Path) -> Result<(), MutationError> {
    let file = std::fs::OpenOptions::new()
        .access_mode(FILE_WRITE_EA.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
        .open(path)
        .map_err(io_error)?;
    let name = b"SORTER_ALBUM_TEST";
    let offset = std::mem::offset_of!(FILE_FULL_EA_INFORMATION, EaName);
    let length = offset + name.len() + 2;
    let mut allocation = vec![0u32; length.div_ceil(size_of::<u32>())];
    let pointer = allocation.as_mut_ptr().cast::<FILE_FULL_EA_INFORMATION>();
    unsafe {
        (*pointer).NextEntryOffset = 0;
        (*pointer).Flags = 0;
        (*pointer).EaNameLength = name.len() as u8;
        (*pointer).EaValueLength = 1;
        std::ptr::copy_nonoverlapping(name.as_ptr(), pointer.cast::<u8>().add(offset), name.len());
        *pointer.cast::<u8>().add(offset + name.len() + 1) = b'x';
    }
    let mut status_block = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtSetEaFile(
            HANDLE(file.as_raw_handle()),
            &mut status_block,
            pointer.cast(),
            length as u32,
        )
    };
    if status.0 != 0 || unsafe { status_block.Anonymous.Status }.0 != 0 {
        return Err(err(
            MutationErrorKind::Io,
            &format!("fixture EA failed: {status:?}"),
        ));
    }
    Ok(())
}

#[test]
fn inventory_lists_lossless_ids_with_every_child_file_lease_held() {
    let (mut storage, _, _, path) = fixture();
    std::fs::create_dir(path.join("album/disc")).unwrap();
    let names = [
        name("שיר𐐀.mp3"),
        MutationName::Windows(vec![0xd800, 46, 109, 112, 51]),
    ];
    for leaf in &names {
        std::fs::write(
            path.join("album")
                .join(OsString::from_wide(units(leaf).unwrap())),
            b"owned song",
        )
        .unwrap();
    }
    let album = storage.parent(Path::new("album")).unwrap();
    let expected = names
        .iter()
        .map(|leaf| storage.observe_source(&album, leaf).unwrap())
        .collect::<Vec<_>>();
    let leases = expected
        .iter()
        .map(|source| storage.acquire_source(&album, source).unwrap())
        .collect::<Vec<_>>();
    let members = storage.source_directory_members(&album).unwrap();
    assert_eq!(members.len(), 3);
    for source in expected {
        assert!(members.contains(&WindowsDirectoryMember {
            name: source.name,
            identity: source.identity,
            kind: EntryKind::File,
        }));
    }
    assert!(
        members
            .iter()
            .any(|member| member.name == name("disc") && member.kind == EntryKind::Directory)
    );
    // Ordinary child reopening really is denied; enumeration must not use it.
    assert!(
        File::open(
            path.join("album")
                .join(OsString::from_wide(units(&names[0]).unwrap()))
        )
        .is_err()
    );
    assert_eq!(storage.source_directory_members(&album).unwrap(), members);
    drop(leases);
}

#[test]
fn inventory_detects_late_add_remove_and_same_name_foreign_identity() {
    let (storage, _, _, path) = fixture();
    let album = storage.parent(Path::new("album")).unwrap();
    assert!(storage.source_directory_members(&album).unwrap().is_empty());
    std::fs::write(path.join("album/song.mp3"), b"first").unwrap();
    let first = storage.source_directory_members(&album).unwrap();
    assert_eq!(first.len(), 1);
    // Retain the old object at another name so the filesystem cannot reuse its ID.
    std::fs::rename(path.join("album/song.mp3"), path.join("album/original.mp3")).unwrap();
    std::fs::write(path.join("album/song.mp3"), b"foreign").unwrap();
    let changed = storage.source_directory_members(&album).unwrap();
    let replacement = changed
        .iter()
        .find(|member| member.name == name("song.mp3"))
        .unwrap();
    assert_ne!(replacement.identity, first[0].identity);
    assert_eq!(changed.len(), 2);
    std::fs::remove_file(path.join("album/song.mp3")).unwrap();
    std::fs::remove_file(path.join("album/original.mp3")).unwrap();
    assert!(storage.source_directory_members(&album).unwrap().is_empty());
}

#[test]
fn inventory_handles_root_nested_and_multi_buffer_directories_while_pinned() {
    let (storage, root, _, path) = fixture();
    std::fs::create_dir(path.join("album/disc")).unwrap();
    for n in 0..900 {
        std::fs::write(
            path.join("album/disc")
                .join(format!("{n:04}-owned-song.mp3")),
            b"song",
        )
        .unwrap();
    }
    let disc = storage.parent(Path::new("album/disc")).unwrap();
    assert_eq!(storage.source_directory_members(&root).unwrap().len(), 1);
    assert_eq!(storage.source_directory_members(&disc).unwrap().len(), 900);
    assert!(std::fs::rename(path.join("album"), path.join("replacement")).is_err());
    assert_eq!(storage.source_directory_members(&disc).unwrap().len(), 900);
}

#[test]
fn inventory_reports_reparse_child_without_following_it_and_rejects_changed_parent() {
    let (storage, root, _, path) = fixture();
    std::fs::create_dir(path.join("outside")).unwrap();
    std::fs::write(path.join("outside/sentinel"), b"outside").unwrap();
    reparse_fixture::set_junction(&path.join("album"), &path.join("outside"));
    let members = storage.source_directory_members(&root).unwrap();
    assert!(
        members
            .iter()
            .any(|member| member.name == name("album") && member.kind == EntryKind::LinkOrReparse)
    );
    assert!(storage.parent(Path::new("album")).is_err());
    std::fs::remove_dir(path.join("album")).unwrap();
    std::fs::create_dir(path.join("album")).unwrap();
    let album = storage.parent(Path::new("album")).unwrap();
    reparse_fixture::set_junction(&path.join("album"), &path.join("outside"));
    assert_eq!(
        storage.source_directory_members(&album).unwrap_err().kind,
        MutationErrorKind::UnsafeReparse
    );
    drop(album);
    std::fs::remove_dir(path.join("album")).unwrap();
    assert_eq!(
        std::fs::read(path.join("outside/sentinel")).unwrap(),
        b"outside"
    );
}

#[test]
fn empty_directory_stream_inventory_accepts_only_no_streams_and_cleanup_removes_link() {
    let (storage, parent, expected, path) = fixture();
    {
        let file =
            open_empty_directory_for_retirement(parent.handle(), units(&name("album")).unwrap())
                .unwrap();
        check_empty_directory_streams(&file).unwrap();
        check_empty_directory_representation(&file).unwrap();
    }
    let observed = applied(diagnostic_remove(
        &storage,
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
    ));
    assert_eq!(observed.identity, expected);
    assert_eq!(observed.parent_identity, *parent.identity());
    assert!(!path.join("album").exists());
    assert!(path.exists());
}

#[test]
fn nonempty_directory_is_preserved_and_kernel_rejects_last_instant_child() {
    for late in [false, true] {
        let (storage, parent, expected, path) = fixture();
        if !late {
            std::fs::write(path.join("album/sentinel"), b"retain").unwrap();
        }
        no_effect(storage.remove_empty_source_directory_with(
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false),
            |point| {
                if late && point == EmptyDirectoryCheckpoint::BeforeDisposition {
                    std::fs::write(path.join("album/sentinel"), b"retain").unwrap();
                }
                Ok(())
            },
            close_directory_once,
        ));
        assert_eq!(
            std::fs::read(path.join("album/sentinel")).unwrap(),
            b"retain"
        );
        assert_eq!(directory_identity(&path.join("album")), expected);
    }
}

#[test]
fn child_guards_must_be_released_but_parent_remains_pinned() {
    let (storage, parent, expected, path) = fixture();
    let child = storage.parent(Path::new("album")).unwrap();
    let clone = child.clone();
    drop(child);
    no_effect(diagnostic_remove(
        &storage,
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
    ));
    assert!(path.join("album").is_dir());
    assert!(std::fs::rename(&path, path.with_extension("moved")).is_err());
    drop(clone);
    applied(diagnostic_remove(
        &storage,
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
    ));
    assert!(path.exists());
}

#[test]
fn directory_named_stream_and_extended_attributes_are_preserved() {
    for stream in [false, true] {
        let (storage, parent, expected, path) = fixture();
        if stream {
            std::fs::write(path.join("album:notes"), b"retained stream").unwrap();
        } else {
            set_ea(&path.join("album")).unwrap();
        }
        let error = no_effect(diagnostic_remove(
            &storage,
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false),
        ));
        assert_eq!(error.kind, MutationErrorKind::Unsupported);
        assert_eq!(directory_identity(&path.join("album")), expected);
        if stream {
            assert_eq!(
                std::fs::read(path.join("album:notes")).unwrap(),
                b"retained stream"
            );
        } else {
            let file = open_empty_directory_for_retirement(
                parent.handle(),
                units(&name("album")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                check_repair_extended_attributes(&file).unwrap_err().kind,
                MutationErrorKind::Unsupported
            );
        }
    }
}

#[test]
fn directory_representation_is_rechecked_at_dispatch() {
    for stream in [false, true] {
        let (storage, parent, expected, path) = fixture();
        let changed = Cell::new(false);
        let outcome = storage.remove_empty_source_directory_with(
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false),
            |point| {
                if point == EmptyDirectoryCheckpoint::BeforeFinalChecks {
                    let result = if stream {
                        std::fs::write(path.join("album:late"), b"retain").map_err(io_error)
                    } else {
                        set_ea(&path.join("album"))
                    };
                    match result {
                        Ok(()) => changed.set(true),
                        Err(error) => assert_eq!(error.kind, MutationErrorKind::Locked),
                    }
                }
                Ok(())
            },
            close_directory_once,
        );
        if changed.get() {
            assert_eq!(no_effect(outcome).kind, MutationErrorKind::Unsupported);
            assert_eq!(directory_identity(&path.join("album")), expected);
            if stream {
                assert_eq!(std::fs::read(path.join("album:late")).unwrap(), b"retain");
            }
        } else {
            applied(outcome); // OS sharing prevented the competing metadata write.
            assert!(!path.join("album").exists());
        }
    }
}

#[test]
fn readonly_directory_is_refused_without_clearing_its_attribute() {
    let (storage, parent, expected, path) = fixture();
    let album = path.join("album");
    let mut permissions = std::fs::metadata(&album).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&album, permissions).unwrap();
    let error = no_effect(diagnostic_remove(
        &storage,
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
    ));
    assert!(matches!(
        error.kind,
        MutationErrorKind::Unsupported | MutationErrorKind::PermissionDenied
    ));
    let mut permissions = std::fs::metadata(&album).unwrap().permissions();
    assert!(permissions.readonly());
    permissions.set_readonly(false); // Owned fixture teardown only.
    std::fs::set_permissions(&album, permissions).unwrap();
    assert_eq!(directory_identity(&album), expected);
}

#[test]
fn foreign_identity_and_reparse_directory_are_never_deleted() {
    let (storage, parent, expected, path) = fixture();
    std::fs::rename(path.join("album"), path.join("original")).unwrap();
    std::fs::create_dir(path.join("album")).unwrap();
    let foreign = directory_identity(&path.join("album"));
    assert_ne!(foreign, expected);
    assert_eq!(
        no_effect(diagnostic_remove(
            &storage,
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false)
        ))
        .kind,
        MutationErrorKind::StalePlan
    );
    assert_eq!(directory_identity(&path.join("album")), foreign);
    reparse_fixture::set_junction(&path.join("album"), &path.join("original"));
    let error = no_effect(diagnostic_remove(
        &storage,
        &parent,
        &name("album"),
        &foreign,
        &AtomicBool::new(false),
    ));
    assert_eq!(error.kind, MutationErrorKind::UnsafeReparse);
    std::fs::remove_dir(path.join("album")).unwrap();
    assert_eq!(directory_identity(&path.join("original")), expected);
}

#[test]
fn cleanup_parent_reparse_change_before_dispatch_stops_without_following_it() {
    let (storage, parent, expected, path) = fixture();
    std::fs::create_dir(path.join("outside")).unwrap();
    std::fs::create_dir(path.join("outside/album")).unwrap();
    // A nonempty parent cannot become a junction; use another held empty
    // parent and its known sibling as the untrusted expected child instead.
    std::fs::create_dir(path.join("empty-parent")).unwrap();
    let empty_parent = storage.parent(Path::new("empty-parent")).unwrap();
    reparse_fixture::set_junction(&path.join("empty-parent"), &path.join("outside"));
    assert_eq!(
        no_effect(diagnostic_remove(
            &storage,
            &empty_parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false)
        ))
        .kind,
        MutationErrorKind::UnsafeReparse
    );
    assert!(path.join("outside/album").is_dir());
    drop(empty_parent);
    std::fs::remove_dir(path.join("empty-parent")).unwrap();
    applied(diagnostic_remove(
        &storage,
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
    ));
}

#[test]
fn cancellation_before_dispatch_is_no_effect_after_dispatch_is_pending() {
    let (storage, parent, expected, path) = fixture();
    let cancel = AtomicBool::new(true);
    assert_eq!(
        no_effect(diagnostic_remove(
            &storage,
            &parent,
            &name("album"),
            &expected,
            &cancel
        ))
        .kind,
        MutationErrorKind::Cancelled
    );
    assert!(path.join("album").is_dir());
    for point in [
        EmptyDirectoryCheckpoint::BeforeFinalChecks,
        EmptyDirectoryCheckpoint::BeforeDisposition,
        EmptyDirectoryCheckpoint::AfterDisposition,
        EmptyDirectoryCheckpoint::AfterClose,
    ] {
        let (storage, parent, expected, path) = fixture();
        let cancel = AtomicBool::new(false);
        let close_count = Cell::new(0);
        let outcome = storage.remove_empty_source_directory_with(
            &parent,
            &name("album"),
            &expected,
            &cancel,
            |current| {
                if current == point {
                    cancel.store(true, Ordering::Release);
                }
                Ok(())
            },
            |file| {
                close_count.set(close_count.get() + 1);
                close_directory_once(file)
            },
        );
        if matches!(
            point,
            EmptyDirectoryCheckpoint::BeforeFinalChecks
                | EmptyDirectoryCheckpoint::BeforeDisposition
        ) {
            assert_eq!(no_effect(outcome).kind, MutationErrorKind::Cancelled);
            assert_eq!(close_count.get(), 0);
            assert!(path.join("album").is_dir());
        } else {
            pending(outcome, &expected);
            assert_eq!(close_count.get(), 1);
            assert!(!path.join("album").exists());
        }
    }
}

#[test]
fn response_loss_and_close_response_loss_consume_handle_once_and_never_replay() {
    for loss in [
        EmptyDirectoryCheckpoint::AfterDisposition,
        EmptyDirectoryCheckpoint::AfterClose,
    ] {
        let (storage, parent, expected, path) = fixture();
        let close_count = Cell::new(0);
        let outcome = storage.remove_empty_source_directory_with(
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false),
            |point| {
                if point == loss {
                    Err(injected())
                } else {
                    Ok(())
                }
            },
            |file| {
                close_count.set(close_count.get() + 1);
                close_directory_once(file)
            },
        );
        pending(outcome, &expected);
        assert_eq!(close_count.get(), 1);
        assert!(!path.join("album").exists());
    }
    let (storage, parent, expected, path) = fixture();
    let close_count = Cell::new(0);
    let outcome = storage.remove_empty_source_directory_with(
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
        |_| Ok(()),
        |file| {
            close_count.set(close_count.get() + 1);
            close_directory_once(file)?;
            std::fs::create_dir(path.join("album")).unwrap();
            std::fs::write(path.join("album/foreign"), b"retain").unwrap();
            Err(injected()) // Real close succeeded, but its response was lost.
        },
    );
    pending(outcome, &expected);
    assert_eq!(close_count.get(), 1);
    assert_eq!(
        std::fs::read(path.join("album/foreign")).unwrap(),
        b"retain"
    );
}

#[test]
fn foreign_replacement_after_close_survives_success_and_observation_failure() {
    for fail in [false, true] {
        let (storage, parent, expected, path) = fixture();
        let foreign = std::cell::RefCell::new(None);
        let outcome = storage.remove_empty_source_directory_with(
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false),
            |point| {
                if point == EmptyDirectoryCheckpoint::AfterClose {
                    std::fs::create_dir(path.join("album")).unwrap();
                    std::fs::write(path.join("album/sentinel"), b"foreign").unwrap();
                    *foreign.borrow_mut() = Some(directory_identity(&path.join("album")));
                    if fail {
                        return Err(injected());
                    }
                }
                Ok(())
            },
            close_directory_once,
        );
        if fail {
            pending(outcome, &expected);
        } else {
            applied(outcome);
        }
        assert_eq!(
            std::fs::read(path.join("album/sentinel")).unwrap(),
            b"foreign"
        );
        assert_eq!(
            directory_identity(&path.join("album")),
            foreign.into_inner().unwrap()
        );
    }
}

#[test]
fn selected_roles_repair_and_root_boundary_are_enforced_without_cleanup() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    for path in [&source, &target, &journal] {
        std::fs::create_dir(path).unwrap();
    }
    for path in [&source, &target] {
        std::fs::create_dir(path.join("album")).unwrap();
    }
    let roots =
        WindowsMutationStorage::for_selected_roots(grant(&source), grant(&target), grant(&journal))
            .unwrap();
    let source_parent = roots.selected_source_root().unwrap();
    let target_parent = roots.selected_target_root().unwrap();
    assert!(roots.source_directory_members(&target_parent).is_err());
    no_effect(roots.remove_empty_source_directory(
        &target_parent,
        &name("album"),
        &directory_identity(&target.join("album")),
        &AtomicBool::new(false),
    ));
    no_effect(roots.remove_empty_source_directory(
        &source_parent,
        &name(".."),
        &directory_identity(&source),
        &AtomicBool::new(false),
    ));
    assert_eq!(
        no_effect(roots.remove_empty_source_directory(
            &source_parent,
            &name("album"),
            &directory_identity(&source.join("album")),
            &AtomicBool::new(false)
        ))
        .kind,
        MutationErrorKind::Unsupported
    );
    assert!(source.join("album").is_dir());
    assert!(target.join("album").is_dir());
    drop(source_parent);
    drop(target_parent);
    drop(roots);
    let repair =
        WindowsMutationStorage::for_selected_repair_roots(grant(&source), grant(&journal)).unwrap();
    let parent = repair.selected_repair_root().unwrap();
    assert_eq!(
        no_effect(repair.remove_empty_source_directory(
            &parent,
            &name("album"),
            &directory_identity(&source.join("album")),
            &AtomicBool::new(false)
        ))
        .kind,
        MutationErrorKind::Unsupported
    );
    assert!(source.join("album").is_dir());
    drop(parent);
    drop(repair);
    assert!(source.is_dir());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmptyDirectoryCheckpoint {
    BeforeFinalChecks,
    BeforeDisposition,
    AfterDisposition,
    AfterClose,
}

impl WindowsMutationStorage {
    // Owned-fixture diagnostics only. This code studies actual Windows behavior;
    // it is deliberately absent from production builds and is not an admission
    // mechanism for selected roots or a source-preservation guarantee.
    fn remove_empty_source_directory_with(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        expected: &ObjectIdentity,
        cancel: &AtomicBool,
        mut checkpoint: impl FnMut(EmptyDirectoryCheckpoint) -> Result<(), MutationError>,
        close: impl FnOnce(File) -> Result<(), MutationError>,
    ) -> MutationOutcome<MutationObservation> {
        let prepared = (|| {
            cancelled(cancel)?;
            if self.domain != WindowsMutationDomain::OwnedSyntheticTree {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "directory disposition diagnostics require an owned fixture",
                ));
            }
            self.check_role(parent, RootRole::Source)?;
            if !matches!(expected, ObjectIdentity::Windows { .. }) {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "full Windows directory identity required",
                ));
            }
            let file = open_empty_directory_for_retirement(parent.handle(), units(name)?)?;
            check_empty_directory_representation(&file)?;
            let observation = observe(&file, parent)?;
            if &observation.identity != expected || &observation.actual_name != name {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "cleanup directory identity/name changed",
                )
                .with_observation(observation));
            }
            if !directory_members(&file)?.is_empty() {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "cleanup directory is not empty",
                ));
            }
            checkpoint(EmptyDirectoryCheckpoint::BeforeFinalChecks)?;
            // Attribute/EA changes are not excluded by a share-denying handle.
            // Repeat these guards at dispatch; never infer preservation from an
            // earlier plan or from membership emptiness alone.
            self.check_role(parent, RootRole::Source)?;
            check_empty_directory_representation(&file)?;
            if !directory_members(&file)?.is_empty() {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "cleanup directory gained a member",
                ));
            }
            checkpoint(EmptyDirectoryCheckpoint::BeforeDisposition)?;
            cancelled(cancel)?;
            Ok((file, observation))
        })();
        let (file, observation) = match prepared {
            Ok(value) => value,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        // No recursive/path deletion, readonly override, ON_CLOSE update, or
        // fallback to weaker flags on older OS/filesystem implementations.
        let disposition = FILE_DISPOSITION_INFO_EX {
            Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        };
        if let Err(error) = unsafe {
            SetFileInformationByHandle(
                HANDLE(file.as_raw_handle()),
                FileDispositionInfoEx,
                (&disposition as *const FILE_DISPOSITION_INFO_EX).cast(),
                size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
            )
        } {
            return MutationOutcome::NoEffect(win_error(error));
        }
        // POSIX deletion becomes visible when this exact DELETE handle closes.
        // Always consume it once even when the disposition response is lost.
        let response = checkpoint(EmptyDirectoryCheckpoint::AfterDisposition);
        let closed = close(file);
        let verified = (|| {
            response?;
            closed?;
            checkpoint(EmptyDirectoryCheckpoint::AfterClose)?;
            cancelled(cancel)?;
            self.check_role(parent, RootRole::Source)?;
            match super::super::open_child(parent.handle(), units(name)?, false) {
                Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(()),
                Ok(other) if identity(&other).map_err(read_error)? != *expected => Ok(()),
                Ok(_) => Err(err(
                    MutationErrorKind::UnknownOutcome,
                    "original directory link remains after cleanup",
                )),
                Err(error) => Err(read_error(error)),
            }
        })();
        match verified {
            Ok(()) => MutationOutcome::Applied(observation),
            Err(error) => MutationOutcome::PendingUnknown(
                err(MutationErrorKind::UnknownOutcome,
                    &format!("empty-directory disposition dispatched; reconcile without replay: {error:?}"))
                    .with_observation(observation),
            ),
        }
    }
}

fn directory_members(directory: &File) -> Result<Vec<WindowsDirectoryMember>, MutationError> {
    directory_members_with_limits(directory, &AtomicBool::new(false), 100_000, 1024)
}

fn open_empty_directory_for_retirement(parent: &File, name: &[u16]) -> Result<File, MutationError> {
    ensure_directory(parent)?;
    let mut name = name.to_vec();
    let string = UNICODE_STRING {
        Length: (name.len() * 2) as u16,
        MaximumLength: (name.len() * 2) as u16,
        Buffer: PWSTR(name.as_mut_ptr()),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &string,
        Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE,
        ..Default::default()
    };
    let mut handle = HANDLE::default();
    let mut status_block = IO_STATUS_BLOCK::default();
    // SYNCHRONIZE + NONALERT makes the operation synchronous. FILE_OPEN cannot
    // create/truncate; no DELETE_ON_CLOSE means failures before disposition have
    // no deletion effect. No children are opened or recursively removed.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            DELETE | FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            &attributes,
            &mut status_block,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            FILE_SHARE_MODE(0),
            FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            None,
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "cleanup encountered reparse point",
        ));
    }
    if status.0 < 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(err(
            MutationErrorKind::Unsupported,
            "cleanup open returned no handle",
        ));
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    if status.0 != 0 || unsafe { status_block.Anonymous.Status }.0 != 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "cleanup open did not report synchronous completion",
        ));
    }
    Ok(file)
}

fn check_empty_directory_representation(file: &File) -> Result<(), MutationError> {
    ensure_directory(file)?;
    let attributes = repair_basic(file)?.FileAttributes;
    let ordinary = FILE_ATTRIBUTE_DIRECTORY.0
        | FILE_ATTRIBUTE_ARCHIVE.0
        | FILE_ATTRIBUTE_HIDDEN.0
        | FILE_ATTRIBUTE_SYSTEM.0
        | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0;
    if attributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0 || attributes & !ordinary != 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "directory cleanup rejects readonly or unsupported attributes",
        ));
    }
    check_empty_directory_streams(file)?;
    check_repair_extended_attributes(file)
}

fn check_empty_directory_streams(file: &File) -> Result<(), MutationError> {
    // Directories have no unnamed $DATA stream. Win32 explicitly documents
    // ERROR_HANDLE_EOF for FileStreamInfo with no returned streams. A successful
    // record, buffer overflow, unsupported call or access error is never emptiness.
    // u64 storage provides FILE_STREAM_INFO's required 8-byte alignment.
    let mut buffer = [0u64; 128];
    match unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileStreamInfo,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
        )
    } {
        Err(error) if error.code().0 as u32 & 0xffff == 38 => Ok(()), // ERROR_HANDLE_EOF
        Ok(()) => Err(err(
            MutationErrorKind::Unsupported,
            "directory has data streams",
        )),
        Err(error) => Err(err(
            MutationErrorKind::Unsupported,
            &format!("cannot exclude directory data streams: {error}"),
        )),
    }
}

fn close_directory_once(file: File) -> Result<(), MutationError> {
    use std::os::windows::io::IntoRawHandle;
    // Do not reconstruct File or retry after an uncertain CloseHandle response:
    // the numerical handle may already have been recycled for an unrelated object.
    let handle = HANDLE(file.into_raw_handle());
    unsafe { windows::Win32::Foundation::CloseHandle(handle) }.map_err(win_error)
}

#[test]
fn inventory_entry_and_batch_caps_and_cancellation_discard_partial_data() {
    let (storage, root, _, path) = fixture();
    let cancel = AtomicBool::new(true);
    assert_eq!(
        storage
            .source_directory_members_with_cancel(&root, &cancel)
            .unwrap_err()
            .kind,
        MutationErrorKind::Cancelled
    );
    for n in 0..900 {
        std::fs::write(
            path.join("album").join(format!("{n:04}-owned-song.mp3")),
            b"song",
        )
        .unwrap();
    }
    let directory =
        super::super::open_child(root.handle(), units(&name("album")).unwrap(), true).unwrap();
    let cancel = AtomicBool::new(false);
    assert_eq!(
        directory_members_with_limits(&directory, &cancel, 1, 1024)
            .unwrap_err()
            .kind,
        MutationErrorKind::Unsupported
    );
    assert_eq!(
        directory_members_with_limits(&directory, &cancel, 100_000, 1)
            .unwrap_err()
            .kind,
        MutationErrorKind::Unsupported
    );
    assert_eq!(
        directory_members_with_limits(&directory, &cancel, 100_000, 1024)
            .unwrap()
            .len(),
        900
    );
}

#[test]
fn product_cleanup_gate_never_dispatches_even_for_an_empty_owned_directory() {
    let (storage, parent, expected, path) = fixture();
    assert_eq!(
        no_effect(storage.remove_empty_source_directory(
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false)
        ))
        .kind,
        MutationErrorKind::Unsupported
    );
    assert_eq!(directory_identity(&path.join("album")), expected);
    assert!(path.join("album").is_dir());
}

#[test]
fn late_compatible_ea_write_is_possible_after_final_checks_so_product_delete_stays_disabled() {
    let (storage, parent, expected, path) = fixture();
    let close_count = Cell::new(0);
    let outcome = storage.remove_empty_source_directory_with(
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
        |point| {
            if point == EmptyDirectoryCheckpoint::BeforeDisposition {
                // Strict characterization, not a silent skip: FILE_WRITE_EA with
                // read/write/delete sharing must expose the preservation gap.
                set_ea(&path.join("album")).unwrap();
                return Err(injected()); // Stop the owned diagnostic before delete.
            }
            Ok(())
        },
        |file| {
            close_count.set(close_count.get() + 1);
            close_directory_once(file)
        },
    );
    assert_eq!(no_effect(outcome).kind, MutationErrorKind::Io);
    assert_eq!(close_count.get(), 0);
    assert_eq!(directory_identity(&path.join("album")), expected);
    let file = open_empty_directory_for_retirement(parent.handle(), units(&name("album")).unwrap())
        .unwrap();
    assert_eq!(
        check_repair_extended_attributes(&file).unwrap_err().kind,
        MutationErrorKind::Unsupported
    );
}

#[test]
fn late_compatible_ads_probe_never_authorizes_product_directory_deletion() {
    let (storage, parent, expected, path) = fixture();
    let inserted = Cell::new(false);
    let outcome = storage.remove_empty_source_directory_with(
        &parent,
        &name("album"),
        &expected,
        &AtomicBool::new(false),
        |point| {
            if point == EmptyDirectoryCheckpoint::BeforeDisposition {
                let attempt = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
                    .open(path.join("album:late"));
                match attempt {
                    Ok(mut stream) => {
                        stream.write_all(b"retained late stream").unwrap();
                        inserted.set(true);
                    }
                    Err(error) => assert_eq!(io_error(error).kind, MutationErrorKind::Locked),
                }
                return Err(injected()); // No disposition follows either outcome.
            }
            Ok(())
        },
        close_directory_once,
    );
    assert_eq!(no_effect(outcome).kind, MutationErrorKind::Io);
    assert_eq!(directory_identity(&path.join("album")), expected);
    if inserted.get() {
        assert_eq!(
            std::fs::read(path.join("album:late")).unwrap(),
            b"retained late stream"
        );
    }
    assert_eq!(
        no_effect(storage.remove_empty_source_directory(
            &parent,
            &name("album"),
            &expected,
            &AtomicBool::new(false)
        ))
        .kind,
        MutationErrorKind::Unsupported
    );
}
