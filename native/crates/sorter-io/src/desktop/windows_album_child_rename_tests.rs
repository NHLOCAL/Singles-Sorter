use super::*;
use std::cell::Cell;
use std::os::windows::fs::OpenOptionsExt;
use windows::Wdk::Storage::FileSystem::{FILE_FULL_EA_INFORMATION, NtSetEaFile};

fn name(value: &str) -> MutationName {
    MutationName::Windows(value.encode_utf16().collect())
}
fn id(path: &Path) -> ObjectIdentity {
    identity(chain(path).unwrap().last().unwrap()).unwrap()
}
fn grant(path: &Path) -> WindowsSelectedRootGrant {
    WindowsSelectedRootGrant::new(path.to_path_buf(), id(path)).unwrap()
}
fn entry(root: &Path, parts: &[&str], kind: EntryKind) -> WindowsAlbumTreeEntry {
    let path = parts
        .iter()
        .fold(root.to_path_buf(), |path, part| path.join(part));
    WindowsAlbumTreeEntry {
        relative: parts.iter().map(|part| name(part)).collect(),
        identity: id(&path),
        kind,
        digest: (kind == EntryKind::File).then(|| {
            let bytes = std::fs::read(path).unwrap();
            ContentDigest {
                length: bytes.len() as u64,
                sha256: Sha256::digest(bytes).into(),
            }
        }),
    }
}
fn make_tree(path: &Path) -> Vec<WindowsAlbumTreeEntry> {
    std::fs::create_dir(path).unwrap();
    std::fs::write(path.join("old.mp3"), b"song").unwrap();
    std::fs::write(path.join("keep.jpg"), b"cover").unwrap();
    std::fs::create_dir(path.join("old disc")).unwrap();
    std::fs::write(path.join("old disc/שיר𐐀.mp3"), b"nested").unwrap();
    vec![
        entry(path, &["old disc", "שיר𐐀.mp3"], EntryKind::File),
        entry(path, &["old.mp3"], EntryKind::File),
        entry(path, &["old disc"], EntryKind::Directory),
        entry(path, &["keep.jpg"], EntryKind::File),
    ]
}
fn ordered(entries: &[WindowsAlbumTreeEntry]) -> Vec<WindowsAlbumChildRename> {
    [
        ("keep.jpg", "keep.jpg"),
        ("old.mp3", "song.mp3"),
        ("old disc", "disc"),
    ]
    .into_iter()
    .map(|(source, destination)| {
        let source = name(source);
        let identity = entries
            .iter()
            .find(|entry| entry.relative == vec![source.clone()])
            .unwrap()
            .identity
            .clone();
        WindowsAlbumChildRename {
            source,
            destination: name(destination),
            identity,
        }
    })
    .collect()
}
struct Fixture {
    storage: WindowsMutationStorage,
    source: WindowsParent,
    target: WindowsParent,
    expected: ObjectIdentity,
    entries: Vec<WindowsAlbumTreeEntry>,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_path_buf();
        std::fs::create_dir(path.join("source")).unwrap();
        std::fs::create_dir(path.join("target")).unwrap();
        let entries = make_tree(&path.join("source/album"));
        let expected = id(&path.join("source/album"));
        let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let source = storage.parent(Path::new("source")).unwrap();
        let target = storage.parent(Path::new("target")).unwrap();
        Self {
            storage,
            source,
            target,
            expected,
            entries,
            path,
        }
    }
    fn acquire(&self, cancel: &AtomicBool) -> WindowsAlbumChildRenameScope {
        self.storage
            .acquire_album_directory_for_child_renames(
                &self.source,
                &name("album"),
                &self.expected,
                self.entries.clone(),
                ordered(&self.entries),
                cancel,
            )
            .unwrap()
    }
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
fn convert(scope: WindowsAlbumChildRenameScope) -> WindowsAlbumDirectoryLease {
    match scope.into_album_directory_lease() {
        Ok(value) => value,
        Err(_) => panic!("expected complete cursor"),
    }
}
fn set_ea(path: &Path) {
    let file = std::fs::OpenOptions::new()
        .access_mode(FILE_WRITE_EA.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
        .open(path)
        .unwrap();
    let name = b"SORTER_CHILD";
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
    assert_eq!(
        unsafe {
            NtSetEaFile(
                HANDLE(file.as_raw_handle()),
                &mut status,
                pointer.cast(),
                length as u32,
            )
        }
        .0,
        0
    );
    assert_eq!(unsafe { status.Anonymous.Status }.0, 0);
}

#[test]
fn exact_planner_order_includes_unchanged_leaves_and_updates_all_descendant_prefixes() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let order = ordered(&f.entries);
    let mut scope = f.acquire(&cancel);
    assert_eq!(scope.ordered_children(), order);
    assert_eq!(scope.next_child_rename().unwrap().0, 1);
    let first = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
    assert_eq!(first.planner_index, 1);
    assert_eq!(first.observation.identity, order[1].identity);
    assert_eq!(scope.next_child_rename().unwrap().0, 2);
    let second = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
    assert_eq!(second.planner_index, 2);
    assert_eq!(second.observation.identity, order[2].identity);
    assert!(
        second
            .entries
            .iter()
            .any(|entry| entry.relative == vec![name("disc"), name("שיר𐐀.mp3")])
    );
    assert!(scope.is_complete());
    assert!(scope.next_child_rename().is_none());
    no_effect(f.storage.rename_next_album_child(&mut scope, &cancel));
    let mut lease = convert(scope);
    let published =
        applied(
            f.storage
                .rename_album_directory(&mut lease, &f.target, &name("album"), &cancel),
        );
    assert_eq!(published.observation().identity, f.expected);
    assert_eq!(
        f.storage
            .verify_album_publication(&published, &cancel)
            .unwrap()
            .as_slice(),
        published.entries()
    );
    for before in &second.entries {
        let after = published
            .entries()
            .iter()
            .find(|entry| entry.relative == before.relative)
            .unwrap();
        assert_eq!(after.observation.identity, before.observation.identity);
    }
    drop(published);
    assert_eq!(
        std::fs::read(f.path.join("target/album/disc/שיר𐐀.mp3")).unwrap(),
        b"nested"
    );
}

