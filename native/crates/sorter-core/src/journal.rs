//! Append-only journal evidence for an already inspected supplied-facts plan.
//!
//! This seam only opens the explicitly supplied journal path. Source and
//! destination identities stay opaque; recording success never verifies a
//! physical file or authorizes execution. All Phase 1 plans remain non-executable.
//! Replay returns evidence, not copy/delete/recovery instructions. An incomplete
//! final line is preserved and blocks every later append until a separate future
//! recovery decision. There is deliberately no truncation or repair API.
//!
//! File flush/sync and cooperative locking are exercised on Linux only. They do
//! not establish Windows crash durability, durable parent-directory creation,
//! filesystem/hardware guarantees, or Android SAF atomicity.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{DestinationDisposition, EngineError, ErrorCode, Plan, source_delete_allowed};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JournalBinding {
    pub plan_id: String,
    pub operations: Vec<OperationBinding>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    pub operation_id: String,
    pub source_id: String,
    pub copy_mode: bool,
    pub required_destination_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Prepared,
    DestinationVerified,
    Committed,
    SourceDeleted,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JournalEvent {
    pub sequence: u64,
    pub plan_id: String,
    pub operation_id: String,
    pub destination_id: Option<String>,
    pub state: OperationState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayState {
    /// Complete, ordered, transition-validated records. No synthetic events.
    pub events: Vec<JournalEvent>,
    /// Trailing bytes without a newline were retained but not accepted.
    pub incomplete_tail: bool,
}

/// An exclusively locked append-only journal, bound to an owned immutable copy
/// of the supplied binding. Drop releases the file lock. Read-only replay and a
/// second writer fail promptly while this handle is alive.
pub struct Journal {
    file: File,
    binding: JournalBinding,
    tracker: Tracker,
    incomplete_tail: bool,
    poisoned: bool,
    expected_len: u64,
}

impl JournalBinding {
    /// Capture the inspected plan's ordered identities and Required intents.
    ///
    /// The caller remains responsible for `verify_plan_binding` against the
    /// original request when that verification is needed. This conversion is
    /// not plan authentication, physical verification or execution eligibility.
    /// It neither rehashes nor silently changes any existing Plan semantics.
    pub fn from_plan(plan: &Plan) -> Result<Self, EngineError> {
        let mut destination_ids = BTreeSet::new();
        let mut operations = Vec::with_capacity(plan.items.len());
        for item in &plan.items {
            let mut required_destination_ids = vec![];
            for destination in &item.destinations {
                if destination.destination_id.is_empty()
                    || !destination_ids.insert(&destination.destination_id)
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Plan destination identities must be nonempty and globally unique",
                    ));
                }
                if destination.disposition == DestinationDisposition::Required {
                    required_destination_ids.push(destination.destination_id.clone());
                }
            }
            operations.push(OperationBinding {
                operation_id: item.operation_id.clone(),
                source_id: item.source_id.clone(),
                copy_mode: item.copy_mode,
                required_destination_ids,
            });
        }
        let binding = Self {
            plan_id: plan.plan_id.clone(),
            operations,
        };
        validate_binding(&binding)?;
        Ok(binding)
    }
}

