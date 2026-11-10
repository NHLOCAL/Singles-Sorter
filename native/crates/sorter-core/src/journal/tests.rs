use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::*;

// Every filesystem operation here owns a disposable journal directory. Song
// and destination identities are opaque strings, never paths to execute.
struct JournalPath(PathBuf);

impl JournalPath {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "singles-journal-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory.join("journal.jsonl"))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(&self.0).unwrap()
    }

    fn raw_append(&self, bytes: &[u8]) {
        let mut file = OpenOptions::new().append(true).open(&self.0).unwrap();
        file.write_all(bytes).unwrap();
        file.sync_all().unwrap();
    }

    fn event_append(&self, event: &JournalEvent, newline: bool) {
        let mut bytes = serde_json::to_vec(event).unwrap();
        if newline {
            bytes.push(b'\n');
        }
        self.raw_append(&bytes);
    }
}

impl Drop for JournalPath {
    fn drop(&mut self) {
        fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}

fn inspected_plan(copy_mode: bool) -> Plan {
    plan(&PlanRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings {
                copy_mode,
                duet_mode: true,
                ..Settings::default()
            },
            aliases: vec![],
            recognition: RecognitionFacts::Configured {
                bundle_id: "supplied-only".into(),
                health: RecognitionHealth::Healthy,
                classifier: BTreeMap::new(),
                ner: BTreeMap::from([(
                    "song".into(),
                    NerFact::Artists(vec!["ראשון".into(), "שני".into()]),
                )]),
            },
        },
        songs: vec![SongFacts {
            source_id: "opaque-source-not-a-file".into(),
            filename: "song.mp3".into(),
            metadata: MetadataFacts::Present {
                artist: String::new(),
                title: String::new(),
            },
        }],
        target: TargetSnapshot {
            root_id: "opaque-target-not-a-directory".into(),
            existing_directories: vec![],
            existing_files: vec![],
        },
    })
    .unwrap()
}

fn binding(copy_mode: bool) -> JournalBinding {
    JournalBinding::from_plan(&inspected_plan(copy_mode)).unwrap()
}

fn event(
    binding: &JournalBinding,
    sequence: u64,
    state: OperationState,
    destination: Option<usize>,
) -> JournalEvent {
    JournalEvent {
        sequence,
        plan_id: binding.plan_id.clone(),
        operation_id: binding.operations[0].operation_id.clone(),
        destination_id: destination
            .map(|index| binding.operations[0].required_destination_ids[index].clone()),
        state,
    }
}

fn append_success_prefix(journal: &mut Journal, binding: &JournalBinding) -> Vec<JournalEvent> {
    let events = vec![
        event(binding, 1, OperationState::Prepared, None),
        event(binding, 2, OperationState::DestinationVerified, Some(0)),
        event(binding, 3, OperationState::DestinationVerified, Some(1)),
        event(binding, 4, OperationState::Committed, None),
    ];
    for item in &events {
        journal.append(item).unwrap();
    }
    events
}

// Windows locks are mandatory even for a second handle in the same process.
// Observe bytes through the writer's exact File, not a reopen or cloned handle.
// Restore its cursor even if seeking/reading fails; never release the lock or
// reconstruct the Journal, so all transition checks exercise its live tracker.
fn writer_bytes(journal: &mut Journal) -> Vec<u8> {
    let position = journal.file.stream_position().unwrap();
    let mut bytes = Vec::new();
    let read_result = (|| {
        journal.file.seek(SeekFrom::Start(0))?;
        journal.file.read_to_end(&mut bytes)
    })();
    let restore_result = journal.file.seek(SeekFrom::Start(position));
    restore_result.unwrap();
    read_result.unwrap();
    bytes
}

fn rejects_without_writing(journal: &mut Journal, event: &JournalEvent) {
    let before = writer_bytes(journal);
    assert_eq!(
        journal.append(event).unwrap_err().code,
        ErrorCode::InvalidJournalTransition
    );
    assert_eq!(writer_bytes(journal), before);
}

