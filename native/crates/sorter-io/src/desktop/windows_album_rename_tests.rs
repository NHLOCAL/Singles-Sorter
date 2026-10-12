use super::*;
use std::os::windows::fs::OpenOptionsExt;
use windows::Wdk::Storage::FileSystem::{FILE_FULL_EA_INFORMATION, NtSetEaFile};

mod reparse_fixture {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/windows_reparse.rs"));
}
fn name(value: &str) -> MutationName { MutationName::Windows(value.encode_utf16().collect()) }
fn id(path: &Path) -> ObjectIdentity { identity(chain(path).unwrap().last().unwrap()).unwrap() }
fn grant(path: &Path) -> WindowsSelectedRootGrant { WindowsSelectedRootGrant::new(path.to_path_buf(), id(path)).unwrap() }
fn digest_bytes(bytes: &[u8]) -> ContentDigest { ContentDigest { sha256: Sha256::digest(bytes).into(), length: bytes.len() as u64 } }
fn entry(root: &Path, path: &[&str], kind: EntryKind) -> WindowsAlbumTreeEntry {
    let full = path.iter().fold(root.to_path_buf(), |parent, leaf| parent.join(leaf));
    WindowsAlbumTreeEntry {
        relative: path.iter().map(|part| name(part)).collect(), identity: id(&full), kind,
        digest: (kind == EntryKind::File).then(|| digest_bytes(&std::fs::read(&full).unwrap())),
    }
}
fn make_tree(root: &Path) -> Vec<WindowsAlbumTreeEntry> {
    std::fs::create_dir(root).unwrap();
    std::fs::create_dir(root.join("disc")).unwrap();
    std::fs::write(root.join("שיר𐐀.mp3"), b"first song").unwrap();
    std::fs::write(root.join("disc/track.mp3"), b"second song").unwrap();
    // Deliberately not sorted; native verification must compare sets, not the
    // planner's transfer order or a filesystem's enumeration order.
    vec![entry(root, &["שיר𐐀.mp3"], EntryKind::File),
        entry(root, &["disc", "track.mp3"], EntryKind::File),
        entry(root, &["disc"], EntryKind::Directory)]
}
fn fixture() -> (WindowsMutationStorage, WindowsParent, WindowsParent, ObjectIdentity, Vec<WindowsAlbumTreeEntry>, PathBuf) {
    let tree = tempfile::tempdir().unwrap();
    let path = tree.path().to_path_buf();
    std::fs::create_dir(path.join("source")).unwrap();
    std::fs::create_dir(path.join("target")).unwrap();
    let entries = make_tree(&path.join("source/album"));
    let expected = id(&path.join("source/album"));
    let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let source = storage.parent(Path::new("source")).unwrap();
    let target = storage.parent(Path::new("target")).unwrap();
    (storage, source, target, expected, entries, path)
}
fn applied<T>(outcome: MutationOutcome<T>) -> T {
    match outcome {
        MutationOutcome::Applied(value) => value,
        MutationOutcome::NoEffect(error) => panic!("NoEffect: {error:?}"),
        MutationOutcome::PendingUnknown(error) => panic!("PendingUnknown: {error:?}"),
    }
}
fn no_effect<T>(outcome: MutationOutcome<T>) -> MutationError {
    match outcome { MutationOutcome::NoEffect(error) => error, _ => panic!("expected NoEffect") }
}
fn pending<T>(outcome: MutationOutcome<T>) -> MutationError {
    match outcome {
        MutationOutcome::PendingUnknown(error) => { assert_eq!(error.kind, MutationErrorKind::UnknownOutcome); error }
        _ => panic!("expected PendingUnknown"),
    }
}
fn set_ea(path: &Path) {
    let file = std::fs::OpenOptions::new().access_mode(FILE_WRITE_EA.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0).open(path).unwrap();
    let name = b"SORTER_ALBUM_MOVE";
    let offset = std::mem::offset_of!(FILE_FULL_EA_INFORMATION, EaName);
    let length = offset + name.len() + 2;
    let mut bytes = vec![0u32; length.div_ceil(4)];
    let pointer = bytes.as_mut_ptr().cast::<FILE_FULL_EA_INFORMATION>();
    unsafe {
        (*pointer).EaNameLength = name.len() as u8;
        (*pointer).EaValueLength = 1;
        std::ptr::copy_nonoverlapping(name.as_ptr(), pointer.cast::<u8>().add(offset), name.len());
        *pointer.cast::<u8>().add(offset + name.len() + 1) = b'x';
    }
    let mut status = IO_STATUS_BLOCK::default();
    assert_eq!(unsafe { NtSetEaFile(HANDLE(file.as_raw_handle()), &mut status, pointer.cast(), length as u32) }.0, 0);
    assert_eq!(unsafe { status.Anonymous.Status }.0, 0);
}

