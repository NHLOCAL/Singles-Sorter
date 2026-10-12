//! Append-only JSONL framing. Durability is a sink contract, not an fsync claim.
use crate::inspection_control::{InspectionControl, checkpoint, any as controlled_any, find as controlled_find};
use crate::{ExecutionError as E, Result, binding::*, hash};
use serde::{Deserialize, Serialize};
use sorter_core::discovery::ObjectIdentity;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Prepared,
    TempAllocationPrepared,
    TempAllocated,
    BytesWritePrepared,
    BytesSynced,
    AuxiliaryStreamsPrepared,
    AuxiliaryStreamsObserved,
    CopyMetadataPrepared,
    CopyMetadataObserved,
    DestinationPublishPrepared,
    DestinationPublished,
    DestinationVerified,
    Committed,
    SourceRetirementPrepared,
    SourceRetired,
    AtomicRenamePrepared,
    AtomicRenameObserved,
    Failed,
    Cancelled,
    NeedsRecovery,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyMetadataEvidence {
    pub source_identity: ObjectIdentity,
    pub destination_identity: ObjectIdentity,
    pub access_time: i64,
    pub write_time: i64,
    pub readonly: bool,
    pub retained_creation_time: i64,
    pub retained_attributes: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_metadata: Option<CopyMetadataEvidence>,
    pub operation: usize,
    pub destination_id: String,
    pub expected_source: ObjectIdentity,
    pub observed_identity: Option<ObjectIdentity>,
    pub sha256: String,
    pub phase: Phase,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: u16,
    job_id: String,
    binding: BindingData,
    binding_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    sequence: u64,
    job_id: String,
    plan_sha256: String,
    previous: String,
    event: Event,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<T> {
    body: T,
    sha256: String,
}
/// An implementation must have exclusive ownership of private job storage.
/// append may partially write; sync may fail after durable persistence. Every error
/// poisons the writer. No truncation or blind retry API exists here.
pub trait JournalStore {
    fn read_all(&mut self) -> Result<Vec<u8>>;
    fn append(&mut self, bytes: &[u8]) -> Result<()>;
    fn sync(&mut self) -> Result<()>;
}
#[derive(Clone, Debug)]
pub struct PhysicalJournalV2 {
    binding: ExecutionBinding,
    job_id: String,
    bytes: Vec<u8>,
    events: Vec<Event>,
    previous: String,
    tail: Vec<u8>,
    poisoned: bool,
    #[cfg(test)]
    test_record_budget: Option<(usize, usize)>,
}
fn line<T: Serialize>(body: T) -> Result<(Vec<u8>, String)> {
    let sha256 = hash(&body)?;
    let mut bytes = serde_json::to_vec(&Envelope {
        body,
        sha256: sha256.clone(),
    })
    .map_err(|_| E::CorruptJournal)?;
    bytes.push(b'\n');
    Ok((bytes, sha256))
}
impl PhysicalJournalV2 {
    fn header_line(binding: &ExecutionBinding, job_id: &str) -> Result<(Vec<u8>, String)> {
        line(Header { schema: 2, job_id: job_id.into(), binding: binding.data.clone(), binding_sha256: binding.sha256()? })
    }
    pub(crate) fn validate_controlled_creation(binding: &ExecutionBinding, job_id: &str) -> Result<()> {
        use crate::inspection_control::{MAX_OPERATIONS, MAX_RECORDS, check_recorded_write};
        // At most 15 physical records per copied operation, including retirement.
        if job_id.is_empty() || binding.operations().len() > MAX_OPERATIONS
            || binding.operations().len().checked_mul(15).and_then(|n| n.checked_add(1)).is_none_or(|n| n > MAX_RECORDS)
        { return Err(E::Unsupported); }
        let (bytes, _) = Self::header_line(binding, job_id)?;
        check_recorded_write(bytes.len(), bytes.len(), 1)
    }
    pub fn create(
        binding: &ExecutionBinding,
        job_id: &str,
        sink: &mut dyn JournalStore,
    ) -> Result<Self> {
        if binding.recovery_only {
            return Err(E::Scope);
        }
        if job_id.is_empty() || !sink.read_all()?.is_empty() {
            return Err(E::CorruptJournal);
        }
        let (bytes, previous) = Self::header_line(binding, job_id)?;
        if binding.data.copied_move_sources.is_some() { Self::validate_controlled_creation(binding, job_id)?; }
        sink.append(&bytes)?;
        sink.sync()?;
        Ok(Self {
            binding: binding.clone(),
            job_id: job_id.into(),
            bytes,
            events: vec![],
            previous,
            tail: vec![],
            poisoned: false,
            #[cfg(test)]
            test_record_budget: None,
        })
    }
    /// `job_id` and the freshly host-authorized binding must match the header.
    /// An incomplete final record is retained byte-for-byte and blocks append.
    pub fn open(binding: &ExecutionBinding, job_id: &str, bytes: &[u8]) -> Result<Self> {
        Self::open_with_control(binding, job_id, bytes, None)
    }
    pub fn open_controlled(binding: &ExecutionBinding, job_id: &str, bytes: &[u8], control: &InspectionControl) -> Result<Self> {
        Self::open_with_control(binding, job_id, bytes, Some(control))
    }
    pub(crate) fn open_with_control(binding: &ExecutionBinding, job_id: &str, bytes: &[u8], control: Option<&InspectionControl>) -> Result<Self> {
        crate::inspection_control::check_recorded_input(bytes, control)?;
        checkpoint(control)?;
        if control.is_some() && binding.operations().len() > crate::inspection_control::MAX_OPERATIONS { return Err(E::Unsupported); }
        if !binding.data.valid_domain() {
            return Err(E::InvalidBinding);
        }
        let end = bytes
            .iter()
            .rposition(|b| *b == b'\n')
            .ok_or(E::CorruptJournal)?
            + 1;
        let mut lines = bytes[..end].split(|b| *b == b'\n');
        let head: Envelope<Header> = serde_json::from_slice(lines.next().ok_or(E::CorruptJournal)?)
            .map_err(|_| E::CorruptJournal)?;
        if hash(&head.body)? != head.sha256
            || head.body.schema != 2
            || head.body.job_id != job_id
            || head.body.binding != binding.data
            || head.body.binding_sha256 != binding.sha256()?
        {
            return Err(E::CorruptJournal);
        }
        let mut journal = Self {
            binding: binding.clone(),
            job_id: job_id.into(),
            bytes: bytes.to_vec(),
            events: vec![],
            previous: head.sha256,
            tail: bytes[end..].to_vec(),
            poisoned: false,
            #[cfg(test)]
            test_record_budget: None,
        };
        // The terminated input has one final empty split. Avoid collecting a
        // second record vector and check cancellation while consuming each line.
        let mut records = lines.peekable();
        while let Some(raw) = records.next() {
            if records.peek().is_none() { break; }
            checkpoint(control)?;
            let record: Envelope<Record> =
                serde_json::from_slice(raw).map_err(|_| E::CorruptJournal)?;
            if hash(&record.body)? != record.sha256
                || record.body.sequence != journal.events.len() as u64 + 1
                || record.body.previous != journal.previous
                || record.body.job_id != job_id
                || record.body.plan_sha256 != binding.plan_sha256()
            {
                return Err(E::CorruptJournal);
            }
            journal.validate_with_control(&record.body.event, control)
                .map_err(|error| if error == E::Cancelled { error } else { E::CorruptJournal })?;
            if let Some(control) = control { control.record()?; }
            journal.events.push(record.body.event);
            journal.previous = record.sha256;
        }
        checkpoint(control)?;
        Ok(journal)
    }
    /// Reconstruct only read-only reconciliation authority. Host consent must
    /// independently identify the complete inspected binding and original scope.
    /// This does not require the pre-job source still to exist after a rename.
    pub fn open_for_recovery(scope: &RecoveryScope, job_id: &str, bytes: &[u8]) -> Result<Self> {
        Self::open_for_recovery_domain(scope, job_id, bytes, ExecutionDomain::Synthetic)
    }
    pub(crate) fn open_for_recovery_domain(
        scope: &RecoveryScope,
        job_id: &str,
        bytes: &[u8],
        domain: ExecutionDomain,
    ) -> Result<Self> {
        Self::open_for_recovery_domain_with_control(scope, job_id, bytes, domain, None)
    }
    pub(crate) fn open_for_recovery_domain_with_control(
        scope: &RecoveryScope, job_id: &str, bytes: &[u8], domain: ExecutionDomain,
        control: Option<&InspectionControl>,
    ) -> Result<Self> {
        crate::inspection_control::check_recorded_input(bytes, control)?;
        checkpoint(control)?;
        let end = bytes
            .iter()
            .position(|b| *b == b'\n')
            .ok_or(E::CorruptJournal)?;
        let head: Envelope<Header> =
            serde_json::from_slice(&bytes[..end]).map_err(|_| E::CorruptJournal)?;
        if control.is_some() && head.body.binding.operations.len() > crate::inspection_control::MAX_OPERATIONS { return Err(E::Unsupported); }
        checkpoint(control)?;
        if hash(&head.body.binding)? != scope.inspected_binding_sha256
            || head.body.binding.scope_sha256 != scope_digest(&scope.original_scope)?
            || head.body.binding.plan_sha256 != scope.original_scope.inspected_plan
            || head.body.binding.resources != scope.original_scope.resources
            || head.body.binding.source_root.identity.as_ref()
                != Some(&scope.original_scope.source_root)
            || head.body.binding.target_root.identity.as_ref()
                != Some(&scope.original_scope.target_root)
            || !head.body.binding.valid_domain()
            || head.body.binding.domain() != domain
        {
            return Err(E::Scope);
        }
        let binding = ExecutionBinding {
            data: head.body.binding,
            recovery_only: true,
        };
        Self::open_with_control(&binding, job_id, bytes, control)
    }
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn incomplete_tail(&self) -> &[u8] {
        &self.tail
    }
    pub fn needs_recovery(&self) -> bool {
        self.poisoned || !self.tail.is_empty()
    }
    pub fn binding(&self) -> &ExecutionBinding {
        &self.binding
    }
    pub fn append(&mut self, sink: &mut dyn JournalStore, event: Event) -> Result<()> {
        if self.binding.recovery_only {
            return Err(E::Scope);
        }
        if !self.tail.is_empty() {
            return Err(E::IncompleteTail);
        }
        if self.poisoned {
            return Err(E::NeedsRecovery);
        }
        self.validate(&event)?;
        // Detect mismatched/externally extended stores rather than appending on stale state.
        self.poisoned = true;
        if sink.read_all()? != self.bytes {
            return Err(E::CorruptJournal);
        }
        let record = Record {
            sequence: self.events.len() as u64 + 1,
            job_id: self.job_id.clone(),
            plan_sha256: self.binding.plan_sha256().into(),
            previous: self.previous.clone(),
            event: event.clone(),
        };
        let (bytes, next) = line(record)?;
        if self.binding.data.copied_move_sources.is_some() {
            let total = self.bytes.len().checked_add(bytes.len()).ok_or(E::NeedsRecovery)?;
            let records = self.events.len().checked_add(2).ok_or(E::NeedsRecovery)?;
            #[cfg(test)]
            if self.test_record_budget.is_some_and(|(line, budget)| bytes.len() > line || total > budget) {
                return Err(E::NeedsRecovery);
            }
            crate::inspection_control::check_recorded_write(bytes.len(), total, records).map_err(|_| E::NeedsRecovery)?;
        }

        sink.append(&bytes)?;
        sink.sync()?;
        self.bytes.extend(bytes);
        self.events.push(event);
        self.previous = next;
        self.poisoned = false;
        Ok(())
    }
    fn validate(&self, event: &Event) -> Result<()> {
        self.validate_with_control(event, None)
    }
    fn history_with_control(&self, operation: usize, control: Option<&InspectionControl>) -> Result<Vec<&Event>> {
        let mut history = Vec::new();
        for event in &self.events {
            checkpoint(control)?;
            if event.operation == operation { history.push(event); }
        }
        Ok(history)
    }
    fn validate_with_control(&self, event: &Event, control: Option<&InspectionControl>) -> Result<()> {
        checkpoint(control)?;
        let op = self
            .binding
            .operations()
            .get(event.operation)
            .ok_or(E::InvalidTransition)?;
        if event.destination_id != op.destination.id
            || Some(&event.expected_source) != op.source.observation.identity.as_ref()
            || event.sha256 != op.source.sha256
        {
            return Err(E::InvalidTransition);
        }
        let history = self.history_with_control(event.operation, control)?;
        let last = history.last().map(|e| e.phase);
        use Phase::*;
        if self.binding.data.windows_rename()
            && !matches!(
                event.phase,
                Prepared
                    | AtomicRenamePrepared
                    | AtomicRenameObserved
                    | DestinationVerified
                    | Committed
                    | Failed
                    | Cancelled
                    | NeedsRecovery
            )
        {
            return Err(E::InvalidTransition);
        }
        let valid = match (last, event.phase) {
            (None, Prepared) => true,
            (Some(Prepared), TempAllocationPrepared | AtomicRenamePrepared)
            | (Some(TempAllocationPrepared), TempAllocated)
            | (Some(TempAllocated), BytesWritePrepared)
            | (Some(BytesWritePrepared), BytesSynced)
            | (Some(BytesSynced), AuxiliaryStreamsPrepared)
            | (Some(AuxiliaryStreamsPrepared), AuxiliaryStreamsObserved)
            | (Some(AuxiliaryStreamsObserved), CopyMetadataPrepared)
            | (Some(BytesSynced), CopyMetadataPrepared)
            | (Some(CopyMetadataPrepared), CopyMetadataObserved)
            | (Some(CopyMetadataObserved), DestinationPublishPrepared)
            | (Some(BytesSynced), DestinationPublishPrepared)
            | (Some(DestinationPublishPrepared), DestinationPublished)
            | (Some(DestinationPublished), DestinationVerified)
            | (Some(AtomicRenamePrepared), AtomicRenameObserved)
            | (Some(AtomicRenameObserved), DestinationVerified)
            | (Some(DestinationVerified), Committed)
            | (Some(Committed), SourceRetirementPrepared)
            | (Some(SourceRetirementPrepared), SourceRetired) => true,
            (Some(p), Failed | Cancelled | NeedsRecovery) => {
                !matches!(p, Failed | Cancelled | NeedsRecovery | SourceRetired)
            }
            _ => false,
        };
        if !valid {
            return Err(E::InvalidTransition);
        }
        let streams_required = self.binding.data.copied_move_operation(op);
        if self.binding.data.copied_move_sources.is_some()
            && !streams_required
            && matches!(
                event.phase,
                TempAllocationPrepared | SourceRetirementPrepared
            )
        {
            return Err(E::InvalidTransition);
        }
        if matches!(
            event.phase,
            AuxiliaryStreamsPrepared | AuxiliaryStreamsObserved
        ) && !streams_required
        {
            return Err(E::InvalidTransition);
        }
        if streams_required
            && (event.phase == AtomicRenamePrepared
                || event.phase == CopyMetadataPrepared && last != Some(AuxiliaryStreamsObserved))
        {
            return Err(E::InvalidTransition);
        }
        let metadata_required = self.binding.data.metadata_required_for(op);
        if matches!(event.phase, CopyMetadataPrepared | CopyMetadataObserved) && !metadata_required
        {
            return Err(E::InvalidTransition);
        }
        if event.phase == DestinationPublishPrepared
            && metadata_required
            && last != Some(CopyMetadataObserved)
        {
            return Err(E::InvalidTransition);
        }
        if event.phase == CopyMetadataObserved {
            let evidence = event.copy_metadata.as_ref().ok_or(E::InvalidTransition)?;
            if evidence.source_identity != event.expected_source
                || Some(&evidence.destination_identity) != event.observed_identity.as_ref()
                || evidence.access_time <= 0
                || evidence.write_time <= 0
                || evidence.retained_attributes & 0x81 != 0
            {
                return Err(E::InvalidTransition);
            }
        } else if event.copy_metadata.is_some() {
            return Err(E::InvalidTransition);
        }

        // Gate new effects, not observations of already-issued effects. The
        // distinct intent phases make that distinction durable across crashes.
        if matches!(
            event.phase,
            Prepared
                | TempAllocationPrepared
                | BytesWritePrepared
                | AuxiliaryStreamsPrepared
                | CopyMetadataPrepared
                | DestinationPublishPrepared
                | AtomicRenamePrepared
                | SourceRetirementPrepared
        ) && !self.dependencies_ready_with_control(event.operation, control)?
        {
            return Err(E::InvalidTransition);
        }
        let mut required = Vec::new();
        for operation in self.binding.operations() {
            checkpoint(control)?;
            if operation.source.observation.reference == op.source.observation.reference {
                required.push(operation.destination.id.as_str());
            }
        }
        let atomic = controlled_any(history.iter(), control, |event| event.phase == AtomicRenamePrepared)?;
        if event.phase == AtomicRenamePrepared
            && (op.mode != TransferMode::Move || required.len() != 1)
        {
            return Err(E::InvalidTransition);
        }
        if event.phase == SourceRetirementPrepared {
            if op.mode != TransferMode::Move || atomic {
                return Err(E::InvalidTransition);
            }
            if controlled_any(self.events.iter(), control, |e| {
                matches!(e.phase, Failed | Cancelled | NeedsRecovery)
                    && self.binding.operations()[e.operation]
                        .source
                        .observation
                        .reference
                        == op.source.observation.reference
            })? {
                return Err(E::InvalidTransition);
            }
            let mut committed = HashSet::new();
            for prior in &self.events {
                checkpoint(control)?;
                if prior.phase == Committed { committed.insert(prior.destination_id.as_str()); }
            }
            if required.is_empty() || controlled_any(required.iter(), control, |id| !committed.contains(id))? {
                return Err(E::InvalidTransition);
            }
            // One retirement owner per source; another operation cannot retire it twice.
            if controlled_any(self.events.iter(), control, |e| {
                e.operation != event.operation
                    && matches!(e.phase, SourceRetirementPrepared | SourceRetired)
                    && self.binding.operations()[e.operation]
                        .source
                        .observation
                        .reference
                        == op.source.observation.reference
            })? {
                return Err(E::InvalidTransition);
            }
        }
        let expected = if matches!(
            event.phase,
            AtomicRenamePrepared | AtomicRenameObserved | SourceRetirementPrepared | SourceRetired
        ) || atomic
        {
            Some(&event.expected_source)
        } else {
            controlled_find(history.iter(), control, |event| event.phase == TempAllocated)?
                .and_then(|event| event.observed_identity.as_ref())
        };
        if event.phase == TempAllocated {
            if !(matches!(event.observed_identity, Some(ObjectIdentity::Test { .. }))
                && self.binding.data.synthetic_only
                || matches!(
                    event.observed_identity,
                    Some(ObjectIdentity::Windows { .. })
                ) && (self.binding.data.windows_copy()
                    || self.binding.data.copied_move_sources.is_some()))
                || event.observed_identity == self.binding.data.source_root.identity
                || event.observed_identity == self.binding.data.target_root.identity
                || controlled_any(self.binding.operations().iter(), control, |o| event.observed_identity == o.source.observation.identity)?
                || event.observed_identity.as_ref() == Some(&event.expected_source)
                || controlled_any(self.events.iter(), control, |e| {
                    e.phase == TempAllocated && e.observed_identity == event.observed_identity
                })?
            {
                return Err(E::InvalidTransition);
            }
        } else if matches!(event.phase, Prepared | TempAllocationPrepared) {
            if event.observed_identity.is_some() {
                return Err(E::InvalidTransition);
            }
        } else if !matches!(event.phase, Failed | Cancelled | NeedsRecovery)
            && (event.observed_identity.as_ref() != expected || expected.is_none())
        {
            return Err(E::InvalidTransition);
        }
        checkpoint(control)
    }
    /// Destination commitment alone does not complete a move dependency.
    /// The sole pre-retirement exception is another copy of the same duet source.
    pub(crate) fn dependency_ready_with_control(&self, dependency: usize, dependent: usize, control: Option<&InspectionControl>) -> Result<bool> {
        checkpoint(control)?;
        let prior = &self.binding.operations()[dependency];
        let next = &self.binding.operations()[dependent];
        let history = self.history_with_control(dependency, control)?;
        if !history.last().is_some_and(|event| matches!(event.phase, Phase::Committed | Phase::SourceRetired)) {
            return Ok(false);
        }
        if controlled_any(self.events.iter(), control, |event| {
            matches!(event.phase, Phase::Failed | Phase::Cancelled | Phase::NeedsRecovery)
                && self.binding.operations()[event.operation].source.observation.reference == prior.source.observation.reference
        })? { return Ok(false); }
        if prior.mode == TransferMode::Copy || same_duet_cohort(prior, next) { return Ok(true); }
        Ok(controlled_any(self.events.iter(), control, |event| {
            event.phase == Phase::SourceRetired
                && self.binding.operations()[event.operation].source.observation.reference == prior.source.observation.reference
        })? || controlled_any(history.iter(), control, |event| event.phase == Phase::AtomicRenameObserved)?)
    }
    pub(crate) fn dependencies_ready(&self, dependent: usize) -> bool {
        self.dependencies_ready_with_control(dependent, None).unwrap_or(false)
    }
    fn dependencies_ready_with_control(&self, dependent: usize, control: Option<&InspectionControl>) -> Result<bool> {
        for dependency in &self.binding.operations()[dependent].depends_on {
            checkpoint(control)?;
            if !self.dependency_ready_with_control(*dependency, dependent, control)? { return Ok(false); }
        }
        Ok(true)
    }
    pub fn latest_phases(&self) -> HashMap<usize, Phase> {
        self.events.iter().map(|e| (e.operation, e.phase)).collect()
    }
}

#[cfg(test)]
mod copy_metadata_compatibility_tests {
    use super::*;
    const OLD: &[u8] = include_bytes!("../tests/fixtures/physical-copy-v2-before-metadata.jsonl");
    fn old_binding() -> ExecutionBinding {
        let first = OLD.split(|byte| *byte == b'\n').next().unwrap();
        let header: Envelope<Header> = serde_json::from_slice(first).unwrap();
        ExecutionBinding {
            data: header.body.binding,
            recovery_only: true,
        }
    }
    #[test]
    fn golden_old_copy_journal_remains_byte_compatible_without_metadata_claim() {
        assert_eq!(
            crate::digest(OLD),
            "e6ed3bafe8538bb1b4ccc755cdc494157a1810d4a858c26b1bfb40e9b27d6c2b"
        );
        let binding = old_binding();
        assert_eq!(binding.data.copy_metadata_version, None);
        let journal = PhysicalJournalV2::open(&binding, "old-copy-v2", OLD).unwrap();
        assert_eq!(journal.events().len(), 9);
        assert_eq!(journal.events().last().unwrap().phase, Phase::Committed);
        assert!(
            journal
                .events()
                .iter()
                .all(|event| event.copy_metadata.is_none())
        );
        for line in OLD
            .split(|byte| *byte == b'\n')
            .skip(1)
            .filter(|line| !line.is_empty())
        {
            let envelope: Envelope<Record> = serde_json::from_slice(line).unwrap();
            assert_eq!(serde_json::to_vec(&envelope).unwrap(), line);
        }
    }
    #[test]
    fn unknown_or_inconsistent_copy_metadata_requirement_is_invalid() {
        for version in [1, 2, 255] {
            let mut binding = old_binding();
            binding.data.copy_metadata_version = Some(version);
            assert!(!binding.data.valid_domain());
            assert!(PhysicalJournalV2::open(&binding, "old-copy-v2", OLD).is_err());
        }
        let mut raw = serde_json::to_value(&old_binding().data).unwrap();
        raw["copy_metadata_version"] = serde_json::json!("required");
        assert!(serde_json::from_value::<BindingData>(raw).is_err());
    }
    #[test]
    fn old_binding_cannot_acquire_metadata_claim_from_an_event() {
        let binding = old_binding();
        let journal = PhysicalJournalV2::open(&binding, "old-copy-v2", OLD).unwrap();
        let mut event = journal.events()[4].clone();
        event.phase = Phase::CopyMetadataPrepared;
        assert!(journal.validate(&event).is_err());
        event.phase = Phase::Committed;
        event.copy_metadata = Some(CopyMetadataEvidence {
            source_identity: event.expected_source.clone(),
            destination_identity: event.observed_identity.clone().unwrap(),
            access_time: 1,
            write_time: 1,
            readonly: false,
            retained_creation_time: 1,
            retained_attributes: 0,
        });
        assert!(journal.validate(&event).is_err());
    }
}

#[cfg(all(test, windows, feature = "selected-windows-execution"))]
mod selected_copy_metadata_transition_tests {
    use super::*;
    #[derive(Default)]
    pub(super) struct Sink(Vec<u8>);
    impl JournalStore for Sink {
        fn read_all(&mut self) -> Result<Vec<u8>> {
            Ok(self.0.clone())
        }
        fn append(&mut self, bytes: &[u8]) -> Result<()> {
            self.0.extend_from_slice(bytes);
            Ok(())
        }
        fn sync(&mut self) -> Result<()> {
            Ok(())
        }
    }
    fn identity(byte: u8) -> sorter_core::discovery::ObjectIdentity {
        ObjectIdentity::Windows {
            volume: 1,
            file_id: [byte; 16],
        }
    }
    pub(super) fn binding() -> ExecutionBinding {
        let first = include_bytes!("../tests/fixtures/physical-copy-v2-before-metadata.jsonl")
            .split(|byte| *byte == b'\n')
            .next()
            .unwrap();
        let header: serde_json::Value = serde_json::from_slice(first).unwrap();
        let mut data = header["body"]["binding"].clone();
        fn convert(value: &mut serde_json::Value) {
            match value {
                serde_json::Value::Object(map) => {
                    if let Some(inner) = map.remove("Test") {
                        let id = inner["id"].as_str().unwrap();
                        let byte = match id {
                            "source" => 1,
                            "target" => 2,
                            "song" => 3,
                            _ => 4,
                        };
                        *value = serde_json::to_value(identity(byte)).unwrap();
                    } else if let Some(inner) = map.remove("TestOpaque") {
                        let id = inner["id"].as_str().unwrap();
                        let path = if id == "song" {
                            r"C:\fixture\source\song".to_owned()
                        } else {
                            format!(r"C:\fixture\{id}")
                        };
                        *value =
                            serde_json::to_value(sorter_core::discovery::StorageRef::WindowsPath {
                                utf16: path.encode_utf16().collect(),
                            })
                            .unwrap();
                    } else {
                        for child in map.values_mut() {
                            convert(child);
                        }
                    }
                }
                serde_json::Value::Array(items) => {
                    for child in items {
                        convert(child);
                    }
                }
                _ => {}
            }
        }
        convert(&mut data);
        data["synthetic_only"] = serde_json::json!(false);
        data["selected_windows_copy"] = serde_json::json!(true);
        data["copy_metadata_version"] = serde_json::json!(1);
        let data: BindingData = serde_json::from_value(data).unwrap();
        assert!(data.valid_domain());
        ExecutionBinding {
            data,
            recovery_only: false,
        }
    }
    pub(super) fn event(binding: &ExecutionBinding, phase: Phase) -> Event {
        let op = &binding.operations()[0];
        Event {
            copy_metadata: None,
            operation: 0,
            destination_id: op.destination.id.clone(),
            expected_source: op.source.observation.identity.clone().unwrap(),
            observed_identity: if matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared) {
                None
            } else {
                Some(identity(9))
            },
            sha256: op.source.sha256.clone(),
            phase,
        }
    }
    fn ready() -> (ExecutionBinding, PhysicalJournalV2, Sink) {
        let binding = binding();
        let mut sink = Sink::default();
        let mut journal =
            PhysicalJournalV2::create(&binding, "metadata-transition", &mut sink).unwrap();
        for phase in [
            Phase::Prepared,
            Phase::TempAllocationPrepared,
            Phase::TempAllocated,
            Phase::BytesWritePrepared,
            Phase::BytesSynced,
        ] {
            journal.append(&mut sink, event(&binding, phase)).unwrap();
        }
        (binding, journal, sink)
    }
    pub(super) fn observed(binding: &ExecutionBinding) -> Event {
        let mut event = event(binding, Phase::CopyMetadataObserved);
        event.copy_metadata = Some(CopyMetadataEvidence {
            source_identity: event.expected_source.clone(),
            destination_identity: identity(9),
            access_time: 10,
            write_time: 20,
            readonly: true,
            retained_creation_time: 30,
            retained_attributes: 0x20,
        });
        event
    }
    #[test]
    fn selected_metadata_transition_requires_intent_observation_and_matching_evidence() {
        let (binding, mut journal, mut sink) = ready();
        assert!(
            journal
                .validate(&event(&binding, Phase::DestinationPublishPrepared))
                .is_err()
        );
        assert!(journal.validate(&observed(&binding)).is_err());
        journal
            .append(&mut sink, event(&binding, Phase::CopyMetadataPrepared))
            .unwrap();
        assert!(
            journal
                .validate(&event(&binding, Phase::CopyMetadataObserved))
                .is_err()
        );
        for bad in 0..4 {
            let mut observation = observed(&binding);
            let evidence = observation.copy_metadata.as_mut().unwrap();
            match bad {
                0 => evidence.source_identity = identity(77),
                1 => evidence.destination_identity = identity(78),
                2 => evidence.access_time = 0,
                _ => evidence.retained_attributes |= 1,
            }
            assert!(journal.validate(&observation).is_err());
        }
        journal.append(&mut sink, observed(&binding)).unwrap();
        journal
            .append(
                &mut sink,
                event(&binding, Phase::DestinationPublishPrepared),
            )
            .unwrap();
        assert!(PhysicalJournalV2::open(&binding, "metadata-transition", &sink.0).is_ok());
    }
    #[test]
    fn selected_metadata_transition_rejects_rehashed_missing_acknowledgment() {
        let (binding, mut journal, mut sink) = ready();
        journal
            .append(&mut sink, event(&binding, Phase::CopyMetadataPrepared))
            .unwrap();
        journal.append(&mut sink, observed(&binding)).unwrap();
        journal
            .append(
                &mut sink,
                event(&binding, Phase::DestinationPublishPrepared),
            )
            .unwrap();
        let mut lines = sink.0.split(|byte| *byte == b'\n');
        let head = lines.next().unwrap();
        let envelope: Envelope<Header> = serde_json::from_slice(head).unwrap();
        let mut previous = envelope.sha256;
        let mut bytes = head.to_vec();
        bytes.push(b'\n');
        let mut sequence = 1;
        for line in lines.filter(|line| !line.is_empty()) {
            let mut envelope: Envelope<Record> = serde_json::from_slice(line).unwrap();
            if envelope.body.event.phase == Phase::CopyMetadataObserved {
                continue;
            }
            envelope.body.sequence = sequence;
            sequence += 1;
            envelope.body.previous = previous;
            envelope.sha256 = hash(&envelope.body).unwrap();
            previous = envelope.sha256.clone();
            bytes.extend(serde_json::to_vec(&envelope).unwrap());
            bytes.push(b'\n');
        }
        assert!(matches!(
            PhysicalJournalV2::open(&binding, "metadata-transition", &bytes),
            Err(E::CorruptJournal)
        ));
    }
    #[test]
    fn selected_metadata_transition_retains_pending_intent_without_publication() {
        let (binding, mut journal, mut sink) = ready();
        journal
            .append(&mut sink, event(&binding, Phase::CopyMetadataPrepared))
            .unwrap();
        let recovered = PhysicalJournalV2::open(&binding, "metadata-transition", &sink.0).unwrap();
        assert_eq!(
            recovered.events().last().unwrap().phase,
            Phase::CopyMetadataPrepared
        );
        assert!(
            recovered
                .events()
                .iter()
                .all(|event| event.copy_metadata.is_none()
                    && event.phase != Phase::DestinationPublishPrepared)
        );
    }
}

