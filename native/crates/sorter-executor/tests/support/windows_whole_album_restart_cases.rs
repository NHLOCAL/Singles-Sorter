use super::recorded_restart::{grants, journal_bytes, owned_tree_snapshot};
use super::whole_recorded_descriptor::{completed, header_only, manifest};
use super::*;
use sorter_executor::{inspection_control::InspectionControl, recovery::RecoveryStatus};
use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize},
};
fn control() -> InspectionControl<'static> {
    InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {})
}
fn descriptor(f: &Fixture) -> RecordedWholeAlbumJob {
    inspect_recorded_whole_album_job(&manifest(f), &control()).unwrap()
}
fn records(f: &Fixture, d: &RecordedWholeAlbumJob) -> Vec<AlbumFileInspectionJournal> {
    let mut records = recorded_restart::records(f);
    let mut names = vec![format!("{}.album-move.jsonl", d.job_id)];
    if let Some(job) = &d.singles_job_id {
        names.push(format!("{job}.album.jsonl"));
        names.push(format!("{job}.transfers.jsonl"));
    }
    let mut ordered = Vec::new();
    for name in names {
        let name = MutationName::Windows(name.encode_utf16().collect());
        if let Some(i) = records.iter().position(|r| r.name == name) {
            ordered.push(records.remove(i));
        }
    }
    assert!(records.is_empty());
    ordered
}
fn inspect(
    f: &Fixture,
    d: &RecordedWholeAlbumJob,
    root: bool,
    c: &InspectionControl,
    cursor: usize,
) -> sorter_executor::Result<RecordedWholeAlbumInspection> {
    recover_recorded_whole_album_job(d, grants(f, root), records(f, d), c, cursor)
}
fn complete(r: &RecordedWholeAlbumInspection) {
    assert_eq!(r.content_scope, "recorded_main_hash_identity_membership");
    assert!(r.recovery.recorded_finished);
    assert!(!r.recovery.incomplete_tail);
    assert!(r.recovery.pending_albums.is_empty());
    assert!(r.recovery.pending_directories.is_empty());
    assert!(r.recovery.recorded.iter().all(|r| r.status == "complete"));
    assert!(
        r.recovery
            .source_evidence
            .iter()
            .all(|r| matches!(r.status, AlbumDirectoryEvidenceStatus::Absent))
    );
    assert!(
        r.recovery
            .destination_evidence
            .iter()
            .all(|r| matches!(r.status, AlbumDirectoryEvidenceStatus::SameIdentity))
    );
    assert_eq!(r.files.len(), r.recorded.files.len());
    for row in &r.files {
        match (&row.main, &row.transfer) {
            (Some(main), None) => assert_eq!(main.status, "verified"),
            (None, Some(transfer)) => assert_eq!(transfer.status, RecoveryStatus::Committed),
            other => panic!("missing or invented file evidence: {other:?}"),
        }
    }
    assert_eq!(r.membership.len(), r.recorded.directories.len());
    assert!(r.membership.iter().all(|m| m.status == "verified"));
    for expected in &r.recorded.directories {
        assert!(
            r.directories
                .iter()
                .any(|d| d.reference == expected.destination
                    && d.expected_identity == expected.identity
                    && matches!(d.status, AlbumDirectoryEvidenceStatus::SameIdentity))
        );
    }
}
#[test]
fn whole_restart_fresh_parent_or_source_verifies_every_atomic_descendant_read_only() {
    for root in [false, true] {
        let (f, _) = completed(root, false);
        let d = descriptor(&f);
        let before = owned_tree_snapshot(f._tree.path());
        let journals = journal_bytes(&f);
        let c = control();
        let result = inspect(&f, &d, root, &c, 0).unwrap();
        complete(&result);
        assert_eq!(c.snapshot().unwrap().completed_operations, 5);
        assert_eq!(owned_tree_snapshot(f._tree.path()), before);
        assert_eq!(journal_bytes(&f), journals);
    }
}
#[test]
fn whole_restart_mixed_rows_preserve_one_album_and_real_deferred_single() {
    let (f, _) = completed(false, true);
    let d = descriptor(&f);
    let before = owned_tree_snapshot(f._tree.path());
    let c = control();
    let result = inspect(&f, &d, false, &c, d.files.len()).unwrap();
    complete(&result);
    assert_eq!(result.files.len(), 6);
    assert!(
        result.files[..5]
            .iter()
            .all(|r| r.main.is_some() && r.transfer.is_none())
    );
    assert!(result.files[5].main.is_none());
    assert_eq!(result.files[5].transfer.as_ref().unwrap().operation, 1);
    assert_eq!(c.snapshot().unwrap().completed_operations, 6);
    assert_eq!(owned_tree_snapshot(f._tree.path()), before);
}
#[test]
fn whole_restart_detects_changed_main_and_unrecorded_members_without_writes() {
    for extra in [false, true] {
        let (f, _) = completed(true, false);
        let d = descriptor(&f);
        if extra {
            std::fs::write(f.destination().join("foreign.txt"), b"foreign").unwrap();
        } else {
            std::fs::write(f.destination().join("cover.jpg"), b"changed content").unwrap();
        }
        let before = owned_tree_snapshot(f._tree.path());
        let result = inspect(&f, &d, true, &control(), 0).unwrap();
        if extra {
            assert!(result.membership.iter().any(|m| m.status == "changed"));
        } else {
            assert!(
                result
                    .files
                    .iter()
                    .any(|r| r.main.as_ref().is_some_and(|m| m.status != "verified"))
            );
        }
        assert_eq!(owned_tree_snapshot(f._tree.path()), before);
    }
}
#[test]
fn whole_restart_first_media_chunk_cancels_and_fresh_cursor_rereads_all() {
    let (f, _) = completed(true, false);
    let d = descriptor(&f);
    let before = owned_tree_snapshot(f._tree.path());
    let flag = Arc::new(AtomicBool::new(false));
    let cancelled = flag.clone();
    let baseline = Arc::new(AtomicU64::new(0));
    let operation = Arc::new(AtomicUsize::new(usize::MAX));
    let fired = Arc::new(AtomicBool::new(false));
    let fired_callback = fired.clone();
    let c = InspectionControl::with_progress_interval(flag, std::time::Duration::ZERO, move |p| {
        if let Some(current) = p.operation {
            if operation
                .compare_exchange(usize::MAX, current, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                baseline.store(p.bytes_read, Ordering::Release);
            } else if p.bytes_read > baseline.load(Ordering::Acquire) {
                assert_eq!(p.completed_operations, 0);
                fired_callback.store(true, Ordering::Release);
                cancelled.store(true, Ordering::Release);
            }
        }
    });
    assert!(matches!(inspect(&f, &d, true, &c, 0), Err(E::Cancelled)));
    assert!(fired.load(Ordering::Acquire));
    let c = control();
    complete(&inspect(&f, &d, true, &c, 3).unwrap());
    assert_eq!(c.snapshot().unwrap().completed_operations, 5);
    assert_eq!(owned_tree_snapshot(f._tree.path()), before);
}
#[test]
fn whole_restart_rejects_changed_retained_digest_and_reappeared_original() {
    let (f, _) = completed(true, false);
    let d = descriptor(&f);
    let mut selected = records(&f, &d);
    selected[0].sha256 = "0".repeat(64);
    let before = owned_tree_snapshot(f._tree.path());
    assert!(matches!(
        recover_recorded_whole_album_job(&d, grants(&f, true), selected, &control(), 0),
        Err(E::Stale)
    ));
    assert_eq!(owned_tree_snapshot(f._tree.path()), before);
    std::fs::create_dir(&f.source).unwrap();
    let before = owned_tree_snapshot(f._tree.path());
    assert!(matches!(
        inspect(&f, &d, true, &control(), 0),
        Err(E::Stale)
    ));
    assert_eq!(owned_tree_snapshot(f._tree.path()), before);
}
#[test]
fn whole_restart_native_prefix_is_rebound_to_fresh_grants_not_header_consistency() {
    for source in [false, true] {
        let (f, bytes) = completed(true, false);
        let forged = header_only(&bytes, |value| {
            let field = if source {
                &mut value["observed_sources"][0]["reference"]
            } else {
                &mut value["observed_target"]
            };
            let r: StorageRef = serde_json::from_value(field.clone()).unwrap();
            let StorageRef::WindowsPath { mut utf16 } = r else {
                panic!("Windows fixture")
            };
            let index = utf16.iter().rposition(|u| *u == b'\\' as u16).unwrap() + 1;
            utf16.splice(index..index, "foreign-prefix\\".encode_utf16());
            *field = serde_json::to_value(StorageRef::WindowsPath { utf16 }).unwrap();
        });
        let d = inspect_recorded_whole_album_job(&forged, &control()).unwrap();
        let path = std::fs::read_dir(&f.journal)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".album-move.jsonl")
            })
            .unwrap();
        std::fs::write(path, forged).unwrap();
        let before = owned_tree_snapshot(f._tree.path());
        assert!(inspect(&f, &d, true, &control(), 0).is_err());
        assert_eq!(owned_tree_snapshot(f._tree.path()), before);
    }
}
#[test]
fn whole_restart_changed_file_and_directory_children_have_exact_location_coverage() {
    for root in [false, true] {
        let (f, _) = completed(root, false);
        let d = descriptor(&f);
        let before = owned_tree_snapshot(f._tree.path());
        let result = inspect(&f, &d, root, &control(), 0).unwrap();
        complete(&result);
        assert_eq!(d.child_locations.len(), 2);
        assert_eq!(result.recovery.child_renames.len(), 2);
        assert_eq!(result.recovery.child_locations.len(), 4);
        for expected in &d.child_locations {
            for at_destination in [false, true] {
                let rows = result
                    .recovery
                    .child_locations
                    .iter()
                    .filter(|r| {
                        r.operation == expected.operation
                            && r.planner_index == expected.planner_index
                            && r.at_destination == at_destination
                    })
                    .collect::<Vec<_>>();
                assert_eq!(rows.len(), 1);
                assert!(rows[0].observed.is_none());
                assert!(rows[0].error.is_none());
            }
        }
        assert_eq!(owned_tree_snapshot(f._tree.path()), before);
    }
}