#[test]
fn actual_selected_root_sanitization_converts_without_unpinning_and_keeps_journal_access() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("album");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    let entries = make_tree(&source);
    let order = ordered(&entries);
    let expected = id(&source);
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&journal).unwrap();
    let cancel = AtomicBool::new(false);
    let mut session = WindowsMutationStorage::for_selected_album_root_child_renames(
        grant(&source),
        grant(&target),
        grant(&journal),
        entries,
        order,
        &cancel,
    )
    .unwrap();
    assert_eq!(session.selected_source_identity(), &expected);
    assert!(session.target_and_journal().selected_source_root().is_err());
    let parent = session
        .target_and_journal()
        .selected_journal_root()
        .unwrap();
    let mut writer = session
        .target_and_journal()
        .create_journal(&parent, &name("intent.log"))
        .unwrap();
    while session.next_child_rename().is_some() {
        writer.append(b"intent").unwrap();
        let _ = writer.flush_file_buffers().unwrap();
        let _ = applied(session.rename_next_album_child(&cancel));
        writer.append(b"observed").unwrap();
        let _ = writer.flush_file_buffers().unwrap();
    }
    assert!(session.is_complete());
    let mut session = match session.into_root_rename_session() {
        Ok(value) => value,
        Err(_) => panic!("root cursor incomplete"),
    };
    let destination = session.target_and_journal().selected_target_root().unwrap();
    let publication =
        applied(session.rename_album_directory(&destination, &name("moved"), &cancel));
    assert_eq!(publication.observation().identity, expected);
    assert_eq!(session.inspect_original_location().unwrap(), None);
    writer.append(b"published").unwrap();
    let _ = writer.flush_file_buffers().unwrap();
    drop(writer);
    drop(publication);
    drop(session);
    assert_eq!(
        std::fs::read(target.join("moved/song.mp3")).unwrap(),
        b"song"
    );
    assert_eq!(
        std::fs::read(target.join("moved/disc/שיר𐐀.mp3")).unwrap(),
        b"nested"
    );
}

#[test]
fn incomplete_conversion_returns_the_still_pinned_original_root() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let scope = f.acquire(&cancel);
    let scope = match scope.into_album_directory_lease() {
        Err(value) => value,
        Ok(_) => panic!("incomplete conversion"),
    };
    assert_eq!(
        f.storage
            .inspect_original_album_child_rename_location(&scope)
            .unwrap()
            .unwrap()
            .identity,
        f.expected
    );
    assert!(std::fs::rename(f.path.join("source/album"), f.path.join("source/escaped")).is_err());
}

