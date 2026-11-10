//! Actual Windows operations on generated fixtures; selected roots never own cleanup.
#![cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, binding::*, digest, results::*, windows_copy::WindowsPlanningStorage,
    windows_job::*,
};
use sorter_io::desktop::{DesktopStorage, windows_mutation::*};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
struct Metadata;
impl MetadataProvider for Metadata {
    fn open(&mut self, _: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        Err(MetadataError::LegacyOpen {
            kind: "untagged".into(),
            detail: "fixture".into(),
        })
    }
}
struct Ai;
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        AiProfile::Disabled
    }
    fn classify(&mut self, _: &str) -> Result<sorter_core::ClassifierFact, RecognitionError> {
        panic!("disabled")
    }
    fn ner(&mut self, _: &str) -> Result<Vec<String>, RecognitionError> {
        panic!("disabled")
    }
}
// The released alias path intentionally returns only its first match. A real
// duet decision requires multiple NER artists, supplied deterministically here.
struct DuetAi;
impl RecognitionProvider for DuetAi {
    fn profile(&self) -> AiProfile {
        AiProfile::Configured {
            bundle_id: "fixture-duet-ner".into(),
            available: true,
        }
    }
    fn classify(&mut self, _: &str) -> Result<sorter_core::ClassifierFact, RecognitionError> {
        Ok(sorter_core::ClassifierFact::Accept)
    }
    fn ner(&mut self, _: &str) -> Result<Vec<String>, RecognitionError> {
        Ok(vec!["אמן".into(), "זמר".into()])
    }
}
struct Fixture {
    tree: tempfile::TempDir,
    source: PathBuf,
    target: PathBuf,
    journal: PathBuf,
    plan: JobPlanV2,
    scope: ExecutionScope,
}
impl Fixture {
    fn new(copy: bool) -> Self {
        Self::with_content(copy, None)
    }
    fn with_content(copy: bool, content: Option<&[u8]>) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let source = tree.path().join("source");
        let target = tree.path().join("target");
        let journal = tree.path().join("journal");
        for p in [&source, &target, &journal] {
            std::fs::create_dir(p).unwrap();
        }
        for i in 0..2 {
            std::fs::write(
                source.join(format!("song-{i}-שלום.mp3")),
                content.map_or_else(
                    || format!("owned-content-{i}").into_bytes(),
                    |bytes| bytes.to_vec(),
                ),
            )
            .unwrap();
        }
        let mut reads = WindowsPlanningStorage::for_preview(&source, &target).unwrap();
        let request = JobRequestV2 {
            mode: JobMode::Sort,
            source: reads.reference(&source).unwrap(),
            target: Some(reads.reference(&target).unwrap()),
            settings: Settings {
                copy_mode: copy,
                main_folder_only: true,
                singles_folder: false,
                ..Settings::default()
            },
            alias_rows: vec![vec!["song".into(), "אמן".into()]],
            resource_ids: vec!["fixture".into()],
            adapter_versions: vec!["untagged".into()],
            ai_profile: AiProfile::Disabled,
        };
        let plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai).unwrap();
        assert_eq!(plan.operations.len(), 2, "{plan:?}");
        let resources = Resources {
            resource_ids: request.resource_ids,
            adapter_versions: request.adapter_versions,
            ai_profile: AiProfile::Disabled,
            resource_hashes: vec![digest(b"fixture")],
            adapter_hashes: vec![digest(b"untagged")],
            model_hash: None,
        };
        let scope = prepare_scope(&plan, &mut reads, &resources, &AtomicBool::new(false)).unwrap();
        Self {
            tree,
            source,
            target,
            journal,
            plan,
            scope,
        }
    }
    fn roots(&self) -> WindowsMutationStorage {
        let mut reads = DesktopStorage::new(&[
            self.source.clone(),
            self.target.clone(),
            self.journal.clone(),
        ])
        .unwrap();
        let mut grant = |p: &PathBuf| {
            let r = reads.reference(p).unwrap();
            let id = reads.stat(&r).unwrap().identity.unwrap();
            WindowsSelectedRootGrant::new(p.clone(), id).unwrap()
        };
        WindowsMutationStorage::for_selected_roots(
            grant(&self.source),
            grant(&self.target),
            grant(&self.journal),
        )
        .unwrap()
    }
    fn job(&self) -> SelectedWindowsJob {
        SelectedWindowsJob::prepare(
            self.roots(),
            &self.plan,
            &mut Metadata,
            &mut Ai,
            &self.scope.resources,
            &self.scope,
        )
        .unwrap()
    }
    fn check_contents(&self, copy: bool) {
        for i in 0..2 {
            let n = format!("song-{i}-שלום.mp3");
            assert_eq!(
                std::fs::read(self.target.join("אמן").join(&n)).unwrap(),
                format!("owned-content-{i}").as_bytes()
            );
            assert_eq!(self.source.join(n).exists(), copy);
        }
    }
}
#[test]
fn selected_copy_creates_artist_folder_and_runs_full_job() {
    let f = Fixture::new(true);
    let mut job = f.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 2);
    assert_eq!(result.committed_operations, vec![0, 1]);
    assert_eq!(job.created_directory_receipts().len(), 1);
    assert_eq!(result.effect_receipts.len(), 2);
    assert!(
        result
            .effect_receipts
            .iter()
            .all(|receipt| receipt.kind == "file_published")
    );
    assert_eq!(result.moved_sources, 0);
    assert_eq!(job.journal_locations().len(), 2);
    let recovery = RecoveryScope {
        original_scope: f.scope.clone(),
        inspected_binding_sha256: job.binding_sha256().unwrap(),
    };
    let report = job.inspect_recovery(&recovery).unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::Committed
                && operation.copy_metadata
                    == sorter_executor::recovery::CopyMetadataRecoveryStatus::Verified)
    );
    drop(job);
    f.check_contents(true);
    assert!(f.tree.path().exists());
}
#[test]
fn selected_rename_creates_artist_folder_and_moves_known_objects() {
    let f = Fixture::new(false);
    let mut job = f.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.moved_sources, 2);
    drop(job);
    f.check_contents(false);
}
#[test]
fn preview_content_change_is_not_reauthorized() {
    let f = Fixture::new(true);
    std::fs::write(f.source.join("song-0-שלום.mp3"), b"changed").unwrap();
    assert!(matches!(
        SelectedWindowsJob::prepare(
            f.roots(),
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        ),
        Err(E::Stale)
    ));
    assert!(!f.target.join("אמן").exists());
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
}
#[test]
fn selected_job_cancellation_and_second_run_never_replay() {
    let f = Fixture::new(true);
    let mut job = f.job();
    assert!(matches!(
        job.run(&AtomicBool::new(true), &mut |_| {}),
        Err(E::Cancelled)
    ));
    assert!(matches!(
        job.run(&AtomicBool::new(false), &mut |_| {}),
        Err(E::NeedsRecovery)
    ));
    assert!(!f.target.join("אמן").exists());
}
#[test]
fn selected_copy_cancellation_during_bytes_keeps_sources() {
    let f = Fixture::new(true);
    let mut job = f.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |_| cancel.store(true, Ordering::Release))
        .unwrap();
    assert_eq!(result.status, JobStatus::Cancelled);
    assert_eq!(result.moved_sources, 0);
    assert_eq!(result.committed_destinations, 0);
    drop(job);
    for i in 0..2 {
        assert!(f.source.join(format!("song-{i}-שלום.mp3")).exists());
    }
}
#[test]
fn selected_target_collision_after_preview_fails_without_overwrite() {
    let f = Fixture::new(true);
    let mut job = f.job();
    std::fs::create_dir(f.target.join("אמן")).unwrap();
    std::fs::write(f.target.join("אמן/song-0-שלום.mp3"), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    drop(job);
    assert_eq!(
        std::fs::read(f.target.join("אמן/song-0-שלום.mp3")).unwrap(),
        b"foreign"
    );
    assert!(f.source.join("song-0-שלום.mp3").exists());
}
#[test]
fn selected_job_read_only_recovery_checks_both_journals() {
    let f = Fixture::new(false);
    let mut job = f.job();
    let recovery = RecoveryScope {
        original_scope: f.scope.clone(),
        inspected_binding_sha256: job.binding_sha256().unwrap(),
    };
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    let roots = f.roots();
    let locations = job.journal_locations();
    let dirs = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    let report =
        inspect_recovery(&roots, &recovery, job.job_id(), &dirs, Some(&transfers)).unwrap();
    assert!(report.directories.complete);
    assert_eq!(report.directories.observed.len(), 1);
    assert!(report.directories.pending.is_empty());
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|o| o.status == sorter_executor::recovery::RecoveryStatus::Committed)
    );
    let mut corrupt = dirs.clone();
    corrupt[1] = b'!';
    assert!(inspect_recovery(&roots, &recovery, job.job_id(), &corrupt, Some(&transfers)).is_err());
    assert_eq!(
        dirs,
        roots
            .read_selected_journal(locations[0].name(), locations[0].identity())
            .unwrap()
    );
}

