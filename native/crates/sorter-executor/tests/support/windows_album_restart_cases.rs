//! Fresh read-only restart ownership over actual writer journals and owned media.
use super::*;
use sorter_executor::{inspection_control::InspectionControl, recovery::RecoveryStatus};
use std::{os::windows::ffi::OsStrExt, sync::Arc};

fn control() -> InspectionControl<'static> {
    InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {})
}
fn observed_identity(path: &std::path::Path) -> ObjectIdentity {
    let mut reads = DesktopStorage::new(&[path.to_path_buf()]).unwrap();
    let reference = reads.reference(path).unwrap();
    reads.stat(&reference).unwrap().identity.unwrap()
}
fn grant(path: &std::path::Path) -> WindowsSelectedRootGrant {
    WindowsSelectedRootGrant::new(path.to_path_buf(), observed_identity(path)).unwrap()
}
pub(super) fn records(fixture: &Fixture) -> Vec<AlbumFileInspectionJournal> {
    let mut paths = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    let mut reads = DesktopStorage::new(std::slice::from_ref(&fixture.journal)).unwrap();
    paths
        .into_iter()
        .map(|path| {
            let reference = reads.reference(&path).unwrap();
            let entry = reads.stat(&reference).unwrap();
            let bytes = std::fs::read(&path).unwrap();
            AlbumFileInspectionJournal {
                name: sorter_storage_contracts::MutationName::Windows(
                    path.file_name().unwrap().encode_wide().collect(),
                ),
                identity: entry.identity.unwrap(),
                length: bytes.len() as u64,
                sha256: digest(&bytes),
            }
        })
        .collect()
}
fn descriptor(fixture: &Fixture) -> RecordedAlbumFileJob {
    let path = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".album.jsonl")
        })
        .unwrap();
    inspect_recorded_album_file_job(&std::fs::read(path).unwrap(), &control()).unwrap()
}
pub(super) fn grants(fixture: &Fixture, missing: bool) -> AlbumFileInspectionGrants {
    let source = if missing {
        let parent = fixture.source.parent().unwrap();
        AlbumFileInspectionSource::Missing(
            WindowsAlbumInspectionParentGrant::new(parent.to_path_buf(), observed_identity(parent))
                .unwrap(),
        )
    } else {
        AlbumFileInspectionSource::Present(grant(&fixture.source))
    };
    AlbumFileInspectionGrants {
        source,
        target: grant(&fixture.target),
        journal: grant(&fixture.journal),
    }
}
pub(super) fn journal_bytes(fixture: &Fixture) -> Vec<(PathBuf, Vec<u8>)> {
    let mut paths = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}
