//! Generated ordinary/quiescent NTFS fixtures; no production paths are touched.
#![cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    binding::*, digest, recovery::RecoveryStatus, results::JobStatus,
    windows_copy::WindowsPlanningStorage, windows_job::*,
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
struct Ai(bool);
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        if self.0 {
            AiProfile::Configured {
                bundle_id: "fixture-duet".into(),
                available: true,
            }
        } else {
            AiProfile::Disabled
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
    _tree: tempfile::TempDir,
    _target_tree: Option<tempfile::TempDir>,
    source: PathBuf,
    target: PathBuf,
    journal: PathBuf,
    plan: JobPlanV2,
    scope: CopiedMoveScope,
    duet: bool,
}
impl Fixture {
    fn new(duet: bool) -> Self {
        Self::with_second_volume(duet, !duet)
    }
    fn with_second_volume(duet: bool, second_volume: bool) -> Self {
        Self::with_contents(duet, second_volume, b"owned-main-audio", b"owned-auxiliary")
    }
    fn with_contents(duet: bool, second_volume: bool, main: &[u8], named: &[u8]) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let root = tree.path().canonicalize().unwrap();
        let target_tree =
            second_volume.then(|| tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap());
        let source = root.join("source");
        let target = target_tree.as_ref().map_or_else(
            || root.join("target"),
            |tree| tree.path().canonicalize().unwrap().join("target"),
        );
        let journal = root.join("journal");
        for path in [&source, &target, &journal] {
            std::fs::create_dir(path).unwrap();
        }
        std::fs::write(source.join("song.mp3"), main).unwrap();
        std::fs::write(source.join("song.mp3:notes"), named).unwrap();
        let mut reads = WindowsPlanningStorage::for_preview(&source, &target).unwrap();
        let request = JobRequestV2 {
            mode: JobMode::Sort,
            source: reads.reference(&source).unwrap(),
            target: Some(reads.reference(&target).unwrap()),
            settings: Settings {
                copy_mode: false,
                duet_mode: true,
                main_folder_only: true,
                singles_folder: false,
                ..Settings::default()
            },
            alias_rows: if duet {
                vec![]
            } else {
                vec![vec!["song".into(), "אמן".into()]]
            },
            resource_ids: vec!["fixture".into()],
            adapter_versions: vec!["untagged".into()],
            ai_profile: Ai(duet).profile(),
        };
        let plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(duet)).unwrap();
        assert_eq!(plan.operations.len(), if duet { 2 } else { 1 }, "{plan:?}");
        let resources = Resources {
            resource_ids: request.resource_ids,
            adapter_versions: request.adapter_versions,
            ai_profile: request.ai_profile,
            resource_hashes: vec![digest(b"fixture")],
            adapter_hashes: vec![digest(b"untagged")],
            model_hash: duet.then(|| digest(b"model")),
        };
        let roots = grants(&source, &target, &journal);
        let scope = prepare_copied_move_scope(
            &plan,
            &mut reads,
            &roots,
            &resources,
            &AtomicBool::new(false),
        )
        .unwrap();
        Self {
            _tree: tree,
            _target_tree: target_tree,
            source,
            target,
            journal,
            plan,
            scope,
            duet,
        }
    }
    fn job(&self) -> Result<SelectedWindowsJob, sorter_executor::ExecutionError> {
        SelectedWindowsJob::prepare_copied_move(
            grants(&self.source, &self.target, &self.journal),
            &self.plan,
            &mut Metadata,
            &mut Ai(self.duet),
            &self.scope.execution_scope().resources,
            &self.scope,
            &AtomicBool::new(false),
        )
    }
    fn recovery(&self, job: &SelectedWindowsJob) -> RecoveryScope {
        RecoveryScope {
            original_scope: self.scope.execution_scope().clone(),
            inspected_binding_sha256: job.binding_sha256().unwrap(),
        }
    }
    fn verify_destination(&self, artist: &str) {
        assert_eq!(
            std::fs::read(self.target.join(artist).join("song.mp3")).unwrap(),
            b"owned-main-audio"
        );
        assert_eq!(
            std::fs::read(self.target.join(artist).join("song.mp3:notes")).unwrap(),
            b"owned-auxiliary"
        );
    }
}
fn grants(source: &PathBuf, target: &PathBuf, journal: &PathBuf) -> WindowsMutationStorage {
    let mut reads =
        DesktopStorage::new(&[source.clone(), target.clone(), journal.clone()]).unwrap();
    let mut grant = |path: &PathBuf| {
        let reference = reads.reference(path).unwrap();
        WindowsSelectedRootGrant::new(
            path.clone(),
            reads.stat(&reference).unwrap().identity.unwrap(),
        )
        .unwrap()
    };
    WindowsMutationStorage::for_selected_roots(grant(source), grant(target), grant(journal))
        .unwrap()
}
#[test]
fn ordinary_copied_move_preserves_named_bytes_and_reconciles_retirement() {
    let f = Fixture::new(false);
    let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (1, 1)
    );
    assert_eq!(result.committed_operations, vec![0]);
    assert_eq!(
        result
            .effect_receipts
            .iter()
            .map(|receipt| receipt.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["file_published", "source_retired"]
    );
    assert!(!f.source.join("song.mp3").exists());
    f.verify_destination("אמן");
    let report = job.inspect_recovery(&f.recovery(&job)).unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
    assert!(job.run(&AtomicBool::new(false), &mut |_| {}).is_err());
}
#[test]
fn genuine_duet_retires_once_after_both_named_stream_destinations() {
    let f = Fixture::new(true);
    let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (2, 1)
    );
    assert!(!f.source.join("song.mp3").exists());
    f.verify_destination("אמן");
    f.verify_destination("זמר");
    assert!(
        job.inspect_recovery(&f.recovery(&job))
            .unwrap()
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
}
#[test]
fn stale_auxiliary_preview_refuses_before_any_journal_or_target_effect() {
    for replacement in [false, true] {
        let f = Fixture::new(false);
        let leaf = if replacement {
            "song.mp3:notes"
        } else {
            "song.mp3:late"
        };
        std::fs::write(f.source.join(leaf), b"changed-bytes").unwrap();
        assert!(f.job().is_err());
        assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
        assert!(f.source.join("song.mp3").exists());
    }
}
#[test]
fn cancel_second_duet_destination_keeps_complete_original() {
    let f = Fixture::new(true);
    let mut job = f.job().unwrap();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.operation == 1 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (1, 0)
    );
    assert_eq!(
        std::fs::read(f.source.join("song.mp3")).unwrap(),
        b"owned-main-audio"
    );
    assert_eq!(
        std::fs::read(f.source.join("song.mp3:notes")).unwrap(),
        b"owned-auxiliary"
    );
}
#[test]
fn collision_after_prepare_preserves_foreign_file_and_source_streams() {
    let f = Fixture::new(false);
    let mut job = f.job().unwrap();
    std::fs::create_dir(f.target.join("אמן")).unwrap();
    std::fs::write(f.target.join("אמן/song.mp3"), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.moved_sources, 0);
    assert_eq!(
        std::fs::read(f.target.join("אמן/song.mp3")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        std::fs::read(f.source.join("song.mp3:notes")).unwrap(),
        b"owned-auxiliary"
    );
}
#[test]
fn changed_destination_stream_invalidates_entire_retired_duet_recovery() {
    let f = Fixture::new(true);
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    std::fs::write(f.target.join("זמר/song.mp3:notes"), b"foreign-auxiliary").unwrap();
    let report = job.inspect_recovery(&f.recovery(&job)).unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::NeedsRecovery)
    );
}
#[test]
fn reappeared_original_leaf_is_reported_and_never_deleted() {
    let f = Fixture::new(false);
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    std::fs::write(f.source.join("song.mp3"), b"foreign-source").unwrap();
    assert_eq!(
        job.inspect_recovery(&f.recovery(&job))
            .unwrap()
            .transfers
            .unwrap()
            .operations[0]
            .status,
        RecoveryStatus::NeedsRecovery
    );
    assert_eq!(
        std::fs::read(f.source.join("song.mp3")).unwrap(),
        b"foreign-source"
    );
}
#[test]
fn cancelled_before_run_leaves_no_journal_or_artist_directory() {
    let f = Fixture::new(false);
    let mut job = f.job().unwrap();
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
    let (
        ObjectIdentity::Windows { volume: source, .. },
        ObjectIdentity::Windows { volume: target, .. },
    ) = (&execution.source_root, &execution.target_root)
    else {
        panic!("Windows root identities required")
    };
    assert_ne!(
        source, target,
        "cross-volume acceptance requires TEMP and checkout on distinct NTFS volumes"
    );
    let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(result.moved_sources, 1);
    assert!(!f.source.join("song.mp3").exists());
    f.verify_destination("אמן");
    assert_eq!(
        job.inspect_recovery(&f.recovery(&job))
            .unwrap()
            .transfers
            .unwrap()
            .operations[0]
            .status,
        RecoveryStatus::Committed
    );
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
    resources.ai_profile = AiProfile::Disabled;
    resources.model_hash = None;
    assert!(matches!(
        prepare_copied_move_scope(
            &plan,
            &mut reads,
            &grants(&f.source, &f.target, &f.journal),
            &resources,
            &AtomicBool::new(false)
        ),
        Err(sorter_executor::ExecutionError::Unsupported)
    ));
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
}

