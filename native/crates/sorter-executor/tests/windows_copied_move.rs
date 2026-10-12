//! Generated ordinary/quiescent NTFS fixtures; no production paths are touched.
#![cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{binding::*, digest, recovery::RecoveryStatus, results::JobStatus, windows_copy::WindowsPlanningStorage, windows_job::*};
use sorter_io::desktop::{DesktopStorage, windows_mutation::*};
use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering}};
struct Metadata;
impl MetadataProvider for Metadata {
    fn open(&mut self, _: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        Err(MetadataError::LegacyOpen { kind: "untagged".into(), detail: "fixture".into() })
    }
}
struct Ai(bool);
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        if self.0 { AiProfile::Configured { bundle_id: "fixture-duet".into(), available: true } } else { AiProfile::Disabled }
    }
    fn classify(&mut self, _: &str) -> Result<sorter_core::ClassifierFact, RecognitionError> { Ok(sorter_core::ClassifierFact::Accept) }
    fn ner(&mut self, _: &str) -> Result<Vec<String>, RecognitionError> { Ok(vec!["אמן".into(), "זמר".into()]) }
}
struct Fixture {
    _tree: tempfile::TempDir, _target_tree: Option<tempfile::TempDir>, source: PathBuf, target: PathBuf, journal: PathBuf,
    plan: JobPlanV2, scope: CopiedMoveScope, duet: bool,
}
impl Fixture {
    fn new(duet: bool) -> Self { Self::with_second_volume(duet, !duet) }
    fn with_second_volume(duet: bool, second_volume: bool) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let root = tree.path().canonicalize().unwrap();
        let target_tree = second_volume.then(|| tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap());
        let source = root.join("source");
        let target = target_tree.as_ref().map_or_else(|| root.join("target"), |tree| tree.path().canonicalize().unwrap().join("target"));
        let journal = root.join("journal");
        for path in [&source, &target, &journal] { std::fs::create_dir(path).unwrap(); }
        std::fs::write(source.join("song.mp3"), b"owned-main-audio").unwrap();
        std::fs::write(source.join("song.mp3:notes"), b"owned-auxiliary").unwrap();
        let mut reads = WindowsPlanningStorage::for_preview(&source, &target).unwrap();
        let request = JobRequestV2 {
            mode: JobMode::Sort, source: reads.reference(&source).unwrap(), target: Some(reads.reference(&target).unwrap()),
            settings: Settings { copy_mode: false, duet_mode: true, main_folder_only: true, singles_folder: false, ..Settings::default() },
            alias_rows: if duet { vec![] } else { vec![vec!["song".into(), "אמן".into()]] },
            resource_ids: vec!["fixture".into()], adapter_versions: vec!["untagged".into()], ai_profile: Ai(duet).profile(),
        };
        let plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(duet)).unwrap();
        assert_eq!(plan.operations.len(), if duet { 2 } else { 1 }, "{plan:?}");
        let resources = Resources { resource_ids: request.resource_ids, adapter_versions: request.adapter_versions,
            ai_profile: request.ai_profile, resource_hashes: vec![digest(b"fixture")], adapter_hashes: vec![digest(b"untagged")], model_hash: duet.then(|| digest(b"model")) };
        let roots = grants(&source, &target, &journal);
        let scope = prepare_copied_move_scope(&plan, &mut reads, &roots, &resources, &AtomicBool::new(false)).unwrap();
        Self { _tree: tree, _target_tree: target_tree, source, target, journal, plan, scope, duet }
    }
    fn job(&self) -> Result<SelectedWindowsJob, sorter_executor::ExecutionError> {
        SelectedWindowsJob::prepare_copied_move(grants(&self.source, &self.target, &self.journal), &self.plan,
            &mut Metadata, &mut Ai(self.duet), &self.scope.execution_scope().resources, &self.scope, &AtomicBool::new(false))
    }
    fn recovery(&self, job: &SelectedWindowsJob) -> RecoveryScope {
        RecoveryScope { original_scope: self.scope.execution_scope().clone(), inspected_binding_sha256: job.binding_sha256().unwrap() }
    }
    fn verify_destination(&self, artist: &str) {
        assert_eq!(std::fs::read(self.target.join(artist).join("song.mp3")).unwrap(), b"owned-main-audio");
        assert_eq!(std::fs::read(self.target.join(artist).join("song.mp3:notes")).unwrap(), b"owned-auxiliary");
    }
}
fn grants(source: &PathBuf, target: &PathBuf, journal: &PathBuf) -> WindowsMutationStorage {
    let mut reads = DesktopStorage::new(&[source.clone(), target.clone(), journal.clone()]).unwrap();
    let mut grant = |path: &PathBuf| { let reference = reads.reference(path).unwrap();
        WindowsSelectedRootGrant::new(path.clone(), reads.stat(&reference).unwrap().identity.unwrap()).unwrap() };
    WindowsMutationStorage::for_selected_roots(grant(source), grant(target), grant(journal)).unwrap()
}
#[test]
fn ordinary_copied_move_preserves_named_bytes_and_reconciles_retirement() {
    let f = Fixture::new(false); let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!((result.committed_destinations, result.moved_sources), (1, 1));
    assert_eq!(result.committed_operations, vec![0]);
    assert_eq!(result.effect_receipts.iter().map(|receipt| receipt.kind.as_str()).collect::<Vec<_>>(), vec!["file_published", "source_retired"]);
    assert!(!f.source.join("song.mp3").exists()); f.verify_destination("אמן");
    let report = job.inspect_recovery(&f.recovery(&job)).unwrap();
    assert!(report.transfers.unwrap().operations.iter().all(|operation| operation.status == RecoveryStatus::Committed));
    assert!(job.run(&AtomicBool::new(false), &mut |_| {}).is_err());
}
#[test]
fn genuine_duet_retires_once_after_both_named_stream_destinations() {
    let f = Fixture::new(true); let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!((result.committed_destinations, result.moved_sources), (2, 1));
    assert!(!f.source.join("song.mp3").exists()); f.verify_destination("אמן"); f.verify_destination("זמר");
    assert!(job.inspect_recovery(&f.recovery(&job)).unwrap().transfers.unwrap().operations.iter().all(|operation| operation.status == RecoveryStatus::Committed));
}
#[test]
fn stale_auxiliary_preview_refuses_before_any_journal_or_target_effect() {
    for replacement in [false, true] {
        let f = Fixture::new(false);
        let leaf = if replacement { "song.mp3:notes" } else { "song.mp3:late" };
        std::fs::write(f.source.join(leaf), b"changed-bytes").unwrap();
        assert!(f.job().is_err());
        assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
        assert!(f.source.join("song.mp3").exists());
    }
}
#[test]
fn cancel_second_duet_destination_keeps_complete_original() {
    let f = Fixture::new(true); let mut job = f.job().unwrap(); let cancel = AtomicBool::new(false);
    let result = job.run(&cancel, &mut |progress| { if progress.operation == 1 { cancel.store(true, Ordering::Release); } }).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!((result.committed_destinations, result.moved_sources), (1, 0));
    assert_eq!(std::fs::read(f.source.join("song.mp3")).unwrap(), b"owned-main-audio");
    assert_eq!(std::fs::read(f.source.join("song.mp3:notes")).unwrap(), b"owned-auxiliary");
}
#[test]
fn collision_after_prepare_preserves_foreign_file_and_source_streams() {
    let f = Fixture::new(false); let mut job = f.job().unwrap();
    std::fs::create_dir(f.target.join("אמן")).unwrap();
    std::fs::write(f.target.join("אמן/song.mp3"), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete); assert_eq!(result.moved_sources, 0);
    assert_eq!(std::fs::read(f.target.join("אמן/song.mp3")).unwrap(), b"foreign");
    assert_eq!(std::fs::read(f.source.join("song.mp3:notes")).unwrap(), b"owned-auxiliary");
}
#[test]
fn changed_destination_stream_invalidates_entire_retired_duet_recovery() {
    let f = Fixture::new(true); let mut job = f.job().unwrap();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, JobStatus::Complete);
    std::fs::write(f.target.join("זמר/song.mp3:notes"), b"foreign-auxiliary").unwrap();
    let report = job.inspect_recovery(&f.recovery(&job)).unwrap();
    assert!(report.transfers.unwrap().operations.iter().all(|operation| operation.status == RecoveryStatus::NeedsRecovery));
}
#[test]
fn reappeared_original_leaf_is_reported_and_never_deleted() {
    let f = Fixture::new(false); let mut job = f.job().unwrap();
    assert_eq!(job.run(&AtomicBool::new(false), &mut |_| {}).unwrap().status, JobStatus::Complete);
    std::fs::write(f.source.join("song.mp3"), b"foreign-source").unwrap();
    assert_eq!(job.inspect_recovery(&f.recovery(&job)).unwrap().transfers.unwrap().operations[0].status, RecoveryStatus::NeedsRecovery);
    assert_eq!(std::fs::read(f.source.join("song.mp3")).unwrap(), b"foreign-source");
}
#[test]
fn cancelled_before_run_leaves_no_journal_or_artist_directory() {
    let f = Fixture::new(false); let mut job = f.job().unwrap();
    assert!(job.run(&AtomicBool::new(true), &mut |_| {}).is_err());
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
}

