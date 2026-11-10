//! Native acceptance tests. Compiling or skipping on Linux is NOT a pass.
#![cfg(all(windows, feature = "experimental-windows-mutation"))]
use sorter_io::{desktop::windows_mutation::WindowsMutationStorage, mutation::*};
use std::{io::Write, path::Path, sync::atomic::AtomicBool};
fn name(s: &str) -> MutationName {
    MutationName::Windows(s.encode_utf16().collect())
}
fn applied<T>(outcome: MutationOutcome<T>) -> T {
    match outcome {
        MutationOutcome::Applied(v) => v,
        other => panic!(
            "expected applied, got {}",
            match other {
                MutationOutcome::NoEffect(_) => "no effect",
                _ => "unknown",
            }
        ),
    }
}
#[test]
fn exact_destination_collision_preserves_both_objects() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let cancel = AtomicBool::new(false);
    let mut temp = applied(storage.create_temp(&parent, "collision", &cancel));
    temp.write_all(b"new bytes").unwrap();
    std::fs::write(root.join("שיר_1.mp3"), b"competitor").unwrap();
    assert!(
        matches!(storage.publish_no_replace(&mut temp, &parent, &name("שיר_1.mp3"), &cancel), MutationOutcome::NoEffect(e) if e.kind == MutationErrorKind::StalePlan)
    );
    assert_eq!(
        std::fs::read(root.join("שיר_1.mp3")).unwrap(),
        b"competitor"
    );
    assert!(!root.join("שיר_2.mp3").exists());
}
#[test]
fn exclusive_create_and_unicode_publication_are_handle_bound() {
    let tree = tempfile::tempdir().unwrap();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let cancel = AtomicBool::new(false);
    let mut temp = applied(storage.create_temp(&parent, "unicode", &cancel));
    assert!(matches!(
        storage.create_temp(&parent, "unicode", &cancel),
        MutationOutcome::NoEffect(_)
    ));
    temp.write_all(b"payload").unwrap();
    temp.sync_data().unwrap();
    let published =
        applied(storage.publish_no_replace(&mut temp, &parent, &name("שלום-🎵.mp3"), &cancel));
    assert_eq!(
        storage.inspect(&published).unwrap().actual_name,
        name("שלום-🎵.mp3")
    );
}
#[test]
fn source_lease_blocks_writers_replacement_and_ancestor_rename() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    std::fs::create_dir(root.join("parent")).unwrap();
    std::fs::write(root.join("parent/source.mp3"), b"original").unwrap();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("parent")).unwrap();
    let expected = storage
        .observe_source(&parent, &name("source.mp3"))
        .unwrap();
    let lease = storage.acquire_source(&parent, &expected).unwrap();
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(root.join("parent/source.mp3"))
            .is_err()
    );
    assert!(std::fs::remove_file(root.join("parent/source.mp3")).is_err());
    assert!(std::fs::rename(root.join("parent"), root.join("replaced-parent")).is_err());
    drop(lease);
    assert_eq!(
        std::fs::read(root.join("parent/source.mp3")).unwrap(),
        b"original"
    );
}
#[test]
fn changed_source_or_hardlinked_source_cannot_be_leased() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    std::fs::write(root.join("source"), b"first").unwrap();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let expected = storage.observe_source(&parent, &name("source")).unwrap();
    std::fs::write(root.join("source"), b"other").unwrap();
    assert!(
        matches!(storage.acquire_source(&parent,&expected),Err(e) if e.kind==MutationErrorKind::StalePlan)
    );
    std::fs::hard_link(root.join("source"), root.join("alias")).unwrap();
    assert!(
        matches!(storage.observe_source(&parent,&name("source")),Err(e) if e.kind==MutationErrorKind::Unsupported)
    );
}
#[test]
fn case_collision_is_not_renumbered_and_reserved_names_are_rejected() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let cancel = AtomicBool::new(false);
    let mut temp = applied(storage.create_temp(&parent, "case", &cancel));
    std::fs::write(root.join("SONG_3.MP3"), b"old").unwrap();
    assert!(
        matches!(storage.publish_no_replace(&mut temp,&parent,&name("song_3.mp3"),&cancel),MutationOutcome::NoEffect(e) if e.kind==MutationErrorKind::StalePlan)
    );
    for reserved in [
        "NUL.mp3",
        "COM1",
        "LPT².txt",
        "../escape",
        "x:y",
        "trailing.",
    ] {
        assert!(
            matches!(storage.publish_no_replace(&mut temp,&parent,&name(reserved),&cancel),MutationOutcome::NoEffect(e) if e.kind==MutationErrorKind::InvalidName)
        );
    }
    assert_eq!(std::fs::read(root.join("SONG_3.MP3")).unwrap(), b"old");
}
#[test]
fn same_handle_rename_preserves_full_identity_and_retirement_is_not_false_completion() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    std::fs::write(root.join("source"), b"original").unwrap();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let cancel = AtomicBool::new(false);
    let expected = storage.observe_source(&parent, &name("source")).unwrap();
    let mut lease = storage.acquire_source(&parent, &expected).unwrap();
    let published =
        applied(storage.rename_no_replace(&mut lease, &parent, &name("renamed"), &cancel));
    assert_eq!(
        storage
            .verify(&published, &expected.digest)
            .unwrap()
            .identity,
        expected.identity
    );
    assert!(!root.join("source").exists());
    drop(published);
    let expected = storage.observe_source(&parent, &name("renamed")).unwrap();
    let mut lease = storage.acquire_source(&parent, &expected).unwrap();
    assert!(matches!(
        applied(storage.retire_source(&mut lease, &cancel)),
        RetirementOutcome::DispositionRequested(_)
    ));
    assert!(!root.join("renamed").exists());
}
#[test]
fn cancel_before_dispatch_creates_nothing_and_cross_scope_is_rejected() {
    let mut a =
        WindowsMutationStorage::for_owned_synthetic_tree(tempfile::tempdir().unwrap()).unwrap();
    let b = WindowsMutationStorage::for_owned_synthetic_tree(tempfile::tempdir().unwrap()).unwrap();
    let parent = b.parent(Path::new("")).unwrap();
    assert!(
        matches!(a.create_temp(&parent,"outside",&AtomicBool::new(false)),MutationOutcome::NoEffect(e) if e.kind==MutationErrorKind::StalePlan)
    );
    let parent = a.parent(Path::new("")).unwrap();
    assert!(
        matches!(a.create_temp(&parent,"cancelled",&AtomicBool::new(true)),MutationOutcome::NoEffect(e) if e.kind==MutationErrorKind::Cancelled)
    );
    let _temp = applied(a.create_temp(&parent, "cancelled", &AtomicBool::new(false)));
}
#[test]
fn long_hebrew_paths_are_resolved_component_relative() {
    let tree = tempfile::tempdir().unwrap();
    let mut relative = std::path::PathBuf::new();
    for n in 0..12 {
        relative.push(format!("ספרייה-{n}-{}", "א".repeat(20)));
        std::fs::create_dir(tree.path().join(&relative)).unwrap();
    }
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(&relative).unwrap();
    let cancel = AtomicBool::new(false);
    let mut temp = applied(storage.create_temp(&parent, "long", &cancel));
    temp.write_all(b"long").unwrap();
    let published =
        applied(storage.publish_no_replace(&mut temp, &parent, &name("שיר-🎵"), &cancel));
    assert_eq!(storage.inspect(&published).unwrap().size, Some(4));
}
#[test]
fn junction_ancestor_is_rejected_without_following_it() {
    let tree = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    let result = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(root.join("junction"))
        .arg(outside.path())
        .status()
        .unwrap();
    assert!(
        result.success(),
        "junction fixture setup failed: native gate is blocked"
    );
    let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    assert!(storage.parent(Path::new("junction")).is_err());
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}
#[test]
fn concurrent_destination_creator_never_gets_overwritten() {
    use std::sync::{Arc, Barrier};
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let cancel = AtomicBool::new(false);
    for i in 0..64 {
        let leaf = format!("planned_{i}.mp3");
        let mut temp = applied(storage.create_temp(&parent, &format!("race-{i}"), &cancel));
        temp.write_all(b"ours").unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let worker_barrier = barrier.clone();
        let destination = root.join(&leaf);
        let worker_path = destination.clone();
        let worker = std::thread::spawn(move || {
            worker_barrier.wait();
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(worker_path)
            {
                Ok(mut f) => {
                    f.write_all(b"theirs").unwrap();
                    true
                }
                Err(_) => false,
            }
        });
        barrier.wait();
        let result = storage.publish_no_replace(&mut temp, &parent, &name(&leaf), &cancel);
        let competitor_won = worker.join().unwrap();
        match result {
            MutationOutcome::Applied(p) => {
                assert!(!competitor_won);
                drop(p);
                assert_eq!(std::fs::read(&destination).unwrap(), b"ours");
            }
            MutationOutcome::NoEffect(_) => {
                assert!(competitor_won);
                assert_eq!(std::fs::read(&destination).unwrap(), b"theirs");
            }
            MutationOutcome::PendingUnknown(e) => panic!("unexpected ambiguous native result: {e}"),
        }
    }
}
#[test]
fn writable_mapping_surviving_file_close_prevents_source_lease() {
    use std::os::windows::io::AsRawHandle;
    use windows::{
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            System::Memory::*,
        },
        core::PCWSTR,
    };
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    std::fs::write(root.join("mapped"), b"mapped data").unwrap();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let expected = storage.observe_source(&parent, &name("mapped")).unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("mapped"))
        .unwrap();
    let mapping = unsafe {
        CreateFileMappingW(
            HANDLE(file.as_raw_handle()),
            None,
            PAGE_READWRITE,
            0,
            0,
            PCWSTR::null(),
        )
    }
    .unwrap();
    let view = unsafe { MapViewOfFile(mapping, FILE_MAP_WRITE, 0, 0, 0) };
    assert!(!view.Value.is_null());
    drop(file);
    let result = storage.acquire_source(&parent, &expected);
    unsafe {
        UnmapViewOfFile(view).unwrap();
        CloseHandle(mapping).unwrap();
    }
    assert!(
        matches!(result,Err(e) if e.kind==MutationErrorKind::Locked),
        "mapped writer must prevent lease acquisition"
    );
}
#[test]
fn read_only_source_failure_does_not_report_a_move_or_remove_it() {
    let tree = tempfile::tempdir().unwrap();
    let root = tree.path().to_owned();
    let path = root.join("read-only");
    std::fs::write(&path, b"keep").unwrap();
    let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let expected = storage.observe_source(&parent, &name("read-only")).unwrap();
    let original_permissions = std::fs::metadata(&path).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();
    match storage.acquire_source(&parent, &expected) {
        Err(e) => assert_eq!(e.kind, MutationErrorKind::PermissionDenied),
        Ok(mut source) => assert!(
            matches!(storage.retire_source(&mut source,&AtomicBool::new(false)),MutationOutcome::NoEffect(e) if e.kind==MutationErrorKind::PermissionDenied)
        ),
    }
    assert_eq!(std::fs::read(&path).unwrap(), b"keep");
    std::fs::set_permissions(&path, original_permissions).unwrap();
}
