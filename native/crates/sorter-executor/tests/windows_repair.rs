//! Owned NTFS fixtures exercise the product FixNames driver and retained backups.
#![cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, binding::Resources, digest, windows_copy::WindowsPlanningStorage,
    windows_repair::*,
};
use sorter_io::desktop::{DesktopStorage, windows_mutation::*};
use std::{
    ffi::OsString,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
#[derive(Clone, Copy)]
struct Metadata(Option<&'static str>);
impl MetadataProvider for Metadata {
    fn open(&mut self, _: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        match self.0 {
            Some(value) => Ok(Box::new(Fields(value))),
            None => Err(MetadataError::LegacyOpen {
                kind: "fixture".into(),
                detail: "untagged".into(),
            }),
        }
    }
}
struct Fields(&'static str);
impl MetadataFields for Fields {
    fn get(&mut self, _: LegacyField) -> Result<LegacyItem, FieldError> {
        let value = LegacyScalar::Text(self.0.into());
        Ok(LegacyItem {
            values: vec![value.clone()],
            truthy: !self.0.is_empty(),
            value: Ok(value),
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
fn sync(size: usize) -> [u8; 4] {
    [
        ((size >> 21) & 127) as u8,
        ((size >> 14) & 127) as u8,
        ((size >> 7) & 127) as u8,
        (size & 127) as u8,
    ]
}
fn media() -> Vec<u8> {
    let file = include_bytes!("../../../tests/fixtures/media/tagged.mp3");
    let size = file[6..10]
        .iter()
        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
    file[10 + size..].to_vec()
}
fn mp3() -> Vec<u8> {
    let mut body = vec![];
    for id in [b"TPE1", b"TPE2", b"TIT2", b"TALB", b"TCON"] {
        body.extend(id);
        body.extend(sync(6));
        body.extend([0, 0, 0, 0xe0, 0xe1, 0xf8, 0xe4, 0xed]);
    }
    body.extend([0; 256]);
    let mut file = b"ID3\x04\0\0".to_vec();
    file.extend(sync(body.len()));
    file.extend(body);
    file.extend(media());
    file
}
struct Fixture {
    _tree: tempfile::TempDir,
    source: PathBuf,
    journal: PathBuf,
    plan: JobPlanV2,
    scope: RepairScope,
    metadata: Metadata,
}
impl Fixture {
    fn new(files: &[(&str, Vec<u8>)], metadata: Metadata) -> Self {
        Self::new_with_prefix(files, metadata, false)
    }
    fn new_with_prefix(files: &[(&str, Vec<u8>)], metadata: Metadata, verbatim: bool) -> Self {
        Self::new_with_spelling(files, metadata, Some(verbatim))
    }
    fn new_with_spelling(
        files: &[(&str, Vec<u8>)],
        metadata: Metadata,
        verbatim: Option<bool>,
    ) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let source = tree.path().join("source");
        let journal = tree.path().join("journal");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&journal).unwrap();
        for (name, bytes) in files {
            std::fs::write(source.join(name), bytes).unwrap();
        }
        let spelling = |path: PathBuf| {
            let Some(verbatim) = verbatim else {
                return path;
            };
            let canonical = std::fs::canonicalize(path).unwrap();
            if verbatim {
                canonical
            } else {
                let mut units = canonical.as_os_str().encode_wide().collect::<Vec<_>>();
                if units.starts_with(&[92, 92, 63, 92]) {
                    units.drain(..4);
                }
                PathBuf::from(OsString::from_wide(&units))
            }
        };
        let source = spelling(source);
        let journal = spelling(journal);
        let mut reads = WindowsPlanningStorage::for_repair_preview(&source).unwrap();
        let request = JobRequestV2 {
            mode: JobMode::FixNames,
            source: reads.reference(&source).unwrap(),
            target: None,
            settings: Settings {
                main_folder_only: true,
                ..Settings::default()
            },
            alias_rows: vec![],
            resource_ids: vec!["fixture".into()],
            adapter_versions: vec!["metadata-fixture".into()],
            ai_profile: AiProfile::Disabled,
        };
        let plan = plan_job(&request, &mut reads, &mut { metadata }, &mut Ai).unwrap();
        assert_eq!(plan.operations.len(), files.len(), "{plan:?}");
        let resources = Resources {
            resource_ids: request.resource_ids,
            adapter_versions: request.adapter_versions,
            ai_profile: AiProfile::Disabled,
            resource_hashes: vec![digest(b"fixture")],
            adapter_hashes: vec![digest(b"metadata-fixture")],
            model_hash: None,
        };
        let scope =
            prepare_repair_scope(&plan, &mut reads, &resources, &AtomicBool::new(false)).unwrap();
        Self {
            _tree: tree,
            source,
            journal,
            plan,
            scope,
            metadata,
        }
    }
    fn roots(&self) -> WindowsMutationStorage {
        let mut reads = DesktopStorage::new(&[self.source.clone(), self.journal.clone()]).unwrap();
        let mut grant = |path: &PathBuf| {
            let reference = reads.reference(path).unwrap();
            WindowsSelectedRootGrant::new(
                path.clone(),
                reads.stat(&reference).unwrap().identity.unwrap(),
            )
            .unwrap()
        };
        WindowsMutationStorage::for_selected_repair_roots(grant(&self.source), grant(&self.journal))
            .unwrap()
    }
    fn prepare(&self) -> sorter_executor::Result<SelectedWindowsRepairJob> {
        SelectedWindowsRepairJob::prepare(
            self.roots(),
            &self.plan,
            &mut { self.metadata },
            &mut Ai,
            self.scope.resources(),
            &self.scope,
            &AtomicBool::new(false),
        )
    }
    fn job(&self) -> SelectedWindowsRepairJob {
        self.prepare().unwrap()
    }
    fn final_name(&self, index: usize) -> &str {
        let OperationKind::FixFile(fix) = &self.plan.operations[index].kind else {
            panic!("fix")
        };
        fix.intended_name.as_deref().unwrap()
    }
}
#[test]
fn five_field_mp3_repair_keeps_backup_and_exact_audio() {
    let before = mp3();
    let fixture = Fixture::new(
        &[("song__demo.mp3", before.clone())],
        Metadata(Some("àáøäí")),
    );
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    assert_eq!((result.renamed_files, result.repaired_files), (1, 1));
    assert_eq!(result.retained_backup_bytes, before.len() as u64);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(report.recorded_finished);
    assert!(
        report.operations[0]
            .evidence
            .iter()
            .all(|e| matches!(e.status, RepairEvidenceStatus::Verified))
    );
    drop(job);
    let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
    assert_ne!(after, before);
    assert_eq!(&after[after.len() - media().len()..], media());
    let backup = std::fs::read_dir(&fixture.source)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "backup"))
        .unwrap();
    assert_eq!(std::fs::read(backup).unwrap(), before);
}
#[test]
fn filename_repairs_cover_all_released_extensions_even_when_metadata_fails() {
    let files = [
        "m4a", "wma", "wav", "aiff", "flac", "aac", "alac", "wv", "ogg", "dsf", "opus", "mp3",
    ]
    .map(|ext| (format!("song__demo.{ext}"), b"owned bytes".to_vec()));
    let borrowed = files
        .iter()
        .map(|(name, data)| (name.as_str(), data.clone()))
        .collect::<Vec<_>>();
    let fixture = Fixture::new(&borrowed, Metadata(None));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Partial);
    assert_eq!(result.renamed_files, 12);
    assert_eq!(result.repaired_files, 0);
    assert_eq!(result.retained_backup_bytes, 0);
    drop(job);
    for index in 0..12 {
        assert_eq!(
            std::fs::read(fixture.source.join(fixture.final_name(index))).unwrap(),
            b"owned bytes"
        );
    }
}
#[test]
fn unchanged_tags_do_not_create_a_replacement_or_backup() {
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("שלום")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete);
    assert_eq!(
        (
            result.repaired_files,
            result.unchanged_files,
            result.retained_backup_bytes
        ),
        (0, 1, 0)
    );
    drop(job);
    assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 1);
}
#[test]
fn unsupported_format_reports_filename_only_and_retains_original_bytes() {
    let fixture = Fixture::new(
        &[("song__demo.flac", b"owned unsupported".to_vec())],
        Metadata(Some("àáøäí")),
    );
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Partial);
    assert_eq!(
        (
            result.renamed_files,
            result.repaired_files,
            result.unsupported_files
        ),
        (1, 0, 1)
    );
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        b"owned unsupported"
    );
}
#[test]
fn preview_content_change_is_rejected_before_journal_or_mutation() {
    let fixture = Fixture::new(&[("song__demo.mp3", mp3())], Metadata(Some("àáøäí")));
    std::fs::write(fixture.source.join("song__demo.mp3"), b"changed").unwrap();
    assert!(matches!(fixture.prepare(), Err(E::Stale)));
    assert_eq!(std::fs::read_dir(&fixture.journal).unwrap().count(), 0);
}
#[test]
fn late_filename_collision_preserves_both_files() {
    let fixture = Fixture::new(&[("song__demo.mp3", mp3())], Metadata(None));
    let mut job = fixture.job();
    std::fs::write(fixture.source.join(fixture.final_name(0)), b"foreign").unwrap();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.renamed_files, 0);
    assert_ne!(result.status, RepairJobStatus::Complete);
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        b"foreign"
    );
    assert_eq!(
        std::fs::read(fixture.source.join("song__demo.mp3")).unwrap(),
        mp3()
    );
}
#[test]
fn cancel_during_staging_preserves_source_and_never_replays() {
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("àáøäí")));
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |p| {
            if p.phase == "staging_tags" {
                cancel.store(true, Ordering::Release)
            }
        })
        .unwrap();
    assert_eq!(result.status, RepairJobStatus::Cancelled);
    assert_eq!(result.repaired_files, 0);
    assert!(matches!(
        job.run(&AtomicBool::new(false), &mut |_| {}),
        Err(E::NeedsRecovery)
    ));
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join("song.mp3")).unwrap(),
        mp3()
    );
}
#[test]
fn cancel_after_backup_reports_recovery_and_preserves_both_artifacts() {
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("àáøäí")));
    let mut job = fixture.job();
    let cancel = AtomicBool::new(false);
    let result = job
        .run(&cancel, &mut |p| {
            if p.phase == "original_backed_up" {
                cancel.store(true, Ordering::Release)
            }
        })
        .unwrap();
    assert_eq!(result.status, RepairJobStatus::NeedsRecovery);
    assert_eq!(result.repaired_files, 0);
    assert_eq!(result.retained_backup_bytes, mp3().len() as u64);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(
        report.operations[0]
            .evidence
            .iter()
            .any(|e| e.role == "backup" && matches!(e.status, RepairEvidenceStatus::Verified))
    );
    drop(job);
    assert!(!fixture.source.join("song.mp3").exists());
    assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 2);
}
#[test]
fn unsupported_id3_flags_leave_filename_outcome_without_staging() {
    let mut before = mp3();
    before[5] = 0x80;
    let fixture = Fixture::new(
        &[("song__demo.mp3", before.clone())],
        Metadata(Some("àáøäí")),
    );
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(
        (
            result.renamed_files,
            result.unsupported_files,
            result.repaired_files
        ),
        (1, 1, 0)
    );
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        before
    );
    assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 1);
}
#[test]
fn repair_recovery_rejects_corruption_and_preserves_journal() {
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("שלום")));
    let mut job = fixture.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    drop(job);
    let journal = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = std::fs::read(&journal).unwrap();
    let roots = fixture.roots();
    let mut corrupt = bytes.clone();
    corrupt[10] ^= 1;
    assert!(recover_repair_journal(&roots, &corrupt, &AtomicBool::new(false)).is_err());
    let mut torn = bytes.clone();
    torn.extend(b"{\"partial");
    let report = recover_repair_journal(&roots, &torn, &AtomicBool::new(false)).unwrap();
    assert!(report.incomplete_tail);
    assert_eq!(std::fs::read(journal).unwrap(), bytes);
}
#[test]
fn hash_valid_forbidden_phase_and_oversized_result_are_rejected() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Body {
        sequence: u64,
        previous: String,
        job_id: String,
        operation: Option<usize>,
        phase: String,
        payload: serde_json::Value,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Envelope {
        body: Body,
        sha256: String,
    }
    fn rewrite(bytes: &[u8], mut edit: impl FnMut(&mut Body)) -> Vec<u8> {
        let mut previous = String::new();
        let mut output = Vec::new();
        for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
            let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
            edit(&mut envelope.body);
            envelope.body.previous = previous;
            envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
            previous = envelope.sha256.clone();
            output.extend(serde_json::to_vec(&envelope).unwrap());
            output.push(b'\n');
        }
        output
    }
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("שלום")));
    let mut job = fixture.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    drop(job);
    let journal = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = std::fs::read(&journal).unwrap();
    let roots = fixture.roots();
    let forbidden = rewrite(&bytes, |body| {
        if body.phase == "OperationPrepared" {
            body.phase = "TempObserved".into();
        }
    });
    assert!(matches!(
        recover_repair_journal(&roots, &forbidden, &AtomicBool::new(false)),
        Err(E::CorruptJournal)
    ));
    let oversized = rewrite(&bytes, |body| {
        if body.phase == "Finished" {
            let items = body.payload["items"].as_array_mut().unwrap();
            items.push(items[0].clone());
        }
    });
    assert!(matches!(
        recover_repair_journal(&roots, &oversized, &AtomicBool::new(false)),
        Err(E::CorruptJournal)
    ));
    assert_eq!(std::fs::read(journal).unwrap(), bytes);
    let wrong_size = rewrite(&bytes, |body| {
        if body.phase == "Committed" {
            body.payload["current"]["size"] = serde_json::json!(999999);
        }
    });
    assert!(matches!(
        recover_repair_journal(&roots, &wrong_size, &AtomicBool::new(false)),
        Err(E::CorruptJournal)
    ));
    let substituted = rewrite(&bytes, |body| {
        if body.phase == "Committed" {
            body.payload["current"]["actual_name"] =
                serde_json::json!({"Windows":"foreign.mp3".encode_utf16().collect::<Vec<_>>()});
        }
    });
    assert!(matches!(
        recover_repair_journal(&roots, &substituted, &AtomicBool::new(false)),
        Err(E::CorruptJournal)
    ));
    let renamed = Fixture::new(&[("song__demo.mp3", mp3())], Metadata(None));
    let mut job = renamed.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    drop(job);
    let path = std::fs::read_dir(&renamed.journal)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let actual = std::fs::read(path).unwrap();
    let mut records = actual
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Envelope>(line).unwrap())
        .collect::<Vec<_>>();
    let mut stopped = records.remove(4);
    assert_eq!(stopped.body.phase, "Stopped");
    records.truncate(3);
    stopped.body.sequence = 3;
    stopped.body.payload["status"] = serde_json::json!("needs_recovery");
    stopped.body.payload["current"] = serde_json::Value::Null;
    stopped.body.payload["renamed"] = serde_json::json!(false);
    stopped.body.payload["processed"] = serde_json::json!(false);
    records.push(stopped);
    let mut lost = Vec::new();
    for record in records {
        lost.extend(serde_json::to_vec(&record).unwrap());
        lost.push(b'\n');
    }
    let lost = rewrite(&lost, |_| {});
    let roots = renamed.roots();
    let report = recover_repair_journal(&roots, &lost, &AtomicBool::new(false)).unwrap();
    assert!(report.operations[0].evidence.iter().any(|e|e.role=="planned_name"&&matches!(e.status,RepairEvidenceStatus::Verified)));
}