#[cfg(all(test, windows, feature = "selected-windows-execution"))]
mod copied_move_journal_tests {
    use super::selected_copy_metadata_transition_tests::{
        Sink, binding as copy_binding, event, observed,
    };
    use super::*;
    use sorter_core::discovery::ObjectIdentity;
    fn binding() -> ExecutionBinding {
        let mut binding = copy_binding();
        binding.data.selected_windows_copy = false;
        if let Some(ObjectIdentity::Windows { volume, .. }) = &mut binding.data.target_root.identity
        {
            *volume = 2;
        }
        if let ObjectIdentity::Windows { volume, .. } = &mut binding.data.scope_evidence.target_root
        {
            *volume = 2;
        }
        binding.data.scope_evidence.allowed_modes = vec![TransferMode::Move];
        binding.data.operations[0].mode = TransferMode::Move;
        binding.data.operations[0].copy_effect = false;
        let source = &binding.data.operations[0].source;
        let digest = crate::transfer::source_expected(&binding.data.operations[0])
            .unwrap()
            .digest;
        binding.data.copied_move_sources = Some(vec![CopiedMoveSourceEvidence {
            source: source.observation.reference.clone(),
            identity: source.observation.identity.clone().unwrap(),
            attributes: 0,
            streams: vec![CopiedMoveStreamEvidence {
                name: "::$DATA".encode_utf16().collect(),
                length: digest.length,
                sha256: digest.sha256,
            }],
        }]);
        assert!(binding.data.valid_domain());
        binding
    }
    fn ready() -> (ExecutionBinding, PhysicalJournalV2, Sink) {
        let binding = binding();
        let mut sink = Sink::default();
        let mut journal =
            PhysicalJournalV2::create(&binding, "copied-move-phases", &mut sink).unwrap();
        for phase in [
            Phase::Prepared,
            Phase::TempAllocationPrepared,
            Phase::TempAllocated,
            Phase::BytesWritePrepared,
            Phase::BytesSynced,
        ] {
            journal.append(&mut sink, event(&binding, phase)).unwrap();
        }
        (binding, journal, sink)
    }
    #[test]
    fn copied_move_record_budget_refusal_preserves_prefix_and_poison() {
        for line_budget in [true, false] {
            let (binding, mut journal, mut sink) = ready();
            let prefix = sink.0.clone(); let events = journal.events().to_vec();
            journal.test_record_budget = Some(if line_budget { (0, usize::MAX) } else { (usize::MAX, prefix.len()) });
            let next = event(&binding, Phase::AuxiliaryStreamsPrepared);
            assert_eq!(journal.append(&mut sink, next.clone()), Err(E::NeedsRecovery));
            assert_eq!(sink.0, prefix); assert_eq!(journal.events(), events); assert!(journal.poisoned);
            let control = InspectionControl::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), |_| {});
            let reopened = PhysicalJournalV2::open_controlled(&binding, "copied-move-phases", &prefix, &control).unwrap();
            assert_eq!(reopened.events(), events); assert_eq!(reopened.events().last().unwrap().phase, Phase::BytesSynced);
            journal.test_record_budget = None;
            assert_eq!(journal.append(&mut sink, next), Err(E::NeedsRecovery));
            assert_eq!(sink.0, prefix); assert_eq!(journal.events(), events);
        }
    }
    #[test]
    fn copied_move_requires_auxiliary_ack_before_metadata_and_publication() {
        let (binding, mut journal, mut sink) = ready();
        assert!(
            journal
                .validate(&event(&binding, Phase::CopyMetadataPrepared))
                .is_err()
        );
        assert!(
            journal
                .validate(&event(&binding, Phase::DestinationPublishPrepared))
                .is_err()
        );
        journal
            .append(&mut sink, event(&binding, Phase::AuxiliaryStreamsPrepared))
            .unwrap();
        assert!(
            journal
                .validate(&event(&binding, Phase::CopyMetadataPrepared))
                .is_err()
        );
        journal
            .append(&mut sink, event(&binding, Phase::AuxiliaryStreamsObserved))
            .unwrap();
        journal
            .append(&mut sink, event(&binding, Phase::CopyMetadataPrepared))
            .unwrap();
        journal.append(&mut sink, observed(&binding)).unwrap();
        journal
            .append(
                &mut sink,
                event(&binding, Phase::DestinationPublishPrepared),
            )
            .unwrap();
    }
    #[test]
    fn copied_move_auxiliary_ack_binds_the_exact_temp_identity() {
        let (binding, mut journal, mut sink) = ready();
        journal
            .append(&mut sink, event(&binding, Phase::AuxiliaryStreamsPrepared))
            .unwrap();
        let mut wrong = event(&binding, Phase::AuxiliaryStreamsObserved);
        wrong.observed_identity = binding.data.source_root.identity.clone();
        assert!(journal.validate(&wrong).is_err());
        wrong.observed_identity = None;
        assert!(journal.validate(&wrong).is_err());
    }
    #[test]
    fn ordinary_copy_does_not_accept_auxiliary_move_phases() {
        let binding = copy_binding();
        let mut sink = Sink::default();
        let mut journal = PhysicalJournalV2::create(&binding, "ordinary-copy", &mut sink).unwrap();
        for phase in [
            Phase::Prepared,
            Phase::TempAllocationPrepared,
            Phase::TempAllocated,
            Phase::BytesWritePrepared,
            Phase::BytesSynced,
        ] {
            journal.append(&mut sink, event(&binding, phase)).unwrap();
        }
        assert!(
            journal
                .validate(&event(&binding, Phase::AuxiliaryStreamsPrepared))
                .is_err()
        );
    }
    #[test]
    fn copied_move_domain_rejects_missing_or_inconsistent_stream_markers() {
        for variant in 0..5 {
            let mut binding = binding();
            match variant {
                0 => binding.data.copied_move_sources = Some(vec![]),
                1 => binding.data.copy_metadata_version = None,
                2 => binding.data.selected_windows_copy = true,
                3 => binding.data.copied_move_sources.as_mut().unwrap()[0].streams[0]
                    .name
                    .clear(),
                _ => binding.data.copied_move_sources.as_mut().unwrap()[0].streams[0].length += 1,
            }
            let bytes = serde_json::to_vec(&binding.data).unwrap();
            let decoded: BindingData = serde_json::from_slice(&bytes).unwrap();
            assert!(!decoded.valid_domain());
        }
    }
    #[test]
    fn mixed_route_ordinary_source_accepts_only_rename_and_needs_no_copy_metadata() {
        let mut binding = binding();
        if let Some(ObjectIdentity::Windows { volume, .. }) = &mut binding.data.target_root.identity
        {
            *volume = 1;
        }
        binding.data.operations[0].copy_effect = true;
        binding.data.operations[0].duet_cohort = Some(vec![0, 1]);
        let mut duet = binding.data.operations[0].clone();
        duet.id = 1;
        duet.destination.id = "second".into();
        duet.destination.components = vec!["second".into()];
        binding.data.operations.push(duet);
        let mut ordinary = binding.data.operations[0].clone();
        ordinary.id = 2;
        ordinary.copy_effect = false;
        ordinary.duet_cohort = None;
        ordinary.depends_on.clear();
        ordinary.source.observation.reference = sorter_core::discovery::StorageRef::WindowsPath {
            utf16: r"C:\fixture\source\ordinary".encode_utf16().collect(),
        };
        ordinary.source.observation.name = "ordinary".into();
        ordinary.source.observation.identity = Some(ObjectIdentity::Windows {
            volume: 1,
            file_id: [8; 16],
        });
        ordinary.destination.id = "ordinary".into();
        ordinary.destination.components = vec!["ordinary".into()];
        binding.data.operations.push(ordinary.clone());
        assert!(binding.data.valid_domain());
        assert!(
            binding
                .data
                .metadata_required_for(&binding.data.operations[0])
        );
        assert!(!binding.data.metadata_required_for(&ordinary));
        let mut sink = Sink::default();
        let mut journal = PhysicalJournalV2::create(&binding, "mixed-route", &mut sink).unwrap();
        let mut prepared = event(&binding, Phase::Prepared);
        prepared.operation = 2;
        prepared.destination_id = ordinary.destination.id.clone();
        prepared.expected_source = ordinary.source.observation.identity.clone().unwrap();
        journal.append(&mut sink, prepared.clone()).unwrap();
        let mut temp = prepared.clone();
        temp.phase = Phase::TempAllocationPrepared;
        assert!(journal.validate(&temp).is_err());
        let mut rename = prepared;
        rename.phase = Phase::AtomicRenamePrepared;
        rename.observed_identity = Some(rename.expected_source.clone());
        journal.append(&mut sink, rename).unwrap();
        let mut forged = binding.data.copied_move_sources.as_ref().unwrap()[0].clone();
        forged.source = ordinary.source.observation.reference;
        forged.identity = ordinary.source.observation.identity.unwrap();
        binding
            .data
            .copied_move_sources
            .as_mut()
            .unwrap()
            .push(forged);
        assert!(!binding.data.valid_domain());
    }

    #[test]
    fn copied_move_never_accepts_atomic_rename_intent() {
        let binding = binding();
        let mut sink = Sink::default();
        let mut journal =
            PhysicalJournalV2::create(&binding, "copied-no-rename", &mut sink).unwrap();
        journal
            .append(&mut sink, event(&binding, Phase::Prepared))
            .unwrap();
        let mut rename = event(&binding, Phase::AtomicRenamePrepared);
        rename.observed_identity = Some(rename.expected_source.clone());
        assert!(journal.validate(&rename).is_err());
    }
}