#[test]
fn child_album_moves_same_tree_container_streams_and_eas_without_recreating_objects() {
    let (storage, source, target, expected, entries, path) = fixture();
    std::fs::write(path.join("source/album:notes"), b"album stream").unwrap();
    std::fs::write(path.join("source/album/disc/track.mp3:notes"), b"track stream").unwrap();
    set_ea(&path.join("source/album"));
    set_ea(&path.join("source/album/disc/track.mp3"));
    let cancel = AtomicBool::new(false);
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries.clone(), &cancel).unwrap();
    let published = applied(storage.rename_album_directory(&mut lease, &target, &name("new album"), &cancel));
    assert!(lease.file.is_none());
    assert_eq!(published.observation().identity, expected);
    assert_eq!(published.entries().len(), entries.len());
    assert_eq!(storage.verify_album_publication(&published, &cancel).unwrap().as_slice(), published.entries());
    for expected_entry in entries {
        let actual = published.entries().iter().find(|entry| entry.relative == expected_entry.relative).unwrap();
        assert_eq!(actual.observation.identity, expected_entry.identity);
    }
    assert!(check_repair_extended_attributes(&published.file).is_err());
    assert!(!path.join("source/album").exists());
    drop(published);
    assert_eq!(std::fs::read(path.join("target/new album:notes")).unwrap(), b"album stream");
    assert_eq!(std::fs::read(path.join("target/new album/disc/track.mp3:notes")).unwrap(), b"track stream");
    assert_eq!(std::fs::read(path.join("target/new album/disc/track.mp3")).unwrap(), b"second song");
    assert_eq!(id(&path.join("target/new album")), expected);
}

#[test]
fn actual_selected_source_root_album_moves_with_usable_target_and_journal_authority() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source album");
    let target = tree.path().join("target");
    let journal = tree.path().join("journals");
    let entries = make_tree(&source);
    std::fs::create_dir(&target).unwrap(); std::fs::create_dir(&journal).unwrap();
    let expected = id(&source);
    let cancel = AtomicBool::new(false);
    let mut session = WindowsMutationStorage::for_selected_album_root_rename(grant(&source), grant(&target), grant(&journal), entries, &cancel).unwrap();
    assert_eq!(session.selected_source_identity(), &expected);
    assert_eq!(session.selected_source_reference(), &StorageRef::WindowsPath { utf16: source.as_os_str().encode_wide().collect() });
    assert!(session.target_and_journal().selected_source_root().is_err());
    let destination = session.target_and_journal().selected_target_root().unwrap();
    let journal_parent = session.target_and_journal().selected_journal_root().unwrap();
    let mut writer = session.target_and_journal().create_journal(&journal_parent, &name("album.log")).unwrap();
    writer.append(b"prepared").unwrap(); let _ = writer.flush_file_buffers().unwrap();
    let published = applied(session.rename_album_directory(&destination, &name("album"), &cancel));
    writer.append(b"observed").unwrap(); let _ = writer.flush_file_buffers().unwrap();
    assert_eq!(session.verify_album_publication(&published, &cancel).unwrap().len(), 3);
    assert!(!source.exists());
    let location = writer.location(); drop(writer);
    assert_eq!(session.target_and_journal().read_journal(&location).unwrap(), b"preparedobserved");
    drop(published); drop(session);
    assert_eq!(id(&target.join("album")), expected);
    assert_eq!(std::fs::read(target.join("album/disc/track.mp3")).unwrap(), b"second song");
}

#[test]
fn existing_album_destination_is_never_merged_or_replaced() {
    for directory in [false, true] {
        let (storage, source, target, expected, entries, path) = fixture();
        if directory { std::fs::create_dir(path.join("target/album")).unwrap(); }
        else { std::fs::write(path.join("target/album"), b"foreign").unwrap(); }
        let cancel = AtomicBool::new(false);
        let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
        no_effect(storage.rename_album_directory(&mut lease, &target, &name("album"), &cancel));
        assert!(lease.file.is_some()); assert!(path.join("source/album/disc/track.mp3").exists());
        if directory { assert!(path.join("target/album").is_dir()); }
        else { assert_eq!(std::fs::read(path.join("target/album")).unwrap(), b"foreign"); }
    }
}