#[test]
fn verbatim_selected_paths_reconcile_the_same_held_objects() {
    let fixture =
        Fixture::new_with_prefix(&[("song__demo.mp3", mp3())], Metadata(Some("àáøäí")), true);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(
        report.operations[0]
            .evidence
            .iter()
            .all(|e| matches!(e.status, RepairEvidenceStatus::Verified))
    );
}

#[test]
fn raw_selected_temp_root_recovery_uses_held_root_alias() {
    // Unlike the prefix fixtures, preserve TEMP's original spelling, including
    // any real short ancestor such as RUNNER~1 used by Windows CI/CLI callers.
    let fixture =
        Fixture::new_with_spelling(&[("song__demo.mp3", mp3())], Metadata(Some("àáøäí")), None);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete);
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    assert!(report.recorded_finished);
    assert!(
        report
            .operations
            .iter()
            .flat_map(|op| &op.evidence)
            .any(|e| e.role == "backup" && matches!(e.status, RepairEvidenceStatus::Verified))
    );
    assert!(
        report
            .operations
            .iter()
            .flat_map(|op| &op.evidence)
            .any(|e| e.role == "current" && matches!(e.status, RepairEvidenceStatus::Verified))
    );
}

#[path = "support/windows_flac_repair_cases.rs"]
mod flac;