#[test]
fn cancellation_during_original_plan_revalidation_is_not_mislabeled_stale() {
    struct CancelAi<'a>(&'a AtomicBool);
    impl RecognitionProvider for CancelAi<'_> {
        fn profile(&self) -> AiProfile {
            Ai(true).profile()
        }
        fn classify(&mut self, _: &str) -> Result<sorter_core::ClassifierFact, RecognitionError> {
            Ok(sorter_core::ClassifierFact::Accept)
        }
        fn ner(&mut self, _: &str) -> Result<Vec<String>, RecognitionError> {
            // Untagged song.mp3 reaches filename NER, not metadata classification.
            self.0.store(true, Ordering::Release);
            Ai(true).ner("")
        }
    }
    let f = Fixture::new(true);
    let cancel = AtomicBool::new(false);
    let result = SelectedWindowsJob::prepare_copied_move(
        grants(&f.source, &f.target, &f.journal),
        &f.plan,
        &mut Metadata,
        &mut CancelAi(&cancel),
        &f.scope.execution_scope().resources,
        &f.scope,
        &cancel,
    );
    assert!(
        cancel.load(Ordering::Acquire),
        "fixture must reach original filename NER"
    );
    assert_eq!(
        result.err(),
        Some(sorter_executor::ExecutionError::Cancelled)
    );
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
}