#[test]
fn full_coverage_identity_and_invalid_names_are_rejected_before_effects() {
    for change in 0..7 {
        let f = Fixture::new();
        let mut plan = ordered(&f.entries);
        match change {
            0 => {
                let _ = plan.pop();
            }
            1 => plan.push(plan[0].clone()),
            2 => plan[2] = plan[1].clone(),
            3 => plan[1].identity = f.expected.clone(),
            4 => plan[1].destination = name(""),
            5 => plan[1].destination = name("nested/name"),
            _ => plan[1].source = name("OLD.MP3"),
        }
        assert!(
            f.storage
                .acquire_album_directory_for_child_renames(
                    &f.source,
                    &name("album"),
                    &f.expected,
                    f.entries.clone(),
                    plan,
                    &AtomicBool::new(false)
                )
                .is_err()
        );
        assert_eq!(
            std::fs::read(f.path.join("source/album/old.mp3")).unwrap(),
            b"song"
        );
        assert!(!f.path.join("source/album/song.mp3").exists());
    }
}

#[test]
fn occupied_destination_and_case_alias_are_rejected_before_first_effect() {
    for destination in ["keep.jpg", "KEEP.JPG", "old disc", "OLD.MP3"] {
        let f = Fixture::new();
        let mut plan = ordered(&f.entries);
        plan[1].destination = name(destination);
        assert!(
            f.storage
                .acquire_album_directory_for_child_renames(
                    &f.source,
                    &name("album"),
                    &f.expected,
                    f.entries.clone(),
                    plan,
                    &AtomicBool::new(false)
                )
                .is_err()
        );
        assert!(f.path.join("source/album/old.mp3").exists());
        assert!(f.path.join("source/album/old disc/שיר𐐀.mp3").exists());
    }
}

#[test]
fn future_absent_ordinal_aliases_are_rejected_without_rewriting_names() {
    for (a, b) in [("new", "NEW"), ("é", "É"), ("same", "same")] {
        let f = Fixture::new();
        let mut plan = ordered(&f.entries);
        plan[1].destination = name(a);
        plan[2].destination = name(b);
        assert!(
            f.storage
                .acquire_album_directory_for_child_renames(
                    &f.source,
                    &name("album"),
                    &f.expected,
                    f.entries.clone(),
                    plan,
                    &AtomicBool::new(false)
                )
                .is_err()
        );
        assert!(!f.path.join("source/album/song.mp3").exists());
    }
}

#[test]
fn changed_membership_content_or_identity_stops_before_child_dispatch() {
    for change in 0..2 {
        let f = Fixture::new();
        let cancel = AtomicBool::new(false);
        let mut scope = f.acquire(&cancel);
        let original = scope.current_tree().to_vec();
        if change == 0 {
            std::fs::write(f.path.join("source/album/late.txt"), b"late").unwrap();
        } else {
            std::fs::write(f.path.join("source/album/old.mp3"), b"different").unwrap();
        }
        no_effect(f.storage.rename_next_album_child(&mut scope, &cancel));
        assert!(!f.path.join("source/album/song.mp3").exists());
        assert_eq!(scope.current_tree(), original);
    }
    // Moving the child to another parent is permitted by Windows here. Keep
    // the album lease held and require the final inventory to reject a new ID
    // at the same leaf, with unchanged member count and identical file bytes.
    let f = Fixture::new();
    let mut scope = f.acquire(&AtomicBool::new(false));
    let retained_tree = scope.current_tree().to_vec();
    let original = f.path.join("source/album/old.mp3");
    let retained = f.path.join("source/retained.mp3");
    std::fs::rename(&original, &retained).unwrap();
    std::fs::write(&original, b"song").unwrap();
    no_effect(f.storage.rename_next_album_child(&mut scope, &AtomicBool::new(false)));
    assert_eq!(scope.current_tree(), retained_tree);
    assert!(!f.path.join("source/album/song.mp3").exists());
    assert_eq!(std::fs::read(&original).unwrap(), b"song");
    assert_eq!(std::fs::read(&retained).unwrap(), b"song");
}