#[test]
fn actual_distinct_ntfs_volumes_move_main_and_named_streams() {
    // Windows CI checkout is on D: and TEMP on C:. This acceptance case must
    // actually observe different volume identities; it never relabels a same-
    // volume copy as cross-volume evidence or silently skips a missing volume.
    let f = Fixture::with_second_volume(false, true);
    let execution = f.scope.execution_scope();
    let (ObjectIdentity::Windows { volume: source, .. }, ObjectIdentity::Windows { volume: target, .. }) =
        (&execution.source_root, &execution.target_root) else { panic!("Windows root identities required") };
    assert_ne!(source, target, "cross-volume acceptance requires TEMP and checkout on distinct NTFS volumes");
    let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(result.moved_sources, 1);
    assert!(!f.source.join("song.mp3").exists()); f.verify_destination("אמן");
    assert_eq!(job.inspect_recovery(&f.recovery(&job)).unwrap().transfers.unwrap().operations[0].status, RecoveryStatus::Committed);
}

#[test]
fn same_volume_single_is_not_admitted_to_copied_move() {
    let f = Fixture::new(true);
    let mut reads = WindowsPlanningStorage::for_preview(&f.source, &f.target).unwrap();
    let mut request = f.plan.request.clone();
    request.alias_rows = vec![vec!["song".into(), "אמן".into()]];
    request.ai_profile = AiProfile::Disabled;
    let plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(false)).unwrap();
    assert_eq!(plan.operations.len(), 1);
    let mut resources = f.scope.execution_scope().resources.clone();
    resources.ai_profile = AiProfile::Disabled; resources.model_hash = None;
    assert!(matches!(prepare_copied_move_scope(&plan, &mut reads, &grants(&f.source, &f.target, &f.journal), &resources, &AtomicBool::new(false)), Err(sorter_executor::ExecutionError::Unsupported)));
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
}