#[test]
fn locked_snapshot_preserves_cursor_lock_and_subsequent_append() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    let before = writer_bytes(&mut journal);
    for position in [0, 7, before.len() as u64 + 10] {
        journal.file.seek(SeekFrom::Start(position)).unwrap();
        assert_eq!(writer_bytes(&mut journal), before);
        assert_eq!(journal.file.stream_position().unwrap(), position);
        assert_eq!(
            Journal::open(path.path(), &value).err().unwrap().code,
            ErrorCode::InvalidJournalTransition
        );
    }
    let prepared = event(&value, 1, OperationState::Prepared, None);
    journal.append(&prepared).unwrap();
    let mut expected = before;
    expected.extend(serde_json::to_vec(&prepared).unwrap());
    expected.push(b'\n');
    assert_eq!(writer_bytes(&mut journal), expected);
    drop(journal);
    assert_eq!(path.bytes(), expected);
}

#[test]
fn plan_binding_keeps_source_mode_and_distinct_ordered_required_ids() {
    for copy_mode in [false, true] {
        let plan = inspected_plan(copy_mode);
        let result = JournalBinding::from_plan(&plan).unwrap();
        assert!(!plan.executable);
        assert_eq!(result.plan_id, plan.plan_id);
        assert_eq!(result.operations.len(), 1);
        assert_eq!(
            result.operations[0].operation_id,
            plan.items[0].operation_id
        );
        assert_eq!(result.operations[0].source_id, plan.items[0].source_id);
        assert_eq!(result.operations[0].copy_mode, copy_mode);
        assert_eq!(
            result.operations[0].required_destination_ids,
            plan.items[0]
                .destinations
                .iter()
                .map(|item| item.destination_id.clone())
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn blocked_and_skipped_intents_are_never_required_verification_ids() {
    let mut plan = inspected_plan(false);
    plan.items[0].destinations[0].disposition = DestinationDisposition::Blocked;
    plan.items[0].destinations[1].disposition = DestinationDisposition::SkippedExistingOnly;
    let result = JournalBinding::from_plan(&plan).unwrap();
    assert!(result.operations[0].required_destination_ids.is_empty());
    let path = JournalPath::new();
    let mut journal = Journal::open(path.path(), &result).unwrap();
    journal
        .append(&event(&result, 1, OperationState::Prepared, None))
        .unwrap();
    for destination in &plan.items[0].destinations {
        let mut verification = event(&result, 2, OperationState::DestinationVerified, None);
        verification.destination_id = Some(destination.destination_id.clone());
        rejects_without_writing(&mut journal, &verification);
    }
    rejects_without_writing(
        &mut journal,
        &event(&result, 2, OperationState::Committed, None),
    );
}

#[test]
fn duplicate_plan_operation_or_destination_ids_are_invalid_input() {
    let mut plan = inspected_plan(false);
    plan.items.push(plan.items[0].clone());
    assert_eq!(
        JournalBinding::from_plan(&plan).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let mut plan = inspected_plan(false);
    plan.items[0].destinations[1].destination_id =
        plan.items[0].destinations[0].destination_id.clone();
    assert_eq!(
        JournalBinding::from_plan(&plan).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn manual_bindings_reject_duplicate_ids_globally_before_creating_a_file() {
    let value = binding(false);
    for duplicate_operation in [true, false] {
        let mut invalid = value.clone();
        let mut second = invalid.operations[0].clone();
        if !duplicate_operation {
            second.operation_id = "different-operation".into();
            second.source_id = "different-source".into();
        }
        invalid.operations.push(second);
        let path = JournalPath::new();
        assert_eq!(
            Journal::open(path.path(), &invalid).err().unwrap().code,
            ErrorCode::InvalidInput
        );
        assert!(!path.path().exists());
    }
}

#[test]
fn new_header_persists_the_entire_binding_before_any_events() {
    let path = JournalPath::new();
    let value = binding(false);
    drop(Journal::open(path.path(), &value).unwrap());
    let bytes = path.bytes();
    assert_eq!(bytes.last(), Some(&b'\n'));
    assert_eq!(bytes.iter().filter(|byte| **byte == b'\n').count(), 1);
    let header: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(header["binding"], serde_json::to_value(&value).unwrap());
    let state = replay(path.path(), &value).unwrap();
    assert!(state.events.is_empty());
    assert!(!state.incomplete_tail);
}

#[test]
fn ordered_move_events_replay_and_reopen_without_rewriting_evidence() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    let mut expected = append_success_prefix(&mut journal, &value);
    drop(journal);
    let before = path.bytes();
    let mut journal = Journal::open(path.path(), &value).unwrap();
    assert_eq!(writer_bytes(&mut journal), before);
    let deleted = event(&value, 5, OperationState::SourceDeleted, None);
    journal.append(&deleted).unwrap();
    expected.push(deleted);
    drop(journal);
    let result = replay(path.path(), &value).unwrap();
    assert_eq!(result.events, expected);
    assert!(!result.incomplete_tail);
    assert!(path.bytes().starts_with(&before));
}

#[test]
fn event_sequence_is_global_contiguous_and_starts_at_one() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    for sequence in [0, 2, u64::MAX] {
        rejects_without_writing(
            &mut journal,
            &event(&value, sequence, OperationState::Prepared, None),
        );
    }
    journal
        .append(&event(&value, 1, OperationState::Prepared, None))
        .unwrap();
    for sequence in [1, 3] {
        rejects_without_writing(
            &mut journal,
            &event(
                &value,
                sequence,
                OperationState::DestinationVerified,
                Some(0),
            ),
        );
    }
}

#[test]
fn every_event_rejects_wrong_plan_operation_and_destination_identity() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    journal
        .append(&event(&value, 1, OperationState::Prepared, None))
        .unwrap();
    for field in ["plan", "operation", "destination"] {
        let mut wrong = event(&value, 2, OperationState::DestinationVerified, Some(0));
        match field {
            "plan" => wrong.plan_id = "wrong-plan".into(),
            "operation" => wrong.operation_id = "wrong-operation".into(),
            _ => wrong.destination_id = Some("wrong-destination".into()),
        }
        rejects_without_writing(&mut journal, &wrong);
    }
}

#[test]
fn reopen_and_replay_compare_plan_source_mode_operations_and_destination_order() {
    let path = JournalPath::new();
    let value = binding(false);
    drop(Journal::open(path.path(), &value).unwrap());
    let before = path.bytes();
    for field in [
        "plan",
        "source",
        "mode",
        "operation",
        "destination",
        "order",
    ] {
        let mut changed = value.clone();
        match field {
            "plan" => changed.plan_id = "wrong-plan".into(),
            "source" => changed.operations[0].source_id = "wrong-source".into(),
            "mode" => changed.operations[0].copy_mode = true,
            "operation" => changed.operations[0].operation_id = "wrong-operation".into(),
            "destination" => {
                changed.operations[0].required_destination_ids[0] = "wrong-destination".into()
            }
            _ => changed.operations[0].required_destination_ids.reverse(),
        }
        assert_eq!(
            Journal::open(path.path(), &changed).err().unwrap().code,
            ErrorCode::InvalidJournalTransition,
            "open {field}"
        );
        assert_eq!(
            replay(path.path(), &changed).unwrap_err().code,
            ErrorCode::InvalidJournalTransition,
            "replay {field}"
        );
        assert_eq!(path.bytes(), before);
    }
}

#[test]
fn prepared_is_required_before_verification_commit_or_source_deletion() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    for (state, destination) in [
        (OperationState::DestinationVerified, Some(0)),
        (OperationState::Committed, None),
        (OperationState::SourceDeleted, None),
        (OperationState::Failed, None),
        (OperationState::Cancelled, None),
    ] {
        rejects_without_writing(&mut journal, &event(&value, 1, state, destination));
    }
}

#[test]
fn duplicate_verification_cannot_substitute_for_a_different_required_id() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    journal
        .append(&event(&value, 1, OperationState::Prepared, None))
        .unwrap();
    journal
        .append(&event(
            &value,
            2,
            OperationState::DestinationVerified,
            Some(0),
        ))
        .unwrap();
    rejects_without_writing(
        &mut journal,
        &event(&value, 3, OperationState::DestinationVerified, Some(0)),
    );
    rejects_without_writing(
        &mut journal,
        &event(&value, 3, OperationState::Committed, None),
    );
    rejects_without_writing(
        &mut journal,
        &event(&value, 3, OperationState::SourceDeleted, None),
    );
    drop(journal);
    path.event_append(
        &event(&value, 3, OperationState::DestinationVerified, Some(0)),
        true,
    );
    assert_eq!(
        replay(path.path(), &value).unwrap_err().code,
        ErrorCode::InvalidJournalTransition
    );
}

#[test]
fn duplicate_commit_and_prepared_events_are_rejected_without_writing() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    append_success_prefix(&mut journal, &value);
    for state in [OperationState::Committed, OperationState::Prepared] {
        rejects_without_writing(&mut journal, &event(&value, 5, state, None));
    }
}

