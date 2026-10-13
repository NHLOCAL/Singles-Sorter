//! Host-selected local NTFS jobs. Selection and content inspection are live host
//! authority; serialized plans and journal headers are evidence only.
use crate::inspection_control::{InspectionControl, any as controlled_any, checkpoint, probe};
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
    CopiedMove(copied_move::CopiedMoveStorage),
}
impl Storage {
    fn roots(&self) -> &WindowsMutationStorage {
        match self {
            Self::Copy(s) => s,
            Self::Rename(s) => s.owned(),
            Self::CopiedMove(s) => &s.roots,
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
    created_directory_receipts: Vec<(StorageRef, ObjectIdentity)>,
}
impl SelectedWindowsJob {
    pub fn execution_decisions(&self) -> Option<&crate::execution_decisions::ExecutionDecisions> {
        self.binding.execution_decisions()
    }
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
            created_directory_receipts: vec![],
        })
    }
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    pub fn binding_sha256(&self) -> Result<String> {
        self.binding.sha256()
    }
    /// Only newly created directories whose exact identity was journaled and synced.
    /// References retain the originally selected root spelling.
    pub fn created_directory_receipts(&self) -> &[(StorageRef, ObjectIdentity)] {
        &self.created_directory_receipts
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
    /// Reconcile after Apply using the host's existing cancellation owner. An
    /// interrupted check returns no all-clear report and never resumes effects.
    pub fn inspect_recovery_controlled(
        &self,
        scope: &RecoveryScope,
        control: &InspectionControl,
    ) -> Result<SelectedRecoveryReport> {
        control.stage("journals")?;
        let directory = self.journal_locations.first().ok_or(E::Unsupported)?;
        let roots = self.storage.roots();
        let mut observe = |event| crate::windows_copy::inspection_event(control, event);
        let directories = roots
            .read_selected_journal_observed(
                directory.name(),
                directory.identity(),
                control.cancellation_flag(),
                &mut observe,
            )
            .map_err(crate::windows_copy::inspection_error)?;
        let transfers = self
            .journal_locations
            .get(1)
            .map(|location| {
                roots
                    .read_selected_journal_observed(
                        location.name(),
                        location.identity(),
                        control.cancellation_flag(),
                        &mut observe,
                    )
                    .map_err(crate::windows_copy::inspection_error)
            })
            .transpose()?;
        inspect_recovery_controlled(
            roots,
            scope,
            &self.job_id,
            &directories,
            transfers.as_deref(),
            control,
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
            Storage::CopiedMove(storage) => run_job(
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
                        self.created_directory_receipts
                            .push((r.clone(), created.identity().clone()));
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
        binding: Box<BindingData>,
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
    bytes_written: usize,
    limit: usize,
    sink: WindowsJournalStore,
    sequence: u64,
    previous: String,
    poisoned: bool,
}
impl DirectoryJournal {
    fn header(binding: &ExecutionBinding, job_id: &str) -> Result<DirectoryEvent> {
        Ok(DirectoryEvent::Header {
            schema: 1,
            job_id: job_id.into(),
            binding_sha256: binding.sha256()?,
            binding: Box::new(binding.data.clone()),
            target_root: binding.data.scope_evidence.target_root.clone(),
        })
    }
    fn validate_controlled_creation(binding: &ExecutionBinding, job_id: &str) -> Result<()> {
        let body = DirectoryRecord {
            sequence: 0,
            previous: String::new(),
            event: Self::header(binding, job_id)?,
        };
        let sha256 = crate::hash(&body)?;
        let bytes = serde_json::to_vec(&DirectoryEnvelope { body, sha256 }).map_err(|_| E::Io)?;
        let length = bytes.len().checked_add(1).ok_or(E::Unsupported)?;
        let components = binding.operations().iter().try_fold(0usize, |n, op| {
            n.checked_add(op.destination.components.len())
                .ok_or(E::Unsupported)
        })?;
        let records = components
            .checked_mul(2)
            .and_then(|n| n.checked_add(2))
            .ok_or(E::Unsupported)?;
        crate::inspection_control::check_recorded_write(length, length, records)
    }
    fn new(sink: WindowsJournalStore, binding: &ExecutionBinding, job_id: &str) -> Result<Self> {
        let mut value = Self {
            bytes_written: 0,
            limit: if binding.data.copied_move_sources.is_some() {
                64 * 1024 * 1024
            } else {
                usize::MAX
            },
            sink,
            sequence: 0,
            previous: String::new(),
            poisoned: false,
        };
        if binding.data.copied_move_sources.is_some() {
            Self::validate_controlled_creation(binding, job_id)?;
        }
        value.append(Self::header(binding, job_id)?)?;
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
        let total = self
            .bytes_written
            .checked_add(bytes.len())
            .filter(|total| *total <= self.limit)
            .ok_or(E::NeedsRecovery)?;
        if self.limit != usize::MAX {
            crate::inspection_control::check_recorded_write(
                bytes.len(),
                total,
                usize::try_from(self.sequence)
                    .ok()
                    .and_then(|n| n.checked_add(1))
                    .ok_or(E::NeedsRecovery)?,
            )
            .map_err(|_| E::NeedsRecovery)?;
        }
        self.sink
            .append(&bytes)
            .and_then(|()| self.sink.sync())
            .map_err(|_| E::NeedsRecovery)?;
        self.bytes_written = total;
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
    /// Recorded created-directory identities not freshly proven at the same path.
    pub unverified: Vec<StorageRef>,
}
#[derive(Clone, Debug)]
pub struct SelectedRecoveryReport {
    pub directories: DirectoryRecovery,
    pub transfers: Option<crate::recovery::RecoveryReport>,
    pub observation_limit_reached: bool,
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
    inspect_recovery_inner(
        Some(roots),
        scope,
        job_id,
        directories,
        transfers,
        RecoveryReadOptions::default(),
    )
}
pub fn inspect_recovery_controlled(
    roots: &WindowsMutationStorage,
    scope: &RecoveryScope,
    job_id: &str,
    directories: &[u8],
    transfers: Option<&[u8]>,
    control: &InspectionControl,
) -> Result<SelectedRecoveryReport> {
    inspect_recovery_inner(
        Some(roots),
        scope,
        job_id,
        directories,
        transfers,
        RecoveryReadOptions {
            control: Some(control),
            ..RecoveryReadOptions::default()
        },
    )
}
#[derive(Default)]
struct RecoveryReadOptions<'a> {
    control: Option<&'a InspectionControl<'a>>,
    cursor: usize,
    restart: bool,
    bounded: bool,
}
fn inspect_recovery_inner(
    roots: Option<&WindowsMutationStorage>,
    scope: &RecoveryScope,
    job_id: &str,
    directories: &[u8],
    transfers: Option<&[u8]>,
    options: RecoveryReadOptions<'_>,
) -> Result<SelectedRecoveryReport> {
    let control = options.control;
    crate::inspection_control::check_recorded_input(directories, control)?;
    if let Some(control) = control {
        control.stage("directory_journal")?;
    }
    if roots.is_some_and(|roots| roots.domain() != WindowsMutationDomain::SelectedRoots) {
        return Err(E::Scope);
    }
    let mut report = DirectoryRecovery {
        observed: vec![],
        pending: vec![],
        rejected: vec![],
        complete: false,
        incomplete_tail: false,
        unverified: vec![],
    };
    let mut offset = 0;
    let mut sequence = 0;
    let mut previous = String::new();
    let mut binding = None;
    let mut pending: Option<StorageRef> = None;
    while offset < directories.len() {
        checkpoint(control)?;
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
        if let Some(control) = control {
            control.record()?;
        }
        previous = envelope.sha256;
        match envelope.body.event {
            DirectoryEvent::Header {
                schema,
                job_id: record_job,
                binding_sha256,
                binding: header,
                target_root,
            } => {
                if control.is_some()
                    && header.operations.len() > crate::inspection_control::MAX_OPERATIONS
                {
                    return Err(E::Unsupported);
                }
                checkpoint(control)?;
                if sequence != 1
                    || schema != 1
                    || record_job != job_id
                    || crate::hash(&header)? != binding_sha256
                    || binding_sha256 != scope.inspected_binding_sha256
                    || header.scope_sha256 != scope_digest(&scope.original_scope)?
                    || header.scope_evidence.resources != scope.original_scope.resources
                    || header.plan_sha256 != scope.original_scope.inspected_plan
                    || !header.valid_domain()
                    || !(header.selected_windows_copy
                        || header.selected_windows_rename
                        || header.copied_move_sources.is_some())
                    || target_root != scope.original_scope.target_root
                    || header.source_root.identity.as_ref()
                        != Some(&scope.original_scope.source_root)
                    || header.target_root.identity.as_ref()
                        != Some(&scope.original_scope.target_root)
                {
                    return Err(E::Scope);
                }
                if let Some(roots) = roots
                    && (probe(control, || {
                        roots.selected_source_reference().map_err(|_| E::Scope)
                    })? != header.source_root.reference
                        || probe(control, || {
                            roots.selected_target_reference().map_err(|_| E::Scope)
                        })? != header.target_root.reference
                        || probe(control, || {
                            roots.selected_source_root().map_err(|_| E::Scope)
                        })?
                        .identity()
                            != &scope.original_scope.source_root
                        || probe(control, || {
                            roots.selected_target_root().map_err(|_| E::Scope)
                        })?
                        .identity()
                            != &scope.original_scope.target_root)
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
                let mut intended = false;
                for operation in &header.operations {
                    checkpoint(control)?;
                    let Ok(mut parent_path) = path(&operation.destination.root) else {
                        continue;
                    };
                    for component in &operation.destination.components
                        [..operation.destination.components.len().saturating_sub(1)]
                    {
                        checkpoint(control)?;
                        parent_path.push(component);
                        if self::reference(&parent_path) == reference {
                            intended = true;
                            break;
                        }
                    }
                    if intended {
                        break;
                    }
                }
                if !intended
                    || controlled_any(report.observed.iter(), control, |(prior, _)| {
                        prior == &reference
                    })?
                    || controlled_any(report.rejected.iter(), control, |prior| prior == &reference)?
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
                if control.is_some() && report.observed.len() >= MAX_STREAMING_DIRECTORIES {
                    return Err(E::Unsupported);
                }
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
    // Pin matched created directories across all later content observations.
    let mut held_directories = Vec::new();
    let mut observation_limit_reached = false;
    if let Some(roots) = roots
        && (control.is_some() || options.restart)
    {
        for (index, (reference, expected)) in report.observed.iter().enumerate() {
            checkpoint(control)?;
            if options.bounded && index >= MAX_RESTART_OBSERVATIONS {
                observation_limit_reached = true;
                report.unverified.push(reference.clone());
                continue;
            }
            match probe(control, || {
                roots.parent_reference(reference).map_err(|_| E::Stale)
            }) {
                Ok(parent) if parent.identity() == expected => held_directories.push(parent),
                Err(E::Cancelled) => return Err(E::Cancelled),
                _ if options.restart => report.unverified.push(reference.clone()),
                _ => return Err(E::Stale),
            }
        }
    }
    let transfer_report = if let Some(bytes) = transfers {
        if !report.complete || report.incomplete_tail {
            return Err(E::CorruptJournal);
        }
        let roots = roots.ok_or(E::Scope)?;
        if let Some(control) = control {
            control.stage("transfer_journal")?;
        }
        let journal = crate::windows_copy::open_owned_recovery_domain_with_control(
            roots,
            scope,
            job_id,
            bytes,
            binding.domain(),
            control,
        )?;
        let mut reads = crate::windows_copy::WindowsCopyRecovery::new_domain_with_control(
            roots,
            journal.binding(),
            binding.domain(),
            control,
        )?;
        if options.bounded {
            reads.limit_reads(
                MAX_RESTART_OBSERVATIONS.saturating_sub(report.observed.len()),
                MAX_RESTART_CONTENT_BYTES,
            )?;
        }
        let recovered = crate::recovery::recover_job_inner(
            journal.binding(),
            &journal,
            &mut reads,
            control,
            options.cursor,
        )?;
        observation_limit_reached |= reads.limit_reached();
        Some(recovered)
    } else {
        None
    };
    checkpoint(control)?;
    drop(held_directories);
    Ok(SelectedRecoveryReport {
        directories: report,
        transfers: transfer_report,
        observation_limit_reached,
    })
}

#[path = "windows_copied_move.rs"]
pub(crate) mod copied_move;
pub use copied_move::{CopiedMoveScope, prepare_copied_move_scope, supported_copied_move_plan};

/// Bounded input limit shared with the host's discovery/read path.
pub const MAX_SELECTED_JOURNAL_BYTES: usize = 64 * 1024 * 1024;
/// Tighter restart-only work limits: bound strict parser/event scans as well as
/// native file reads. Existing in-process mutation/reconciliation is unchanged.
pub const MAX_RESTART_JOURNAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_RESTART_RECORDS: usize = 2048;
const MAX_RESTART_OPERATIONS: usize = 128;
const MAX_RESTART_OBSERVATIONS: usize = 256;
pub const MAX_RESTART_CONTENT_BYTES: u64 = 64 * 1024 * 1024;
// These larger ceilings are reachable only through cancellable control paths.
pub const MAX_STREAMING_JOURNAL_BYTES: usize = 64 * 1024 * 1024;
const MAX_STREAMING_RECORD_BYTES: usize = 4 * 1024 * 1024;
const MAX_STREAMING_RECORDS: usize = 65_536;
const MAX_STREAMING_OPERATIONS: usize = 4_096;
const MAX_STREAMING_DIRECTORIES: usize = 8_192;
fn check_recorded_input(bytes: &[u8], control: Option<&InspectionControl>) -> Result<()> {
    let (max_bytes, max_records) = if control.is_some() {
        (MAX_STREAMING_JOURNAL_BYTES, MAX_STREAMING_RECORDS)
    } else {
        (MAX_RESTART_JOURNAL_BYTES, MAX_RESTART_RECORDS)
    };
    if bytes.len() > max_bytes {
        return Err(E::Unsupported);
    }
    let mut records = 0usize;
    let mut record_bytes = 0usize;
    for chunk in bytes.chunks(65_536) {
        checkpoint(control)?;
        for byte in chunk {
            record_bytes += 1;
            if record_bytes > MAX_STREAMING_RECORD_BYTES {
                return Err(E::Unsupported);
            }
            if *byte == b'\n' {
                records += 1;
                record_bytes = 0;
            }
            if records > max_records {
                return Err(E::Unsupported);
            }
        }
    }
    checkpoint(control)
}
/// Display/evidence only. This cannot execute, append, replay, or grant native
/// access; callers must obtain fresh root selections before reconciliation.
#[derive(Clone, Debug, Serialize)]
pub struct RecordedSelectedJob {
    pub job_id: String,
    pub source: StorageRef,
    pub source_identity: ObjectIdentity,
    pub target: StorageRef,
    pub target_identity: ObjectIdentity,
    pub binding_sha256: String,
    pub operations: Vec<RecordedSelectedOperation>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecordedSelectedOperation {
    pub id: usize,
    pub source: StorageRef,
    pub destination: DestinationEvidence,
}
fn recorded_header(bytes: &[u8]) -> Result<(RecordedSelectedJob, RecoveryScope, ExecutionDomain)> {
    recorded_header_with_control(bytes, None)
}
fn recorded_header_with_control(
    bytes: &[u8],
    control: Option<&InspectionControl>,
) -> Result<(RecordedSelectedJob, RecoveryScope, ExecutionDomain)> {
    check_recorded_input(bytes, control)?;
    let end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .ok_or(E::CorruptJournal)?;
    let envelope: DirectoryEnvelope =
        serde_json::from_slice(&bytes[..end]).map_err(|_| E::CorruptJournal)?;
    if envelope.body.sequence != 0
        || !envelope.body.previous.is_empty()
        || crate::hash(&envelope.body)? != envelope.sha256
    {
        return Err(E::CorruptJournal);
    }
    let DirectoryEvent::Header {
        schema,
        job_id,
        binding_sha256,
        binding,
        target_root,
    } = envelope.body.event
    else {
        return Err(E::CorruptJournal);
    };
    let max_operations = if control.is_some() {
        MAX_STREAMING_OPERATIONS
    } else {
        MAX_RESTART_OPERATIONS
    };
    if binding.operations.len() > max_operations {
        return Err(E::Unsupported);
    }
    if schema != 1
        || job_id.is_empty()
        || job_id.len() > 100
        || !job_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || crate::hash(&binding)? != binding_sha256
        || !binding.valid_domain()
        || !(binding.selected_windows_copy
            || binding.selected_windows_rename
            || binding.copied_move_sources.is_some())
    {
        return Err(E::CorruptJournal);
    }
    let evidence = &binding.scope_evidence;
    if binding.source_root.identity.as_ref() != Some(&evidence.source_root)
        || binding.target_root.identity.as_ref() != Some(&evidence.target_root)
        || target_root != evidence.target_root
        || binding.plan_sha256 != evidence.inspected_plan
        || binding.resources != evidence.resources
    {
        return Err(E::CorruptJournal);
    }
    let source_path = path(&binding.source_root.reference)?;
    let target_path = path(&binding.target_root.reference)?;
    if !source_path.is_absolute()
        || !target_path.is_absolute()
        || source_path.starts_with(&target_path)
        || target_path.starts_with(&source_path)
    {
        return Err(E::Scope);
    }
    for (index, op) in binding.operations.iter().enumerate() {
        checkpoint(control)?;
        if op.id != index
            || op.depends_on.iter().any(|d| *d >= index)
            || op.destination.components.is_empty()
            || op.destination.components.iter().any(|c| {
                c.is_empty() || c == "." || c == ".." || c.contains(['/', '\\', ':', '\0'])
            })
            || !path(&op.source.observation.reference)?.starts_with(&source_path)
            || !evidence.inspected_sources.contains(&op.source)
        {
            return Err(E::CorruptJournal);
        }
    }
    let scope = ExecutionScope {
        inspected_plan: evidence.inspected_plan.clone(),
        source_root: evidence.source_root.clone(),
        target_root: evidence.target_root.clone(),
        inspected_sources: evidence.inspected_sources.clone(),
        resources: evidence.resources.clone(),
        allowed_modes: evidence.allowed_modes.clone(),
    };
    if scope_digest(&scope)? != binding.scope_sha256 {
        return Err(E::CorruptJournal);
    }
    let domain = binding.domain();
    let operations = binding
        .operations
        .iter()
        .map(|op| RecordedSelectedOperation {
            id: op.id,
            source: op.source.observation.reference.clone(),
            destination: op.destination.clone(),
        })
        .collect();
    let recorded = RecordedSelectedJob {
        job_id,
        source: binding.source_root.reference.clone(),
        source_identity: evidence.source_root.clone(),
        target: binding.target_root.reference.clone(),
        target_identity: evidence.target_root.clone(),
        binding_sha256: binding_sha256.clone(),
        operations,
    };
    Ok((
        recorded,
        RecoveryScope {
            original_scope: scope,
            inspected_binding_sha256: binding_sha256,
        },
        domain,
    ))
}
/// Parse bounded untrusted journal bytes for display; never return authority.
/// The full directory hash chain and phase sequence are validated, including any
/// preserved incomplete tail. Physical journal reconciliation is a separate step.
pub fn inspect_recorded_selected_job(directories: &[u8]) -> Result<RecordedSelectedJob> {
    let (recorded, scope, _) = recorded_header(directories)?;
    inspect_recovery_inner(
        None,
        &scope,
        &recorded.job_id,
        directories,
        None,
        RecoveryReadOptions::default(),
    )?;
    Ok(recorded)
}
/// Reconstruct only read-only recovery scope after fresh explicitly selected
/// root grants match recorded roles, lossless paths and full identities exactly.
pub fn recover_selected_journals(
    roots: &WindowsMutationStorage,
    directories: &[u8],
    transfers: Option<&[u8]>,
) -> Result<SelectedRecoveryReport> {
    if let Some(bytes) = transfers {
        check_recorded_input(bytes, None)?;
    }
    let (recorded, scope, domain) = recorded_header(directories)?;
    // Copied Move full-stream verification always requires a live control.
    if domain == ExecutionDomain::SelectedWindowsCopiedMove {
        return Err(E::Unsupported);
    }
    inspect_recovery_inner(
        Some(roots),
        &scope,
        &recorded.job_id,
        directories,
        transfers,
        RecoveryReadOptions {
            restart: true,
            bounded: true,
            ..RecoveryReadOptions::default()
        },
    )
}

/// Current observations across a cancellable inspection interval, never an
/// atomic library snapshot. A resume cursor does not reuse old hash evidence.
pub fn recover_selected_journals_controlled(
    roots: &WindowsMutationStorage,
    directories: &[u8],
    transfers: Option<&[u8]>,
    control: &InspectionControl,
    start_operation: usize,
) -> Result<SelectedRecoveryReport> {
    control.stage("journals")?;
    if let Some(bytes) = transfers {
        check_recorded_input(bytes, Some(control))?;
    }
    let (recorded, scope, _) = recorded_header_with_control(directories, Some(control))?;
    inspect_recovery_inner(
        Some(roots),
        &scope,
        &recorded.job_id,
        directories,
        transfers,
        RecoveryReadOptions {
            control: Some(control),
            cursor: start_operation,
            restart: true,
            bounded: false,
        },
    )
}

/// Historical completion only, never proof of current filesystem state.
#[derive(Clone, Debug, Serialize)]
pub struct RecordedSelectedSummary {
    pub job: RecordedSelectedJob,
    pub recorded_complete: bool,
}
pub fn summarize_recorded_selected_job(
    directories: &[u8],
    transfers: Option<&[u8]>,
) -> Result<RecordedSelectedSummary> {
    summarize_recorded_selected_job_with_control(directories, transfers, None)
}
pub fn summarize_recorded_selected_job_with_control(
    directories: &[u8],
    transfers: Option<&[u8]>,
    control: Option<&InspectionControl>,
) -> Result<RecordedSelectedSummary> {
    if let Some(control) = control {
        control.stage("journals")?;
    }
    let (job, scope, domain) = recorded_header_with_control(directories, control)?;
    let report = inspect_recovery_inner(
        None,
        &scope,
        &job.job_id,
        directories,
        None,
        RecoveryReadOptions {
            control,
            ..RecoveryReadOptions::default()
        },
    )?;
    let mut recorded_complete = false;
    if let Some(bytes) = transfers {
        check_recorded_input(bytes, control)?;
        if !report.directories.complete || report.directories.incomplete_tail {
            return Err(E::CorruptJournal);
        }
        let journal = PhysicalJournalV2::open_for_recovery_domain_with_control(
            &scope,
            &job.job_id,
            bytes,
            domain,
            control,
        )?;
        let mut phases = std::collections::HashMap::new();
        for event in journal.events() {
            checkpoint(control)?;
            phases.insert(event.operation, event.phase);
        }
        // A copied-Move descriptor never certifies a retired cohort. Fresh
        // granted-root full-stream reconciliation is required even for a final log.
        recorded_complete = domain != ExecutionDomain::SelectedWindowsCopiedMove
            && !journal.needs_recovery()
            && journal.incomplete_tail().is_empty()
            && !job.operations.is_empty()
            && !controlled_any(job.operations.iter(), control, |op| {
                phases.get(&op.id) != Some(&crate::physical_journal::Phase::Committed)
            })?;
    }
    checkpoint(control)?;
    Ok(RecordedSelectedSummary {
        job,
        recorded_complete,
    })
}

#[cfg(test)]
mod controlled_writer_boundary_tests {
    use super::*;
    #[test]
    fn directory_exact_envelope_and_newline_are_admitted_before_creation() {
        let value: serde_json::Value = serde_json::from_slice(
            include_bytes!("../tests/fixtures/physical-copy-v2-before-metadata.jsonl")
                .split(|b| *b == b'\n')
                .next()
                .unwrap(),
        )
        .unwrap();
        let mut binding = ExecutionBinding {
            execution_decisions: None,
            data: serde_json::from_value(value["body"]["binding"].clone()).unwrap(),
            recovery_only: false,
        };
        binding.data.resources.resource_ids.push(String::new());
        let encoded = |binding: &ExecutionBinding| {
            let body = DirectoryRecord {
                sequence: 0,
                previous: String::new(),
                event: DirectoryJournal::header(binding, "boundary").unwrap(),
            };
            let sha256 = crate::hash(&body).unwrap();
            serde_json::to_vec(&DirectoryEnvelope { body, sha256 })
                .unwrap()
                .len()
                + 1
        };
        let padding = crate::inspection_control::MAX_RECORD_BYTES - encoded(&binding);
        *binding.data.resources.resource_ids.last_mut().unwrap() = "x".repeat(padding);
        assert_eq!(
            encoded(&binding),
            crate::inspection_control::MAX_RECORD_BYTES
        );
        assert_eq!(
            DirectoryJournal::validate_controlled_creation(&binding, "boundary"),
            Ok(())
        );
        assert_eq!(
            DirectoryJournal::validate_controlled_creation(&binding, "boundaryx"),
            Err(E::Unsupported)
        );
    }
}