#[test]
fn cancellation_during_original_plan_revalidation_is_not_mislabeled_stale() {
    struct CancelAi<'a>(&'a AtomicBool);
    impl RecognitionProvider for CancelAi<'_> {
        fn profile(&self) -> AiProfile { Ai(true).profile() }
        fn classify(&mut self, _: &str) -> Result<sorter_core::ClassifierFact, RecognitionError> {
            self.0.store(true, Ordering::Release); Ok(sorter_core::ClassifierFact::Accept)
        }
        fn ner(&mut self, _: &str) -> Result<Vec<String>, RecognitionError> { Ai(true).ner("") }
    }
    let f = Fixture::new(true); let cancel = AtomicBool::new(false);
    assert!(matches!(SelectedWindowsJob::prepare_copied_move(grants(&f.source, &f.target, &f.journal), &f.plan,
        &mut Metadata, &mut CancelAi(&cancel), &f.scope.execution_scope().resources, &f.scope, &cancel), Err(sorter_executor::ExecutionError::Cancelled)));
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
}

fn mixed_fixture() -> Fixture {
    let mut fixture = Fixture::new(true);
    std::fs::write(fixture.source.join("zzz-solo.mp3"), b"ordinary-main").unwrap();
    std::fs::write(fixture.source.join("zzz-solo.mp3:extra"), b"whole-object-extra").unwrap();
    let mut reads = WindowsPlanningStorage::for_preview(&fixture.source, &fixture.target).unwrap();
    let mut request = fixture.plan.request.clone();
    request.alias_rows = vec![vec!["solo".into(), "יחיד".into()]];
    fixture.plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(true)).unwrap();
    assert_eq!(fixture.plan.operations.len(), 3);
    fixture.scope = prepare_copied_move_scope(&fixture.plan, &mut reads,
        &grants(&fixture.source, &fixture.target, &fixture.journal), &fixture.scope.execution_scope().resources,
        &AtomicBool::new(false)).unwrap();
    assert_eq!(fixture.scope.sources().len(), 1);
    assert_eq!(fixture.scope.execution_scope().inspected_sources.len(), 2);
    fixture
}
#[test]
fn mixed_same_volume_job_renames_single_object_and_retires_duet_in_original_order() {
    let f = mixed_fixture();
    let original = f.scope.execution_scope().inspected_sources.iter().find(|source| source.observation.name == "zzz-solo.mp3").unwrap().observation.identity.clone().unwrap();
    let mut job = f.job().unwrap(); let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!((result.committed_destinations, result.moved_sources), (3, 2));
    assert_eq!(result.committed_operations, vec![0, 1, 2]);
    assert_eq!(result.effect_receipts.iter().map(|receipt| receipt.kind.as_str()).collect::<Vec<_>>(), vec!["file_published", "file_published", "source_retired", "file_renamed"]);
    assert_eq!(job.created_directory_receipts().len(), 3);
    let report = job.inspect_recovery(&f.recovery(&job)).unwrap();
    assert!(report.transfers.unwrap().operations.iter().all(|operation| operation.status == RecoveryStatus::Committed));
    let bytes = std::fs::read(f.journal.join(format!("{}.transfers.jsonl", job.job_id()))).unwrap();
    let events = bytes.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()).skip(1)
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap()["body"]["event"].clone()).collect::<Vec<_>>();
    let committed = events.iter().filter(|event| event["phase"] == "Committed").map(|event| event["operation"].as_u64().unwrap()).collect::<Vec<_>>();
    assert_eq!(committed, vec![0, 1, 2]);
    assert_eq!(events.iter().filter(|event| event["phase"] == "AtomicRenameObserved").count(), 1);
    assert_eq!(events.iter().filter(|event| event["phase"] == "SourceRetired").count(), 1);
    let mut reads = DesktopStorage::new(std::slice::from_ref(&f.target)).unwrap();
    let renamed = reads.reference(&f.target.join("יחיד/zzz-solo.mp3")).unwrap();
    assert_eq!(reads.stat(&renamed).unwrap().identity, Some(original));
    assert_eq!(std::fs::read(f.target.join("יחיד/zzz-solo.mp3:extra")).unwrap(), b"whole-object-extra");
    assert!(!f.source.join("zzz-solo.mp3").exists());
    f.verify_destination("אמן"); f.verify_destination("זמר");
}
#[test]
fn late_ordinary_collision_keeps_prior_completed_duet_and_never_overwrites() {
    let f = mixed_fixture();
    let OperationKind::SingleTransfer { source, .. } = &f.plan.operations.last().unwrap().kind else { panic!("single fixture") };
    assert_eq!(source.name, "zzz-solo.mp3");
    let mut job = f.job().unwrap();
    std::fs::create_dir(f.target.join("יחיד")).unwrap();
    std::fs::write(f.target.join("יחיד/zzz-solo.mp3"), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!((result.committed_destinations, result.moved_sources), (2, 1));
    assert_eq!(std::fs::read(f.target.join("יחיד/zzz-solo.mp3")).unwrap(), b"foreign");
    assert_eq!(std::fs::read(f.source.join("zzz-solo.mp3:extra")).unwrap(), b"whole-object-extra");
    f.verify_destination("אמן"); f.verify_destination("זמר");
}
