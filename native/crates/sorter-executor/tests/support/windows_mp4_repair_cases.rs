//! Owned product fixtures for the exact-extent MP4 stream transformer.
use super::*;
#[path = "../../../../tests/staged_mp4/fixture_support.rs"]
mod media;
fn name(alac: bool) -> &'static str {
    if alac {
        "song__demo.alac"
    } else {
        "song__demo.m4a"
    }
}
fn file(alac: bool, multi: bool) -> Vec<u8> {
    let values = if multi { vec!["à", "á"] } else { vec!["à"] };
    media::fixture(
        alac,
        &[
            (b"\xa9ART", values.clone()),
            (b"aART", values.clone()),
            (b"\xa9nam", values.clone()),
            (b"\xa9alb", values.clone()),
            (b"\xa9gen", values),
        ],
    )
}
fn backup(fixture: &Fixture) -> PathBuf {
    std::fs::read_dir(&fixture.source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "backup")
        })
        .unwrap()
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
fn preserved(before: &[u8], after: &[u8]) {
    assert_eq!(before.len(), after.len());
    assert_eq!(media::media(before), media::media(after));
    for key in [b"covr", b"zzzz"] {
        assert_eq!(
            media::metadata_item(before, key),
            media::metadata_item(after, key)
        );
    }
}
#[test]
fn aac_and_alac_five_field_repair_retains_exact_original_and_live_evidence() {
    for alac in [false, true] {
        let before = file(alac, false);
        let fixture = Fixture::new(&[(name(alac), before.clone())], Metadata(Some("à")));
        let mut job = fixture.job();
        let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
        assert_eq!((result.renamed_files, result.repaired_files), (1, 1));
        assert_eq!(result.retained_backup_bytes, before.len() as u64);
        let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
        verified(&report, "current");
        verified(&report, "backup");
        drop(job);
        preserved(
            &before,
            &std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        );
        assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    }
}
#[test]
fn fast_start_multi_value_repair_keeps_original_length_and_order() {
    let before = media::fast_start(&file(false, true));
    let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    drop(job);
    let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
    preserved(&before, &after);
    assert_eq!(media::children(&after)[1].0, *b"moov");
    assert_eq!(
        media::metadata_item(&after, b"\xa9ART"),
        vec![media::text(b"\xa9ART", &["א, ב"])]
    );
}
#[test]
fn actual_native_preview_and_apply_bind_all_five_mp4_fields() {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    let before = file(true, true);
    let fixture = Fixture::new(&[(name(true), before.clone())], Metadata(Some("à, á")));
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
        assert_eq!(repair.original, "à, á");
        assert_eq!(repair.replacement, "א, ב");
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
    let report = job.inspect_recovery(&AtomicBool::new(false)).unwrap();
    verified(&report, "current");
    verified(&report, "backup");
}
#[test]
fn duplicate_outer_field_consolidates_and_opaque_structure_stays_partial() {
    let before = media::fixture(
        false,
        &[
            (b"\xa9ART", vec!["à"]),
            (b"\xa9ART", vec!["á"]),
            (b"aART", vec!["à", "á"]),
            (b"\xa9nam", vec!["à", "á"]),
            (b"\xa9alb", vec!["à", "á"]),
            (b"\xa9gen", vec!["à", "á"]),
        ],
    );
    let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à, á")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Complete, "{result:?}");
    assert_eq!(
        (
            result.renamed_files,
            result.repaired_files,
            result.unsupported_files
        ),
        (1, 1, 0)
    );
    assert_eq!(result.retained_backup_bytes, before.len() as u64);
    drop(job);
    let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
    preserved(&before, &after);
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
    assert_eq!(
        media::metadata_item(&after, b"\xa9ART"),
        vec![media::text(b"\xa9ART", &["א, ב"])]
    );
    let opaque = media::replace_leaf(
        &file(false, false),
        b"mvhd",
        &media::atom(b"uuid", b"unproven structural payload"),
    );
    let fixture = Fixture::new(&[(name(false), opaque.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, RepairJobStatus::Partial);
    assert_eq!(result.repaired_files, 0);
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        opaque
    );
}

#[test]
fn cancelling_mp4_staging_preserves_original_without_replay() {
    let before = file(false, false);
    let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let flag = AtomicBool::new(false);
    let result = job
        .run(&flag, &mut |progress| {
            if progress.phase == "staging_tags" {
                flag.store(true, Ordering::Release);
            }
        })
        .unwrap();
    assert_eq!(result.status, RepairJobStatus::Cancelled, "{result:?}");
    assert_eq!(result.repaired_files, 0);
    assert!(matches!(
        job.run(&AtomicBool::new(false), &mut |_| {}),
        Err(E::NeedsRecovery)
    ));
    drop(job);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        before
    );
}
#[test]
fn cancelling_after_mp4_backup_keeps_both_exact_length_objects() {
    let before = file(true, false);
    let fixture = Fixture::new(&[(name(true), before.clone())], Metadata(Some("à")));
    let mut job = fixture.job();
    let flag = AtomicBool::new(false);
    let result = job
        .run(&flag, &mut |progress| {
            if progress.phase == "original_backed_up" {
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
fn hash_valid_mp4_replacement_length_change_is_rejected() {
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
    let fixture = Fixture::new(&[(name(false), file(false, false))], Metadata(Some("à")));
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        RepairJobStatus::Complete
    );
    drop(job);
    let bytes = restart_journal(&fixture);
    for edit in [false, true] {
        let mut previous = String::new();
        let mut output = vec![];
        let mut touched = false;
        for line in bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let mut envelope: Envelope = serde_json::from_slice(line).unwrap();
            if edit && envelope.body.phase == "TempPrepared" {
                let length = envelope.body.payload["replacement_length"]
                    .as_u64()
                    .unwrap();
                envelope.body.payload["replacement_length"] = serde_json::json!(length + 1);
                touched = true;
            }
            envelope.body.previous = previous;
            envelope.sha256 = digest(&serde_json::to_vec(&envelope.body).unwrap());
            previous = envelope.sha256.clone();
            output.extend(serde_json::to_vec(&envelope).unwrap());
            output.push(b'\n');
        }
        assert_eq!(touched, edit);
        let result = recover_repair_journal(&fixture.roots(), &output, &AtomicBool::new(false));
        if edit {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }
}
#[test]
fn mp4_restart_reopens_actual_replacement_and_retained_original() {
    for alac in [false, true] {
        assert_streaming_repair_restart(Fixture::new(
            &[(name(alac), file(alac, true))],
            Metadata(Some("à, á")),
        ));
    }
}
#[test]
fn real_preview_replan_repairs_later_duplicate_and_preserves_original() {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    let before = media::fixture(
        false,
        &[(b"\xa9ART", vec!["שלום"]), (b"\xa9ART", vec!["à"])],
    );
    let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("שלום, à")));
    let mut reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata_reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata = StorageMetadataProvider {
        storage: &mut metadata_reads,
        reader: MetadataReader,
    };
    let plan = plan_job(&fixture.plan.request, &mut reads, &mut metadata, &mut Ai).unwrap();
    assert_eq!(plan.operations.len(), 1);
    let OperationKind::FixFile(fix) = &plan.operations[0].kind else {
        panic!("actionable FixFile required")
    };
    assert_eq!(fix.repairs.len(), 1);
    assert_eq!(
        fix.repairs[0].field,
        sorter_core::metadata::LegacyField::Artist
    );
    assert_eq!(fix.repairs[0].original, "שלום, à");
    assert_eq!(fix.repairs[0].replacement, "שלום, א");
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
    assert_eq!(
        (
            result.renamed_files,
            result.repaired_files,
            result.unsupported_files
        ),
        (1, 1, 0)
    );
    assert_eq!(result.retained_backup_bytes, before.len() as u64);
    drop(job);
    drop(metadata_reads);
    drop(reads);
    let after = std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap();
    preserved(&before, &after);
    assert_eq!(
        media::metadata_item(&after, b"\xa9ART"),
        vec![media::text(b"\xa9ART", &["שלום, א"])]
    );
    assert_eq!(std::fs::read(backup(&fixture)).unwrap(), before);
}

#[test]
fn real_native_preview_reads_tags_but_unknown_structural_uuid_blocks_tag_writes() {
    use sorter_io::metadata::{MetadataReader, StorageMetadataProvider};
    let original = file(false, false);
    let mut before = Vec::new();
    for (kind, bytes) in media::children(&original) {
        if kind == *b"moov" {
            let mut body = bytes[8..].to_vec();
            body.extend(media::atom(b"uuid", b"opaque structural integrity bytes"));
            before.extend(media::atom(b"moov", &body));
        } else {
            before.extend(bytes);
        }
    }
    let fixture = Fixture::new(&[(name(false), before.clone())], Metadata(Some("à")));
    let mut reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata_reads = WindowsPlanningStorage::for_repair_preview(&fixture.source).unwrap();
    let mut metadata = StorageMetadataProvider {
        storage: &mut metadata_reads,
        reader: MetadataReader,
    };
    let plan = plan_job(&fixture.plan.request, &mut reads, &mut metadata, &mut Ai).unwrap();
    let OperationKind::FixFile(fix) = &plan.operations[0].kind else {
        panic!("FixFile")
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
    assert_eq!(result.status, RepairJobStatus::Partial, "{result:?}");
    assert_eq!(
        (
            result.renamed_files,
            result.repaired_files,
            result.unsupported_files
        ),
        (1, 0, 1)
    );
    assert_eq!(result.retained_backup_bytes, 0);
    drop(job);
    drop(metadata_reads);
    drop(reads);
    assert_eq!(
        std::fs::read(fixture.source.join(fixture.final_name(0))).unwrap(),
        before
    );
}