#[test]
fn cancellation_before_dispatch_has_no_effect_and_retains_cursor() {
    for boundary in [
        ChildRenameCheckpoint::BeforeFinalVerification,
        ChildRenameCheckpoint::BeforeRename,
    ] {
        let f = Fixture::new();
        let cancel = AtomicBool::new(false);
        let mut scope = f.acquire(&cancel);
        let error = no_effect(f.storage.rename_next_album_child_with(
            &mut scope,
            &cancel,
            (
                false,
                |point| {
                    if point == boundary {
                        cancel.store(true, Ordering::Release);
                    }
                    Ok(())
                },
                close_child_once,
            ),
        ));
        assert_eq!(error.kind, MutationErrorKind::Cancelled);
        assert!(!scope.is_stopped());
        assert_eq!(scope.next_child_rename().unwrap().0, 1);
        assert!(f.path.join("source/album/old.mp3").exists());
        assert!(!f.path.join("source/album/song.mp3").exists());
    }
}

#[test]
fn cancellation_after_dispatch_closes_child_once_and_poisons_without_manifest_advance() {
    for boundary in [
        ChildRenameCheckpoint::AfterRename,
        ChildRenameCheckpoint::AfterClose,
        ChildRenameCheckpoint::AfterVerification,
    ] {
        let f = Fixture::new();
        let cancel = AtomicBool::new(false);
        let mut scope = f.acquire(&cancel);
        let original = scope.current_tree().to_vec();
        let closed = Cell::new(0);
        pending(f.storage.rename_next_album_child_with(
            &mut scope,
            &cancel,
            (
                false,
                |point| {
                    if point == boundary {
                        cancel.store(true, Ordering::Release);
                    }
                    Ok(())
                },
                |file| {
                    closed.set(closed.get() + 1);
                    close_child_once(file)
                },
            ),
        ));
        assert_eq!(closed.get(), 1);
        assert!(scope.is_stopped());
        assert!(!scope.is_complete());
        assert_eq!(scope.current_tree(), original);
        assert!(scope.next_child_rename().is_none());
        assert!(f.path.join("source/album/song.mp3").exists());
        assert!(!f.path.join("source/album/old.mp3").exists());
        cancel.store(false, Ordering::Release);
        no_effect(f.storage.rename_next_album_child(&mut scope, &cancel));
        assert_eq!(
            f.storage
                .inspect_original_album_child_rename_location(&scope)
                .unwrap()
                .unwrap()
                .identity,
            f.expected
        );
    }
}

#[test]
fn lost_response_or_close_response_consumes_child_once_and_retains_root() {
    for close_failure in [false, true] {
        let f = Fixture::new();
        let cancel = AtomicBool::new(false);
        let mut scope = f.acquire(&cancel);
        let closed = Cell::new(0);
        pending(f.storage.rename_next_album_child_with(
            &mut scope,
            &cancel,
            (
                false,
                |point| {
                    if !close_failure && point == ChildRenameCheckpoint::AfterRename {
                        return Err(err(MutationErrorKind::Io, "lost rename response"));
                    }
                    Ok(())
                },
                |file| {
                    closed.set(closed.get() + 1);
                    close_child_once(file)?;
                    if close_failure {
                        Err(err(MutationErrorKind::Io, "lost close response"))
                    } else {
                        Ok(())
                    }
                },
            ),
        ));
        assert_eq!(closed.get(), 1);
        assert!(scope.is_stopped());
        assert!(f.path.join("source/album/song.mp3").exists());
        assert!(f.path.join("source/album/old disc").exists());
        assert!(scope.into_album_directory_lease().is_err());
    }
}

#[test]
fn late_member_is_preserved_and_observed_as_uncertain_inside_original_container() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    pending(f.storage.rename_next_album_child_with(
        &mut scope,
        &cancel,
        (
            false,
            |point| {
                if point == ChildRenameCheckpoint::BeforeRename {
                    std::fs::write(f.path.join("source/album/late.txt"), b"preserve").unwrap();
                }
                Ok(())
            },
            close_child_once,
        ),
    ));
    assert!(scope.is_stopped());
    assert_eq!(
        std::fs::read(f.path.join("source/album/late.txt")).unwrap(),
        b"preserve"
    );
    assert_eq!(
        std::fs::read(f.path.join("source/album/song.mp3")).unwrap(),
        b"song"
    );
    assert!(!f.path.join("target/album").exists());
}