#[test]
fn album_preflight_rejects_changed_membership_content_and_identity() {
    for change in 0..3 {
        let (storage, source, target, expected, entries, path) = fixture();
        let cancel = AtomicBool::new(false);
        let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
        match change {
            0 => std::fs::write(path.join("source/album/late.txt"), b"late").unwrap(),
            1 => std::fs::write(path.join("source/album/disc/track.mp3"), b"changed").unwrap(),
            _ => {
                std::fs::rename(path.join("source/album/disc/track.mp3"), path.join("source/album/disc/original.mp3")).unwrap();
                std::fs::write(path.join("source/album/disc/track.mp3"), b"second song").unwrap();
            }
        }
        no_effect(storage.rename_album_directory(&mut lease, &target, &name("album"), &cancel));
        assert!(path.join("source/album").exists()); assert!(!path.join("target/album").exists());
    }
}

#[test]
fn last_instant_external_change_moves_whole_object_but_reports_pending() {
    let (storage, source, target, expected, entries, path) = fixture();
    let cancel = AtomicBool::new(false);
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
    pending(storage.rename_album_directory_with(&mut lease, &target, &name("album"), &cancel, (false, |point| {
        if point == AlbumRenameCheckpoint::BeforeRename { std::fs::write(path.join("source/album/late.txt"), b"retain late data").unwrap(); }
        Ok(())
    })));
    assert!(lease.file.is_none()); assert!(!path.join("source/album").exists());
    assert_eq!(std::fs::read(path.join("target/album/late.txt")).unwrap(), b"retain late data");
    assert_eq!(std::fs::read(path.join("target/album/disc/track.mp3")).unwrap(), b"second song");
}

#[test]
fn album_cancellation_and_lost_response_never_replay_a_dispatched_rename() {
    for point in [AlbumRenameCheckpoint::BeforeFinalVerification, AlbumRenameCheckpoint::BeforeRename,
        AlbumRenameCheckpoint::AfterRename, AlbumRenameCheckpoint::AfterVerification] {
        let (storage, source, target, expected, entries, path) = fixture();
        let cancel = AtomicBool::new(false);
        let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
        let result = storage.rename_album_directory_with(&mut lease, &target, &name("album"), &cancel, (false, |current| {
            if current == point { cancel.store(true, Ordering::Release); } Ok(())
        }));
        if matches!(point, AlbumRenameCheckpoint::BeforeFinalVerification | AlbumRenameCheckpoint::BeforeRename) {
            assert_eq!(no_effect(result).kind, MutationErrorKind::Cancelled);
            assert!(path.join("source/album").exists()); assert!(lease.file.is_some());
        } else {
            pending(result); assert!(!path.join("source/album").exists()); assert!(lease.file.is_none());
            assert_eq!(std::fs::read(path.join("target/album/disc/track.mp3")).unwrap(), b"second song");
        }
    }
    let (storage, source, target, expected, entries, path) = fixture();
    let cancel = AtomicBool::new(false);
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
    pending(storage.rename_album_directory_with(&mut lease, &target, &name("album"), &cancel, (false, |point| {
        if point == AlbumRenameCheckpoint::AfterRename { Err(err(MutationErrorKind::Io, "owned lost response")) } else { Ok(()) }
    })));
    assert_eq!(no_effect(storage.rename_album_directory(&mut lease, &target, &name("again"), &cancel)).kind, MutationErrorKind::StalePlan);
    assert!(path.join("target/album").exists()); assert!(!path.join("target/again").exists());
}