impl Journal {
    /// Atomically create a new journal, or inspect every existing record without
    /// rewriting it. An existing empty/partial header is corrupt, never a new
    /// journal. A complete header with an incomplete event tail remains openable
    /// for inspection, but the returned handle cannot append.
    pub fn open(path: &Path, binding: &JournalBinding) -> Result<Self, EngineError> {
        validate_binding(binding)?;
        let (mut file, created) = match OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => (file, true),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (
                OpenOptions::new()
                    .read(true)
                    .append(true)
                    .open(path)
                    .map_err(io_error)?,
                false,
            ),
            Err(error) => return Err(io_error(error)),
        };
        file.try_lock().map_err(lock_error)?;
        let (tracker, incomplete_tail) = if created {
            let header = Header {
                kind: HeaderKind::JournalHeader,
                journal_version: 1,
                binding: binding.clone(),
            };
            write_record(&mut file, &header)?;
            (Tracker::new(binding), false)
        } else {
            let (state, tracker) = replay_file(&mut file, binding)?;
            (tracker, state.incomplete_tail)
        };
        let expected_len = file.metadata().map_err(io_error)?.len();
        Ok(Self {
            file,
            binding: binding.clone(),
            tracker,
            incomplete_tail,
            poisoned: false,
            expected_len,
        })
    }

    /// Validate first; append one JSONL record and flush/sync before accepting it
    /// in memory. All events get a sync barrier, including Committed and
    /// SourceDeleted. An uncertain I/O result poisons this handle: evidence is
    /// preserved and only a later explicit reopen/replay can inspect it.
    pub fn append(&mut self, event: &JournalEvent) -> Result<(), EngineError> {
        if self.incomplete_tail || self.poisoned {
            return Err(transition_error(
                "Append is blocked by an incomplete tail or an uncertain prior write",
            ));
        }
        if self.file.metadata().map_err(io_error)?.len() != self.expected_len {
            self.poisoned = true;
            return Err(transition_error(
                "Journal length changed outside this exclusively locked handle",
            ));
        }
        let mut next = self.tracker.clone();
        next.accept(&self.binding, event)?;
        // Once writing starts, an error may leave any prefix, including a full
        // record whose sync failed. Never continue based on a guessed outcome.
        self.poisoned = true;
        write_record(&mut self.file, event)?;
        self.expected_len = self.file.metadata().map_err(io_error)?.len();
        self.tracker = next;
        self.poisoned = false;
        Ok(())
    }
}

/// Read and validate a journal under a shared lock, without mutation or physical
/// replay. The whole stored binding must exactly match the caller's binding.
pub fn replay(path: &Path, binding: &JournalBinding) -> Result<ReplayState, EngineError> {
    validate_binding(binding)?;
    let mut file = File::open(path).map_err(io_error)?;
    file.try_lock_shared().map_err(lock_error)?;
    replay_file(&mut file, binding).map(|(state, _)| state)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Header {
    kind: HeaderKind,
    journal_version: u16,
    binding: JournalBinding,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum HeaderKind {
    JournalHeader,
}

fn validate_binding(binding: &JournalBinding) -> Result<(), EngineError> {
    if binding.plan_id.is_empty() {
        return Err(error(ErrorCode::InvalidInput, "plan_id must not be empty"));
    }
    let mut operation_ids = BTreeSet::new();
    let mut destination_ids = BTreeSet::new();
    for operation in &binding.operations {
        if operation.operation_id.is_empty()
            || operation.source_id.is_empty()
            || !operation_ids.insert(&operation.operation_id)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Operation identities must be nonempty and unique; source_id must not be empty",
            ));
        }
        for destination_id in &operation.required_destination_ids {
            if destination_id.is_empty() || !destination_ids.insert(destination_id) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Required destination identities must be nonempty and globally unique",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Default)]
struct Progress {
    state: Option<OperationState>,
    verified: BTreeSet<String>,
}

#[derive(Clone)]
struct Tracker {
    last_sequence: u64,
    operations: BTreeMap<String, Progress>,
}

impl Tracker {
    fn new(binding: &JournalBinding) -> Self {
        Self {
            last_sequence: 0,
            operations: binding
                .operations
                .iter()
                .map(|operation| (operation.operation_id.clone(), Progress::default()))
                .collect(),
        }
    }

    fn accept(
        &mut self,
        binding: &JournalBinding,
        event: &JournalEvent,
    ) -> Result<(), EngineError> {
        if event.plan_id != binding.plan_id
            || self.last_sequence.checked_add(1) != Some(event.sequence)
        {
            return Err(transition_error(
                "Event plan_id must match and sequence must be contiguous from one",
            ));
        }
        let operation = binding
            .operations
            .iter()
            .find(|operation| operation.operation_id == event.operation_id)
            .ok_or_else(|| {
                transition_error("Event operation_id is not in the immutable binding")
            })?;
        let progress = self
            .operations
            .get_mut(&event.operation_id)
            .ok_or_else(|| transition_error("Event operation_id has no bound state"))?;
        if event.state != OperationState::DestinationVerified && event.destination_id.is_some() {
            return Err(transition_error(
                "Only DestinationVerified may contain a destination_id",
            ));
        }
        match event.state {
            OperationState::Prepared if progress.state.is_none() => {}
            OperationState::DestinationVerified
                if matches!(
                    progress.state,
                    Some(OperationState::Prepared | OperationState::DestinationVerified)
                ) =>
            {
                let id = event.destination_id.as_ref().ok_or_else(|| {
                    transition_error("DestinationVerified requires a listed destination_id")
                })?;
                if !operation.required_destination_ids.contains(id)
                    || progress.verified.contains(id)
                {
                    return Err(transition_error(
                        "Destination must be Required for this operation and verified exactly once",
                    ));
                }
                progress.verified.insert(id.clone());
            }
            OperationState::Committed
                if matches!(
                    progress.state,
                    Some(OperationState::Prepared | OperationState::DestinationVerified)
                ) && !operation.required_destination_ids.is_empty()
                    && progress.verified.len() == operation.required_destination_ids.len() => {}
            OperationState::SourceDeleted
                if progress.state == Some(OperationState::Committed)
                    && source_delete_allowed(
                        operation.copy_mode,
                        operation.required_destination_ids.len(),
                        progress.verified.len(),
                        false,
                    ) => {}
            OperationState::Failed | OperationState::Cancelled
                if matches!(
                    progress.state,
                    Some(
                        OperationState::Prepared
                            | OperationState::DestinationVerified
                            | OperationState::Committed
                    )
                ) => {}
            _ => {
                return Err(transition_error(
                    "Invalid operation transition: require Prepared, distinct verification, full commit, and a committed move before source deletion; terminal states cannot advance",
                ));
            }
        }
        progress.state = Some(event.state);
        self.last_sequence = event.sequence;
        Ok(())
    }
}

fn replay_file(
    file: &mut File,
    binding: &JournalBinding,
) -> Result<(ReplayState, Tracker), EngineError> {
    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let mut reader = BufReader::new(file);
    let mut line = vec![];
    reader.read_until(b'\n', &mut line).map_err(io_error)?;
    if line.last() != Some(&b'\n') {
        return Err(error(
            ErrorCode::JournalCorrupt,
            "A complete journal binding header is missing",
        ));
    }
    let header: Header = serde_json::from_slice(&line).map_err(json_error)?;
    if header.journal_version != 1 {
        return Err(error(
            ErrorCode::JournalCorrupt,
            "Unsupported journal header version",
        ));
    }
    validate_binding(&header.binding).map_err(|invalid| EngineError {
        code: ErrorCode::JournalCorrupt,
        message: format!("Invalid persisted journal binding: {}", invalid.message),
    })?;
    if header.binding != *binding {
        return Err(transition_error(
            "The complete persisted journal binding differs from the supplied binding",
        ));
    }
    let mut tracker = Tracker::new(binding);
    let mut state = ReplayState {
        events: vec![],
        incomplete_tail: false,
    };
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line).map_err(io_error)? == 0 {
            break;
        }
        if line.last() != Some(&b'\n') {
            state.incomplete_tail = true;
            break;
        }
        let event: JournalEvent = serde_json::from_slice(&line).map_err(json_error)?;
        tracker.accept(binding, &event)?;
        state.events.push(event);
    }
    Ok((state, tracker))
}