#[test]
fn native_late_collision_stops_scope_and_preserves_both_objects_without_retry() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    no_effect(f.storage.rename_next_album_child_with(
        &mut scope,
        &cancel,
        (
            false,
            |point| {
                if point == ChildRenameCheckpoint::BeforeRename {
                    std::fs::write(f.path.join("source/album/song.mp3"), b"foreign").unwrap();
                }
                Ok(())
            },
            close_child_once,
        ),
    ));
    assert!(scope.is_stopped());
    no_effect(f.storage.rename_next_album_child(&mut scope, &cancel));
    assert_eq!(
        std::fs::read(f.path.join("source/album/song.mp3")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        std::fs::read(f.path.join("source/album/old.mp3")).unwrap(),
        b"song"
    );
}

#[test]
fn streams_eas_and_nested_readonly_survive_child_then_container_renames() {
    let f = Fixture::new();
    let root = f.path.join("source/album");
    let cancel = AtomicBool::new(false);
    for path in [root.join("old.mp3"), root.join("old disc")] {
        let stream = PathBuf::from(format!("{}:notes", path.display()));
        std::fs::write(stream, b"preserve stream").unwrap();
        set_ea(&path);
    }
    let nested = root.join("old disc/שיר𐐀.mp3");
    let mut permissions = std::fs::metadata(&nested).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&nested, permissions).unwrap();
    let mut scope = f.acquire(&cancel);
    while scope.next_child_rename().is_some() {
        let _ = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
    }
    let root_handle = scope.source.file.as_ref().unwrap();
    for leaf in ["song.mp3", "disc"] {
        let file =
            super::super::super::super::open_child(root_handle, units(&name(leaf)).unwrap(), false)
                .unwrap();
        assert!(check_repair_extended_attributes(&file).is_err());
    }
    let mut lease = convert(scope);
    let publication =
        applied(
            f.storage
                .rename_album_directory(&mut lease, &f.target, &name("album"), &cancel),
        );
    drop(publication);
    assert_eq!(
        std::fs::read(f.path.join("target/album/song.mp3:notes")).unwrap(),
        b"preserve stream"
    );
    assert_eq!(
        std::fs::read(f.path.join("target/album/disc:notes")).unwrap(),
        b"preserve stream"
    );
    assert!(
        std::fs::metadata(f.path.join("target/album/disc/שיר𐐀.mp3"))
            .unwrap()
            .permissions()
            .readonly()
    );
}

#[test]
fn original_root_and_generic_source_file_sharing_stay_strict() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let scope = f.acquire(&cancel);
    assert!(
        std::fs::OpenOptions::new()
            .access_mode(FILE_LIST_DIRECTORY.0)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
            .open(f.path.join("source/album"))
            .is_err()
    );
    assert!(std::fs::rename(f.path.join("source/album"), f.path.join("source/moved")).is_err());
    let child = scope.source.file.as_ref().unwrap();
    let held = open_album_relative(
        child,
        units(&name("old.mp3")).unwrap(),
        FILE_GENERIC_READ,
        FILE_NON_DIRECTORY_FILE,
        FILE_SHARE_MODE(0),
    )
    .unwrap();
    assert!(std::fs::read(f.path.join("source/album/old.mp3")).is_err());
    assert!(std::fs::write(f.path.join("source/album/old.mp3"), b"bad").is_err());
    drop(held);
}

#[test]
fn all_unchanged_order_converts_without_child_dispatch() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut plan = ordered(&f.entries);
    for item in &mut plan {
        item.destination = item.source.clone();
    }
    plan.reverse();
    let scope = f
        .storage
        .acquire_album_directory_for_child_renames(
            &f.source,
            &name("album"),
            &f.expected,
            f.entries.clone(),
            plan.clone(),
            &cancel,
        )
        .unwrap();
    assert_eq!(scope.ordered_children(), plan);
    assert!(scope.next_child_rename().is_none());
    assert!(scope.is_complete());
    let lease = convert(scope);
    assert_eq!(lease.observation().identity, f.expected);
    assert!(f.path.join("source/album/old.mp3").exists());
}