#[test]
fn ordinary_child_pins_and_file_leases_are_not_weakened_for_album_rename() {
    let (mut storage, source, target, expected, entries, path) = fixture();
    let cancel = AtomicBool::new(false);
    let album_pin = storage.parent(Path::new("source/album")).unwrap();
    assert!(storage.acquire_album_directory(&source, &name("album"), &expected, entries.clone(), &cancel).is_err());
    let expected_file = storage.observe_source(&album_pin, &name("שיר𐐀.mp3")).unwrap();
    let file_lease = storage.acquire_source(&album_pin, &expected_file).unwrap();
    drop(album_pin);
    assert!(storage.acquire_album_directory(&source, &name("album"), &expected, entries.clone(), &cancel).is_err());
    drop(file_lease);
    std::fs::write(path.join("source/other.mp3"), b"other cohort").unwrap();
    let other_expected = storage.observe_source(&source, &name("other.mp3")).unwrap();
    let other_lease = storage.acquire_source(&source, &other_expected).unwrap();
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
    let published = applied(storage.rename_album_directory(&mut lease, &target, &name("album"), &cancel));
    assert!(File::open(path.join("source/other.mp3")).is_err());
    drop(published); drop(other_lease);
    assert_eq!(std::fs::read(path.join("source/other.mp3")).unwrap(), b"other cohort");
}

#[test]
fn album_reparse_descendant_or_foreign_root_is_never_traversed_or_moved() {
    let (storage, source, _, expected, entries, path) = fixture();
    std::fs::create_dir(path.join("outside")).unwrap();
    std::fs::write(path.join("outside/sentinel"), b"outside").unwrap();
    std::fs::remove_file(path.join("source/album/disc/track.mp3")).unwrap();
    reparse_fixture::set_junction(&path.join("source/album/disc"), &path.join("outside"));
    assert!(storage.acquire_album_directory(&source, &name("album"), &expected, entries, &AtomicBool::new(false)).is_err());
    std::fs::remove_dir(path.join("source/album/disc")).unwrap();
    std::fs::rename(path.join("source/album"), path.join("source/original")).unwrap();
    std::fs::create_dir(path.join("source/album")).unwrap();
    assert!(storage.acquire_album_directory(&source, &name("album"), &expected, vec![], &AtomicBool::new(false)).is_err());
    assert_eq!(std::fs::read(path.join("outside/sentinel")).unwrap(), b"outside");
    assert!(path.join("source/album").is_dir()); assert!(path.join("source/original").is_dir());
}

#[test]
fn album_manifest_rejects_missing_parent_duplicate_path_invalid_name_and_digest() {
    let (storage, source, _, expected, entries, _) = fixture();
    let cancel = AtomicBool::new(false);
    for change in 0..5 {
        let mut changed = entries.clone();
        match change {
            0 => changed.retain(|entry| entry.kind != EntryKind::Directory),
            1 => changed.push(changed[0].clone()),
            2 => changed[0].relative[0] = name(".."),
            3 => changed[0].digest = None,
            _ => changed[0].digest.as_mut().unwrap().sha256[0] ^= 1,
        }
        assert!(storage.acquire_album_directory(&source, &name("album"), &expected, changed, &cancel).is_err());
    }
}

#[test]
fn foreign_old_name_created_after_album_rename_is_preserved() {
    let (storage, source, target, expected, entries, path) = fixture();
    let cancel = AtomicBool::new(false);
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
    let published = applied(storage.rename_album_directory_with(&mut lease, &target, &name("album"), &cancel, (false, |point| {
        if point == AlbumRenameCheckpoint::AfterRename {
            std::fs::create_dir(path.join("source/album")).unwrap();
            std::fs::write(path.join("source/album/sentinel"), b"foreign").unwrap();
        }
        Ok(())
    })));
    assert_eq!(published.observation().identity, expected);
    assert_eq!(std::fs::read(path.join("source/album/sentinel")).unwrap(), b"foreign");
}

#[test]
fn post_move_child_access_failure_keeps_whole_album_and_reports_pending() {
    let (storage, source, target, expected, entries, path) = fixture();
    let cancel = AtomicBool::new(false);
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
    let mut blocker = None;
    pending(storage.rename_album_directory_with(&mut lease, &target, &name("album"), &cancel, (false, |point| {
        if point == AlbumRenameCheckpoint::AfterRename {
            blocker = Some(std::fs::OpenOptions::new().read(true).share_mode(0).open(path.join("target/album/disc/track.mp3")).unwrap());
        }
        Ok(())
    })));
    drop(blocker);
    assert!(!path.join("source/album").exists());
    assert_eq!(std::fs::read(path.join("target/album/disc/track.mp3")).unwrap(), b"second song");
}