fn mixed_fixture() -> Fixture {
    let mut fixture = Fixture::new(true);
    std::fs::write(fixture.source.join("zzz-solo.mp3"), b"ordinary-main").unwrap();
    std::fs::write(
        fixture.source.join("zzz-solo.mp3:extra"),
        b"whole-object-extra",
    )
    .unwrap();
    let mut reads = WindowsPlanningStorage::for_preview(&fixture.source, &fixture.target).unwrap();
    let mut request = fixture.plan.request.clone();
    request.alias_rows = vec![vec!["solo".into(), "יחיד".into()]];
    fixture.plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(true)).unwrap();
    assert_eq!(fixture.plan.operations.len(), 3);
    fixture.scope = prepare_copied_move_scope(
        &fixture.plan,
        &mut reads,
        &grants(&fixture.source, &fixture.target, &fixture.journal),
        &fixture.scope.execution_scope().resources,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(fixture.scope.sources().len(), 1);
    assert_eq!(fixture.scope.execution_scope().inspected_sources.len(), 2);
    fixture
}
#[test]
fn mixed_same_volume_job_renames_single_object_and_retires_duet_in_original_order() {
    let f = mixed_fixture();
    let original = f
        .scope
        .execution_scope()
        .inspected_sources
        .iter()
        .find(|source| source.observation.name == "zzz-solo.mp3")
        .unwrap()
        .observation
        .identity
        .clone()
        .unwrap();
    let mut job = f.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (3, 2)
    );
    assert_eq!(result.committed_operations, vec![0, 1, 2]);
    assert_eq!(
        result
            .effect_receipts
            .iter()
            .map(|receipt| receipt.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "file_published",
            "file_published",
            "source_retired",
            "file_renamed"
        ]
    );
    assert_eq!(job.created_directory_receipts().len(), 3);
    let report = job.inspect_recovery(&f.recovery(&job)).unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
    let bytes = std::fs::read(f.journal.join(format!("{}.transfers.jsonl", job.job_id()))).unwrap();
    let events = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .skip(1)
        .map(|line| {
            serde_json::from_slice::<serde_json::Value>(line).unwrap()["body"]["event"].clone()
        })
        .collect::<Vec<_>>();
    let committed = events
        .iter()
        .filter(|event| event["phase"] == "Committed")
        .map(|event| event["operation"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(committed, vec![0, 1, 2]);
    assert_eq!(
        events
            .iter()
            .filter(|event| event["phase"] == "AtomicRenameObserved")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event["phase"] == "SourceRetired")
            .count(),
        1
    );
    let mut reads = DesktopStorage::new(std::slice::from_ref(&f.target)).unwrap();
    let renamed = reads
        .reference(&f.target.join("יחיד/zzz-solo.mp3"))
        .unwrap();
    assert_eq!(reads.stat(&renamed).unwrap().identity, Some(original));
    assert_eq!(
        std::fs::read(f.target.join("יחיד/zzz-solo.mp3:extra")).unwrap(),
        b"whole-object-extra"
    );
    assert!(!f.source.join("zzz-solo.mp3").exists());
    f.verify_destination("אמן");
    f.verify_destination("זמר");
}
#[test]
fn late_ordinary_collision_keeps_prior_completed_duet_and_never_overwrites() {
    let f = mixed_fixture();
    let OperationKind::SingleTransfer { source, .. } = &f.plan.operations.last().unwrap().kind
    else {
        panic!("single fixture")
    };
    assert_eq!(source.name, "zzz-solo.mp3");
    let mut job = f.job().unwrap();
    std::fs::create_dir(f.target.join("יחיד")).unwrap();
    std::fs::write(f.target.join("יחיד/zzz-solo.mp3"), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (2, 1)
    );
    assert_eq!(
        std::fs::read(f.target.join("יחיד/zzz-solo.mp3")).unwrap(),
        b"foreign"
    );
    assert_eq!(
        std::fs::read(f.source.join("zzz-solo.mp3:extra")).unwrap(),
        b"whole-object-extra"
    );
    f.verify_destination("אמן");
    f.verify_destination("זמר");
}

