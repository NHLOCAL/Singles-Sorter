//! Owned native read fixtures only; these adapters grant no mutation authority.
#![cfg(all(windows, feature = "experimental-windows-copy"))]
use sorter_core::discovery::*;
use sorter_executor::{ExecutionError, windows_copy::WindowsPlanningStorage};
use sorter_io::desktop::{WindowsDirectoryReadLimits, windows_mutation::WindowsMutationStorage};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tree = tempfile::tempdir().unwrap();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    (tree, source, target)
}
#[test]
fn bounded_album_planning_matches_ordinary_order_and_established_capabilities() {
    let (_tree, source, target) = fixture();
    std::fs::write(source.join("z.mp3"), b"song").unwrap();
    std::fs::create_dir(source.join("disc")).unwrap();
    std::fs::write(source.join("disc/nested.mp3"), b"nested").unwrap();
    let mut ordinary = WindowsPlanningStorage::for_preview(&source, &target).unwrap();
    let source_ref = ordinary.reference(&source).unwrap();
    let cancel = AtomicBool::new(false);
    let mut bounded = WindowsPlanningStorage::for_album_preview(
        &source,
        &target,
        WindowsDirectoryReadLimits::ALBUM_DEFAULT,
        &cancel,
    )
    .unwrap();
    assert_eq!(
        bounded.stat(&source_ref).unwrap(),
        ordinary.stat(&source_ref).unwrap()
    );
    assert_eq!(
        bounded.children(&source_ref).unwrap(),
        ordinary.children(&source_ref).unwrap()
    );
}
#[test]
fn directory_capability_probe_itself_obeys_the_native_entry_cap() {
    let (_tree, source, target) = fixture();
    for n in 0..4 {
        std::fs::write(source.join(format!("{n}.mp3")), b"song").unwrap();
    }
    let cancel = AtomicBool::new(false);
    let mut bounded = WindowsPlanningStorage::for_album_preview(
        &source,
        &target,
        WindowsDirectoryReadLimits {
            max_entries: 3,
            ..Default::default()
        },
        &cancel,
    )
    .unwrap();
    let reference = bounded.reference(&source).unwrap();
    assert_eq!(
        bounded.stat(&reference).unwrap_err().kind,
        StorageErrorKind::BudgetExceeded
    );
    assert_eq!(
        bounded.children(&reference).unwrap_err().kind,
        StorageErrorKind::BudgetExceeded
    );
    let mut ordinary = WindowsPlanningStorage::for_preview(&source, &target).unwrap();
    assert_eq!(ordinary.children(&reference).unwrap().len(), 4);
}
#[test]
fn nested_directory_probe_cannot_bypass_bound_during_parent_children_establishment() {
    let (_tree, source, target) = fixture();
    std::fs::create_dir(source.join("disc")).unwrap();
    for n in 0..4 {
        std::fs::write(source.join(format!("disc/{n}.mp3")), b"song").unwrap();
    }
    let cancel = AtomicBool::new(false);
    let mut bounded = WindowsPlanningStorage::for_album_preview(
        &source,
        &target,
        WindowsDirectoryReadLimits {
            max_entries: 3,
            ..Default::default()
        },
        &cancel,
    )
    .unwrap();
    let reference = bounded.reference(&source).unwrap();
    assert_eq!(
        bounded.children(&reference).unwrap_err().kind,
        StorageErrorKind::BudgetExceeded
    );
}
#[test]
fn album_wrapper_observes_borrowed_cancellation_before_all_read_routes() {
    let (_tree, source, target) = fixture();
    std::fs::write(source.join("song.mp3"), b"song").unwrap();
    let cancel = AtomicBool::new(false);
    let mut bounded = WindowsPlanningStorage::for_album_preview(
        &source,
        &target,
        WindowsDirectoryReadLimits::ALBUM_DEFAULT,
        &cancel,
    )
    .unwrap();
    let root = bounded.reference(&source).unwrap();
    let file = bounded.reference(&source.join("song.mp3")).unwrap();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        bounded.stat(&root).unwrap_err().kind,
        StorageErrorKind::Cancelled
    );
    assert_eq!(
        bounded.children(&root).unwrap_err().kind,
        StorageErrorKind::Cancelled
    );
    assert!(
        matches!(bounded.open_read(&file), Err(error) if error.kind == StorageErrorKind::Cancelled)
    );
    assert_eq!(
        bounded.reference(&source).unwrap_err().kind,
        StorageErrorKind::Cancelled
    );
    assert!(matches!(
        WindowsPlanningStorage::for_album_preview(
            &source,
            &target,
            WindowsDirectoryReadLimits::ALBUM_DEFAULT,
            &cancel
        ),
        Err(ExecutionError::Cancelled)
    ));
}
#[test]
fn owned_album_constructor_retains_exact_roots_and_rejects_outside_references() {
    let (tree, source, target) = fixture();
    std::fs::write(source.join("song.mp3"), b"song").unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("outside.mp3"), b"outside").unwrap();
    let mut preview = WindowsPlanningStorage::for_preview(&source, &target).unwrap();
    let source_ref = preview.reference(&source).unwrap();
    let target_ref = preview.reference(&target).unwrap();
    drop(preview);
    let owned = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let cancel = AtomicBool::new(false);
    let mut bounded = WindowsPlanningStorage::for_album(
        &owned,
        &source_ref,
        &target_ref,
        WindowsDirectoryReadLimits::ALBUM_DEFAULT,
        &cancel,
    )
    .unwrap();
    assert_eq!(bounded.children(&source_ref).unwrap().len(), 1);
    assert_eq!(
        bounded.reference(outside.path()).unwrap_err().kind,
        StorageErrorKind::OutsideRoots
    );
    assert!(std::fs::rename(&source, source.with_file_name("renamed")).is_err());
}
