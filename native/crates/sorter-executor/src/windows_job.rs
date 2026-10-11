//! Host-selected local NTFS jobs. Selection and content inspection are live host
//! authority; serialized plans and journal headers are evidence only.
use crate::{
    ExecutionError as E, Result,
    binding::*,
    physical_journal::{JournalStore, PhysicalJournalV2},
    results::JobResult,
    transfer::{TransferProgress, run_job},
    windows_copy::OwnedCopyReadStorage,
    windows_journal::WindowsJournalStore,
};
use serde::{Deserialize, Serialize};
use sorter_core::{discovery::*, job_plan::*, metadata::MetadataProvider};
use sorter_io::desktop::windows_mutation::*;
use sorter_storage_contracts::{
    MutationErrorKind, MutationName, MutationObservation, MutationOutcome,
};
use std::{
    collections::HashSet,
    ffi::OsString,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

fn path(reference: &StorageRef) -> Result<PathBuf> {
    let StorageRef::WindowsPath { utf16 } = reference else {
        return Err(E::Scope);
    };
    if utf16.contains(&0)
        || utf16
            .split(|c| *c == 47 || *c == 92)
            .any(|part| part == [46] || part == [46, 46])
    {
        return Err(E::Scope);
    }
    Ok(OsString::from_wide(utf16).into())
}
fn reference(path: &std::path::Path) -> StorageRef {
    StorageRef::WindowsPath {
        utf16: path.as_os_str().encode_wide().collect(),
    }
}
fn name(text: &str) -> MutationName {
    MutationName::Windows(text.encode_utf16().collect())
}

/// Read-only capability availability. Detailed policy validation still occurs in
/// prepare, and permissions/collisions are checked again under held handles.
pub fn supported_plan(plan: &JobPlanV2) -> Result<()> {
    if plan.request.mode != JobMode::Sort || plan.operations.is_empty() {
        return Err(E::Unsupported);
    }
    if plan.blockers.iter().any(|b| {
        b != "PHYSICAL_APPLY_UNAVAILABLE"
            && b != "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS"
    }) {
        return Err(E::Unsupported);
    }
    if !plan.request.settings.copy_mode {
        let source = plan
            .observations
            .iter()
            .find(|o| o.reference == plan.request.source)
            .and_then(|o| o.identity.as_ref());
        let target = plan
            .observations
            .iter()
            .find(|o| Some(&o.reference) == plan.request.target.as_ref())
            .and_then(|o| o.identity.as_ref());
        if !matches!((source,target),(Some(ObjectIdentity::Windows{volume:a,..}),Some(ObjectIdentity::Windows{volume:b,..})) if a==b)
        {
            return Err(E::Unsupported);
        }
    }
    for operation in &plan.operations {
        let OperationKind::SingleTransfer {
            source, transfer, ..
        } = &operation.kind
        else {
            return Err(E::Unsupported);
        };
        if source.kind != EntryKind::File
            || !transfer.predicted_success
            || !transfer.blockers.is_empty()
            || transfer.copy != plan.request.settings.copy_mode
        {
            return Err(E::Unsupported);
        }
        let leaf = path(&source.reference)?
            .file_name()
            .ok_or(E::Scope)?
            .encode_wide()
            .collect::<Vec<_>>();
        if String::from_utf16(&leaf).map_err(|_| E::Unsupported)? != source.name {
            return Err(E::Unsupported);
        }
    }
    Ok(())
}

enum Storage {
    Copy(WindowsMutationStorage),
    Rename(WindowsRenameStorage),
}
impl Storage {
    fn roots(&self) -> &WindowsMutationStorage {
        match self {
            Self::Copy(s) => s,
            Self::Rename(s) => s.owned(),
        }
    }
}

/// A one-shot job. All selected root leases survive until this owner is dropped.
/// Construct on the worker thread, retain the preview's scope unchanged, then run.
pub struct SelectedWindowsJob {
    storage: Storage,
    binding: ExecutionBinding,
    job_id: String,
    attempted: bool,
    created_directories: Vec<WindowsParent>,
    journal_locations: Vec<WindowsJournalLocation>,
    directory_observations: Vec<MutationObservation>,
}
impl SelectedWindowsJob {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        roots: WindowsMutationStorage,
        plan: &JobPlanV2,
        metadata: &mut dyn MetadataProvider,
        recognition: &mut dyn RecognitionProvider,
        resources: &Resources,
        scope: &ExecutionScope,
    ) -> Result<Self> {
        if roots.domain() != WindowsMutationDomain::SelectedRoots {
            return Err(E::Scope);
        }
        supported_plan(plan)?;
        if roots.selected_source_reference().map_err(|_| E::Scope)? != plan.request.source
            || Some(roots.selected_target_reference().map_err(|_| E::Scope)?) != plan.request.target
        {
            return Err(E::Scope);
        }
        let source = roots.selected_source_root().map_err(|_| E::Scope)?;
        let target = roots.selected_target_root().map_err(|_| E::Scope)?;
        let request_source = roots
            .parent_reference(&plan.request.source)
            .map_err(|_| E::Scope)?;
        let request_target = roots
            .parent_reference(plan.request.target.as_ref().ok_or(E::Scope)?)
            .map_err(|_| E::Scope)?;
        if source.identity() != &scope.source_root
            || target.identity() != &scope.target_root
            || source.identity() != request_source.identity()
            || target.identity() != request_target.identity()
        {
            return Err(E::Scope);
        }
        let copy = plan.request.settings.copy_mode;
        if !copy
            && !matches!((source.identity(), target.identity()), (ObjectIdentity::Windows {volume:a,..},ObjectIdentity::Windows {volume:b,..}) if a==b)
        {
            return Err(E::Unsupported);
        }
        let source_path = path(&plan.request.source)?;
        let target_path = path(plan.request.target.as_ref().ok_or(E::Scope)?)?;
        for observation in &plan.observations {
            let p = path(&observation.reference)?;
            if !p.starts_with(&source_path) && !p.starts_with(&target_path) {
                return Err(E::Scope);
            }
        }
        let mut reads = OwnedCopyReadStorage::new(
            &roots,
            &plan.request.source,
            plan.request.target.as_ref().ok_or(E::Scope)?,
        )?;
        let domain = if copy {
            ExecutionDomain::SelectedWindowsCopy
        } else {
            ExecutionDomain::SelectedWindowsRename
        };
        let binding = validate_execution_domain(
            plan,
            &mut reads,
            metadata,
            recognition,
            resources,
            scope,
            domain,
        )?;
        if !binding.data.valid_domain() {
            return Err(E::InvalidBinding);
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| E::Io)?
            .as_nanos();
        let job_id = format!(
            "sorter-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        Ok(Self {
            storage: if copy {
                Storage::Copy(roots)
            } else {
                Storage::Rename(WindowsRenameStorage::new(roots))
            },
            binding,
            job_id,
            attempted: false,
            created_directories: vec![],
            journal_locations: vec![],
            directory_observations: vec![],
        })
    }
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    pub fn binding_sha256(&self) -> Result<String> {
        self.binding.sha256()
    }
    pub fn directory_observations(&self) -> &[MutationObservation] {
        &self.directory_observations
    }
    pub fn journal_locations(&self) -> &[WindowsJournalLocation] {
        &self.journal_locations
    }
    /// Reconcile this job's exact retained journal locations without granting
    /// writable reopen or exposing the underlying mutation adapter to the host.
    pub fn inspect_recovery(&self, scope: &RecoveryScope) -> Result<SelectedRecoveryReport> {
        let directory = self.journal_locations.first().ok_or(E::Unsupported)?;
        let roots = self.storage.roots();
        let directories = roots
            .read_selected_journal(directory.name(), directory.identity())
            .map_err(|_| E::Io)?;
        let transfers = self
            .journal_locations
            .get(1)
            .map(|location| {
                roots
                    .read_selected_journal(location.name(), location.identity())
                    .map_err(|_| E::Io)
            })
            .transpose()?;
        inspect_recovery(
            roots,
            scope,
            &self.job_id,
            &directories,
            transfers.as_deref(),
        )
    }
    /// No automatic replay. A second call fails even when the first call lost an
    /// acknowledgment. Caller receives the journal names before effects begin.
    pub fn run(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(TransferProgress),
    ) -> Result<JobResult> {
        if self.attempted {
            return Err(E::NeedsRecovery);
        }
        self.attempted = true;
        if cancel.load(Ordering::Acquire) {
            return Err(E::Cancelled);
        }
        let parent = self
            .storage
            .roots()
            .selected_journal_root()
            .map_err(|_| E::Scope)?;
        let file = self
            .storage
            .roots()
            .create_journal(
                &parent,
                &name(&format!("{}.directories.jsonl", self.job_id)),
            )
            .map_err(|_| E::Io)?;
        self.journal_locations.push(file.location());
        let mut directories =
            DirectoryJournal::new(WindowsJournalStore::new(file), &self.binding, &self.job_id)?;
        self.prepare_directories(&mut directories, cancel)?;
        if cancel.load(Ordering::Acquire) {
            return Err(E::Cancelled);
        }
        let file = self
            .storage
            .roots()
            .create_journal(&parent, &name(&format!("{}.transfers.jsonl", self.job_id)))
            .map_err(|_| E::Io)?;
        self.journal_locations.push(file.location());
        let mut sink = WindowsJournalStore::new(file);
        let mut journal = PhysicalJournalV2::create(&self.binding, &self.job_id, &mut sink)?;
        match &mut self.storage {
            Storage::Copy(storage) => run_job(
                &self.binding,
                storage,
                &mut journal,
                &mut sink,
                cancel,
                progress,
            ),
            Storage::Rename(storage) => run_job(
                &self.binding,
                storage,
                &mut journal,
                &mut sink,
                cancel,
                progress,
            ),
        }
    }
    fn prepare_directories(
        &mut self,
        journal: &mut DirectoryJournal,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let roots = self.storage.roots();
        let mut reads = OwnedCopyReadStorage::new(
            roots,
            &self.binding.data.source_root.reference,
            &self.binding.data.target_root.reference,
        )?;
        let mut seen = HashSet::new();
        for operation in self.binding.operations() {
            let mut directory = path(&operation.destination.root)?;
            let mut parent = roots
                .parent_reference(&operation.destination.root)
                .map_err(|_| E::Scope)?;
            for component in
                &operation.destination.components[..operation.destination.components.len() - 1]
            {
                if cancel.load(Ordering::Acquire) {
                    return Err(E::Cancelled);
                }
                directory.push(component);
                let r = reference(&directory);
                if seen.contains(&r) {
                    parent = roots.parent_reference(&r).map_err(|_| E::Stale)?;
                    continue;
                }
                match reads.stat(&r) {
                    Ok(observation) => {
                        if observation.kind != EntryKind::Directory {
                            return Err(E::Stale);
                        }
                        parent = roots.parent_reference(&r).map_err(|_| E::Stale)?;
                        if observation.identity.as_ref() != Some(parent.identity()) {
                            return Err(E::Stale);
                        }
                        self.created_directories.push(parent.clone());
                    }
                    Err(error) if error.kind == StorageErrorKind::NotFound => {
                        journal.append(DirectoryEvent::Prepared {
                            reference: r.clone(),
                            parent: parent.identity().clone(),
                        })?;
                        if cancel.load(Ordering::Acquire) {
                            return Err(E::Cancelled);
                        }
                        let created = match roots.create_destination_directory(
                            &parent,
                            &name(component),
                            cancel,
                        ) {
                            MutationOutcome::Applied(created) => created,
                            MutationOutcome::NoEffect(error) => {
                                journal.append(DirectoryEvent::Rejected {
                                    reference: r.clone(),
                                    detail: error.detail,
                                })?;
                                return Err(if error.kind == MutationErrorKind::Cancelled {
                                    E::Cancelled
                                } else {
                                    E::Io
                                });
                            }
                            MutationOutcome::PendingUnknown(error) => {
                                if let Some(observed) = error.observed {
                                    self.directory_observations.push(*observed);
                                }
                                return Err(E::NeedsRecovery);
                            }
                        };
                        journal.append(DirectoryEvent::Observed {
                            reference: r.clone(),
                            identity: created.identity().clone(),
                        })?;
                        self.created_directories.push(created.clone());
                        parent = created;
                    }
                    Err(_) => return Err(E::Stale),
                }
                seen.insert(r);
            }
        }
        journal.append(DirectoryEvent::Complete)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum DirectoryEvent {
    Header {
        schema: u16,
        job_id: String,
        binding_sha256: String,
        binding: BindingData,
        target_root: ObjectIdentity,
    },
    Prepared {
        reference: StorageRef,
        parent: ObjectIdentity,
    },
    Observed {
        reference: StorageRef,
        identity: ObjectIdentity,
    },
    Rejected {
        reference: StorageRef,
        detail: String,
    },
    Complete,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryRecord {
    sequence: u64,
    previous: String,
    event: DirectoryEvent,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryEnvelope {
    body: DirectoryRecord,
    sha256: String,
}
struct DirectoryJournal {
    sink: WindowsJournalStore,
    sequence: u64,
    previous: String,
    poisoned: bool,
}
impl DirectoryJournal {
    fn new(sink: WindowsJournalStore, binding: &ExecutionBinding, job_id: &str) -> Result<Self> {
        let mut value = Self {
            sink,
            sequence: 0,
            previous: String::new(),
            poisoned: false,
        };
        value.append(DirectoryEvent::Header {
            schema: 1,
            job_id: job_id.into(),
            binding_sha256: binding.sha256()?,
            binding: binding.data.clone(),
            target_root: binding.data.scope_evidence.target_root.clone(),
        })?;
        Ok(value)
    }
    fn append(&mut self, event: DirectoryEvent) -> Result<()> {
        if self.poisoned {
            return Err(E::NeedsRecovery);
        }
        self.poisoned = true;
        let body = DirectoryRecord {
            sequence: self.sequence,
            previous: self.previous.clone(),
            event,
        };
        let sha256 = crate::hash(&body)?;
        let mut bytes = serde_json::to_vec(&DirectoryEnvelope {
            body,
            sha256: sha256.clone(),
        })
        .map_err(|_| E::Io)?;
        bytes.push(b'\n');
        self.sink
            .append(&bytes)
            .and_then(|()| self.sink.sync())
            .map_err(|_| E::NeedsRecovery)?;
        self.sequence += 1;
        self.previous = sha256;
        self.poisoned = false;
        Ok(())
    }
}

/// Read-only directory setup evidence. `observed` records a returned identity;
/// it does not claim a power-loss-proof namespace or authorize replay/removal.
#[derive(Clone, Debug)]
pub struct DirectoryRecovery {
    pub observed: Vec<(StorageRef, ObjectIdentity)>,
    pub pending: Vec<StorageRef>,
    pub rejected: Vec<StorageRef>,
    pub complete: bool,
    pub incomplete_tail: bool,
}
#[derive(Clone, Debug)]
pub struct SelectedRecoveryReport {
    pub directories: DirectoryRecovery,
    pub transfers: Option<crate::recovery::RecoveryReport>,
}
/// Caller reads identity-checked journal files using the explicitly selected
/// journal-root capability. This function performs no writes or automatic replay.
pub fn inspect_recovery(
    roots: &WindowsMutationStorage,
    scope: &RecoveryScope,
    job_id: &str,
    directories: &[u8],
    transfers: Option<&[u8]>,
) -> Result<SelectedRecoveryReport> {
    if roots.domain() != WindowsMutationDomain::SelectedRoots {
        return Err(E::Scope);
    }
    let mut report = DirectoryRecovery {
        observed: vec![],
        pending: vec![],
        rejected: vec![],
        complete: false,
        incomplete_tail: false,
    };
    let mut offset = 0;
    let mut sequence = 0;
    let mut previous = String::new();
    let mut binding = None;
    let mut pending: Option<StorageRef> = None;
    while offset < directories.len() {
        let Some(end) = directories[offset..].iter().position(|b| *b == b'\n') else {
            report.incomplete_tail = true;
            break;
        };
        let envelope: DirectoryEnvelope =
            serde_json::from_slice(&directories[offset..offset + end])
                .map_err(|_| E::CorruptJournal)?;
        offset += end + 1;
        if envelope.body.sequence != sequence
            || envelope.body.previous != previous
            || crate::hash(&envelope.body)? != envelope.sha256
            || report.complete
        {
            return Err(E::CorruptJournal);
        }
        sequence += 1;
        previous = envelope.sha256;
        match envelope.body.event {
            DirectoryEvent::Header {
                schema,
                job_id: record_job,
                binding_sha256,
                binding: header,
                target_root,
            } => {
                if sequence != 1
                    || schema != 1
                    || record_job != job_id
                    || crate::hash(&header)? != binding_sha256
                    || binding_sha256 != scope.inspected_binding_sha256
                    || header.scope_sha256 != scope_digest(&scope.original_scope)?
                    || header.scope_evidence.resources != scope.original_scope.resources
                    || header.plan_sha256 != scope.original_scope.inspected_plan
                    || !header.valid_domain()
                    || !(header.selected_windows_copy || header.selected_windows_rename)
                    || target_root != scope.original_scope.target_root
                    || header.source_root.identity.as_ref()
                        != Some(&scope.original_scope.source_root)
                    || header.target_root.identity.as_ref()
                        != Some(&scope.original_scope.target_root)
                {
                    return Err(E::Scope);
                }
                if roots.selected_source_reference().map_err(|_| E::Scope)?
                    != header.source_root.reference
                    || roots.selected_target_reference().map_err(|_| E::Scope)?
                        != header.target_root.reference
                    || roots
                        .selected_source_root()
                        .map_err(|_| E::Scope)?
                        .identity()
                        != &scope.original_scope.source_root
                    || roots
                        .selected_target_root()
                        .map_err(|_| E::Scope)?
                        .identity()
                        != &scope.original_scope.target_root
                {
                    return Err(E::Scope);
                }
                binding = Some(header);
            }
            DirectoryEvent::Prepared { reference, parent } => {
                let header = binding.as_ref().ok_or(E::CorruptJournal)?;
                if pending.is_some() || !matches!(parent, ObjectIdentity::Windows { .. }) {
                    return Err(E::CorruptJournal);
                }
                let intended = header.operations.iter().any(|op| {
                    let Ok(mut p) = path(&op.destination.root) else {
                        return false;
                    };
                    op.destination.components[..op.destination.components.len().saturating_sub(1)]
                        .iter()
                        .any(|c| {
                            p.push(c);
                            self::reference(&p) == reference
                        })
                });
                if !intended
                    || report.observed.iter().any(|(r, _)| r == &reference)
                    || report.rejected.contains(&reference)
                {
                    return Err(E::CorruptJournal);
                }
                pending = Some(reference);
            }
            DirectoryEvent::Observed {
                reference,
                identity,
            } => {
                if pending.as_ref() != Some(&reference)
                    || !matches!(identity, ObjectIdentity::Windows { .. })
                {
                    return Err(E::CorruptJournal);
                }
                pending = None;
                report.observed.push((reference, identity));
            }
            DirectoryEvent::Rejected { reference, .. } => {
                if pending.as_ref() != Some(&reference) {
                    return Err(E::CorruptJournal);
                }
                pending = None;
                report.rejected.push(reference);
            }
            DirectoryEvent::Complete => {
                if binding.is_none() || pending.is_some() || !report.rejected.is_empty() {
                    return Err(E::CorruptJournal);
                }
                report.complete = true;
            }
        }
    }
    let binding = binding.ok_or(E::CorruptJournal)?;
    if let Some(reference) = pending {
        report.pending.push(reference);
    }
    let transfer_report = if let Some(bytes) = transfers {
        if !report.complete || report.incomplete_tail {
            return Err(E::CorruptJournal);
        }
        let journal = crate::windows_copy::open_owned_recovery_domain(
            roots,
            scope,
            job_id,
            bytes,
            binding.domain(),
        )?;
        let mut reads = crate::windows_copy::WindowsCopyRecovery::new_domain(
            roots,
            journal.binding(),
            binding.domain(),
        )?;
        Some(crate::recovery::recover_job(
            journal.binding(),
            &journal,
            &mut reads,
        )?)
    } else {
        None
    };
    Ok(SelectedRecoveryReport {
        directories: report,
        transfers: transfer_report,
    })
}
