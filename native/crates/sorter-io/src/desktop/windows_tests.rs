use super::*;
use std::{io::Read, os::windows::ffi::OsStrExt};
mod fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/windows_reparse.rs"
    ));
}
#[test]
fn relative_child_open_after_same_identity_parent_junction_never_reads_outside() {
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("selected");
    let outside = temp.path().join("outside");
    fs::create_dir(&inside).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("secret.mp3"), b"outside sentinel").unwrap();
    let handles = chain(&inside).unwrap();
    let parent = handles.last().unwrap();
    let before = identity(parent).unwrap();
    assert_eq!(kind(&parent.metadata().unwrap()), EntryKind::Directory);
    // The mutation is exactly between ancestor validation and the native child open.
    fixture::set_junction(&inside, &outside);
    assert_eq!(identity(parent).unwrap(), before);
    let result = open_child(
        parent,
        &"secret.mp3".encode_utf16().collect::<Vec<_>>(),
        true,
    );
    assert!(
        result.is_err(),
        "must not obtain outside sentinel handle after parent changes"
    );
    drop(handles);
    fs::remove_dir(&inside).unwrap();
}
#[test]
fn handle_enumeration_after_parent_junction_never_lists_outside() {
    let temp = tempfile::tempdir().unwrap();
    let inside = temp.path().join("selected");
    let outside = temp.path().join("outside");
    fs::create_dir(&inside).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("outside-only.mp3"), b"outside sentinel").unwrap();
    let handles = chain(&inside).unwrap();
    let directory = reopen_bound(&handles, &inside).unwrap();
    assert_eq!(kind(&directory.metadata().unwrap()), EntryKind::Directory);
    // The directory has already acquired listing access and passed validation.
    fixture::set_junction(&inside, &outside);
    if let Ok(entries) = directory_records(&directory) {
        assert!(
            entries.is_empty(),
            "handle enumeration must not resolve a new directory path"
        );
    }
    drop(directory);
    drop(handles);
    fs::remove_dir(&inside).unwrap();
}
#[test]
fn relative_child_reads_remain_bound_to_observed_file_and_parent() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("שיר𐐀.mp3");
    fs::write(&path, b"synthetic selected").unwrap();
    let handles = chain(&path).unwrap();
    let mut file = reopen_bound(&handles, &path).unwrap();
    assert_eq!(
        identity(&file).unwrap(),
        identity(handles.last().unwrap()).unwrap()
    );
    let mut bytes = vec![];
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"synthetic selected");
    let parent = &handles[handles.len() - 2];
    let name: Vec<u16> = path.file_name().unwrap().encode_wide().collect();
    assert_eq!(
        identity(&open_child(parent, &name, false).unwrap()).unwrap(),
        identity(&file).unwrap()
    );
}

#[test]
fn handle_bound_reopen_rejects_different_file_even_with_same_parent() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.mp3");
    let second = temp.path().join("second.mp3");
    fs::write(&first, b"one").unwrap();
    fs::write(&second, b"two").unwrap();
    let handles = chain(&first).unwrap();
    assert_eq!(
        reopen_bound(&handles, &second).err().unwrap().kind,
        StorageErrorKind::Changed
    );
}

#[test]
fn absolute_open_helper_cannot_reopen_a_descendant_path() {
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        open_handle(temp.path(), false).err().unwrap().kind,
        StorageErrorKind::InvalidReference
    );
    let dotted = PathBuf::from(format!("{}\\.\\file.mp3", temp.path().display()));
    assert_eq!(
        validate_path(&dotted).unwrap_err().kind,
        StorageErrorKind::InvalidReference
    );
}

#[test]
fn handle_directory_enumeration_advances_across_multiple_bounded_buffers() {
    let temp = tempfile::tempdir().unwrap();
    for n in 0..600 {
        fs::write(
            temp.path().join(format!("{n:04}-{}.mp3", "ש".repeat(80))),
            b"synthetic",
        )
        .unwrap();
    }
    let handles = chain(temp.path()).unwrap();
    let directory = reopen_bound(&handles, temp.path()).unwrap();
    let records = directory_records(&directory).unwrap();
    assert_eq!(records.len(), 600);
    let ids: std::collections::HashSet<_> = records.iter().map(|r| r.file_id).collect();
    assert_eq!(ids.len(), 600);
    let native_order: Vec<Vec<u16>> = fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().encode_wide().collect())
        .collect();
    assert_eq!(
        records.iter().map(|r| r.name.clone()).collect::<Vec<_>>(),
        native_order
    );
}

