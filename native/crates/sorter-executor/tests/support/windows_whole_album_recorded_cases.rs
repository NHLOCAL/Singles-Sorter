//! Real whole writer journals feed a data-only descriptor, never native grants.
use super::*;
use serde::{Deserialize, Serialize};
use sorter_executor::inspection_control::InspectionControl;
use std::sync::Arc;
fn control() -> InspectionControl<'static> {
    InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {})
}
pub(super) fn manifest(fixture: &Fixture) -> Vec<u8> {
    let path = std::fs::read_dir(&fixture.journal)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".album-move.jsonl")
        })
        .unwrap();
    std::fs::read(path).unwrap()
}
pub(super) fn completed(root: bool, mixed: bool) -> (Fixture, Vec<u8>) {
    let mut fixture = Fixture::new(false, root);
    if mixed {
        std::fs::write(fixture.source.join("loose.mp3"), b"owned loose single").unwrap();
    }
    std::fs::rename(
        fixture.album.join("1-שיר.mp3"),
        fixture.album.join("1-שיר  ארוך.mp3"),
    )
    .unwrap();
    std::fs::rename(
        fixture.album.join("booklet"),
        fixture.album.join("booklet  notes"),
    )
    .unwrap();
    fixture.replan();
    let mut job = fixture.job();
    let result = job.run(&AtomicBool::new(false), &mut |_| {}).unwrap();
    assert_eq!(result.status, JobStatus::Complete, "{result:?}");
    assert_eq!(result.whole_albums.len(), 1);
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
pub(super) fn header_only(bytes: &[u8], change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let end = bytes.iter().position(|b| *b == b'\n').unwrap();
    let mut record: Envelope = serde_json::from_slice(&bytes[..end]).unwrap();
    change(&mut record.body.payload);
    record.sha256 = digest(&serde_json::to_vec(&record.body).unwrap());
    let mut bytes = serde_json::to_vec(&record).unwrap();
    bytes.push(b'\n');
    bytes
}
#[test]
fn whole_root_and_child_descriptors_preserve_atomic_and_child_location_granularity() {
    for root in [false, true] {
        let (fixture, bytes) = completed(root, false);
        let recorded = inspect_recorded_whole_album_job(&bytes, &control()).unwrap();
        assert_eq!(recorded.files.len(), 5);
        assert_eq!(recorded.albums.len(), 1);
        assert_eq!(recorded.original_source.is_some(), root);
        assert_eq!(recorded.child_locations.len(), 2);
        assert_eq!(recorded.directories.len(), 3);
        assert_ne!(
            recorded.child_locations[0].original,
            recorded.child_locations[0].prepared
        );
        assert_eq!(recorded.child_locations[0].operation, 0);
        for (index, file) in recorded.files.iter().enumerate() {
            assert_eq!(file.inspection_index, index);
            assert_eq!(file.operation, 0);
        }
        let json = serde_json::to_value(recorded).unwrap();
        for forbidden in [
            "scope",
            "binding",
            "recorded_finished",
            "completion",
            "grant",
        ] {
            assert!(json.get(forbidden).is_none());
        }
        assert_eq!(manifest(&fixture), bytes);
    }
}
#[test]
fn whole_mixed_descriptor_retains_actual_deferred_single_identity_and_order() {
    let (fixture, bytes) = completed(false, true);
    let recorded = inspect_recorded_whole_album_job(&bytes, &control()).unwrap();
    assert_eq!(recorded.files.len(), 6);
    assert!(recorded.singles_job_id.is_some());
    assert_eq!(recorded.files[5].operation, 1);
    assert_eq!(recorded.files[5].inspection_index, 5);
    assert_eq!(manifest(&fixture), bytes);
}
#[test]
fn whole_header_rejects_rehashed_membership_identity_and_hash_forgery() {
    let (_, bytes) = completed(false, false);
    for change in 0..3 {
        let forged = header_only(&bytes, |value| match change {
            0 => {
                value["snapshot"]["directories"][0]["children"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            1 => value["observed_sources"][0]["identity"] = value["target"]["identity"].clone(),
            _ => value["scope"]["inspected_sources"][0]["sha256"] = serde_json::json!("wrong"),
        });
        assert_eq!(
            inspect_recorded_whole_album_job(&forged, &control()).unwrap_err(),
            E::CorruptJournal
        );
    }
}
#[test]
fn whole_header_rejects_rehashed_deferred_binding_substitution() {
    let (_, bytes) = completed(false, true);
    let forged = header_only(&bytes, |value| {
        value["singles_binding"]["operations"][0]["destination"]["components"] =
            serde_json::json!(["foreign.mp3"])
    });
    assert_eq!(
        inspect_recorded_whole_album_job(&forged, &control()).unwrap_err(),
        E::CorruptJournal
    );
}
#[test]
fn whole_descriptor_cancellation_and_torn_tail_never_claim_completion() {
    let (_, bytes) = completed(false, false);
    let cancelled = InspectionControl::new(Arc::new(AtomicBool::new(true)), |_| {});
    assert_eq!(
        inspect_recorded_whole_album_job(&bytes, &cancelled).unwrap_err(),
        E::Cancelled
    );
    let mut prefix = header_only(&bytes, |_| {});
    prefix.extend_from_slice(b"{partial");
    let recorded = inspect_recorded_whole_album_job(&prefix, &control()).unwrap();
    assert_eq!(recorded.files.len(), 5);
    assert!(
        serde_json::to_value(recorded)
            .unwrap()
            .get("recorded_finished")
            .is_none()
    );
}
#[test]
fn whole_descriptor_rejects_raw_native_dot_components_before_normalization() {
    let (_, bytes) = completed(false, false);
    for source in [false, true] {
        for dot in [".", ".."] {
            let forged = header_only(&bytes, |value| {
                let field = if source {
                    &mut value["observed_sources"][0]["reference"]
                } else {
                    &mut value["observed_target"]
                };
                let reference: StorageRef = serde_json::from_value(field.clone()).unwrap();
                let StorageRef::WindowsPath { mut utf16 } = reference else {
                    panic!("Windows fixture")
                };
                let index = utf16.iter().rposition(|u| *u == b'\\' as u16).unwrap() + 1;
                utf16.splice(index..index, format!("{dot}\\").encode_utf16());
                *field = serde_json::to_value(StorageRef::WindowsPath { utf16 }).unwrap();
            });
            assert_eq!(
                inspect_recorded_whole_album_job(&forged, &control()).unwrap_err(),
                E::CorruptJournal
            );
        }
    }
}