#[test]
fn only_verification_events_may_carry_a_destination_id() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    rejects_without_writing(
        &mut journal,
        &event(&value, 1, OperationState::Prepared, Some(0)),
    );
    journal
        .append(&event(&value, 1, OperationState::Prepared, None))
        .unwrap();
    rejects_without_writing(
        &mut journal,
        &event(&value, 2, OperationState::DestinationVerified, None),
    );
    for state in [OperationState::Failed, OperationState::Cancelled] {
        rejects_without_writing(&mut journal, &event(&value, 2, state, Some(0)));
    }
}

#[test]
fn source_deleted_requires_all_destinations_and_an_actual_commit_in_move_mode() {
    for copy_mode in [false, true] {
        let path = JournalPath::new();
        let value = binding(copy_mode);
        let mut journal = Journal::open(path.path(), &value).unwrap();
        journal
            .append(&event(&value, 1, OperationState::Prepared, None))
            .unwrap();
        journal
            .append(&event(
                &value,
                2,
                OperationState::DestinationVerified,
                Some(0),
            ))
            .unwrap();
        rejects_without_writing(
            &mut journal,
            &event(&value, 3, OperationState::SourceDeleted, None),
        );
        journal
            .append(&event(
                &value,
                3,
                OperationState::DestinationVerified,
                Some(1),
            ))
            .unwrap();
        rejects_without_writing(
            &mut journal,
            &event(&value, 4, OperationState::SourceDeleted, None),
        );
        journal
            .append(&event(&value, 4, OperationState::Committed, None))
            .unwrap();
        if copy_mode {
            rejects_without_writing(
                &mut journal,
                &event(&value, 5, OperationState::SourceDeleted, None),
            );
        } else {
            journal
                .append(&event(&value, 5, OperationState::SourceDeleted, None))
                .unwrap();
            rejects_without_writing(
                &mut journal,
                &event(&value, 6, OperationState::SourceDeleted, None),
            );
        }
    }
}

