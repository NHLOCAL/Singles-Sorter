use super::*;
#[path = "../../../../tests/staged_id3_container/fixture_support.rs"]
mod media;
fn name(kind: &str) -> String {
    format!("song__demo.{kind}")
}
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
fn all_three_containers_repair_five_fields_with_exact_backup() {
    for kind in media::KINDS {
        let before = media::fixture(kind, 4, 512);
        let fixture = Fixture::new(&[(&name(kind), before.clone())], Metadata(Some("à")));
        let mut job = fixture.job();
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
        assert_eq!((result.renamed_files, result.repaired_files), (1, 1));
        assert_eq!(result.retained_backup_bytes, before.len() as u64);
        let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
        verified(&report, "current");
        verified(&report, "backup");
        drop(job);
        let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
        let (a, b) = media::tag_span(&before, kind);
        assert_eq!(&after[..a], &before[..a]);
        assert_eq!(&after[b..], &before[b..]);
        assert_eq!(after.len(), before.len());
        assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    }
}
#[test]
fn actual_native_provider_plans_and_revalidates_container_id3() {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    for kind in media::KINDS {
        let before = media::fixture(kind, 3, 512);
        let fixture = Fixture::new(&[(&name(kind), before)], Metadata(Some("à")));
        let mut reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
        let mut metadata_reads =
            WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
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
            assert_eq!(repair.original, "à");
            assert_eq!(repair.replacement, "א");
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
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    }
}
#[test]
fn unsupported_padding_or_duplicate_chunk_keeps_original_tags() {
    for kind in media::KINDS {
        let before = media::fixture(kind, 3, 0);
        let fixture = Fixture::new(&[(&name(kind), before.clone())], Metadata(Some("à")));
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
    let mut before = media::fixture("wav", 4, 512);
    media::append_chunk(&mut before, "wav", b"ID3 ", &media::tag(4, 512));
    let fixture = Fixture::new(&[(&name("wav"), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        RepairJobStatus::Partial
    );
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        before
    );
}
#[test]
fn cancelled_container_staging_preserves_original() {
    let before = media::fixture("aiff", 4, 512);
    let fixture = Fixture::new(&[(&name("aiff"), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let flag = AtomicBool::new(false);
    let result = job
        .run(&flag, &mut |p| {
            if p.phase == "staging_tags" {
                flag.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(result.status, RepairJobStatus::Cancelled, "{result:?}");
    assert_eq!(result.repaired_files, 0);
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        before
    );
}
#[test]
fn cancelled_after_container_backup_retains_readable_both_objects() {
    let before = media::fixture("dsf", 4, 512);
    let fixture = Fixture::new(&[(&name("dsf"), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let flag = AtomicBool::new(false);
    let result = job
        .run(&flag, &mut |p| {
            if p.phase == "original_backed_up" {
                flag.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(result.status, RepairJobStatus::NeedsRecovery, "{result:?}");
    assert_eq!(
        result.items[0].staged.as_ref().unwrap().size,
        Some(before.len() as u64)
    );
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    verified(&report, "backup");
    verified(&report, "staged");
    drop(job);
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}
#[test]
fn restart_verifies_all_three_container_replacements_and_originals() {
    for kind in media::KINDS {
        assert_streaming_repair_restart(Fixture::new(
            &[(&name(kind), media::fixture(kind, 4, 512))],
            Metadata(Some("à")),
        ));
    }
}