#[test]
fn selected_root_acquisition_failure_and_authority_drop_never_delete_selected_paths() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source"); let target = tree.path().join("target"); let journal = tree.path().join("journal");
    let entries = make_tree(&source); std::fs::create_dir(&target).unwrap(); std::fs::create_dir(&journal).unwrap();
    assert!(WindowsMutationStorage::for_selected_album_root_rename(grant(&source), grant(&source), grant(&journal), entries.clone(), &AtomicBool::new(false)).is_err());
    let session = WindowsMutationStorage::for_selected_album_root_rename(grant(&source), grant(&target), grant(&journal), entries, &AtomicBool::new(false)).unwrap();
    drop(session);
    assert_eq!(std::fs::read(source.join("disc/track.mp3")).unwrap(), b"second song");
    assert!(source.is_dir()); assert!(target.is_dir()); assert!(journal.is_dir());
}

#[test]
fn bound_child_original_location_inspection_reports_original_absence_foreign_and_reparse() {
    let (storage, source, target, expected, entries, path) = fixture();
    let cancel = AtomicBool::new(false);
    let mut lease = storage.acquire_album_directory(&source, &name("album"), &expected, entries, &cancel).unwrap();
    assert_eq!(storage.inspect_original_album_location(&lease).unwrap().unwrap().identity, expected);
    let published = applied(storage.rename_album_directory(&mut lease, &target, &name("album"), &cancel));
    assert!(storage.inspect_original_album_location(&lease).unwrap().is_none());
    std::fs::write(path.join("source/album"), b"foreign file").unwrap();
    let replacement = storage.inspect_original_album_location(&lease).unwrap().unwrap();
    assert_ne!(replacement.identity, expected);
    assert_eq!(std::fs::read(path.join("source/album")).unwrap(), b"foreign file");
    std::fs::remove_file(path.join("source/album")).unwrap();
    std::fs::create_dir(path.join("source/album")).unwrap();
    std::fs::create_dir(path.join("outside")).unwrap();
    reparse_fixture::set_junction(&path.join("source/album"), &path.join("outside"));
    assert_eq!(storage.inspect_original_album_location(&lease).unwrap_err().kind, MutationErrorKind::UnsafeReparse);
    std::fs::remove_dir(path.join("source/album")).unwrap();
    drop(published);
    assert!(path.join("target/album/disc/track.mp3").exists());
}

#[test]
fn selected_root_original_location_inspection_never_grants_ancestor_authority() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source"); let target = tree.path().join("target"); let journal = tree.path().join("journal");
    let entries = make_tree(&source); std::fs::create_dir(&target).unwrap(); std::fs::create_dir(&journal).unwrap();
    let expected = id(&source);
    let cancel = AtomicBool::new(false);
    let mut session = WindowsMutationStorage::for_selected_album_root_rename(grant(&source), grant(&target), grant(&journal), entries, &cancel).unwrap();
    assert_eq!(session.inspect_original_location().unwrap().unwrap().identity, expected);
    let parent = session.target_and_journal().selected_target_root().unwrap();
    let published = applied(session.rename_album_directory(&parent, &name("album"), &cancel));
    assert!(session.inspect_original_location().unwrap().is_none());
    std::fs::create_dir(&source).unwrap(); std::fs::write(source.join("foreign"), b"retain").unwrap();
    assert_ne!(session.inspect_original_location().unwrap().unwrap().identity, expected);
    assert!(session.target_and_journal().parent_reference(&StorageRef::WindowsPath { utf16: tree.path().as_os_str().encode_wide().collect() }).is_err());
    assert_eq!(std::fs::read(source.join("foreign")).unwrap(), b"retain");
    drop(published);
}

#[test]
fn target_root_observed_alias_comes_only_from_its_identity_bound_handle() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source"); let target = tree.path().join("Target Long Name"); let journal = tree.path().join("journal");
    for path in [&source, &target, &journal] { std::fs::create_dir(path).unwrap(); }
    let storage = WindowsMutationStorage::for_selected_roots(grant(&source), grant(&target), grant(&journal)).unwrap();
    let parent = storage.selected_target_root().unwrap();
    let observed = observe(parent.handle(), &parent).unwrap();
    assert_eq!(storage.selected_target_observed_reference().unwrap(), observed.reference);
    assert_eq!(observed.identity, *parent.identity());
    let original_grant = storage.selected_target_reference().unwrap();
    assert_eq!(original_grant, StorageRef::WindowsPath { utf16: target.as_os_str().encode_wide().collect() });
    assert!(storage.parent_reference(&original_grant).is_ok());
}