#[test]
fn selected_duet_copy_requires_metadata_at_every_distinct_destination() {
    use std::os::windows::fs::MetadataExt;
    let mut f = Fixture::new(true);
    for index in 0..2 {
        std::fs::rename(
            f.source.join(format!("song-{index}-שלום.mp3")),
            f.source.join(format!("song other-{index}-שלום.mp3")),
        )
        .unwrap();
    }
    f.plan.request.settings.duet_mode = true;
    f.plan.request.alias_rows.clear();
    f.plan.request.ai_profile = DuetAi.profile();
    f.scope.resources.ai_profile = DuetAi.profile();
    f.scope.resources.model_hash = Some(digest(b"deterministic fixture duet provider"));
    {
        let mut reads = WindowsPlanningStorage::for_preview(&f.source, &f.target).unwrap();
        f.plan = plan_job(&f.plan.request, &mut reads, &mut Metadata, &mut DuetAi).unwrap();
        f.scope = prepare_scope(
            &f.plan,
            &mut reads,
            &f.scope.resources,
            &AtomicBool::new(false),
        )
        .unwrap();
    }
    assert_eq!(f.plan.operations.len(), 4);
    let mut job = SelectedWindowsJob::prepare(
        f.roots(),
        &f.plan,
        &mut Metadata,
        &mut DuetAi,
        &f.scope.resources,
        &f.scope,
    )
    .unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 4);
    assert_eq!(result.committed_operations, vec![0, 1, 2, 3]);
    assert_eq!(job.created_directory_receipts().len(), 2);
    assert_eq!(result.moved_sources, 0);
    let recovery = RecoveryScope {
        original_scope: f.scope.clone(),
        inspected_binding_sha256: job.binding_sha256().unwrap(),
    };
    let report = job.inspect_recovery(&recovery).unwrap();
    let operations = report.transfers.unwrap().operations;
    assert_eq!(operations.len(), 4);
    assert!(operations.iter().all(|operation| operation.status
        == sorter_executor::recovery::RecoveryStatus::Committed
        && operation.copy_metadata
            == sorter_executor::recovery::CopyMetadataRecoveryStatus::Verified));
    drop(job);
    for index in 0..2 {
        let leaf = format!("song other-{index}-שלום.mp3");
        let original = f.source.join(&leaf);
        for artist in ["אמן", "זמר"] {
            let copied = f.target.join(artist).join(&leaf);
            assert_eq!(
                std::fs::read(&copied).unwrap(),
                std::fs::read(&original).unwrap()
            );
            assert_eq!(
                std::fs::metadata(&copied).unwrap().last_write_time(),
                std::fs::metadata(&original).unwrap().last_write_time()
            );
        }
    }
}