#[test]
fn album_bounded_children_preserve_native_order_references_identities_and_capabilities() {
    use std::os::windows::ffi::OsStringExt;
    let tree = tempfile::tempdir().unwrap(); let root = tree.path();
    fs::write(root.join("z.mp3"), b"z").unwrap(); fs::create_dir(root.join("disc")).unwrap();
    fs::write(root.join(std::ffi::OsString::from_wide(&[0x5e9, 0xd800, 46, 109, 112, 51])), b"raw").unwrap();
    let mut storage = DesktopStorage::new(&[root.to_path_buf()]).unwrap(); let reference = storage.reference(root).unwrap();
    let ordinary = storage.children(&reference).unwrap();
    let bounded = storage.children_bounded(&reference, WindowsDirectoryReadLimits::ALBUM_DEFAULT, &std::sync::atomic::AtomicBool::new(false)).unwrap();
    assert_eq!(bounded, ordinary);
    assert!(bounded.iter().all(|entry| entry.identity.is_some() && entry.parent_identity.is_some()));
}

#[test]
fn album_bounded_entry_limit_stops_before_opening_the_excess_child() {
    use std::cell::Cell;
    let tree = tempfile::tempdir().unwrap(); for n in 0..4 { fs::write(tree.path().join(format!("{n}.mp3")), b"song").unwrap(); }
    let mut storage = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap(); let reference = storage.reference(tree.path()).unwrap();
    let count = Cell::new(0); let limits = WindowsDirectoryReadLimits { max_entries: 3, ..Default::default() };
    let error = storage.children_bounded_with(&reference, limits, &std::sync::atomic::AtomicBool::new(false), |point| {
        if point == BoundedReadCheckpoint::BeforeChild { count.set(count.get() + 1); } Ok(())
    }).unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::BudgetExceeded); assert_eq!(count.get(), 3);
    assert_eq!(storage.children(&reference).unwrap().len(), 4);
}

#[test]
fn album_bounded_path_budget_counts_full_lossless_utf16_reference_units() {
    let tree = tempfile::tempdir().unwrap(); let path = tree.path().join("שיר𐐀.mp3"); fs::write(&path, b"song").unwrap();
    let mut storage = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap(); let reference = storage.reference(tree.path()).unwrap();
    let units = path.as_os_str().encode_wide().count(); let cancel = std::sync::atomic::AtomicBool::new(false);
    let limits = WindowsDirectoryReadLimits { max_path_units: units - 1, ..Default::default() };
    assert_eq!(storage.children_bounded_with(&reference, limits, &cancel, |point| {
        assert!(point != BoundedReadCheckpoint::BeforeChild, "over-budget path must not be opened"); Ok(())
    }).unwrap_err().kind, StorageErrorKind::BudgetExceeded);
    assert_eq!(storage.children_bounded(&reference, WindowsDirectoryReadLimits { max_path_units: units, ..limits }, &cancel).unwrap().len(), 1);
}

#[test]
fn album_bounded_native_batch_cap_prevents_unlimited_enumeration() {
    let tree = tempfile::tempdir().unwrap();
    for n in 0..600 { fs::write(tree.path().join(format!("{n:04}-{}.mp3", "ש".repeat(80))), b"song").unwrap(); }
    let mut storage = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap(); let reference = storage.reference(tree.path()).unwrap();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    assert_eq!(storage.children_bounded(&reference, WindowsDirectoryReadLimits { max_batches: 1, ..Default::default() }, &cancel).unwrap_err().kind, StorageErrorKind::BudgetExceeded);
    assert_eq!(storage.children_bounded(&reference, WindowsDirectoryReadLimits::ALBUM_DEFAULT, &cancel).unwrap().len(), 600);
}

