use super::binding::*;
use crate::{
    ExecutionError as E, Result,
    binding::{RecoveryScope, scope_digest},
    hash,
    physical_journal::*,
};
use serde::{Deserialize, Serialize};
use sorter_core::discovery::ObjectIdentity;
use sorter_storage_contracts::MutationObservation;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AlbumEvent {
    DirectoryIntent {
        index: usize,
    },
    DirectoryCreated {
        index: usize,
        observed: MutationObservation,
    },
    DirectorySynced {
        index: usize,
        identity: ObjectIdentity,
    },
    /// Each chunk is one existing physical transfer-journal record, provenance checked.
    TransferRecord {
        bytes: Vec<u8>,
    },
    Complete,
    Stopped {
        detail: String,
        observed: Option<MutationObservation>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Header {
    schema: u16,
    job_id: String,
    binding: AlbumData,
    binding_sha256: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    sequence: usize,
    job_id: String,
    previous: String,
    event: AlbumEvent,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<T> {
    body: T,
    sha256: String,
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
/// Append-only parent journal. Opening is always read-only. Incomplete tails are
/// retained byte-for-byte and poison the entire album, including child records.
#[derive(Clone, Debug)]
pub struct AlbumJournal {
    pub(super) binding: AlbumBinding,
    pub(super) job_id: String,
    bytes: Vec<u8>,
    previous: String,
    events: Vec<AlbumEvent>,
    tail: Vec<u8>,
    poisoned: bool,
    pub(super) child_bytes: Vec<u8>,
}
impl AlbumJournal {
    pub fn create(
        binding: &AlbumBinding,
        job_id: &str,
        sink: &mut dyn JournalStore,
    ) -> Result<Self> {
        if binding.recovery_only {
            return Err(E::Scope);
        }
        if job_id.is_empty() || !sink.read_all()?.is_empty() {
            return Err(E::CorruptJournal);
        }
        let (bytes, previous) = line(Header {
            schema: 1,
            job_id: job_id.into(),
            binding: binding.data.clone(),
            binding_sha256: binding.sha256()?,
        })?;
        sink.append(&bytes)?;
        sink.sync()?;
        Ok(Self {
            binding: binding.clone(),
            job_id: job_id.into(),
            bytes,
            previous,
            events: vec![],
            tail: vec![],
            poisoned: false,
            child_bytes: vec![],
        })
    }
    /// Recovery scope's inspected digest is the entire album binding, not merely
    /// its child journal. The original grant is checked independently.
    pub fn open_for_recovery(scope: &RecoveryScope, job_id: &str, bytes: &[u8]) -> Result<Self> {
        let first = bytes
            .iter()
            .position(|b| *b == b'\n')
            .ok_or(E::CorruptJournal)?;
        let head: Envelope<Header> =
            serde_json::from_slice(&bytes[..first]).map_err(|_| E::CorruptJournal)?;
        let d = &head.body.binding.transfers;
        if hash(&head.body)? != head.sha256
            || head.body.schema != 1
            || head.body.job_id != job_id
            || hash(&head.body.binding)? != head.body.binding_sha256
        {
            return Err(E::CorruptJournal);
        }
        if head.body.binding_sha256 != scope.inspected_binding_sha256
            || d.scope_sha256 != scope_digest(&scope.original_scope)?
            || d.plan_sha256 != scope.original_scope.inspected_plan
            || d.resources != scope.original_scope.resources
            || d.source_root.identity.as_ref() != Some(&scope.original_scope.source_root)
            || d.target_root.identity.as_ref() != Some(&scope.original_scope.target_root)
            || !d.synthetic_only
            || d.album_context.is_none()
        {
            return Err(E::Scope);
        }
        let end = bytes.iter().rposition(|b| *b == b'\n').unwrap() + 1;
        let mut out = Self {
            binding: AlbumBinding {
                data: head.body.binding,
                recovery_only: true,
            },
            job_id: job_id.into(),
            bytes: bytes.to_vec(),
            previous: head.sha256,
            events: vec![],
            tail: bytes[end..].to_vec(),
            poisoned: false,
            child_bytes: vec![],
        };
        let records = &bytes[first + 1..end];
        let lines: Vec<_> = records.split(|b| *b == b'\n').collect();
        for raw in &lines[..lines.len() - 1] {
            let e: Envelope<Record> = serde_json::from_slice(raw).map_err(|_| E::CorruptJournal)?;
            if hash(&e.body)? != e.sha256
                || e.body.sequence != out.events.len() + 1
                || e.body.job_id != job_id
                || e.body.previous != out.previous
            {
                return Err(E::CorruptJournal);
            }
            out.validate(&e.body.event).map_err(|_| E::CorruptJournal)?;
            out.accept(e.body.event);
            out.previous = e.sha256;
        }
        Ok(out)
    }
    pub fn binding(&self) -> &AlbumBinding {
        &self.binding
    }
    pub fn events(&self) -> &[AlbumEvent] {
        &self.events
    }
    pub fn incomplete_tail(&self) -> &[u8] {
        &self.tail
    }
    pub fn needs_recovery(&self) -> bool {
        self.poisoned || !self.tail.is_empty()
    }
    pub(super) fn append(&mut self, sink: &mut dyn JournalStore, event: AlbumEvent) -> Result<()> {
        if self.binding.recovery_only {
            return Err(E::Scope);
        }
        if self.needs_recovery() {
            return Err(E::NeedsRecovery);
        }
        self.validate(&event)?;
        self.poisoned = true;
        if sink.read_all()? != self.bytes {
            return Err(E::CorruptJournal);
        }
        let (bytes, previous) = line(Record {
            sequence: self.events.len() + 1,
            job_id: self.job_id.clone(),
            previous: self.previous.clone(),
            event: event.clone(),
        })?;
        sink.append(&bytes)?;
        sink.sync()?;
        self.bytes.extend(bytes);
        self.previous = previous;
        self.accept(event);
        self.poisoned = false;
        Ok(())
    }
    fn accept(&mut self, event: AlbumEvent) {
        if let AlbumEvent::TransferRecord { bytes } = &event {
            self.child_bytes.extend(bytes);
        }
        self.events.push(event);
    }
    fn ready(&self, index: usize) -> bool {
        self.binding.directories()[index].existing.is_some()
            || self
                .events
                .iter()
                .any(|e| matches!(e, AlbumEvent::DirectorySynced {index:i,..} if *i == index))
    }
    fn validate(&self, event: &AlbumEvent) -> Result<()> {
        if self
            .events
            .iter()
            .any(|e| matches!(e, AlbumEvent::Complete | AlbumEvent::Stopped { .. }))
        {
            return Err(E::InvalidTransition);
        }
        match event {
            AlbumEvent::DirectoryIntent { index } => {
                if *index == 0
                    || *index >= self.binding.directories().len()
                    || self.binding.directories()[*index].existing.is_some()
                    || !(0..*index).all(|i| self.ready(i))
                    || !self.child_bytes.is_empty()
                    || self
                        .events
                        .iter()
                        .any(|e| matches!(e, AlbumEvent::DirectoryIntent {index:i} if i == index))
                {
                    return Err(E::InvalidTransition);
                }
            }
            AlbumEvent::DirectoryCreated { index, observed } => {
                if !matches!(self.events.last(), Some(AlbumEvent::DirectoryIntent {index:i}) if i == index)
                {
                    return Err(E::InvalidTransition);
                }
                let expected = self
                    .binding
                    .directories()
                    .get(*index)
                    .ok_or(E::InvalidTransition)?;
                let parent = self
                    .expected_directory_identity(index - 1)
                    .ok_or(E::InvalidTransition)?;
                if !matches!(observed.identity, ObjectIdentity::Test { .. })
                    || !matches!(
                        observed.reference,
                        sorter_core::discovery::StorageRef::TestOpaque { .. }
                    )
                    || observed.parent_identity != parent
                    || observed.actual_name
                        != sorter_storage_contracts::MutationName::Provider(
                            expected.components.last().unwrap().clone(),
                        )
                    || self
                        .binding
                        .children()
                        .any(|s| s.observation.identity.as_ref() == Some(&observed.identity))
                    || self.binding.source().identity.as_ref() == Some(&observed.identity)
                    || self.binding.data.transfers.source_root.identity.as_ref()
                        == Some(&observed.identity)
                    || (0..self.binding.directories().len()).any(|i| {
                        self.expected_directory_identity(i).as_ref() == Some(&observed.identity)
                    })
                {
                    return Err(E::InvalidTransition);
                }
            }
            AlbumEvent::DirectorySynced { index, identity } => {
                if !matches!(self.events.last(), Some(AlbumEvent::DirectoryCreated {index:i,observed}) if i == index && &observed.identity == identity)
                {
                    return Err(E::InvalidTransition);
                }
            }
            AlbumEvent::TransferRecord { bytes } => {
                if !(0..self.binding.directories().len()).all(|i| self.ready(i))
                    || bytes.is_empty()
                    || !bytes.ends_with(b"\n")
                    || bytes[..bytes.len() - 1].contains(&b'\n')
                {
                    return Err(E::InvalidTransition);
                }
                let mut combined = self.child_bytes.clone();
                combined.extend(bytes);
                let child =
                    PhysicalJournalV2::open(&self.binding.transfers(), &self.job_id, &combined)?;
                // A newly allocated file cannot alias any directory that this
                // parent binding/lease protects. Check at TempAllocated, before
                // the next BytesWritePrepared event can authorize streaming.
                for event in child
                    .events()
                    .iter()
                    .filter(|e| e.phase == Phase::TempAllocated)
                {
                    let identity = event
                        .observed_identity
                        .as_ref()
                        .ok_or(E::InvalidTransition)?;
                    if self.binding.source().identity.as_ref() == Some(identity)
                        || (0..self.binding.directories().len())
                            .any(|i| self.expected_directory_identity(i).as_ref() == Some(identity))
                    {
                        return Err(E::InvalidTransition);
                    }
                }
            }
            AlbumEvent::Complete => {
                if self.child_bytes.is_empty() {
                    return Err(E::InvalidTransition);
                }
                let child = PhysicalJournalV2::open(
                    &self.binding.transfers(),
                    &self.job_id,
                    &self.child_bytes,
                )?;
                if child.needs_recovery()
                    || child.latest_phases().len() != self.binding.data.transfers.operations.len()
                    || child
                        .latest_phases()
                        .values()
                        .any(|p| *p != Phase::Committed)
                {
                    return Err(E::InvalidTransition);
                }
            }
            AlbumEvent::Stopped { .. } => {}
        }
        Ok(())
    }
    pub(super) fn expected_directory_identity(&self, index: usize) -> Option<ObjectIdentity> {
        self.binding
            .directories()
            .get(index)?
            .existing
            .as_ref()
            .and_then(|e| e.identity.clone())
            .or_else(|| {
                self.events.iter().find_map(|e| match e {
                    AlbumEvent::DirectoryCreated { index: i, observed } if *i == index => {
                        Some(observed.identity.clone())
                    }
                    _ => None,
                })
            })
    }
}
pub(super) struct ChildSink<'a> {
    pub journal: &'a mut AlbumJournal,
    pub sink: &'a mut dyn JournalStore,
}
impl JournalStore for ChildSink<'_> {
    fn read_all(&mut self) -> Result<Vec<u8>> {
        if self.journal.needs_recovery() || self.sink.read_all()? != self.journal.bytes {
            return Err(E::NeedsRecovery);
        }
        Ok(self.journal.child_bytes.clone())
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        self.journal.append(
            self.sink,
            AlbumEvent::TransferRecord {
                bytes: bytes.to_vec(),
            },
        )
    }
    // append already durably synchronized the enclosing record. No effect is
    // dispatched between this adapter's append and sync calls.
    fn sync(&mut self) -> Result<()> {
        if self.journal.needs_recovery() {
            Err(E::NeedsRecovery)
        } else {
            Ok(())
        }
    }
}