fn recorded_fixture(copy: bool) -> (Fixture, Vec<u8>, Vec<u8>) {
    let f = Fixture::new(copy);
    let mut job = f.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    let locations = job.journal_locations();
    let roots = f.roots();
    let dirs = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    drop(job);
    drop(roots);
    (f, dirs, transfers)
}
#[test]
fn restart_recovery_reconstructs_only_reads_from_fresh_matching_roots() {
    for copy in [true, false] {
        let (f, dirs, transfers) = recorded_fixture(copy);
        let recorded = inspect_recorded_selected_job(&dirs).unwrap();
        assert_eq!(recorded.operations.len(), 2);
        let summary = summarize_recorded_selected_job(&dirs, Some(&transfers)).unwrap();
        assert!(summary.recorded_complete);
        assert!(
            !summarize_recorded_selected_job(&dirs, None)
                .unwrap()
                .recorded_complete
        );
        let report = recover_selected_journals(&f.roots(), &dirs, Some(&transfers)).unwrap();
        assert!(
            report
                .transfers
                .unwrap()
                .operations
                .iter()
                .all(|o| o.status == sorter_executor::recovery::RecoveryStatus::Committed)
        );
        f.check_contents(copy);
    }
}
#[test]
fn restart_recovery_rejects_fresh_different_root_identity() {
    let (f, dirs, transfers) = recorded_fixture(true);
    let old = f.source.with_file_name("old-source");
    std::fs::rename(&f.source, &old).unwrap();
    std::fs::create_dir(&f.source).unwrap();
    assert!(matches!(
        recover_selected_journals(&f.roots(), &dirs, Some(&transfers)),
        Err(E::Scope)
    ));
    assert!(old.join("song-0-שלום.mp3").exists());
}
#[test]
fn restart_recovery_treats_journal_completion_as_history_not_live_truth() {
    let (f, dirs, transfers) = recorded_fixture(true);
    std::fs::write(f.target.join("אמן/song-0-שלום.mp3"), b"external-change").unwrap();
    assert!(
        summarize_recorded_selected_job(&dirs, Some(&transfers))
            .unwrap()
            .recorded_complete
    );
    let report = recover_selected_journals(&f.roots(), &dirs, Some(&transfers)).unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .any(|o| o.status != sorter_executor::recovery::RecoveryStatus::Committed)
    );
    assert_eq!(
        std::fs::read(f.target.join("אמן/song-0-שלום.mp3")).unwrap(),
        b"external-change"
    );
}
#[test]
fn restart_recovery_rejects_corrupt_oversize_and_unfinished_records() {
    let (_f, dirs, transfers) = recorded_fixture(true);
    let mut bad = dirs.clone();
    bad[1] = b'!';
    assert!(inspect_recorded_selected_job(&bad).is_err());
    assert!(inspect_recorded_selected_job(&vec![0; MAX_SELECTED_JOURNAL_BYTES + 1]).is_err());
    let mut tail = transfers.clone();
    tail.extend_from_slice(b"{unfinished");
    assert!(
        !summarize_recorded_selected_job(&dirs, Some(&tail))
            .unwrap()
            .recorded_complete
    );
    let mut bad = transfers;
    bad[1] = b'!';
    assert!(summarize_recorded_selected_job(&dirs, Some(&bad)).is_err());
}