#[path = "support/windows_ogg_repair_cases.rs"]
mod ogg;

fn restart_control() -> sorter_executor::inspection_control::InspectionControl<'static> {
    sorter_executor::inspection_control::InspectionControl::new(
        std::sync::Arc::new(AtomicBool::new(false)),
        |_| {},
    )
}
fn restart_journal(fixture: &Fixture) -> Vec<u8> {
    let path = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::read(path).unwrap()
}
fn repair_tree_snapshot(
    root: &std::path::Path,
) -> Vec<(PathBuf, EntryObservation, u32, u64, u64, Option<String>)> {
    use std::os::windows::fs::MetadataExt;
    let mut reads = DesktopStorage::new(&[root.to_path_buf()]).unwrap();
    let mut pending = vec![root.to_path_buf()];
    let mut result = Vec::new();
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
        result.push((
            path.strip_prefix(root).unwrap().to_path_buf(),
            observation,
            metadata.file_attributes(),
            metadata.creation_time(),
            metadata.last_write_time(),
            content,
        ));
    }
    result.sort_by(|left, right| left.0.cmp(&right.0));
    result
}
fn assert_streaming_repair_restart(fixture: Fixture) {
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    drop(job);
    let before = repair_tree_snapshot(fixture._tree.path());
    let journal = restart_journal(&fixture);
    let descriptor = inspect_recorded_repair_job(&journal, &restart_control()).unwrap();
    assert_eq!(&descriptor.source_identity, fixture.scope.source_root());
    assert_eq!(descriptor.operations.len(), fixture.plan.operations.len());
    let json = serde_json::to_value(&descriptor).unwrap();
    for field in ["observed_source", "scope", "recorded_complete", "binding"] {
        assert!(json.get(field).is_none());
    }
    let control = restart_control();
    let report = recover_repair_journal_controlled(
        &fixture.roots(),
        &journal,
        &AtomicBool::new(false),
        &control,
        0,
    )
    .unwrap();
    assert!(report.recorded_finished && !report.incomplete_tail);
    assert!(report.operations.iter().all(|operation| {
        operation
            .recorded
            .as_ref()
            .is_some_and(|item| item.status == RepairStatus::Complete)
            && !operation.evidence.is_empty()
            && operation
                .evidence
                .iter()
                .all(|evidence| matches!(evidence.status, RepairEvidenceStatus::Verified))
    }));
    assert!(control.snapshot().unwrap().bytes_read > 0);
    assert_eq!(
        control.snapshot().unwrap().completed_operations,
        descriptor.operations.len()
    );
    assert_eq!(repair_tree_snapshot(fixture._tree.path()), before);
}
#[test]
fn recorded_mp3_descriptor_rebinds_fresh_roots_and_keeps_original_backup() {
    assert_streaming_repair_restart(Fixture::new_with_spelling(
        &[("song__demo.mp3", mp3())],
        Metadata(Some("àáøäí")),
        None,
    ));
}
#[test]
fn controlled_repair_chunk_cancel_and_cursor_reinspection_never_reuse_old_hashes() {
    use sorter_executor::inspection_control::InspectionControl;
    use std::sync::{Arc, Mutex};
    let fixture = Fixture::new(
        &[("song__one.mp3", mp3()), ("song__two.mp3", mp3())],
        Metadata(Some("àáøäí")),
    );
    let mut job = fixture.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    drop(job);
    let before = repair_tree_snapshot(fixture._tree.path());
    let journal = restart_journal(&fixture);
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let control = InspectionControl::with_progress_interval(
        cancel.clone(),
        std::time::Duration::ZERO,
        move |progress| {
            if progress.bytes_read > 0 {
                flag.store(true, Ordering::Release);
            }
        },
    );
    assert!(matches!(
        recover_repair_journal_controlled(&fixture.roots(), &journal, &cancel, &control, 0),
        Err(E::Cancelled)
    ));
    assert_eq!(control.snapshot().unwrap().completed_operations, 0);
    let order = Arc::new(Mutex::new(Vec::new()));
    let seen = order.clone();
    let control = InspectionControl::new(Arc::new(AtomicBool::new(false)), move |progress| {
        if let Some(index) = progress.operation {
            let mut seen = seen.lock().unwrap();
            if seen.last() != Some(&index) {
                seen.push(index);
            }
        }
    });
    let report = recover_repair_journal_controlled(
        &fixture.roots(),
        &journal,
        &AtomicBool::new(false),
        &control,
        1,
    )
    .unwrap();
    assert_eq!(*order.lock().unwrap(), vec![1, 0]);
    assert_eq!(
        report
            .operations
            .iter()
            .map(|item| item.operation)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(
        report
            .operations
            .iter()
            .flat_map(|item| &item.evidence)
            .all(|e| matches!(e.status, RepairEvidenceStatus::Verified))
    );
    assert_eq!(repair_tree_snapshot(fixture._tree.path()), before);
    std::fs::write(
        fixture.source.join(fixture.final_name(0)),
        b"changed after successful inspection",
    )
    .unwrap();
    let changed = repair_tree_snapshot(fixture._tree.path());
    let report = recover_repair_journal_controlled(
        &fixture.roots(),
        &journal,
        &AtomicBool::new(false),
        &restart_control(),
        1,
    )
    .unwrap();
    assert!(
        report.operations[0]
            .evidence
            .iter()
            .any(|e| matches!(e.status, RepairEvidenceStatus::Changed))
    );
    assert_eq!(repair_tree_snapshot(fixture._tree.path()), changed);
}
#[test]
fn controlled_repair_preserves_interrupted_backup_and_publication_evidence() {
    for boundary in ["original_backed_up", "staging_tags"] {
        let fixture = Fixture::new(&[("song__demo.mp3", mp3())], Metadata(Some("àáøäí")));
        let mut job = fixture.job();
        let cancel = AtomicBool::new(false);
        let result = job
            .run(&cancel, &mut |progress| {
                if progress.phase == boundary {
                    cancel.store(true, Ordering::Release);
                }
            })
            .unwrap();
        assert_ne!(result.status, RepairJobStatus::Complete);
        drop(job);
        let before = repair_tree_snapshot(fixture._tree.path());
        let journal = restart_journal(&fixture);
        assert!(inspect_recorded_repair_job(&journal, &restart_control()).is_ok());
        let report = recover_repair_journal_controlled(
            &fixture.roots(),
            &journal,
            &AtomicBool::new(false),
            &restart_control(),
            0,
        )
        .unwrap();
        assert!(report.operations.iter().any(|item| {
            item.recorded
                .as_ref()
                .is_none_or(|recorded| recorded.status != RepairStatus::Complete)
        }));
        if boundary == "original_backed_up" {
            assert!(
                report.operations[0]
                    .evidence
                    .iter()
                    .any(|e| e.role == "backup"
                        && matches!(e.status, RepairEvidenceStatus::Verified))
            );
        }
        assert_eq!(repair_tree_snapshot(fixture._tree.path()), before);
    }
}
#[test]
fn repair_descriptor_frames_do_not_claim_strict_phase_or_current_file_verification() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Body {
        sequence: u64,
        previous: String,
        job_id: String,
        operation: Option<usize>,
        phase: String,
        payload: serde_json::Value,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Envelope {
        body: Body,
        sha256: String,
    }
    let fixture = Fixture::new(&[("song.mp3", mp3())], Metadata(Some("שלום")));
    let mut job = fixture.job();
    job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    drop(job);
    let original = restart_journal(&fixture);
    let mut previous = String::new();
    let mut edited = Vec::new();
    for line in original
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
        if envelope.body.phase == "OperationPrepared" {
            envelope.body.phase = "TempObserved".into();
        }
        envelope.body.previous = previous;
        envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
        previous = envelope.sha256.clone();
        edited.extend(serde_json::to_vec(&envelope).unwrap());
        edited.push(b'\n');
    }
    assert!(inspect_recorded_repair_job(&edited, &restart_control()).is_ok());
    assert!(matches!(
        recover_repair_journal_controlled(
            &fixture.roots(),
            &edited,
            &AtomicBool::new(false),
            &restart_control(),
            0
        ),
        Err(E::CorruptJournal)
    ));
    let mut corrupt = original.clone();
    corrupt[10] ^= 1;
    assert!(inspect_recorded_repair_job(&corrupt, &restart_control()).is_err());
    let mut torn = original.clone();
    torn.extend(b"{incomplete");
    assert!(
        inspect_recorded_repair_job(&torn, &restart_control())
            .unwrap()
            .incomplete_tail
    );
    let mut excessive = original;
    excessive.extend(std::iter::repeat_n(b'\n', 65_537));
    assert!(matches!(
        inspect_recorded_repair_job(&excessive, &restart_control()),
        Err(E::Unsupported)
    ));
}
#[test]
fn repair_restart_rejects_replaced_source_root_and_streams_ordinary_large_jobs() {
    let mut bytes = mp3();
    bytes.resize(35 * 1024 * 1024, 0);
    let fixture = Fixture::new(
        &[("song__one.mp3", bytes.clone()), ("song__two.mp3", bytes)],
        Metadata(Some("שלום")),
    );
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        RepairJobStatus::Complete
    );
    drop(job);
    let journal = restart_journal(&fixture);
    let control = restart_control();
    let before = repair_tree_snapshot(fixture._tree.path());
    let report = recover_repair_journal_controlled(
        &fixture.roots(),
        &journal,
        &AtomicBool::new(false),
        &control,
        0,
    )
    .unwrap();
    assert!(
        report
            .operations
            .iter()
            .flat_map(|item| &item.evidence)
            .all(|e| matches!(e.status, RepairEvidenceStatus::Verified))
    );
    assert_eq!(control.snapshot().unwrap().bytes_read, 70 * 1024 * 1024);
    assert_eq!(repair_tree_snapshot(fixture._tree.path()), before);
    let old = fixture.source.with_file_name("old-source");
    std::fs::rename(&fixture.source, &old).unwrap();
    std::fs::create_dir(&fixture.source).unwrap();
    assert!(inspect_recorded_repair_job(&journal, &restart_control()).is_ok());
    assert!(matches!(
        recover_repair_journal_controlled(
            &fixture.roots(),
            &journal,
            &AtomicBool::new(false),
            &restart_control(),
            0
        ),
        Err(E::Scope)
    ));
    assert!(old.exists());
}