#[test]
fn failed_and_cancelled_are_terminal_before_or_after_commit() {
    for terminal in [OperationState::Failed, OperationState::Cancelled] {
        for committed in [false, true] {
            let path = JournalPath::new();
            let value = binding(false);
            let mut journal = Journal::open(path.path(), &value).unwrap();
            let sequence = if committed {
                append_success_prefix(&mut journal, &value);
                5
            } else {
                journal
                    .append(&event(&value, 1, OperationState::Prepared, None))
                    .unwrap();
                2
            };
            journal
                .append(&event(&value, sequence, terminal, None))
                .unwrap();
            for state in [
                OperationState::Prepared,
                OperationState::DestinationVerified,
                OperationState::Committed,
                OperationState::SourceDeleted,
                OperationState::Failed,
                OperationState::Cancelled,
            ] {
                rejects_without_writing(
                    &mut journal,
                    &event(
                        &value,
                        sequence + 1,
                        state,
                        (state == OperationState::DestinationVerified).then_some(0),
                    ),
                );
            }
        }
    }
}

#[test]
fn interleaved_operations_cannot_borrow_another_sources_verified_destinations() {
    let path = JournalPath::new();
    let mut value = binding(false);
    value.operations.push(OperationBinding {
        operation_id: "second-operation".into(),
        source_id: "second-source".into(),
        copy_mode: false,
        required_destination_ids: vec!["second-destination".into()],
    });
    let mut journal = Journal::open(path.path(), &value).unwrap();
    journal
        .append(&event(&value, 1, OperationState::Prepared, None))
        .unwrap();
    let mut second = event(&value, 2, OperationState::Prepared, None);
    second.operation_id = value.operations[1].operation_id.clone();
    journal.append(&second).unwrap();
    let mut cross = event(&value, 3, OperationState::DestinationVerified, Some(0));
    cross.operation_id = second.operation_id.clone();
    rejects_without_writing(&mut journal, &cross);
    second.sequence = 3;
    second.state = OperationState::DestinationVerified;
    second.destination_id = Some("second-destination".into());
    journal.append(&second).unwrap();
    rejects_without_writing(
        &mut journal,
        &event(&value, 4, OperationState::Committed, None),
    );
    second.sequence = 4;
    second.state = OperationState::Committed;
    second.destination_id = None;
    journal.append(&second).unwrap();
    drop(journal);
    assert_eq!(replay(path.path(), &value).unwrap().events.len(), 4);
}