#[test]
fn restart_recovery_preserves_partial_owned_copy_and_both_journals() {
    let f = Fixture::new(true);
    let mut job = f.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.operation == 1 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 1);
    let locations = job.journal_locations();
    let roots = f.roots();
    let directories = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    let recorded_names = std::fs::read_dir(&f.journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let before = recorded_names
        .iter()
        .map(|path| std::fs::read(path).unwrap())
        .collect::<Vec<_>>();
    drop(roots);
    drop(job);
    // New grants and data-only bytes are the sole restart inputs; no retained
    // executable binding, mutable journal, or resume entry point is reused.
    assert!(
        !summarize_recorded_selected_job(&directories, Some(&transfers))
            .unwrap()
            .recorded_complete
    );
    let report = recover_selected_journals(&f.roots(), &directories, Some(&transfers)).unwrap();
    let operations = report.transfers.unwrap().operations;
    assert_eq!(
        operations[0].status,
        sorter_executor::recovery::RecoveryStatus::Committed
    );
    assert_ne!(
        operations[1].status,
        sorter_executor::recovery::RecoveryStatus::Committed
    );
    assert_eq!(
        std::fs::read(f.target.join("אמן/song-0-שלום.mp3")).unwrap(),
        b"owned-content-0"
    );
    assert!(!f.target.join("אמן/song-1-שלום.mp3").exists());
    for index in 0..2 {
        assert_eq!(
            std::fs::read(f.source.join(format!("song-{index}-שלום.mp3"))).unwrap(),
            format!("owned-content-{index}").as_bytes()
        );
    }
    for (path, expected) in recorded_names.iter().zip(before) {
        assert_eq!(std::fs::read(path).unwrap(), expected);
    }
}