#[test]
fn foreign_old_leaf_after_dispatch_is_preserved_and_stops_the_scope() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    pending(f.storage.rename_next_album_child_with(
        &mut scope,
        &cancel,
        (
            false,
            |point| {
                if point == ChildRenameCheckpoint::AfterRename {
                    std::fs::write(f.path.join("source/album/old.mp3"), b"foreign old leaf")
                        .unwrap();
                }
                Ok(())
            },
            close_child_once,
        ),
    ));
    assert!(scope.is_stopped());
    assert_eq!(
        std::fs::read(f.path.join("source/album/old.mp3")).unwrap(),
        b"foreign old leaf"
    );
    assert_eq!(
        std::fs::read(f.path.join("source/album/song.mp3")).unwrap(),
        b"song"
    );
    assert_eq!(scope.cursor, 1);
}

#[test]
fn selected_root_partial_outcome_keeps_original_container_and_no_general_source_grant() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("album");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    let entries = make_tree(&source);
    let order = ordered(&entries);
    let expected = id(&source);
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&journal).unwrap();
    let cancel = AtomicBool::new(false);
    let mut session = WindowsMutationStorage::for_selected_album_root_child_renames(
        grant(&source),
        grant(&target),
        grant(&journal),
        entries,
        order,
        &cancel,
    )
    .unwrap();
    pending(session.target_and_journal.rename_next_album_child_with(
        &mut session.scope,
        &cancel,
        (
            true,
            |point| {
                if point == ChildRenameCheckpoint::AfterClose {
                    return Err(err(MutationErrorKind::Io, "lost child result"));
                }
                Ok(())
            },
            close_child_once,
        ),
    ));
    assert!(session.is_stopped());
    assert!(!session.is_complete());
    let session = match session.into_root_rename_session() {
        Err(value) => value,
        Ok(_) => panic!("uncertain conversion"),
    };
    assert_eq!(
        session
            .inspect_original_location()
            .unwrap()
            .unwrap()
            .identity,
        expected
    );
    assert!(session.target_and_journal().selected_source_root().is_err());
    assert!(std::fs::rename(&source, tree.path().join("escaped")).is_err());
    assert_eq!(std::fs::read(source.join("song.mp3")).unwrap(), b"song");
    assert!(source.join("old disc/שיר𐐀.mp3").exists());
    assert!(!target.join("album").exists());
}

#[test]
fn retained_scope_verifies_current_tree_without_reopening_its_root() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    assert_eq!(
        f.storage
            .verify_album_child_rename_tree(&scope, &cancel)
            .unwrap()
            .len(),
        f.entries.len()
    );
    let observed = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
    assert_eq!(
        f.storage
            .verify_album_child_rename_tree(&scope, &cancel)
            .unwrap(),
        observed.entries
    );
    assert_eq!(
        f.storage
            .inspect_album_child_rename_location(&scope, 1, false)
            .unwrap(),
        None
    );
    assert_eq!(
        f.storage
            .inspect_album_child_rename_location(&scope, 1, true)
            .unwrap()
            .unwrap(),
        observed.observation
    );
    assert_eq!(
        f.storage
            .inspect_album_child_rename_location(&scope, usize::MAX, true)
            .unwrap_err()
            .kind,
        MutationErrorKind::InvalidName
    );
    assert_eq!(scope.next_child_rename().unwrap().0, 2);
}

#[test]
fn stopped_scope_inspects_actual_child_but_does_not_adopt_uncertain_manifest() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    let expected = scope.ordered_children()[1].identity.clone();
    pending(f.storage.rename_next_album_child_with(
        &mut scope,
        &cancel,
        (
            false,
            |point| {
                if point == ChildRenameCheckpoint::AfterClose {
                    return Err(err(MutationErrorKind::Io, "lost observed receipt"));
                }
                Ok(())
            },
            close_child_once,
        ),
    ));
    assert!(
        f.storage
            .verify_album_child_rename_tree(&scope, &cancel)
            .is_err()
    );
    assert_eq!(
        f.storage
            .inspect_album_child_rename_location(&scope, 1, false)
            .unwrap(),
        None
    );
    let actual = f
        .storage
        .inspect_album_child_rename_location(&scope, 1, true)
        .unwrap()
        .unwrap();
    assert_eq!(actual.identity, expected);
    assert_eq!(actual.parent_identity, f.expected);
    assert_eq!(actual.actual_name, name("song.mp3"));
    std::fs::write(f.path.join("source/album/old.mp3"), b"foreign").unwrap();
    let foreign = f
        .storage
        .inspect_album_child_rename_location(&scope, 1, false)
        .unwrap()
        .unwrap();
    assert_ne!(foreign.identity, expected);
    assert!(scope.is_stopped());
    assert_eq!(scope.cursor, 1);
    assert!(scope.next_child_rename().is_none());
    assert_eq!(
        std::fs::read(f.path.join("source/album/old.mp3")).unwrap(),
        b"foreign"
    );
}

