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
                format!("owned-content-{i}"),
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
