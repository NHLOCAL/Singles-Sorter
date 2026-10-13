//! Native writer fixtures feed the data-only decoder; no serialized scope is used.
use super::*;
use serde::{Deserialize, Serialize};
use sorter_executor::inspection_control::InspectionControl;
use std::sync::Arc;

fn control() -> InspectionControl<'static> {
    InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {})
}
fn manifest(fixture: &Fixture) -> Vec<u8> {
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
    std::fs::read(path).unwrap()
}
fn completed_copy() -> (Fixture, Vec<u8>) {
    let fixture = Fixture::new(true, false);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    drop(job);
    let bytes = manifest(&fixture);
    (fixture, bytes)
}
#[derive(Serialize, Deserialize)]
struct Body {
    sequence: u64,
    previous: String,
    job_id: String,
    phase: String,
    payload: serde_json::Value,
}
#[derive(Serialize, Deserialize)]
struct Envelope {
    body: Body,
    sha256: String,
}
fn header_only(bytes: &[u8], change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let end = bytes.iter().position(|byte| *byte == b'\n').unwrap();
    let mut record: Envelope = serde_json::from_slice(&bytes[..end]).unwrap();
    change(&mut record.body.payload);
    record.sha256 = digest(&serde_json::to_vec(&record.body).unwrap());
    let mut bytes = serde_json::to_vec(&record).unwrap();
    bytes.push(b'\n');
    bytes
}

#[test]
fn completed_copy_header_describes_roots_without_scope_or_completion_authority() {
    let (fixture, bytes) = completed_copy();
    let descriptor = inspect_recorded_album_file_job(&bytes, &control()).unwrap();
    assert_eq!(descriptor.source.reference, fixture.plan.request.source);
    assert_eq!(
        Some(&descriptor.target.reference),
        fixture.plan.request.target.as_ref()
    );
    assert_eq!(descriptor.file_count, 5);
    assert_eq!(descriptor.album_operations, vec![0]);
    assert!(!descriptor.copied_move);
    assert!(descriptor.original_source.is_none());
    let json = serde_json::to_value(descriptor).unwrap();
    for field in [
        "scope",
        "binding",
        "recorded_status",
        "recorded_complete",
        "manifest_finished",
        "recovery_only",
    ] {
        assert!(json.get(field).is_none());
    }
    assert_eq!(manifest(&fixture), bytes);
    fixture.verify_copy();
}

#[test]
fn copied_root_descriptor_keeps_exact_native_original_after_source_disappears() {
    let (fixture, _target_owner) = cross_volume_album(true);
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    drop(job);
    assert!(!fixture.source.exists());
    let bytes = manifest(&fixture);
    let descriptor = inspect_recorded_album_file_job(&bytes, &control()).unwrap();
    assert!(descriptor.copied_move);
    let original = descriptor.original_source.unwrap();
    assert_eq!(Some(original.identity), descriptor.source.identity);
    assert_eq!(
        Some(original.parent_identity),
        descriptor.source.parent_identity
    );
    assert_eq!(
        original.actual_name,
        MutationName::Windows("source".encode_utf16().collect())
    );
    assert_eq!(original.size, None);
    assert_eq!(manifest(&fixture), bytes);
    verify_cross_volume_album(&fixture);
}