#[test]
fn selected_root_stopped_tree_and_child_inspection_keep_authority_private() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("album");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    let entries = make_tree(&source);
    let order = ordered(&entries);
    let expected = order[1].identity.clone();
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&journal).unwrap();
    let cancel = AtomicBool::new(false);
    let mut session = WindowsMutationStorage::for_selected_album_root_child_renames(
        grant(&source),
        grant(&target),
        grant(&journal),
        entries,
        order,
        &cancel,
    )
    .unwrap();
    assert_eq!(session.verify_current_tree(&cancel).unwrap().len(), 4);
    pending(session.target_and_journal.rename_next_album_child_with(
        &mut session.scope,
        &cancel,
        (
            true,
            |point| {
                if point == ChildRenameCheckpoint::AfterClose {
                    return Err(err(MutationErrorKind::Io, "lost response"));
                }
                Ok(())
            },
            close_child_once,
        ),
    ));
    assert!(session.verify_current_tree(&cancel).is_err());
    assert_eq!(
        session
            .inspect_child_location(1, true)
            .unwrap()
            .unwrap()
            .identity,
        expected
    );
    assert_eq!(session.inspect_child_location(1, false).unwrap(), None);
    assert!(session.target_and_journal().selected_source_root().is_err());
    assert!(session.is_stopped());
}

#[test]
fn bound_leaf_inspection_rejects_reparse_replacement_without_following_it() {
    mod reparse_fixture {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/support/windows_reparse.rs"
        ));
    }
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    let _ = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
    std::fs::create_dir(f.path.join("source/album/old.mp3")).unwrap();
    std::fs::create_dir(f.path.join("outside")).unwrap();
    std::fs::write(f.path.join("outside/sentinel"), b"preserve").unwrap();
    reparse_fixture::set_junction(
        &f.path.join("source/album/old.mp3"),
        &f.path.join("outside"),
    );
    assert_eq!(
        f.storage
            .inspect_album_child_rename_location(&scope, 1, false)
            .unwrap_err()
            .kind,
        MutationErrorKind::UnsafeReparse
    );
    assert_eq!(
        std::fs::read(f.path.join("outside/sentinel")).unwrap(),
        b"preserve"
    );
    std::fs::remove_dir(f.path.join("source/album/old.mp3")).unwrap();
}

#[test]
fn completed_child_scope_forwards_container_publication_and_retains_original_location() {
    for unchanged in [false, true] {
        let f = Fixture::new();
        let cancel = AtomicBool::new(false);
        let mut plan = ordered(&f.entries);
        if unchanged {
            for item in &mut plan {
                item.destination = item.source.clone();
            }
        }
        let mut scope = f
            .storage
            .acquire_album_directory_for_child_renames(
                &f.source,
                &name("album"),
                &f.expected,
                f.entries.clone(),
                plan,
                &cancel,
            )
            .unwrap();
        while scope.next_child_rename().is_some() {
            let _ = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
        }
        let publication = applied(f.storage.rename_completed_album_children(
            &mut scope,
            &f.target,
            &name("album"),
            &cancel,
        ));
        assert_eq!(publication.observation().identity, f.expected);
        assert!(scope.source.file.is_none());
        assert_eq!(
            f.storage
                .inspect_original_album_child_rename_location(&scope)
                .unwrap(),
            None
        );
        assert_eq!(
            f.storage
                .verify_album_publication(&publication, &cancel)
                .unwrap()
                .len(),
            f.entries.len()
        );
        no_effect(f.storage.rename_completed_album_children(
            &mut scope,
            &f.target,
            &name("replay"),
            &cancel,
        ));
        std::fs::create_dir(f.path.join("source/album")).unwrap();
        std::fs::write(f.path.join("source/album/foreign"), b"retain").unwrap();
        assert_ne!(
            f.storage
                .inspect_original_album_child_rename_location(&scope)
                .unwrap()
                .unwrap()
                .identity,
            f.expected
        );
        assert_eq!(
            std::fs::read(f.path.join("source/album/foreign")).unwrap(),
            b"retain"
        );
        assert!(!f.path.join("target/replay").exists());
        drop(publication);
    }
}

