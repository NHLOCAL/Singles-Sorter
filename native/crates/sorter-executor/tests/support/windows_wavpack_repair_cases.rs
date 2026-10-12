use super::*;
#[path = "../../../../tests/staged_wavpack/fixture_support.rs"]
mod media;
fn verified(report: &RepairRecoveryReport, role: &str) {
    assert!(
        report.operations[0]
            .evidence
            .iter()
            .any(|e| e.role == role && matches!(e.status, RepairEvidenceStatus::Verified)),
        "{report:?}"
    );
}
fn backup(fixture: &Fixture) -> PathBuf {
    std::fs::read_dir(&fixture.source)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "backup"))
        .unwrap()
}
#[test]
fn wavpack_five_fields_and_nul_values_keep_exact_original() {
    for header in [false, true] {
        let before = media::fixture(header, true);
        let fixture = Fixture::new(&[("song__demo.wv", before.clone())], Metadata(Some("à\0á")));
        let mut job = fixture.job();
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
        assert_eq!((result.renamed_files, result.repaired_files), (1, 1));
        let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
        verified(&report, "current");
        verified(&report, "backup");
        drop(job);
        let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
        assert_eq!(after.len(), before.len());
        assert_eq!(&after[..media::AUDIO.len()], media::AUDIO);
        assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    }
}
#[test]
fn actual_wavpack_provider_plans_and_revalidates_nul_scalar() {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    let before = media::fixture(true, true);
    let fixture = Fixture::new(&[("song__demo.wv", before)], Metadata(Some("à\0á")));
    let mut reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata_reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata = StorageMetadataProvider {
        storage: &mut metadata_reads,
        reader: MetadataReader,
    };
    let plan = plan_job(&fixture.plan.request, &mut reads, &mut metadata, &mut Ai).unwrap();
    let OperationKind::FixFile(fix) = &plan.operations[0].kind else {
        panic!("fix")
    };
    assert_eq!(fix.repairs.len(), 5);
    for repair in &fix.repairs {
        assert_eq!(repair.original, "à\0á");
        assert_eq!(repair.replacement, "א\0ב");
    }
    let scope = prepare_repair_scope(
        &plan,
        &mut reads,
        fixture.scope.resources(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut job = SelectedWindowsRepairJob::prepare(
        fixture.roots(),
        &plan,
        &mut metadata,
        &mut Ai,
        scope.resources(),
        &scope,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        RepairJobStatus::Complete
    );
}
#[test]
fn wavpack_unsupported_representation_preserves_filename_only_original() {
    let mut before = media::fixture(true, false);
    let offset = media::AUDIO.len() + 32 + 4;
    before[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
    let fixture = Fixture::new(&[("song__demo.wv", before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Partial, "{result:?}");
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
        before
    );
}
#[test]
fn wavpack_cancellation_before_and_after_backup_retains_source_evidence() {
    for phase in ["staging_tags", "original_backed_up"] {
        let before = media::fixture(true, false);
        let fixture = Fixture::new(&[("song__demo.wv", before.clone())], Metadata(Some("à")));
        let mut job = fixture.job();
        let flag = AtomicBool::new(false);
        let result = job
            .run(&flag, &mut |p| {
                if p.phase == phase {
                    flag.store(true, Ordering::Release);
                }
            })
            .unwrap();
        assert_eq!(result.repaired_files, 0);
        if phase == "staging_tags" {
            assert_eq!(result.status, RepairJobStatus::Cancelled);
        } else {
            assert_eq!(result.status, RepairJobStatus::NeedsRecovery);
            let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
            verified(&report, "backup");
            verified(&report, "staged");
        }
        drop(job);
        let path = if phase == "staging_tags" {
            fixture.source.join(fixture.final_name(0))
        } else {
            backup(&fixture)
        };
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
#[test]
fn wavpack_restart_reconciles_nul_value_replacement_and_backup() {
    assert_streaming_repair_restart(Fixture::new(
        &[("song__demo.wv", media::fixture(true, true))],
        Metadata(Some("à\0á")),
    ));
}
