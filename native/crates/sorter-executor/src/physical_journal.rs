//! Append-only JSONL framing. Durability is a sink contract, not an fsync claim.
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
pub struct Event {
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
        let header = Header {
            schema: 2,
            job_id: job_id.into(),
            binding: binding.data.clone(),
            binding_sha256: binding.sha256()?,
        };
        let (bytes, previous) = line(header)?;
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
        })
    }
    /// `job_id` and the freshly host-authorized binding must match the header.
    /// An incomplete final record is retained byte-for-byte and blocks append.
    pub fn open(binding: &ExecutionBinding, job_id: &str, bytes: &[u8]) -> Result<Self> {
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
        };
        let all: Vec<_> = lines.collect();
        for raw in &all[..all.len() - 1] {
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
            journal
                .validate(&record.body.event)
                .map_err(|_| E::CorruptJournal)?;
            journal.events.push(record.body.event);
            journal.previous = record.sha256;
        }
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
        let end = bytes
            .iter()
            .position(|b| *b == b'\n')
            .ok_or(E::CorruptJournal)?;
        let head: Envelope<Header> =
            serde_json::from_slice(&bytes[..end]).map_err(|_| E::CorruptJournal)?;
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
        Self::open(&binding, job_id, bytes)
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
        sink.append(&bytes)?;
        sink.sync()?;
        self.bytes.extend(bytes);
        self.events.push(event);
        self.previous = next;
        self.poisoned = false;
        Ok(())
    }
    fn validate(&self, event: &Event) -> Result<()> {
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
        let history: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.operation == event.operation)
            .collect();
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
        // Gate new effects, not observations of already-issued effects. The
        // distinct intent phases make that distinction durable across crashes.
        if matches!(
            event.phase,
            Prepared
                | TempAllocationPrepared
                | BytesWritePrepared
                | DestinationPublishPrepared
                | AtomicRenamePrepared
                | SourceRetirementPrepared
        ) && !self.dependencies_ready(event.operation)
        {
            return Err(E::InvalidTransition);
        }
        let required = self
            .binding
            .required_destinations(&op.source.observation.reference);
        let atomic = history.iter().any(|e| e.phase == AtomicRenamePrepared);
        if event.phase == AtomicRenamePrepared
            && (op.mode != TransferMode::Move || required.len() != 1)
        {
            return Err(E::InvalidTransition);
        }
        if event.phase == SourceRetirementPrepared {
            if op.mode != TransferMode::Move || atomic {
                return Err(E::InvalidTransition);
            }
            if self.events.iter().any(|e| {
                matches!(e.phase, Failed | Cancelled | NeedsRecovery)
                    && self.binding.operations()[e.operation]
                        .source
                        .observation
                        .reference
                        == op.source.observation.reference
            }) {
                return Err(E::InvalidTransition);
            }
            let committed: HashSet<_> = self
                .events
                .iter()
                .filter(|e| e.phase == Committed)
                .map(|e| e.destination_id.as_str())
                .collect();
            if required.is_empty() || !required.iter().all(|id| committed.contains(id)) {
                return Err(E::InvalidTransition);
            }
            // One retirement owner per source; another operation cannot retire it twice.
            if self.events.iter().any(|e| {
                e.operation != event.operation
                    && matches!(e.phase, SourceRetirementPrepared | SourceRetired)
                    && self.binding.operations()[e.operation]
                        .source
                        .observation
                        .reference
                        == op.source.observation.reference
            }) {
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
            history
                .iter()
                .find(|e| e.phase == TempAllocated)
                .and_then(|e| e.observed_identity.as_ref())
        };
        if event.phase == TempAllocated {
            if !(matches!(event.observed_identity, Some(ObjectIdentity::Test { .. }))
                && self.binding.data.synthetic_only
                || matches!(
                    event.observed_identity,
                    Some(ObjectIdentity::Windows { .. })
                ) && self.binding.data.windows_copy())
                || event.observed_identity == self.binding.data.source_root.identity
                || event.observed_identity == self.binding.data.target_root.identity
                || self
                    .binding
                    .operations()
                    .iter()
                    .any(|o| event.observed_identity == o.source.observation.identity)
                || event.observed_identity.as_ref() == Some(&event.expected_source)
                || self.events.iter().any(|e| {
                    e.phase == TempAllocated && e.observed_identity == event.observed_identity
                })
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
        Ok(())
    }
    /// Destination commitment alone does not complete a move dependency.
    /// The sole pre-retirement exception is another copy of the same duet source.
    pub(crate) fn dependency_ready(&self, dependency: usize, dependent: usize) -> bool {
        let prior = &self.binding.operations()[dependency];
        let next = &self.binding.operations()[dependent];
        let history: Vec<_> = self
            .events
            .iter()
            .filter(|e| e.operation == dependency)
            .collect();
        if !history
            .last()
            .is_some_and(|e| matches!(e.phase, Phase::Committed | Phase::SourceRetired))
        {
            return false;
        }
        if self.events.iter().any(|e| {
            matches!(
                e.phase,
                Phase::Failed | Phase::Cancelled | Phase::NeedsRecovery
            ) && self.binding.operations()[e.operation]
                .source
                .observation
                .reference
                == prior.source.observation.reference
        }) {
            return false;
        }
        if prior.mode == TransferMode::Copy {
            return true;
        }
        if same_duet_cohort(prior, next) {
            return true;
        }
        self.events.iter().any(|e| {
            e.phase == Phase::SourceRetired
                && self.binding.operations()[e.operation]
                    .source
                    .observation
                    .reference
                    == prior.source.observation.reference
        }) || history
            .iter()
            .any(|e| e.phase == Phase::AtomicRenameObserved)
    }
    pub(crate) fn dependencies_ready(&self, dependent: usize) -> bool {
        self.binding.operations()[dependent]
            .depends_on
            .iter()
            .all(|d| self.dependency_ready(*d, dependent))
    }
    pub fn latest_phases(&self) -> HashMap<usize, Phase> {
        self.events.iter().map(|e| (e.operation, e.phase)).collect()
    }
}