#[cfg(test)]
mod controlled_writer_boundary_tests {
    use super::*;
    #[test]
    fn physical_exact_envelope_and_newline_match_reader_limit() {
        let header: Envelope<Header> = serde_json::from_slice(include_bytes!("../tests/fixtures/physical-copy-v2-before-metadata.jsonl").split(|b| *b == b'\n').next().unwrap()).unwrap();
        let mut binding = ExecutionBinding { data: header.body.binding, recovery_only: false };
        binding.data.resources.resource_ids.push(String::new());
        let initial = PhysicalJournalV2::header_line(&binding, "boundary").unwrap().0.len();
        let padding = crate::inspection_control::MAX_RECORD_BYTES - initial;
        *binding.data.resources.resource_ids.last_mut().unwrap() = "x".repeat(padding);
        let bytes = PhysicalJournalV2::header_line(&binding, "boundary").unwrap().0;
        assert_eq!(bytes.len(), crate::inspection_control::MAX_RECORD_BYTES);
        assert!(serde_json::to_vec(&binding.data).unwrap().len() < bytes.len());
        assert_eq!(PhysicalJournalV2::validate_controlled_creation(&binding, "boundary"), Ok(()));
        assert_eq!(PhysicalJournalV2::validate_controlled_creation(&binding, "boundaryx"), Err(E::Unsupported));
        let control = InspectionControl::new(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), |_| {});
        assert_eq!(crate::inspection_control::check_recorded_input(&bytes, Some(&control)), Ok(()));
        let larger = PhysicalJournalV2::header_line(&binding, "boundaryx").unwrap().0;
        assert_eq!(crate::inspection_control::check_recorded_input(&larger, Some(&control)), Err(E::Unsupported));
    }
}
