//! Native selected-root capability acceptance. Every path is a generated fixture.
//! Compiling or skipping this file on Linux is not a Windows runtime pass.
#![cfg(all(windows, feature = "experimental-windows-mutation"))]

mod support;

use sorter_io::{
    desktop::windows_mutation::{
        WindowsMutationDomain, WindowsMutationStorage, WindowsRenameStorage,
        WindowsSelectedRootGrant,
    },
    mutation::*,
    storage::{ObjectIdentity, StorageRef},
};
use std::{
    fs::{self, OpenOptions},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::*};

fn name(value: &str) -> MutationName {
    MutationName::Windows(value.encode_utf16().collect())
}
fn reference(path: &Path) -> StorageRef {
    StorageRef::WindowsPath {
        utf16: path.as_os_str().encode_wide().collect(),
    }
}
fn identity(path: &Path) -> ObjectIdentity {
    let file = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS).0)
        .open(path)
        .unwrap();
    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .unwrap();
    ObjectIdentity::Windows {
        volume: info.VolumeSerialNumber,
        file_id: info.FileId.Identifier,
    }
}
fn grant(path: &Path) -> WindowsSelectedRootGrant {
    WindowsSelectedRootGrant::new(path.to_owned(), identity(path)).unwrap()
}
fn selected(source: &Path, target: &Path, journal: &Path) -> Result<WindowsMutationStorage, MutationError> {
    WindowsMutationStorage::for_selected_roots(grant(source), grant(target), grant(journal))
}
fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    for path in [&source, &target, &journal] {
        fs::create_dir(path).unwrap();
    }
    (tree, source, target, journal)
}
fn applied<T>(outcome: MutationOutcome<T>) -> T {
    match outcome {
        MutationOutcome::Applied(value) => value,
        MutationOutcome::NoEffect(error) => panic!("unexpected no-effect: {error:?}"),
        MutationOutcome::PendingUnknown(error) => panic!("unexpected uncertainty: {error:?}"),
    }
}

#[test]
fn selected_roots_are_independent_and_never_cleanup_on_drop() {
    let (tree, source, target, journal) = fixture();
    fs::write(source.join("song.mp3"), b"source remains").unwrap();
    fs::write(tree.path().join("outside.txt"), b"outside remains").unwrap();
    let storage = selected(&source, &target, &journal).unwrap();
    assert_eq!(storage.domain(), WindowsMutationDomain::SelectedRoots);
    assert_eq!(storage.selected_source_reference().unwrap(), reference(&source));
    assert_eq!(storage.selected_target_reference().unwrap(), reference(&target));
    assert_eq!(storage.selected_journal_reference().unwrap(), reference(&journal));
    assert!(matches!(storage.parent(Path::new("")), Err(error) if error.kind == MutationErrorKind::Unsupported));
    for path in [tree.path(), &tree.path().join("outside"), &source.join("../target")] {
        assert!(storage.parent_reference(&reference(path)).is_err());
    }
    let source_parent = storage.selected_source_root().unwrap();
    assert_eq!(source_parent.identity(), &identity(&source));
    assert!(fs::rename(&source, tree.path().join("source-replaced")).is_err());
    drop(storage);
    // The parent capability retains its exact root pins after storage is dropped.
    assert!(fs::rename(&source, tree.path().join("source-replaced")).is_err());
    drop(source_parent);
    assert_eq!(fs::read(source.join("song.mp3")).unwrap(), b"source remains");
    assert_eq!(fs::read(tree.path().join("outside.txt")).unwrap(), b"outside remains");
    assert!(target.is_dir());
    assert!(journal.is_dir());
}

#[test]
fn synthetic_constructor_keeps_its_distinct_domain_and_cleanup() {
    let tree = tempfile::tempdir().unwrap();
    let path = tree.path().to_owned();
    let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    assert_eq!(storage.domain(), WindowsMutationDomain::OwnedSyntheticTree);
    assert!(storage.selected_source_root().is_err());
    assert!(storage.selected_target_reference().is_err());
    assert!(storage.parent(Path::new("")).is_ok());
    drop(storage);
    assert!(!path.exists());
}

#[test]
fn full_expected_identity_is_rechecked_without_cleanup_on_failure() {
    let (_tree, source, target, journal) = fixture();
    fs::write(source.join("preserved"), b"unchanged").unwrap();
    let wrong = WindowsSelectedRootGrant::new(source.clone(), identity(&target)).unwrap();
    assert!(matches!(
        WindowsMutationStorage::for_selected_roots(wrong, grant(&target), grant(&journal)),
        Err(error) if error.kind == MutationErrorKind::StalePlan
    ));
    assert!(matches!(WindowsSelectedRootGrant::new(source.clone(), ObjectIdentity::Test { id: "not-native".into() }), Err(error) if error.kind == MutationErrorKind::Unsupported));
    assert_eq!(fs::read(source.join("preserved")).unwrap(), b"unchanged");
    assert!(target.is_dir());
    assert!(journal.is_dir());
}