#[test]
fn malformed_complete_record_is_corrupt_even_before_an_incomplete_tail() {
    for malformed in [b"not-json\n".as_slice(), b"{}\n", b"\n", b"\xff\n"] {
        let path = JournalPath::new();
        let value = binding(false);
        drop(Journal::open(path.path(), &value).unwrap());
        path.raw_append(malformed);
        path.raw_append(b"{\"sequence\":");
        let before = path.bytes();
        assert_eq!(
            replay(path.path(), &value).unwrap_err().code,
            ErrorCode::JournalCorrupt
        );
        assert_eq!(
            Journal::open(path.path(), &value).err().unwrap().code,
            ErrorCode::JournalCorrupt
        );
        assert_eq!(path.bytes(), before);
    }
}

#[test]
fn incomplete_final_source_deleted_record_never_promotes_and_blocks_append() {
    for well_formed in [false, true] {
        let path = JournalPath::new();
        let value = binding(false);
        let mut journal = Journal::open(path.path(), &value).unwrap();
        let prefix = append_success_prefix(&mut journal, &value);
        drop(journal);
        let deleted = event(&value, 5, OperationState::SourceDeleted, None);
        if well_formed {
            path.event_append(&deleted, false);
        } else {
            path.raw_append(b"{\"sequence\":5,\"state\":\"source_deleted\"");
        }
        let before = path.bytes();
        let recovered = replay(path.path(), &value).unwrap();
        assert_eq!(recovered.events, prefix);
        assert!(recovered.incomplete_tail);
        let mut journal = Journal::open(path.path(), &value).unwrap();
        rejects_without_writing(&mut journal, &deleted);
        assert_eq!(writer_bytes(&mut journal), before);
    }
}

#[test]
fn partial_commit_and_partial_utf8_keep_only_complete_prior_evidence() {
    for tail_kind in ["partial-json", "complete-json", "partial-utf8"] {
        let path = JournalPath::new();
        let value = binding(false);
        let mut journal = Journal::open(path.path(), &value).unwrap();
        let prefix = vec![
            event(&value, 1, OperationState::Prepared, None),
            event(&value, 2, OperationState::DestinationVerified, Some(0)),
            event(&value, 3, OperationState::DestinationVerified, Some(1)),
        ];
        for item in &prefix {
            journal.append(item).unwrap();
        }
        drop(journal);
        match tail_kind {
            "partial-json" => path.raw_append(b"{\"state\":\"committed\""),
            "complete-json" => {
                path.event_append(&event(&value, 4, OperationState::Committed, None), false)
            }
            _ => path.raw_append(b"\xff"),
        }
        let recovered = replay(path.path(), &value).unwrap();
        assert_eq!(recovered.events, prefix);
        assert!(recovered.incomplete_tail);
        let mut journal = Journal::open(path.path(), &value).unwrap();
        rejects_without_writing(
            &mut journal,
            &event(&value, 4, OperationState::Committed, None),
        );
    }
}

#[test]
fn header_corruption_missing_header_and_partial_header_are_never_initialized_over() {
    for bytes in [b"".as_slice(), b"{}\n", b"{\"binding\":", b"not-json\n"] {
        let path = JournalPath::new();
        let value = binding(false);
        fs::write(path.path(), bytes).unwrap();
        assert_eq!(
            Journal::open(path.path(), &value).err().unwrap().code,
            ErrorCode::JournalCorrupt
        );
        assert_eq!(
            replay(path.path(), &value).unwrap_err().code,
            ErrorCode::JournalCorrupt
        );
        assert_eq!(path.bytes(), bytes);
    }
}

#[test]
fn replay_revalidates_wrong_ids_duplicate_commit_and_impossible_state() {
    for tamper in [
        "plan",
        "operation",
        "destination",
        "sequence",
        "commit",
        "delete",
    ] {
        let path = JournalPath::new();
        let value = binding(false);
        let mut journal = Journal::open(path.path(), &value).unwrap();
        append_success_prefix(&mut journal, &value);
        drop(journal);
        let mut wrong = event(&value, 5, OperationState::Committed, None);
        match tamper {
            "plan" => wrong.plan_id = "wrong-plan".into(),
            "operation" => wrong.operation_id = "wrong-operation".into(),
            "destination" => wrong.destination_id = Some("unknown".into()),
            "sequence" => wrong.sequence = 7,
            "delete" => wrong.state = OperationState::Prepared,
            _ => {}
        }
        path.event_append(&wrong, true);
        assert_eq!(
            replay(path.path(), &value).unwrap_err().code,
            ErrorCode::InvalidJournalTransition,
            "{tamper}"
        );
    }
}