fn write_record(file: &mut File, record: &impl Serialize) -> Result<(), EngineError> {
    let mut bytes = serde_json::to_vec(record).map_err(json_error)?;
    bytes.push(b'\n');
    file.write_all(&bytes).map_err(io_error)?;
    file.flush().map_err(io_error)?;
    file.sync_all().map_err(io_error)
}

fn error(code: ErrorCode, message: &str) -> EngineError {
    EngineError {
        code,
        message: message.into(),
    }
}

fn transition_error(message: &str) -> EngineError {
    error(ErrorCode::InvalidJournalTransition, message)
}

fn io_error(error: io::Error) -> EngineError {
    EngineError {
        code: ErrorCode::JournalCorrupt,
        message: format!("Journal I/O failed; evidence was not repaired or discarded: {error}"),
    }
}

fn json_error(error: serde_json::Error) -> EngineError {
    EngineError {
        code: ErrorCode::JournalCorrupt,
        message: format!("Malformed complete journal record: {error}"),
    }
}

fn lock_error(error: std::fs::TryLockError) -> EngineError {
    match error {
        std::fs::TryLockError::WouldBlock => {
            transition_error("Journal is already locked by an active writer or reader")
        }
        std::fs::TryLockError::Error(error) => io_error(error),
    }
}

#[cfg(test)]
mod tests;