#[test]
fn incomplete_and_stopped_child_scopes_cannot_forward_container_rename() {
    for stopped in [false, true] {
        let f = Fixture::new();
        let cancel = AtomicBool::new(false);
        let mut scope = f.acquire(&cancel);
        if stopped {
            pending(f.storage.rename_next_album_child_with(
                &mut scope,
                &cancel,
                (
                    false,
                    |point| {
                        if point == ChildRenameCheckpoint::AfterClose {
                            return Err(err(MutationErrorKind::Io, "lost child result"));
                        }
                        Ok(())
                    },
                    close_child_once,
                ),
            ));
        }
        assert_eq!(
            no_effect(f.storage.rename_completed_album_children(
                &mut scope,
                &f.target,
                &name("album"),
                &cancel
            ))
            .kind,
            MutationErrorKind::StalePlan
        );
        assert!(scope.source.file.is_some());
        assert!(!f.path.join("target/album").exists());
        assert_eq!(
            f.storage
                .inspect_original_album_child_rename_location(&scope)
                .unwrap()
                .unwrap()
                .identity,
            f.expected
        );
    }
}

#[test]
fn selected_root_child_session_forwards_complete_cursor_without_conversion() {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("album");
    let target = tree.path().join("target");
    let journal = tree.path().join("journal");
    let entries = make_tree(&source);
    let order = ordered(&entries);
    let expected = id(&source);
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(&journal).unwrap();
    let cancel = AtomicBool::new(false);
    let mut session = WindowsMutationStorage::for_selected_album_root_child_renames(
        grant(&source),
        grant(&target),
        grant(&journal),
        entries,
        order,
        &cancel,
    )
    .unwrap();
    let destination = session.target_and_journal().selected_target_root().unwrap();
    no_effect(session.rename_completed_album_children(&destination, &name("moved"), &cancel));
    while session.next_child_rename().is_some() {
        let _ = applied(session.rename_next_album_child(&cancel));
    }
    let publication =
        applied(session.rename_completed_album_children(&destination, &name("moved"), &cancel));
    assert_eq!(publication.observation().identity, expected);
    assert_eq!(session.inspect_original_location().unwrap(), None);
    no_effect(session.rename_completed_album_children(&destination, &name("replay"), &cancel));
    assert!(session.target_and_journal().selected_source_root().is_err());
    let parent = session
        .target_and_journal()
        .selected_journal_root()
        .unwrap();
    let mut writer = session
        .target_and_journal()
        .create_journal(&parent, &name("observed.log"))
        .unwrap();
    writer.append(b"observed whole album").unwrap();
    let _ = writer.flush_file_buffers().unwrap();
    assert_eq!(
        session
            .target_and_journal()
            .verify_album_publication(&publication, &cancel)
            .unwrap()
            .len(),
        4
    );
    drop(writer);
    drop(publication);
    assert_eq!(
        std::fs::read(target.join("moved/song.mp3")).unwrap(),
        b"song"
    );
}

#[test]
fn uncertain_forwarded_container_rename_stops_scope_and_keeps_original_probe() {
    let f = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut scope = f.acquire(&cancel);
    while scope.next_child_rename().is_some() {
        let _ = applied(f.storage.rename_next_album_child(&mut scope, &cancel));
    }
    pending(f.storage.rename_completed_album_children_with(
        &mut scope,
        &f.target,
        &name("album"),
        &cancel,
        (false, |point| {
            if point == AlbumRenameCheckpoint::AfterRename {
                return Err(err(MutationErrorKind::Io, "lost container result"));
            }
            Ok(())
        }),
    ));
    assert!(scope.is_stopped());
    assert!(scope.source.file.is_none());
    assert_eq!(
        f.storage
            .inspect_original_album_child_rename_location(&scope)
            .unwrap(),
        None
    );
    no_effect(f.storage.rename_completed_album_children(
        &mut scope,
        &f.target,
        &name("replay"),
        &cancel,
    ));
    assert_eq!(
        std::fs::read(f.path.join("target/album/song.mp3")).unwrap(),
        b"song"
    );
    assert!(!f.path.join("target/replay").exists());
}