// Same owned-tree evidence as the selected-job restart fixtures, extended for
// this suite's known notes ADS. Access time is excluded because reads may update it.
pub(super) type TreeEvidence = Vec<(
    PathBuf,
    EntryObservation,
    u32,
    u64,
    u64,
    Option<String>,
    Option<String>,
)>;
pub(super) fn owned_tree_snapshot(root: &std::path::Path) -> TreeEvidence {
    use std::os::windows::fs::MetadataExt;
    let mut reads = DesktopStorage::new(&[root.to_path_buf()]).unwrap();
    let mut pending = vec![root.to_path_buf()];
    let mut snapshot = Vec::new();
    while let Some(path) = pending.pop() {
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let reference = reads.reference(&path).unwrap();
        let observation = reads.stat(&reference).unwrap();
        let content = metadata
            .is_file()
            .then(|| digest(&std::fs::read(&path).unwrap()));
        let notes = if metadata.is_file() {
            let mut stream = path.as_os_str().to_os_string();
            stream.push(":notes");
            match std::fs::read(PathBuf::from(stream)) {
                Ok(bytes) => Some(digest(&bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => panic!("owned stream snapshot failed: {error}"),
            }
        } else {
            None
        };
        if metadata.is_dir() {
            pending.extend(
                std::fs::read_dir(&path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        }
        snapshot.push((
            path.strip_prefix(root).unwrap().to_path_buf(),
            observation,
            metadata.file_attributes(),
            metadata.creation_time(),
            metadata.last_write_time(),
            content,
            notes,
        ));
    }
    snapshot.sort_by(|left, right| left.0.cmp(&right.0));
    snapshot
}
fn media_snapshot(fixture: &Fixture) -> (TreeEvidence, TreeEvidence) {
    (
        owned_tree_snapshot(fixture._tree.path()),
        owned_tree_snapshot(fixture.target.parent().unwrap()),
    )
}
fn completed_copy(root: bool) -> Fixture {
    let fixture = Fixture::new(true, root);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    drop(job);
    fixture
}
fn complete(report: &RecordedAlbumFileInspection) {
    assert!(report.recovery.manifest_finished);
    assert_eq!(report.recovery.recorded_status.as_deref(), Some("Complete"));
    assert!(report.recovery.pending_directories.is_empty());
    assert!(report.recovery.pending_cleanup.is_empty());
    assert!(report.recovery.pending_metadata.is_empty());
    assert!(
        report
            .recovery
            .directory_metadata_status
            .iter()
            .all(|(_, status)| status == "verified")
    );
    assert_eq!(report.files.len(), report.recorded.file_count);
    for (index, file) in report.files.iter().enumerate() {
        assert_eq!(file.file.physical_operation, index);
        let current = file.recovery.as_ref().unwrap();
        assert_eq!(current.operation, index);
        assert_eq!(current.status, RecoveryStatus::Committed);
    }
}

#[test]
fn copied_album_restart_uses_fresh_present_roots_and_exact_per_file_mapping() {
    for root in [false, true] {
        let fixture = completed_copy(root);
        let before = journal_bytes(&fixture);
        let media_before = media_snapshot(&fixture);
        let report = recover_recorded_album_file_job(
            &descriptor(&fixture),
            grants(&fixture, false),
            records(&fixture),
            &control(),
            0,
        )
        .unwrap();
        complete(&report);
        assert!(matches!(
            report.source_state,
            AlbumInspectionSourceState::Present { .. }
        ));
        assert!(report.files.iter().all(|file| file.file.operation == 0));
        assert_eq!(report.files.len(), 5);
        assert_eq!(journal_bytes(&fixture), before);
        assert_eq!(media_snapshot(&fixture), media_before);
        fixture.verify_copy();
    }
}

#[test]
fn cross_volume_child_album_restart_reads_present_source_root_and_full_streams() {
    let (fixture, _target_owner) = cross_volume_album(false);
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    assert!(fixture.source.exists());
    let before = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    let report = recover_recorded_album_file_job(
        &descriptor(&fixture),
        grants(&fixture, false),
        records(&fixture),
        &control(),
        0,
    )
    .unwrap();
    complete(&report);
    assert!(matches!(
        report.source_state,
        AlbumInspectionSourceState::Present { .. }
    ));
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
    verify_cross_volume_album(&fixture);
}

#[test]
fn cross_volume_removed_root_restart_requires_fresh_parent_and_exact_missing_leaf() {
    let (fixture, _target_owner) = cross_volume_album(true);
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    let recorded = descriptor(&fixture);
    let before = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    let report = recover_recorded_album_file_job(
        &recorded,
        grants(&fixture, true),
        records(&fixture),
        &control(),
        0,
    )
    .unwrap();
    complete(&report);
    assert!(matches!(
        report.source_state,
        AlbumInspectionSourceState::Absent { .. }
    ));
    assert!(
        report
            .recovery
            .directories
            .iter()
            .any(|entry| entry.reference == recorded.source.reference
                && matches!(entry.status, AlbumDirectoryEvidenceStatus::Absent))
    );
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
    verify_cross_volume_album(&fixture);
    std::fs::create_dir(&fixture.source).unwrap();
    let media_before = media_snapshot(&fixture);
    assert!(
        recover_recorded_album_file_job(
            &recorded,
            grants(&fixture, true),
            records(&fixture),
            &control(),
            0
        )
        .is_err()
    );
    assert!(fixture.source.is_dir());
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
}

#[test]
fn restart_rejects_same_identity_length_journal_rewrite_against_discovery_digest() {
    let fixture = completed_copy(false);
    let recorded = descriptor(&fixture);
    let retained = records(&fixture);
    let mut files = journal_bytes(&fixture);
    files[0].1[0] = b' ';
    std::fs::write(&files[0].0, &files[0].1).unwrap();
    let changed = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    assert!(matches!(
        recover_recorded_album_file_job(
            &recorded,
            grants(&fixture, false),
            retained,
            &control(),
            0
        ),
        Err(E::Stale)
    ));
    assert_eq!(journal_bytes(&fixture), changed);
    assert_eq!(media_snapshot(&fixture), media_before);
    fixture.verify_copy();
}

#[test]
fn restart_changed_target_bytes_and_metadata_never_inherit_historical_complete() {
    let fixture = completed_copy(false);
    let recorded = descriptor(&fixture);
    let target = fixture.destination().join("1-שיר.mp3");
    let length = std::fs::metadata(&target).unwrap().len() as usize;
    std::fs::write(&target, vec![b'x'; length]).unwrap();
    let before = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    let report = recover_recorded_album_file_job(
        &recorded,
        grants(&fixture, false),
        records(&fixture),
        &control(),
        0,
    )
    .unwrap();
    assert_eq!(report.recovery.recorded_status.as_deref(), Some("Complete"));
    assert!(
        report
            .files
            .iter()
            .any(|file| file.recovery.as_ref().unwrap().status == RecoveryStatus::NeedsRecovery)
    );
    assert_eq!(std::fs::read(&target).unwrap(), vec![b'x'; length]);
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
}

#[test]
fn restart_partial_copy_preserves_earlier_physical_success_without_album_completion() {
    let fixture = Fixture::new(true, false);
    let cancel = AtomicBool::new(false);
    let mut job = fixture.job();
    let mut streamed_files = 0;
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.bytes_written == progress.source_length {
                streamed_files += 1;
                if streamed_files == 2 {
                    cancel.store(true, Ordering::Release);
                }
            }
        })
        .unwrap();
    assert_eq!(result.status, JobStatus::Cancelled);
    assert_eq!(result.albums_completed, 0);
    drop(job);
    let before = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    let report = recover_recorded_album_file_job(
        &descriptor(&fixture),
        grants(&fixture, false),
        records(&fixture),
        &control(),
        0,
    )
    .unwrap();
    assert_eq!(
        report.recovery.recorded_status.as_deref(),
        Some("Cancelled")
    );
    assert!(report.recovery.completed_albums.is_empty());
    assert!(report.files.iter().any(|file| {
        file.recovery
            .as_ref()
            .is_some_and(|value| value.status == RecoveryStatus::Committed)
    }));
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
    assert!(fixture.album.join("1-שיר.mp3").exists());
}

#[test]
fn restart_cancellation_then_priority_cursor_rereads_every_file_freshly() {
    let mut fixture = Fixture::new(true, false);
    std::fs::write(fixture.album.join("1-שיר.mp3"), vec![41; 196_608]).unwrap();
    fixture.replan();
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    let recorded = descriptor(&fixture);
    let before = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    let cancel = Arc::new(AtomicBool::new(false));
    let observer = cancel.clone();
    let first_file_bytes = std::sync::Mutex::new(None);
    let controlled = InspectionControl::with_progress_interval(
        cancel.clone(),
        std::time::Duration::ZERO,
        move |value| {
            if value.operation.is_some() {
                let mut first = first_file_bytes.lock().unwrap();
                let baseline = first.get_or_insert(value.bytes_read);
                if value.bytes_read > *baseline {
                    observer.store(true, Ordering::Release);
                }
            }
        },
    );
    assert!(matches!(
        recover_recorded_album_file_job(
            &recorded,
            grants(&fixture, false),
            records(&fixture),
            &controlled,
            0
        ),
        Err(E::Cancelled)
    ));
    let resumed_control = control();
    let resumed = recover_recorded_album_file_job(
        &recorded,
        grants(&fixture, false),
        records(&fixture),
        &resumed_control,
        2,
    )
    .unwrap();
    complete(&resumed);
    assert_eq!(resumed_control.snapshot().unwrap().completed_operations, 5);
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
}

#[test]
fn restart_rejects_forged_recorded_descriptor_and_wrong_fresh_target() {
    let fixture = completed_copy(false);
    let mut recorded = descriptor(&fixture);
    let before = journal_bytes(&fixture);
    let media_before = media_snapshot(&fixture);
    recorded.files[0].operation = 999;
    assert!(matches!(
        recover_recorded_album_file_job(
            &recorded,
            grants(&fixture, false),
            records(&fixture),
            &control(),
            0
        ),
        Err(E::Stale)
    ));
    let recorded = descriptor(&fixture);
    let mut wrong = grants(&fixture, false);
    wrong.target = grant(&fixture.source);
    assert!(
        recover_recorded_album_file_job(&recorded, wrong, records(&fixture), &control(), 0)
            .is_err()
    );
    assert_eq!(journal_bytes(&fixture), before);
    assert_eq!(media_snapshot(&fixture), media_before);
    fixture.verify_copy();
}

#[test]
fn copied_album_restart_detects_changed_required_ads_with_unchanged_main_bytes() {
    use std::os::windows::fs::OpenOptionsExt;
    let (fixture, _target_owner) = cross_volume_album(false);
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    let target = fixture.destination().join("1-שיר.mp3");
    let original_main = std::fs::read(&target).unwrap();
    let original_metadata = std::fs::metadata(&target).unwrap();
    let mut ads = target.as_os_str().to_os_string();
    ads.push(":notes");
    let ads = PathBuf::from(ads);
    let original_ads = std::fs::read(&ads).unwrap();
    std::fs::write(&ads, vec![b'x'; original_ads.len()]).unwrap();
    std::fs::OpenOptions::new()
        .access_mode(0x100)
        .open(&target)
        .unwrap()
        .set_times(
            std::fs::FileTimes::new()
                .set_accessed(original_metadata.accessed().unwrap())
                .set_modified(original_metadata.modified().unwrap()),
        )
        .unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), original_main);
    assert_ne!(std::fs::read(&ads).unwrap(), original_ads);
    let before = media_snapshot(&fixture);
    let report = recover_recorded_album_file_job(
        &descriptor(&fixture),
        grants(&fixture, false),
        records(&fixture),
        &control(),
        0,
    )
    .unwrap();
    assert!(
        report
            .files
            .iter()
            .any(|file| file.recovery.as_ref().unwrap().status == RecoveryStatus::NeedsRecovery)
    );
    assert_eq!(media_snapshot(&fixture), before);
}

#[test]
fn album_restart_checks_file_and_directory_metadata_independently_of_content() {
    for directory in [false, true] {
        let fixture = completed_copy(false);
        let target = fixture
            .destination()
            .join(if directory { "booklet" } else { "1-שיר.mp3" });
        let original = std::fs::metadata(&target).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        std::fs::set_permissions(&target, readonly).unwrap();
        let before = media_snapshot(&fixture);
        let report = recover_recorded_album_file_job(
            &descriptor(&fixture),
            grants(&fixture, false),
            records(&fixture),
            &control(),
            0,
        )
        .unwrap();
        if directory {
            assert!(
                report
                    .recovery
                    .directory_metadata_status
                    .iter()
                    .any(|(_, status)| status != "verified")
            );
        } else {
            assert!(report.files.iter().any(
                |file| file.recovery.as_ref().unwrap().status == RecoveryStatus::NeedsRecovery
            ));
        }
        assert_eq!(media_snapshot(&fixture), before);
        std::fs::set_permissions(&target, original).unwrap();
    }
}