#[test]
fn every_root_pair_rejects_equal_nested_and_case_alias_authority() {
    let (tree, source, target, journal) = fixture();
    let nested = source.join("nested");
    fs::create_dir(&nested).unwrap();
    let alias = tree.path().join("SOURCE");
    for (a, b, c) in [
        (&source, &source, &journal),
        (&source, &alias, &journal),
        (&source, &nested, &journal),
        (&nested, &source, &journal),
        (&source, &target, &source),
        (&source, &target, &nested),
        (&nested, &target, &source),
        (&target, &source, &source),
        (&target, &source, &nested),
        (&target, &nested, &source),
    ] {
        assert!(matches!(selected(a, b, c), Err(error) if error.kind == MutationErrorKind::StalePlan));
    }
    assert!(source.is_dir());
    assert!(nested.is_dir());
}

#[test]
fn root_roles_block_journal_data_and_swapped_source_destination_operations() {
    let (_tree, source, target, journal) = fixture();
    fs::write(source.join("song.mp3"), b"source").unwrap();
    fs::write(target.join("song.mp3"), b"target").unwrap();
    let mut storage = selected(&source, &target, &journal).unwrap();
    let source_parent = storage.selected_source_root().unwrap();
    let target_parent = storage.selected_target_root().unwrap();
    let journal_parent = storage.selected_journal_root().unwrap();
    let cancel = AtomicBool::new(false);
    for parent in [&source_parent, &target_parent] {
        assert!(matches!(storage.create_journal(parent, &name("forbidden")), Err(error) if error.kind == MutationErrorKind::StalePlan));
    }
    for parent in [&source_parent, &journal_parent] {
        assert!(matches!(storage.create_temp(parent, "forbidden", &cancel), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan));
        assert!(matches!(storage.create_destination_directory(parent, &name("forbidden"), &cancel), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan));
    }
    let observed_target = storage.observe_source(&target_parent, &name("song.mp3")).unwrap();
    assert!(matches!(storage.acquire_source(&target_parent, &observed_target), Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert!(storage.observe_source(&journal_parent, &name("anything")).is_err());
    let expected = storage.observe_source(&source_parent, &name("song.mp3")).unwrap();
    let mut lease = storage.acquire_source(&source_parent, &expected).unwrap();
    assert!(matches!(storage.rename_no_replace(&mut lease, &journal_parent, &name("forbidden"), &cancel), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan));
    drop(lease);
    assert_eq!(fs::read(source.join("song.mp3")).unwrap(), b"source");
    assert_eq!(fs::read(target.join("song.mp3")).unwrap(), b"target");
    assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
}

#[test]
fn foreign_scope_parents_are_not_adopted_even_for_same_selected_paths() {
    let (_tree, source, target, journal) = fixture();
    let mut a = selected(&source, &target, &journal).unwrap();
    let b = selected(&source, &target, &journal).unwrap();
    let foreign = b.selected_target_root().unwrap();
    assert!(matches!(a.create_temp(&foreign, "foreign", &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan));
    assert!(matches!(a.create_destination_directory(&foreign, &name("foreign"), &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
}

#[test]
fn selected_same_volume_rename_preserves_identity_without_expanding_durability() {
    let (_tree, source, target, journal) = fixture();
    fs::write(source.join("song.mp3"), b"exact contents").unwrap();
    let mut owned = selected(&source, &target, &journal).unwrap();
    let source_parent = owned.selected_source_root().unwrap();
    let target_parent = owned.selected_target_root().unwrap();
    let expected = owned.observe_source(&source_parent, &name("song.mp3")).unwrap();
    let mut storage = WindowsRenameStorage::new(owned);
    assert_eq!(storage.capabilities().durable_flush, Capability::Unknown);
    let mut lease = storage.acquire_source(&source_parent, &expected).unwrap();
    let published = applied(storage.rename_no_replace(&mut lease, &target_parent, &name("שיר-🎵.mp3"), &AtomicBool::new(false)));
    assert_eq!(storage.verify(&published, &expected.digest).unwrap().identity, expected.identity);
    drop(published);
    assert!(!source.join("song.mp3").exists());
    assert_eq!(fs::read(target.join("שיר-🎵.mp3")).unwrap(), b"exact contents");
}

#[test]
fn destination_directory_creation_is_exclusive_cancel_safe_and_handle_bound() {
    let (_tree, source, target, journal) = fixture();
    let storage = selected(&source, &target, &journal).unwrap();
    let parent = storage.selected_target_root().unwrap();
    assert!(matches!(storage.create_destination_directory(&parent, &name("cancelled"), &AtomicBool::new(true)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::Cancelled));
    assert!(!target.join("cancelled").exists());
    let artist = applied(storage.create_destination_directory(&parent, &name("אמן-🎵"), &AtomicBool::new(false)));
    assert_eq!(artist.identity(), &identity(&target.join("אמן-🎵")));
    assert!(fs::rename(target.join("אמן-🎵"), target.join("replaced")).is_err());
    assert!(matches!(storage.create_destination_directory(&parent, &name("אמן-🎵"), &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan));
    assert!(storage.parent_reference(&reference(&target.join("אמן-🎵"))).is_ok());
    let child = applied(storage.create_destination_directory(&artist, &name("nested"), &AtomicBool::new(false)));
    assert_eq!(child.identity(), &identity(&target.join("אמן-🎵/nested")));
    assert!(!target.join("replaced").exists());
}

#[test]
fn selected_journal_can_only_be_reopened_readonly_with_regranted_identity() {
    let (_tree, source, target, journal) = fixture();
    let storage = selected(&source, &target, &journal).unwrap();
    let mut writer = storage.create_journal(&storage.selected_journal_root().unwrap(), &name("execution.journal")).unwrap();
    writer.append(b"journal bytes").unwrap();
    let _ack = writer.flush_file_buffers().unwrap();
    let expected = writer.location().identity().clone();
    drop(writer);
    drop(storage);
    let storage = selected(&source, &target, &journal).unwrap();
    assert_eq!(storage.read_selected_journal(&name("execution.journal"), &expected).unwrap(), b"journal bytes");
    assert!(storage.read_selected_journal(&name("../source/song.mp3"), &expected).is_err());
    fs::rename(journal.join("execution.journal"), journal.join("retained.journal")).unwrap();
    fs::write(journal.join("execution.journal"), b"replacement bytes").unwrap();
    assert!(matches!(storage.read_selected_journal(&name("execution.journal"), &expected), Err(error) if error.kind == MutationErrorKind::StalePlan));
    assert_eq!(fs::read(journal.join("execution.journal")).unwrap(), b"replacement bytes");
    assert_eq!(fs::read(journal.join("retained.journal")).unwrap(), b"journal bytes");
}

#[test]
fn selected_reparse_root_and_in_place_conversion_are_rejected() {
    let (_tree, source, target, journal) = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("untouched"), b"outside").unwrap();
    let storage = selected(&source, &target, &journal).unwrap();
    let parent = storage.selected_target_root().unwrap();
    support::windows_reparse::set_junction(&target, outside.path());
    assert!(matches!(storage.create_destination_directory(&parent, &name("forbidden"), &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::UnsafeReparse));
    assert!(selected(&source, &target, &journal).is_err());
    assert!(!outside.path().join("forbidden").exists());
    assert_eq!(fs::read(outside.path().join("untouched")).unwrap(), b"outside");
}

#[test]
fn replacement_between_host_selection_and_pinning_is_rejected() {
    let (tree, source, target, journal) = fixture();
    let selected_source = grant(&source);
    fs::rename(&source, tree.path().join("previous-source")).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(source.join("replacement"), b"must not mutate").unwrap();
    assert!(matches!(
        WindowsMutationStorage::for_selected_roots(selected_source, grant(&target), grant(&journal)),
        Err(error) if error.kind == MutationErrorKind::StalePlan
    ));
    assert_eq!(fs::read(source.join("replacement")).unwrap(), b"must not mutate");
    assert!(tree.path().join("previous-source").is_dir());
}

#[test]
fn grants_reject_remote_device_anchor_and_ambiguous_paths_without_opening_them() {
    let (_tree, source, _target, _journal) = fixture();
    let expected = identity(&source);
    for path in [
        PathBuf::from(r"\\server\share\folder"),
        PathBuf::from(r"\\?\UNC\server\share\folder"),
        PathBuf::from(r"\\.\C:\folder"),
        PathBuf::from("C:\\"),
        PathBuf::from("relative"),
        source.join("./child"),
        source.join("../child"),
        source.join("child:stream"),
    ] {
        assert!(WindowsSelectedRootGrant::new(path, expected.clone()).is_err());
    }
}