#[test]
fn album_bounded_cancellation_discards_partial_results_at_batch_and_record_boundaries() {
    use std::sync::atomic::{AtomicBool, Ordering};
    for boundary in [BoundedReadCheckpoint::AfterBatch, BoundedReadCheckpoint::BeforeChild] {
        let tree = tempfile::tempdir().unwrap(); fs::write(tree.path().join("song.mp3"), b"song").unwrap();
        let mut storage = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap(); let reference = storage.reference(tree.path()).unwrap();
        let cancel = AtomicBool::new(false);
        assert_eq!(storage.children_bounded_with(&reference, WindowsDirectoryReadLimits::ALBUM_DEFAULT, &cancel, |point| {
            if point == boundary { cancel.store(true, Ordering::Release); } Ok(())
        }).unwrap_err().kind, StorageErrorKind::Cancelled);
        assert_eq!(storage.children_bounded(&reference, WindowsDirectoryReadLimits::ALBUM_DEFAULT, &cancel).unwrap_err().kind, StorageErrorKind::Cancelled);
    }
}

#[test]
fn album_bounded_child_identity_race_remains_changed_not_partial_success() {
    let tree = tempfile::tempdir().unwrap(); let path = tree.path().join("song.mp3"); fs::write(&path, b"original").unwrap();
    let mut storage = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap(); let reference = storage.reference(tree.path()).unwrap();
    let mut changed = false;
    let error = storage.children_bounded_with(&reference, WindowsDirectoryReadLimits::ALBUM_DEFAULT, &std::sync::atomic::AtomicBool::new(false), |point| {
        if point == BoundedReadCheckpoint::BeforeChild && !changed {
            fs::rename(&path, tree.path().join("retained.mp3")).unwrap(); fs::write(&path, b"foreign").unwrap(); changed = true;
        } Ok(())
    }).unwrap_err();
    assert_eq!(error.kind, StorageErrorKind::Changed);
    assert_eq!(fs::read(tree.path().join("retained.mp3")).unwrap(), b"original");
}

#[test]
fn album_bounded_limits_cannot_expand_past_supported_defaults() {
    let tree = tempfile::tempdir().unwrap(); let mut storage = DesktopStorage::new(&[tree.path().to_path_buf()]).unwrap(); let reference = storage.reference(tree.path()).unwrap();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for limits in [WindowsDirectoryReadLimits { max_entries: usize::MAX, ..Default::default() },
        WindowsDirectoryReadLimits { max_path_units: usize::MAX, ..Default::default() },
        WindowsDirectoryReadLimits { max_batches: 0, ..Default::default() }] {
        assert_eq!(storage.children_bounded(&reference, limits, &cancel).unwrap_err().kind, StorageErrorKind::BudgetExceeded);
    }
    assert!(storage.children_bounded(&reference, WindowsDirectoryReadLimits { max_entries: 0, max_path_units: 0, ..Default::default() }, &cancel).unwrap().is_empty());
}

#[test]
fn album_bounded_reparse_entries_match_ordinary_observations_without_following_links() {
    let tree = tempfile::tempdir().unwrap(); let root = tree.path().join("root"); let outside = tree.path().join("outside");
    fs::create_dir(&root).unwrap(); fs::create_dir(&outside).unwrap(); fs::create_dir(root.join("link")).unwrap();
    fs::write(outside.join("sentinel.mp3"), b"outside").unwrap(); fixture::set_junction(&root.join("link"), &outside);
    let mut storage = DesktopStorage::new(&[root.clone()]).unwrap(); let reference = storage.reference(&root).unwrap();
    let ordinary = storage.children(&reference).unwrap();
    let bounded = storage.children_bounded(&reference, WindowsDirectoryReadLimits::ALBUM_DEFAULT, &std::sync::atomic::AtomicBool::new(false)).unwrap();
    assert_eq!(ordinary, bounded); assert_eq!(bounded.len(), 1); assert_eq!(bounded[0].kind, EntryKind::LinkOrReparse);
    assert_eq!(fs::read(outside.join("sentinel.mp3")).unwrap(), b"outside");
    drop(storage); fs::remove_dir(root.join("link")).unwrap();
}