#[test]
fn stopped_whole_root_restart_checks_present_root_membership_and_late_child() {
    let f = Fixture::new(false, true);
    let mut job = f.job();
    // Create an owned collision only after admission. The actual writer records
    // the parent stop and never publishes or removes the selected source root.
    std::fs::create_dir(f.destination().parent().unwrap()).unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Partial, "{result:?}");
    assert_eq!(result.albums_completed, 0);
    drop(job);
    assert!(f.source.is_dir());
    let d = descriptor(&f);
    let before = owned_tree_snapshot(f._tree.path());
    let inspected = inspect(&f, &d, false, &control(), 0).unwrap();
    assert!(inspected.recovery.recorded.is_empty());
    assert!(!inspected.recovery.pending_directories.is_empty());
    assert!(
        inspected
            .membership
            .iter()
            .any(|m| m.reference == d.source.reference && m.status == "verified")
    );
    assert!(
        inspected
            .files
            .iter()
            .all(|f| f.main.as_ref().is_some_and(|m| m.status != "verified"))
    );
    assert_eq!(owned_tree_snapshot(f._tree.path()), before);
    std::fs::write(
        f.source.join("late-unrecorded.txt"),
        b"preserve late source child",
    )
    .unwrap();
    let before = owned_tree_snapshot(f._tree.path());
    let inspected = inspect(&f, &d, false, &control(), 0).unwrap();
    assert!(
        inspected
            .membership
            .iter()
            .any(|m| m.reference == d.source.reference && m.status == "changed")
    );
    assert_eq!(owned_tree_snapshot(f._tree.path()), before);
}