#[test]
fn rewritten_snapshot_mapping_membership_cleanup_and_parent_forgery_are_rejected() {
    let (fixture, bytes) = completed_copy();
    for change in 0..12 {
        let forged = header_only(&bytes, |header| {
            let snapshot = &mut header["snapshot"];
            match change {
                0 => snapshot["files"][0]["original_operation"] = serde_json::json!(999),
                1 => snapshot["albums"][0]["files"] = serde_json::json!([999]),
                2 => snapshot["directories"][0]["children"]
                    .as_array_mut()
                    .unwrap()
                    .pop()
                    .map(|_| ())
                    .unwrap(),
                3 => snapshot["albums"][0]["cleanup"] = serde_json::json!([]),
                4 => {
                    snapshot["albums"][0]["children"][0]["destination_name"] =
                        serde_json::json!("foreign.mp3")
                }
                5 => {
                    snapshot["files"][0]["destination"]["components"] =
                        serde_json::json!(["..", "outside"])
                }
                6 => {
                    snapshot["directories"][0]["children"][0]["parent_identity"] = serde_json::json!({"Windows":{"volume":0,"file_id":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}})
                }
                7..=10 => {
                    let families = ["copied_directories", "destinations", "files", "albums"];
                    let family = families[change - 7];
                    assert!(!snapshot[family].as_array().unwrap().is_empty());
                    snapshot[family][0]["destination"]["unexpected"] = serde_json::json!(true);
                }
                _ => {
                    snapshot["files"][0]["destination"]["components"] = serde_json::json!([]);
                }
            }
        });
        assert!(
            inspect_recorded_album_file_job(&forged, &control()).is_err(),
            "forgery {change}"
        );
    }
    assert_eq!(manifest(&fixture), bytes);
}

#[test]
fn copied_native_leaf_and_source_identity_cannot_be_rebound_by_header_bytes() {
    let (fixture, _target_owner) = cross_volume_album(true);
    let mut job = fixture.job();
    assert_eq!(
        job.run(&AtomicBool::new(false), &mut |_| {})
            .unwrap()
            .status,
        JobStatus::Complete
    );
    drop(job);
    let bytes = manifest(&fixture);
    for change in 0..3 {
        let forged = header_only(&bytes, |header| match change {
            0 => {
                header["original_source"]["actual_name"] =
                    serde_json::to_value(MutationName::Windows("other".encode_utf16().collect()))
                        .unwrap()
            }
            1 => {
                header["original_source"]["identity"] =
                    header["binding"]["target_root"]["identity"].clone()
            }
            _ => header["schema"] = serde_json::json!(1),
        });
        assert!(inspect_recorded_album_file_job(&forged, &control()).is_err());
    }
    assert_eq!(manifest(&fixture), bytes);
    verify_cross_volume_album(&fixture);
}

#[test]
fn partial_framing_never_becomes_a_completed_or_current_observation() {
    let (fixture, bytes) = completed_copy();
    let mut partial = header_only(&bytes, |_| {});
    partial.extend(b"{\"unfinished");
    let descriptor = inspect_recorded_album_file_job(&partial, &control()).unwrap();
    assert_eq!(descriptor.file_count, 5);
    assert!(
        serde_json::to_value(descriptor)
            .unwrap()
            .get("recorded_complete")
            .is_none()
    );
    let mut corrupt = bytes.clone();
    let last = corrupt.iter().rposition(|byte| *byte == b'"').unwrap();
    corrupt[last - 1] ^= 1;
    assert!(inspect_recorded_album_file_job(&corrupt, &control()).is_err());
    assert_eq!(manifest(&fixture), bytes);
}

#[test]
fn descriptor_record_loop_cancellation_and_operation_ceiling_are_explicit() {
    let (fixture, bytes) = completed_copy();
    let cancelled = Arc::new(AtomicBool::new(false));
    let owner = cancelled.clone();
    let control = InspectionControl::with_progress_interval(
        cancelled.clone(),
        std::time::Duration::ZERO,
        move |progress| {
            if progress.records_checked > 0 {
                owner.store(true, Ordering::Release);
            }
        },
    );
    assert!(matches!(
        inspect_recorded_album_file_job(&bytes, &control),
        Err(E::Cancelled)
    ));
    assert!(cancelled.load(Ordering::Acquire));
    let oversized = header_only(&bytes, |header| {
        let file = header["snapshot"]["files"][0].clone();
        header["snapshot"]["files"] = serde_json::Value::Array(vec![file; 4097]);
    });
    assert!(matches!(
        inspect_recorded_album_file_job(&oversized, &self::control()),
        Err(E::Unsupported)
    ));
    assert_eq!(manifest(&fixture), bytes);
}