#[test]
fn restart_recovery_preserves_current_copy_metadata_validation() {
    let (f, directories, transfers) = recorded_fixture(true);
    let destination = f.target.join("אמן/song-0-שלום.mp3");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&destination)
        .unwrap();
    file.set_times(
        std::fs::FileTimes::new()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000)),
    )
    .unwrap();
    drop(file);
    assert!(
        summarize_recorded_selected_job(&directories, Some(&transfers))
            .unwrap()
            .recorded_complete
    );
    let report = recover_selected_journals(&f.roots(), &directories, Some(&transfers)).unwrap();
    let operations = report.transfers.unwrap().operations;
    assert_eq!(
        operations[0].copy_metadata,
        sorter_executor::recovery::CopyMetadataRecoveryStatus::Mismatch
    );
    assert_eq!(
        operations[0].status,
        sorter_executor::recovery::RecoveryStatus::NeedsRecovery
    );
    assert_eq!(std::fs::read(destination).unwrap(), b"owned-content-0");
}

#[test]
fn restart_recovery_rejects_replaced_artist_directory_with_same_files() {
    let (f, directories, transfers) = recorded_fixture(true);
    let artist = f.target.join("אמן");
    let replaced = f.target.join("old-artist-directory");
    std::fs::rename(&artist, &replaced).unwrap();
    std::fs::create_dir(&artist).unwrap();
    for index in 0..2 {
        let name = format!("song-{index}-שלום.mp3");
        // Same-volume rename retains the exact copied file objects and metadata.
        std::fs::rename(replaced.join(&name), artist.join(&name)).unwrap();
    }
    let report = recover_selected_journals(&f.roots(), &directories, Some(&transfers)).unwrap();
    assert_eq!(report.directories.unverified.len(), 1);
    assert!(!report.observation_limit_reached);
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| {
                operation.status == sorter_executor::recovery::RecoveryStatus::Committed
            })
    );
    // Committed file receipts cannot erase the distinct directory identity issue.
    f.check_contents(true);
    assert!(replaced.exists());
}

#[test]
fn restart_recovery_bounds_changed_large_file_reads_without_success() {
    let (f, directories, transfers) = recorded_fixture(true);
    let destination = f.target.join("אמן/song-0-שלום.mp3");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&destination)
        .unwrap();
    file.set_len(MAX_RESTART_CONTENT_BYTES + 1).unwrap();
    drop(file);
    let journals = std::fs::read_dir(&f.journal)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    let report = recover_selected_journals(&f.roots(), &directories, Some(&transfers)).unwrap();
    assert!(report.observation_limit_reached);
    assert_eq!(
        report.transfers.unwrap().operations[0].status,
        sorter_executor::recovery::RecoveryStatus::NeedsRecovery
    );
    assert_eq!(
        std::fs::metadata(destination).unwrap().len(),
        MAX_RESTART_CONTENT_BYTES + 1
    );
    for (path, expected) in journals {
        assert_eq!(std::fs::read(path).unwrap(), expected);
    }
}

#[test]
fn restart_recovery_limits_parser_records_before_strict_phase_scans() {
    let (_, directories, mut transfers) = recorded_fixture(true);
    transfers.extend(std::iter::repeat_n(b'\n', 2049));
    assert!(matches!(
        summarize_recorded_selected_job(&directories, Some(&transfers)),
        Err(E::Unsupported)
    ));
    assert!(matches!(
        inspect_recorded_selected_job(&vec![0; MAX_RESTART_JOURNAL_BYTES + 1]),
        Err(E::Unsupported)
    ));
}

#[test]
fn restart_recovery_budgets_repeated_source_and_destination_bytes_cumulatively() {
    let payload = vec![b'x'; 20 * 1024 * 1024];
    let f = Fixture::with_content(true, Some(&payload));
    let mut job = f.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let roots = f.roots();
    let locations = job.journal_locations();
    let directories = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    drop(roots);
    drop(job);
    let report = recover_selected_journals(&f.roots(), &directories, Some(&transfers)).unwrap();
    assert!(report.observation_limit_reached);
    let operations = report.transfers.unwrap().operations;
    assert_eq!(
        operations[0].status,
        sorter_executor::recovery::RecoveryStatus::Committed
    );
    assert_eq!(
        operations[1].status,
        sorter_executor::recovery::RecoveryStatus::NeedsRecovery
    );
    // Each file is individually below the 64 MiB cap. Both source/destination
    // observations together exceed it and therefore cannot produce an all-clear.
    for index in 0..2 {
        let name = format!("song-{index}-שלום.mp3");
        assert_eq!(
            std::fs::metadata(f.source.join(&name)).unwrap().len(),
            payload.len() as u64
        );
        assert_eq!(
            std::fs::metadata(f.target.join("אמן").join(&name))
                .unwrap()
                .len(),
            payload.len() as u64
        );
    }
}