#[test]
fn live_writer_excludes_a_second_writer_without_changing_the_journal() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    let before = writer_bytes(&mut journal);
    assert_eq!(
        Journal::open(path.path(), &value).err().unwrap().code,
        ErrorCode::InvalidJournalTransition
    );
    assert_eq!(writer_bytes(&mut journal), before);
    drop(journal);
    drop(Journal::open(path.path(), &value).unwrap());
}

#[test]
fn replay_is_read_only_and_has_no_copy_delete_or_recovery_instructions() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    let mut expected = append_success_prefix(&mut journal, &value);
    let deletion_evidence = event(&value, 5, OperationState::SourceDeleted, None);
    journal.append(&deletion_evidence).unwrap();
    expected.push(deletion_evidence);
    drop(journal);
    let before = path.bytes();
    for _ in 0..2 {
        let recovered = replay(path.path(), &value).unwrap();
        assert_eq!(recovered.events, expected);
        assert_eq!(path.bytes(), before);
    }
    assert_eq!(
        fs::read_dir(path.path().parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn invalid_persisted_header_binding_and_unknown_fields_are_corruption() {
    for tamper in [
        "duplicate-operation",
        "duplicate-destination",
        "unknown",
        "version",
    ] {
        let path = JournalPath::new();
        let value = binding(false);
        drop(Journal::open(path.path(), &value).unwrap());
        let mut header: serde_json::Value = serde_json::from_slice(&path.bytes()).unwrap();
        match tamper {
            "duplicate-operation" => {
                let operation = header["binding"]["operations"][0].clone();
                header["binding"]["operations"]
                    .as_array_mut()
                    .unwrap()
                    .push(operation);
            }
            "duplicate-destination" => {
                header["binding"]["operations"][0]["required_destination_ids"][1] =
                    header["binding"]["operations"][0]["required_destination_ids"][0].clone();
            }
            "unknown" => header["verified_count"] = 999.into(),
            _ => header["journal_version"] = 999.into(),
        }
        let mut bytes = serde_json::to_vec(&header).unwrap();
        bytes.push(b'\n');
        fs::write(path.path(), &bytes).unwrap();
        assert_eq!(
            replay(path.path(), &value).unwrap_err().code,
            ErrorCode::JournalCorrupt
        );
        assert_eq!(path.bytes(), bytes);
    }
}

#[test]
fn unknown_event_counts_are_not_accepted_as_verification_evidence() {
    let path = JournalPath::new();
    let value = binding(false);
    drop(Journal::open(path.path(), &value).unwrap());
    let mut wrong = serde_json::to_value(event(&value, 1, OperationState::Prepared, None)).unwrap();
    wrong["verified_count"] = 999.into();
    let mut bytes = serde_json::to_vec(&wrong).unwrap();
    bytes.push(b'\n');
    path.raw_append(&bytes);
    assert_eq!(
        replay(path.path(), &value).unwrap_err().code,
        ErrorCode::JournalCorrupt
    );
}

// Unix file locks are advisory. An unrelated writer that ignores the protocol
// must not cause this handle to append using stale in-memory state.
#[cfg(unix)]
#[test]
fn out_of_protocol_journal_changes_poison_the_open_writer() {
    let path = JournalPath::new();
    let value = binding(false);
    let mut journal = Journal::open(path.path(), &value).unwrap();
    path.raw_append(b"{\"sequence\":");
    rejects_without_writing(
        &mut journal,
        &event(&value, 1, OperationState::Prepared, None),
    );
    rejects_without_writing(
        &mut journal,
        &event(&value, 1, OperationState::Prepared, None),
    );
}

#[test]
fn caller_mutation_cannot_change_an_open_journals_owned_binding() {
    let path = JournalPath::new();
    let mut value = binding(false);
    let original = value.clone();
    let mut journal = Journal::open(path.path(), &value).unwrap();
    value.plan_id = "changed-plan".into();
    value.operations[0].source_id = "changed-source".into();
    value.operations[0].copy_mode = true;
    value.operations[0].required_destination_ids.clear();
    rejects_without_writing(
        &mut journal,
        &event(&value, 1, OperationState::Prepared, None),
    );
    let expected = append_success_prefix(&mut journal, &original);
    drop(journal);
    assert_eq!(replay(path.path(), &original).unwrap().events, expected);
}