fn rehash_scope(header: &mut serde_json::Value) {
    let evidence = &header["binding"]["scope_evidence"];
    let plan: String = serde_json::from_value(evidence["inspected_plan"].clone()).unwrap();
    let source: ObjectIdentity = serde_json::from_value(evidence["source_root"].clone()).unwrap();
    let target: ObjectIdentity = serde_json::from_value(evidence["target_root"].clone()).unwrap();
    let sources: Vec<SourceEvidence> =
        serde_json::from_value(evidence["inspected_sources"].clone()).unwrap();
    let resources: Resources = serde_json::from_value(evidence["resources"].clone()).unwrap();
    let modes: Vec<TransferMode> =
        serde_json::from_value(evidence["allowed_modes"].clone()).unwrap();
    header["binding"]["scope_sha256"] = serde_json::json!(digest(
        &serde_json::to_vec(&(plan, source, target, sources, resources, modes)).unwrap()
    ));
}
fn forge_name_consistently(header: &mut serde_json::Value, file_index: usize) {
    fn change_observations(value: &mut serde_json::Value, reference: &serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if object.get("reference") == Some(reference) && object.contains_key("name") {
                    object.insert("name".into(), serde_json::json!("forged name.mp3"));
                }
                for child in object.values_mut() {
                    change_observations(child, reference);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    change_observations(child, reference);
                }
            }
            _ => {}
        }
    }
    let source = header["snapshot"]["files"][file_index]["source"]["reference"].clone();
    change_observations(header, &source);
    for album in header["snapshot"]["albums"].as_array_mut().unwrap() {
        for child in album["children"].as_array_mut().unwrap() {
            if child["source"]["reference"] == source {
                child["destination_name"] = serde_json::json!("forged name.mp3");
            }
        }
    }
    let destination = &mut header["snapshot"]["files"][file_index]["destination"];
    *destination["components"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap() = serde_json::json!("forged name.mp3");
    let destination = Destination {
        root: serde_json::from_value(destination["root"].clone()).unwrap(),
        components: serde_json::from_value(destination["components"].clone()).unwrap(),
    };
    let operation = &mut header["binding"]["operations"][file_index];
    operation["destination"]["components"] = serde_json::to_value(&destination.components).unwrap();
    operation["destination"]["id"] =
        serde_json::json!(digest(&serde_json::to_vec(&destination).unwrap()));
    rehash_scope(header);
}

#[test]
fn direct_album_file_name_must_equal_its_reference_even_after_consistent_rehash() {
    let (fixture, bytes) = completed_copy();
    let forged = header_only(&bytes, |header| {
        let child = &header["snapshot"]["albums"][0]["children"]
            .as_array()
            .unwrap()
            .iter()
            .find(|child| child["source"]["kind"] == "File")
            .unwrap()["source"]["reference"];
        let index = header["snapshot"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .position(|file| file["source"]["reference"] == *child)
            .unwrap();
        forge_name_consistently(header, index);
    });
    assert!(matches!(
        inspect_recorded_album_file_job(&forged, &control()),
        Err(E::CorruptJournal)
    ));
    assert_eq!(manifest(&fixture), bytes);
}

#[test]
fn standalone_file_name_must_equal_its_reference_even_after_consistent_rehash() {
    let mut fixture = Fixture::new(true, false);
    std::fs::write(fixture.source.join("loose.mp3"), b"owned loose single").unwrap();
    fixture.replan();
    assert!(
        fixture
            .plan
            .operations
            .iter()
            .any(|operation| matches!(operation.kind, OperationKind::SingleTransfer { .. }))
    );
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    drop(job);
    let bytes = manifest(&fixture);
    inspect_recorded_album_file_job(&bytes, &control()).unwrap();
    let forged = header_only(&bytes, |header| {
        let albums = header["snapshot"]["albums"]
            .as_array()
            .unwrap()
            .iter()
            .map(|album| album["operation"].as_u64().unwrap())
            .collect::<Vec<_>>();
        let index = header["snapshot"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .position(|file| !albums.contains(&file["original_operation"].as_u64().unwrap()))
            .unwrap();
        forge_name_consistently(header, index);
    });
    assert!(matches!(
        inspect_recorded_album_file_job(&forged, &control()),
        Err(E::CorruptJournal)
    ));
    assert_eq!(manifest(&fixture), bytes);
}