// Owned fixture evidence excludes access time, which reads may update.
fn restart_owned_snapshot(
    root: &std::path::Path,
) -> Vec<(PathBuf, EntryObservation, u32, u64, u64, Option<String>)> {
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
        ));
    }
    snapshot.sort_by(|left, right| left.0.cmp(&right.0));
    snapshot
}

#[test]
fn streaming_restart_verifies_more_than_the_interim_music_budget() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::Arc;
    let payload = vec![b's'; 20 * 1024 * 1024];
    let f = Fixture::with_content(true, Some(&payload));
    let mut job = f.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let locations = job.journal_locations();
    let roots = f.roots();
    let directories = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    drop(roots);
    drop(job);
    let before = restart_owned_snapshot(f.tree.path());
    let control = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
    let report = recover_selected_journals_controlled(
        &f.roots(),
        &directories,
        Some(&transfers),
        &control,
        0,
    )
    .unwrap();
    assert!(!report.observation_limit_reached);
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::Committed)
    );
    let progress = control.snapshot().unwrap();
    assert_eq!(progress.bytes_read, 4 * payload.len() as u64);
    assert!(progress.bytes_read > MAX_RESTART_CONTENT_BYTES);
    assert_eq!(progress.completed_operations, 2);
    assert_eq!(progress.total_operations, 2);
    assert_eq!(restart_owned_snapshot(f.tree.path()), before);
}

#[test]
fn streaming_restart_cancels_at_actual_chunk_and_reopens_without_cached_hashes() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::Arc;
    let payload = vec![b'c'; 1024 * 1024];
    let f = Fixture::with_content(true, Some(&payload));
    let mut job = f.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    let locations = job.journal_locations();
    let roots = f.roots();
    let directories = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    drop(roots);
    drop(job);
    let before = restart_owned_snapshot(f.tree.path());
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let control = InspectionControl::with_progress_interval(
        cancel,
        std::time::Duration::ZERO,
        move |progress| {
            if progress.bytes_read >= 65_536 {
                flag.store(true, Ordering::Release);
            }
        },
    );
    assert!(matches!(
        recover_selected_journals_controlled(
            &f.roots(),
            &directories,
            Some(&transfers),
            &control,
            0
        ),
        Err(E::Cancelled)
    ));
    assert_eq!(control.snapshot().unwrap().bytes_read, 65_536);
    assert_eq!(control.snapshot().unwrap().completed_operations, 0);
    let fresh = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
    let report =
        recover_selected_journals_controlled(&f.roots(), &directories, Some(&transfers), &fresh, 1)
            .unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status
                == sorter_executor::recovery::RecoveryStatus::Committed)
    );
    assert_eq!(
        fresh.snapshot().unwrap().bytes_read,
        4 * payload.len() as u64
    );
    assert_eq!(restart_owned_snapshot(f.tree.path()), before);
    // A subsequent reopened run cannot reuse that successful result.
    std::fs::write(
        f.target.join("אמן/song-0-שלום.mp3"),
        b"changed-after-inspection",
    )
    .unwrap();
    let changed = restart_owned_snapshot(f.tree.path());
    let fresh = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
    let report =
        recover_selected_journals_controlled(&f.roots(), &directories, Some(&transfers), &fresh, 1)
            .unwrap();
    assert_eq!(
        report.transfers.unwrap().operations[0].status,
        sorter_executor::recovery::RecoveryStatus::NeedsRecovery
    );
    assert_eq!(restart_owned_snapshot(f.tree.path()), changed);
}

#[test]
fn streaming_restart_keeps_parser_bounds_and_cancellation_distinct() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::Arc;
    let (_, directories, mut transfers) = recorded_fixture(true);
    let cancelled = InspectionControl::new(Arc::new(AtomicBool::new(true)), |_| {});
    assert!(matches!(
        summarize_recorded_selected_job_with_control(
            &directories,
            Some(&transfers),
            Some(&cancelled)
        ),
        Err(E::Cancelled)
    ));
    let control = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
    transfers.extend(std::iter::repeat_n(b'\n', 65_537));
    assert!(matches!(
        summarize_recorded_selected_job_with_control(
            &directories,
            Some(&transfers),
            Some(&control)
        ),
        Err(E::Unsupported)
    ));
}