#[test]
fn incomplete_duet_is_refused_during_read_only_readiness() {
    let f = Fixture::new(true);
    let mut plan = f.plan.clone();
    plan.operations.pop();
    for effect in &mut plan.predicted_effects {
        if let PredictedEffect::DuetDeleteIntention {
            after_operations, ..
        } = effect
        {
            after_operations.pop();
        }
    }
    assert!(supported_copied_move_plan(&plan).is_err());
    let mut reads = WindowsPlanningStorage::for_preview(&f.source, &f.target).unwrap();
    assert!(
        prepare_copied_move_scope(
            &plan,
            &mut reads,
            &grants(&f.source, &f.target, &f.journal),
            &f.scope.execution_scope().resources,
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(&f.target).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&f.journal).unwrap().count(), 0);
    assert_eq!(
        std::fs::read(f.source.join("song.mp3:notes")).unwrap(),
        b"owned-auxiliary"
    );
}

// Include roots, directories, identities, kinds, sizes, attributes, creation and
// write times, main hashes, and each generated named stream. Exclude access time.
#[derive(Debug, PartialEq, Eq)]
struct InspectionTreeEntry {
    path: PathBuf,
    observation: EntryObservation,
    attributes: u32,
    created: u64,
    written: u64,
    content: Option<String>,
    streams: Vec<(String, u64, String)>,
}
fn inspection_tree(root: &std::path::Path) -> Vec<InspectionTreeEntry> {
    use std::os::windows::fs::MetadataExt;
    let mut reads = DesktopStorage::new(&[root.to_path_buf()]).unwrap();
    let mut pending = vec![root.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(path) = pending.pop() {
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(!metadata.file_type().is_symlink());
        let reference = reads.reference(&path).unwrap();
        let observation = reads.stat(&reference).unwrap();
        let content = metadata
            .is_file()
            .then(|| digest(&std::fs::read(&path).unwrap()));
        let mut streams = Vec::new();
        if metadata.is_dir() {
            pending.extend(
                std::fs::read_dir(&path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path()),
            );
        } else if path.extension().is_some_and(|extension| extension == "mp3") {
            let suffix = if path.file_name().unwrap() == "zzz-solo.mp3" {
                "extra"
            } else {
                "notes"
            };
            let bytes = std::fs::read(format!("{}:{suffix}", path.display())).unwrap();
            streams.push((suffix.into(), bytes.len() as u64, digest(&bytes)));
        }
        entries.push(InspectionTreeEntry {
            path: path.strip_prefix(root).unwrap().to_path_buf(),
            observation,
            attributes: metadata.file_attributes(),
            created: metadata.creation_time(),
            written: metadata.last_write_time(),
            content,
            streams,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}
fn stopped_after_media_bytes(
    threshold: u64,
) -> (
    sorter_executor::inspection_control::InspectionControl<'static>,
    std::sync::Arc<std::sync::atomic::AtomicU64>,
) {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::{Arc, atomic::AtomicU64};
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let baseline = Arc::new(AtomicU64::new(0));
    let journal_bytes = baseline.clone();
    let control = InspectionControl::with_progress_interval(
        cancel,
        std::time::Duration::ZERO,
        move |progress| {
            if progress.stage == "metadata" {
                journal_bytes.store(progress.bytes_read, Ordering::Release);
            }
            if progress.stage == "files"
                && progress.bytes_read > journal_bytes.load(Ordering::Acquire) + threshold
            {
                flag.store(true, Ordering::Release);
            }
        },
    );
    (control, baseline)
}
fn fresh_inspection_control() -> sorter_executor::inspection_control::InspectionControl<'static> {
    sorter_executor::inspection_control::InspectionControl::new(
        std::sync::Arc::new(AtomicBool::new(false)),
        |_| {},
    )
}
#[test]
fn controlled_main_chunk_cancel_preserves_trees_and_fresh_reinspection() {
    let main = vec![7u8; 256 * 1024];
    let f = Fixture::with_contents(true, false, &main, b"owned-auxiliary");
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let before = inspection_tree(f._tree.path());
    let (control, baseline) = stopped_after_media_bytes(0);
    assert!(matches!(
        job.inspect_recovery_controlled(&f.recovery(&job), &control),
        Err(sorter_executor::ExecutionError::Cancelled)
    ));
    let progress = control.snapshot().unwrap();
    assert_eq!(
        progress.bytes_read - baseline.load(Ordering::Acquire),
        65536
    );
    assert_eq!(progress.completed_operations, 0);
    assert_eq!(inspection_tree(f._tree.path()), before);
    let fresh = fresh_inspection_control();
    let report = job
        .inspect_recovery_controlled(&f.recovery(&job), &fresh)
        .unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
    assert_eq!(fresh.snapshot().unwrap().completed_operations, 2);
    assert_eq!(inspection_tree(f._tree.path()), before);
}
#[test]
fn controlled_destination_named_stream_chunk_cancel_is_explicit() {
    let main = b"owned-main-audio";
    let named = vec![9u8; 256 * 1024];
    let f = Fixture::with_contents(true, false, main, &named);
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let before = inspection_tree(f._tree.path());
    // Initial destination main read, then the full-stream verifier's main read.
    let (control, baseline) = stopped_after_media_bytes(2 * main.len() as u64);
    assert!(matches!(
        job.inspect_recovery_controlled(&f.recovery(&job), &control),
        Err(sorter_executor::ExecutionError::Cancelled)
    ));
    assert_eq!(
        control.snapshot().unwrap().bytes_read - baseline.load(Ordering::Acquire),
        2 * main.len() as u64 + 65536
    );
    assert_eq!(inspection_tree(f._tree.path()), before);
    assert!(
        job.inspect_recovery_controlled(&f.recovery(&job), &fresh_inspection_control())
            .unwrap()
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
}
#[test]
fn controlled_partial_source_named_stream_cancel_keeps_original_and_partial_evidence() {
    let main = b"owned-main-audio";
    let named = vec![3u8; 256 * 1024];
    let f = Fixture::with_contents(true, false, main, &named);
    let mut job = f.job().unwrap();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |progress| {
            if progress.operation == 1 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (1, 0)
    );
    let before = inspection_tree(f._tree.path());
    // Source and committed destination main reads, then source full-stream main.
    let (control, baseline) = stopped_after_media_bytes(3 * main.len() as u64);
    assert!(matches!(
        job.inspect_recovery_controlled(&f.recovery(&job), &control),
        Err(sorter_executor::ExecutionError::Cancelled)
    ));
    assert_eq!(
        control.snapshot().unwrap().bytes_read - baseline.load(Ordering::Acquire),
        3 * main.len() as u64 + 65536
    );
    assert_eq!(inspection_tree(f._tree.path()), before);
    let report = job
        .inspect_recovery_controlled(&f.recovery(&job), &fresh_inspection_control())
        .unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .any(|operation| operation.status != RecoveryStatus::Committed)
    );
    assert_eq!(inspection_tree(f._tree.path()), before);
}
#[test]
fn controlled_journal_read_and_strict_replay_stop_before_media_observation() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::Arc;
    let f = Fixture::new(true);
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let before = inspection_tree(f._tree.path());
    for stage in ["journals", "directory_journal", "transfer_journal"] {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let first_record = std::sync::atomic::AtomicU64::new(u64::MAX);
        let control = InspectionControl::with_progress_interval(
            cancel,
            std::time::Duration::ZERO,
            move |progress| {
                if progress.stage == stage {
                    let _ = first_record.compare_exchange(
                        u64::MAX,
                        progress.records_checked,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                    if stage == "journals" && progress.bytes_read > 0
                        || stage != "journals"
                            && progress.records_checked > first_record.load(Ordering::Acquire)
                    {
                        flag.store(true, Ordering::Release);
                    }
                }
            },
        );
        assert!(matches!(
            job.inspect_recovery_controlled(&f.recovery(&job), &control),
            Err(sorter_executor::ExecutionError::Cancelled)
        ));
        assert_eq!(control.snapshot().unwrap().completed_operations, 0);
        assert_eq!(inspection_tree(f._tree.path()), before);
    }
}
#[test]
fn controlled_mixed_routes_preserve_receipts_and_whole_cohort_metadata_prepass() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::{Arc, Mutex};
    let f = mixed_fixture();
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let before = inspection_tree(f._tree.path());
    let events = Arc::new(Mutex::new(Vec::new()));
    let observed = events.clone();
    let control = InspectionControl::with_progress_interval(
        Arc::new(AtomicBool::new(false)),
        std::time::Duration::ZERO,
        move |progress| {
            observed.lock().unwrap().push(progress);
        },
    );
    let report = job
        .inspect_recovery_controlled(&f.recovery(&job), &control)
        .unwrap()
        .transfers
        .unwrap();
    assert!(
        report
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
    assert_eq!(
        report
            .operations
            .iter()
            .map(|operation| operation.copy_metadata)
            .collect::<Vec<_>>(),
        vec![
            sorter_executor::recovery::CopyMetadataRecoveryStatus::Verified,
            sorter_executor::recovery::CopyMetadataRecoveryStatus::Verified,
            sorter_executor::recovery::CopyMetadataRecoveryStatus::NotRequested,
        ]
    );
    let events = events.lock().unwrap();
    let metadata = events
        .iter()
        .filter(|event| event.stage == "metadata")
        .collect::<Vec<_>>();
    assert!(metadata.len() >= 5); // stage plus each parent/metadata probe
    assert!(
        metadata
            .iter()
            .all(|event| event.bytes_read == metadata[0].bytes_read)
    );
    let first_content = events
        .iter()
        .position(|event| event.stage == "files" && event.bytes_read > metadata[0].bytes_read)
        .unwrap();
    assert!(
        events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.stage == "metadata")
            .all(|(index, _)| index < first_content)
    );
    assert_eq!(inspection_tree(f._tree.path()), before);
}
#[test]
fn controlled_directory_replacement_cannot_be_hidden_by_original_file_objects() {
    let f = Fixture::new(true);
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let scope = f.recovery(&job);
    let id = job.job_id().to_owned();
    let roots = grants(&f.source, &f.target, &f.journal);
    let journals = job
        .journal_locations()
        .iter()
        .map(|location| {
            roots
                .read_selected_journal(location.name(), location.identity())
                .unwrap()
        })
        .collect::<Vec<_>>();
    drop(roots);
    drop(job);
    let artist = f.target.join("אמן");
    let saved = f.target.join("owned-old-artist");
    std::fs::rename(&artist, &saved).unwrap();
    std::fs::create_dir(&artist).unwrap();
    std::fs::rename(saved.join("song.mp3"), artist.join("song.mp3")).unwrap();
    let before = inspection_tree(f._tree.path());
    let roots = grants(&f.source, &f.target, &f.journal);
    assert!(matches!(
        inspect_recovery_controlled(
            &roots,
            &scope,
            &id,
            &journals[0],
            Some(&journals[1]),
            &fresh_inspection_control()
        ),
        Err(sorter_executor::ExecutionError::Stale)
    ));
    assert_eq!(inspection_tree(f._tree.path()), before);
}
#[test]
fn controlled_copied_inspection_streams_more_than_the_old_aggregate_limit() {
    let main = vec![11u8; 17 * 1024 * 1024];
    let f = Fixture::with_contents(true, false, &main, b"owned-auxiliary");
    let mut job = f.job().unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    let before = inspection_tree(f._tree.path());
    let control = fresh_inspection_control();
    let report = job
        .inspect_recovery_controlled(&f.recovery(&job), &control)
        .unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
    let progress = control.snapshot().unwrap();
    assert!(progress.bytes_read > 64 * 1024 * 1024);
    assert_eq!(progress.completed_operations, 2);
    assert_eq!(inspection_tree(f._tree.path()), before);
}

fn recorded_copied_job(f: &Fixture, partial: bool) -> (Vec<u8>, Vec<u8>) {
    let mut job = f.job().unwrap();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |progress| {
            if partial && progress.operation == 1 {
                cancel.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(result.moved_sources, usize::from(!partial));
    let roots = grants(&f.source, &f.target, &f.journal);
    let locations = job.journal_locations();
    let directories = roots
        .read_selected_journal(locations[0].name(), locations[0].identity())
        .unwrap();
    let transfers = roots
        .read_selected_journal(locations[1].name(), locations[1].identity())
        .unwrap();
    drop(roots);
    drop(job);
    (directories, transfers)
}
#[test]
fn restart_copied_descriptor_needs_fresh_exact_grants_and_full_current_evidence() {
    let f = Fixture::new(true);
    let (directories, transfers) = recorded_copied_job(&f, false);
    let before = inspection_tree(f._tree.path());
    let descriptor = inspect_recorded_selected_job(&directories).unwrap();
    assert_eq!(descriptor.operations.len(), 2);
    let serialized = serde_json::to_value(&descriptor).unwrap();
    assert!(serialized.get("scope").is_none());
    assert!(serialized.get("copied_move_sources").is_none());
    assert!(serialized.get("recorded_complete").is_none());
    let summary = summarize_recorded_selected_job_with_control(
        &directories,
        Some(&transfers),
        Some(&fresh_inspection_control()),
    )
    .unwrap();
    assert!(!summary.recorded_complete);
    let roots = grants(&f.source, &f.target, &f.journal);
    assert!(matches!(
        recover_selected_journals(&roots, &directories, Some(&transfers)),
        Err(sorter_executor::ExecutionError::Unsupported)
    ));
    let report = recover_selected_journals_controlled(
        &roots,
        &directories,
        Some(&transfers),
        &fresh_inspection_control(),
        0,
    )
    .unwrap();
    assert!(report.directories.unverified.is_empty());
    assert!(!report.observation_limit_reached);
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::Committed)
    );
    assert_eq!(inspection_tree(f._tree.path()), before);
}
#[test]
fn restart_copied_foreign_selected_root_identity_is_refused() {
    for target in [false, true] {
        let f = Fixture::new(true);
        let (directories, transfers) = recorded_copied_job(&f, false);
        let selected = if target { &f.target } else { &f.source };
        std::fs::rename(
            selected,
            selected.with_file_name(if target {
                "owned-old-target"
            } else {
                "owned-old-source"
            }),
        )
        .unwrap();
        std::fs::create_dir(selected).unwrap();
        let before = inspection_tree(f._tree.path());
        let roots = grants(&f.source, &f.target, &f.journal);
        assert!(matches!(
            recover_selected_journals_controlled(
                &roots,
                &directories,
                Some(&transfers),
                &fresh_inspection_control(),
                0
            ),
            Err(sorter_executor::ExecutionError::Scope)
        ));
        assert_eq!(inspection_tree(f._tree.path()), before);
    }
}
#[test]
fn restart_copied_changed_stream_missing_destination_and_reappeared_source_stay_unresolved() {
    for change in ["stream", "missing", "source"] {
        let f = Fixture::new(true);
        let (directories, transfers) = recorded_copied_job(&f, false);
        match change {
            "stream" => {
                std::fs::write(f.target.join("אמן/song.mp3:notes"), b"changed-owned-stream")
                    .unwrap()
            }
            "missing" => std::fs::rename(
                f.target.join("אמן/song.mp3"),
                f.target.join("owned-saved-song.mp3"),
            )
            .unwrap(),
            "source" => {
                std::fs::write(f.source.join("song.mp3"), b"foreign-reappeared-main").unwrap();
                std::fs::write(
                    f.source.join("song.mp3:notes"),
                    b"foreign-reappeared-stream",
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        // Deliberate owned changes become the new no-write baseline.
        let before = inspection_tree(f._tree.path());
        let roots = grants(&f.source, &f.target, &f.journal);
        let report = recover_selected_journals_controlled(
            &roots,
            &directories,
            Some(&transfers),
            &fresh_inspection_control(),
            0,
        )
        .unwrap();
        assert!(
            report
                .transfers
                .unwrap()
                .operations
                .iter()
                .all(|operation| operation.status == RecoveryStatus::NeedsRecovery)
        );
        assert_eq!(inspection_tree(f._tree.path()), before);
    }
}
#[test]
fn restart_copied_cancelled_observation_cursor_never_reuses_hashes() {
    let f = Fixture::with_contents(true, false, &vec![7; 256 * 1024], b"owned-auxiliary");
    let (directories, transfers) = recorded_copied_job(&f, false);
    let before = inspection_tree(f._tree.path());
    let roots = grants(&f.source, &f.target, &f.journal);
    let (control, _) = stopped_after_media_bytes(0);
    assert!(matches!(
        recover_selected_journals_controlled(&roots, &directories, Some(&transfers), &control, 1),
        Err(sorter_executor::ExecutionError::Cancelled)
    ));
    assert_eq!(control.snapshot().unwrap().bytes_read, 65536);
    assert_eq!(inspection_tree(f._tree.path()), before);
    drop(roots);
    std::fs::write(f.target.join("אמן/song.mp3:notes"), b"changed-after-stop").unwrap();
    let changed = inspection_tree(f._tree.path());
    let fresh = fresh_inspection_control();
    let roots = grants(&f.source, &f.target, &f.journal);
    let report =
        recover_selected_journals_controlled(&roots, &directories, Some(&transfers), &fresh, 1)
            .unwrap()
            .transfers
            .unwrap();
    assert_eq!(
        report
            .operations
            .iter()
            .map(|operation| operation.operation)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(
        report
            .operations
            .iter()
            .all(|operation| operation.status == RecoveryStatus::NeedsRecovery)
    );
    assert_eq!(fresh.snapshot().unwrap().completed_operations, 2);
    assert_eq!(inspection_tree(f._tree.path()), changed);
}
#[test]
fn restart_partial_copied_move_keeps_original_and_never_adopts_missing_retirement() {
    let f = Fixture::new(true);
    let (directories, transfers) = recorded_copied_job(&f, true);
    let before = inspection_tree(f._tree.path());
    let roots = grants(&f.source, &f.target, &f.journal);
    let report = recover_selected_journals_controlled(
        &roots,
        &directories,
        Some(&transfers),
        &fresh_inspection_control(),
        0,
    )
    .unwrap()
    .transfers
    .unwrap();
    assert!(
        report
            .operations
            .iter()
            .any(|operation| operation.status != RecoveryStatus::Committed)
    );
    assert!(f.source.join("song.mp3").exists());
    assert_eq!(inspection_tree(f._tree.path()), before);
}
#[test]
fn restart_copied_strict_phase_parser_rejects_reframed_invalid_stream_history() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Record {
        sequence: u64,
        job_id: String,
        plan_sha256: String,
        previous: String,
        event: sorter_executor::physical_journal::Event,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Envelope {
        body: Record,
        sha256: String,
    }
    let f = Fixture::new(true);
    let (directories, transfers) = recorded_copied_job(&f, false);
    let before = inspection_tree(f._tree.path());
    let mut lines = transfers.split(|byte| *byte == b'\n');
    let header = lines.next().unwrap();
    let value: serde_json::Value = serde_json::from_slice(header).unwrap();
    let mut previous = value["sha256"].as_str().unwrap().to_owned();
    let mut changed = header.to_vec();
    changed.push(b'\n');
    let mut altered = false;
    for line in lines.filter(|line| !line.is_empty()) {
        let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
        if !altered
            && envelope.body.event.phase
                == sorter_executor::physical_journal::Phase::AuxiliaryStreamsObserved
        {
            envelope.body.event.phase = sorter_executor::physical_journal::Phase::BytesSynced;
            altered = true;
        }
        envelope.body.previous = previous;
        envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
        previous = envelope.sha256.clone();
        changed.extend_from_slice(&serde_json::to_vec(&envelope).unwrap());
        changed.push(b'\n');
    }
    assert!(altered);
    assert!(inspect_recorded_selected_job(&directories).is_ok());
    assert!(matches!(
        summarize_recorded_selected_job_with_control(
            &directories,
            Some(&changed),
            Some(&fresh_inspection_control())
        ),
        Err(sorter_executor::ExecutionError::CorruptJournal)
    ));
    let roots = grants(&f.source, &f.target, &f.journal);
    assert!(matches!(
        recover_selected_journals_controlled(
            &roots,
            &directories,
            Some(&changed),
            &fresh_inspection_control(),
            0
        ),
        Err(sorter_executor::ExecutionError::CorruptJournal)
    ));
    assert_eq!(inspection_tree(f._tree.path()), before);
}

fn one_eligible_duet_fixture() -> Fixture {
    let mut fixture = Fixture::new(true);
    std::fs::create_dir(fixture.target.join("אמן")).unwrap();
    let mut request = fixture.plan.request.clone();
    request.settings.exist_only = true;
    let mut reads = WindowsPlanningStorage::for_preview(&fixture.source, &fixture.target).unwrap();
    fixture.plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(true)).unwrap();
    assert_eq!(fixture.plan.operations.len(), 1, "{:?}", fixture.plan);
    fixture.scope = prepare_copied_move_scope(
        &fixture.plan,
        &mut reads,
        &grants(&fixture.source, &fixture.target, &fixture.journal),
        &fixture.scope.execution_scope().resources,
        &AtomicBool::new(false),
    )
    .unwrap();
    fixture
}
#[test]
fn intentional_exist_only_skip_allows_one_verified_destination_then_one_retirement() {
    let fixture = one_eligible_duet_fixture();
    assert!(fixture.plan.predicted_effects.iter().any(|effect|matches!(effect,PredictedEffect::DuetDeleteIntention{after_operations,..} if after_operations==&vec![0])));
    let mut job = fixture.job().unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (1, 1)
    );
    assert_eq!(result.committed_operations, vec![0]);
    assert_eq!(
        result
            .effect_receipts
            .iter()
            .map(|r| r.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["file_published", "source_retired"]
    );
    assert!(!fixture.source.join("song.mp3").exists());
    fixture.verify_destination("אמן");
    assert!(!fixture.target.join("זמר").exists());
    let report = job
        .inspect_recovery_controlled(&fixture.recovery(&job), &fresh_inspection_control())
        .unwrap();
    assert!(
        report
            .transfers
            .unwrap()
            .operations
            .iter()
            .all(|op| op.status == RecoveryStatus::Committed)
    );
}
#[test]
fn zero_eligible_duet_never_acquires_mutation_authority_or_deletes_source() {
    let fixture = Fixture::new(true);
    let mut request = fixture.plan.request.clone();
    request.settings.exist_only = true;
    let before = inspection_tree(fixture._tree.path());
    let mut reads = WindowsPlanningStorage::for_preview(&fixture.source, &fixture.target).unwrap();
    let plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai(true)).unwrap();
    assert!(plan.operations.is_empty());
    assert!(plan.predicted_effects.iter().any(|effect|matches!(effect,PredictedEffect::DuetDeleteIntention{after_operations,..} if after_operations.is_empty())));
    assert!(supported_copied_move_plan(&plan).is_err());
    assert!(
        prepare_copied_move_scope(
            &plan,
            &mut reads,
            &grants(&fixture.source, &fixture.target, &fixture.journal),
            &fixture.scope.execution_scope().resources,
            &AtomicBool::new(false)
        )
        .is_err()
    );
    drop(reads);
    assert_eq!(inspection_tree(fixture._tree.path()), before);
}
#[test]
fn failed_required_single_eligible_destination_keeps_complete_duet_source() {
    let fixture = one_eligible_duet_fixture();
    let mut job = fixture.job().unwrap();
    let destination = fixture.target.join("אמן").join("song.mp3");
    std::fs::write(&destination, b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(
        (result.committed_destinations, result.moved_sources),
        (0, 0)
    );
    assert_eq!(std::fs::read(destination).unwrap(), b"foreign");
    assert_eq!(
        std::fs::read(fixture.source.join("song.mp3")).unwrap(),
        b"owned-main-audio"
    );
    assert_eq!(
        std::fs::read(fixture.source.join("song.mp3:notes")).unwrap(),
        b"owned-auxiliary"
    );
    assert!(!fixture.target.join("זמר").exists());
}
