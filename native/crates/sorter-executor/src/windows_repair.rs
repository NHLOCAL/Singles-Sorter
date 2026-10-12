//! One-shot selected-root FixNames. Journal evidence never grants mutation access.
//! Filename renames precede metadata work, as in the released policy. Originals
//! are retained as same-object backups; no replay, unlink or cleanup is exposed.
use crate::{
    ExecutionError as E, Result,
    binding::{Resources, SourceEvidence},
    physical_journal::JournalStore,
    inspection_control::InspectionControl,
    windows_copy::WindowsPlanningStorage,
    windows_journal::WindowsJournalStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sorter_core::{
    discovery::*,
    fix_plan::{FieldRepair, FixBlocker},
    job_plan::*,
    metadata::{LegacyField, MetadataProvider},
};
use sorter_io::{
    desktop::windows_mutation::*, staged_repair::StagedRepairPlan, staged_tags::TagWriteError,
};
use sorter_storage_contracts::{
    ContentDigest, ExpectedSource, MutationError, MutationErrorKind, MutationName,
    MutationObservation, MutationOutcome,
};
use std::{
    collections::HashSet,
    ffi::OsString,
    io::Read,
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

const MAX_JOURNAL: usize = 64 * 1024 * 1024;
fn path(reference: &StorageRef) -> Result<PathBuf> {
    let StorageRef::WindowsPath { utf16 } = reference else {
        return Err(E::Scope);
    };
    if utf16.contains(&0)
        || utf16
            .split(|c| *c == 47 || *c == 92)
            .any(|p| p == [46] || p == [46, 46])
    {
        return Err(E::Scope);
    }
    Ok(OsString::from_wide(utf16).into())
}
// GetFinalPathNameByHandleW emits the local DOS verbatim prefix while host
// selections may use its ordinary spelling. Only this documented drive prefix
// is aligned; no dot traversal, case folding, UNC or device alias is introduced.
fn comparable_path(reference: &StorageRef) -> Result<PathBuf> {
    let path = path(reference)?;
    let mut units = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if units.len() >= 7
        && units[..4] == [92, 92, 63, 92]
        && ((65..=90).contains(&units[4]) || (97..=122).contains(&units[4]))
        && units[5] == 58
        && units[6] == 92
    {
        units.drain(..4);
    }
    if units.len() >= 2 && units[1] == 58 && units[0] >= 97 && units[0] <= 122 {
        units[0] -= 32;
    }
    Ok(OsString::from_wide(&units).into())
}
fn aligned_read_reference(root: &StorageRef, reference: &StorageRef) -> Result<StorageRef> {
    let candidate = comparable_path(reference)?;
    let root_compare = comparable_path(root)?;
    let relative = candidate.strip_prefix(root_compare).map_err(|_| E::Scope)?;
    let aligned = path(root)?.join(relative);
    Ok(StorageRef::WindowsPath {
        utf16: aligned.as_os_str().encode_wide().collect(),
    })
}
// An alternate root spelling is accepted only when it came from the live
// identity-checked root handle. Child components are never normalized or aliased.
fn aligned_recovery_reference(
    root: &StorageRef,
    observed_root: Option<&StorageRef>,
    reference: &StorageRef,
) -> Result<StorageRef> {
    if let Ok(reference) = aligned_read_reference(root, reference) {
        return Ok(reference);
    }
    let observed_root = observed_root.ok_or(E::Scope)?;
    let candidate = comparable_path(reference)?;
    let observed_root = comparable_path(observed_root)?;
    let relative = candidate
        .strip_prefix(observed_root)
        .map_err(|_| E::Scope)?;
    let aligned = path(root)?.join(relative);
    Ok(StorageRef::WindowsPath {
        utf16: aligned.as_os_str().encode_wide().collect(),
    })
}
fn name(text: &str) -> MutationName {
    MutationName::Windows(text.encode_utf16().collect())
}
fn checked(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(E::Cancelled)
    } else {
        Ok(())
    }
}
fn checked_inspection(cancel: &AtomicBool, control: Option<&InspectionControl>) -> Result<()> {
    checked(cancel)?;
    control.map_or(Ok(()), InspectionControl::checkpoint)
}
fn inspection_probe<T>(cancel: &AtomicBool, control: Option<&InspectionControl>, action: impl FnOnce() -> Result<T>) -> Result<T> {
    checked_inspection(cancel, control)?;
    if let Some(control) = control { control.observation()?; }
    checked_inspection(cancel, control)?;
    let result = action();
    checked_inspection(cancel, control)?;
    result
}
const MAX_RESTART_RECORD: usize = 4 * 1024 * 1024;
const MAX_RESTART_RECORDS: usize = 65_536;
const MAX_RESTART_OPERATIONS: usize = 4_096;
fn check_inspection_input(bytes: &[u8], control: &InspectionControl) -> Result<()> {
    if bytes.len() > MAX_JOURNAL { return Err(E::Unsupported); }
    let mut records = 0usize;
    let mut length = 0usize;
    for chunk in bytes.chunks(65_536) {
        control.checkpoint()?;
        for byte in chunk {
            length += 1;
            if length > MAX_RESTART_RECORD { return Err(E::Unsupported); }
            if *byte == b'\n' { records += 1; length = 0; }
            if records > MAX_RESTART_RECORDS { return Err(E::Unsupported); }
        }
    }
    control.checkpoint()
}
fn digest_value(text: &str, length: u64) -> Result<ContentDigest> {
    if !valid_hash(text) {
        return Err(E::InvalidBinding);
    }
    let mut sha256 = [0; 32];
    for (i, b) in sha256.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).map_err(|_| E::InvalidBinding)?;
    }
    Ok(ContentDigest { sha256, length })
}
fn valid_hash(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
struct Control<'a>(&'a AtomicBool);
impl PlanningControl for Control<'_> {
    fn checkpoint(&self, _: PlanningBoundary) -> std::result::Result<(), PlanningAbort> {
        if self.0.load(Ordering::Acquire) {
            Err(PlanningAbort::cancelled())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairScope {
    inspected_plan: String,
    source_root: ObjectIdentity,
    sources: Vec<SourceEvidence>,
    resources: Resources,
}
impl RepairScope {
    pub fn source_root(&self) -> &ObjectIdentity {
        &self.source_root
    }
    pub fn resources(&self) -> &Resources {
        &self.resources
    }
}

pub fn supported_repair_plan(plan: &JobPlanV2) -> Result<()> {
    verify_job_plan_binding(plan).map_err(|_| E::InvalidBinding)?;
    if plan.request.mode != JobMode::FixNames
        || plan.request.target.is_some()
        || plan.operations.is_empty()
    {
        return Err(E::Unsupported);
    }
    if plan
        .blockers
        .iter()
        .any(|b| b != "PHYSICAL_APPLY_UNAVAILABLE" && b != "METADATA_ADAPTER_LIMITATION")
    {
        return Err(E::Unsupported);
    }
    for (index, op) in plan.operations.iter().enumerate() {
        let OperationKind::FixFile(fix) = &op.kind else {
            return Err(E::Unsupported);
        };
        if op.id != index
            || !op.stop_and_replan_on_effect_mismatch
            || op.depends_on.iter().any(|d| *d >= index)
            || fix.source.kind != EntryKind::File
            || fix.source.size.is_none()
            || fix.source.modified.is_none()
        {
            return Err(E::Unsupported);
        }
        if fix.blockers.iter().any(|b| {
            !matches!(
                b,
                FixBlocker::RenameMustSucceedAndRebindOriginal
                    | FixBlocker::Observation(ApplyBlocker::PhysicalApplyUnavailable)
            )
        }) {
            return Err(E::Unsupported);
        }
        let leaf = path(&fix.source.reference)?
            .file_name()
            .ok_or(E::Scope)?
            .encode_wide()
            .collect::<Vec<_>>();
        if String::from_utf16(&leaf).map_err(|_| E::Unsupported)? != fix.source.name {
            return Err(E::Unsupported);
        }
        let intended = fix.intended_name.as_deref().ok_or(E::Unsupported)?;
        if intended.is_empty()
            || intended == "."
            || intended == ".."
            || intended.contains(['/', '\\', ':', '\0'])
        {
            return Err(E::Unsupported);
        }
    }
    Ok(())
}
fn resources_match(plan: &JobPlanV2, resources: &Resources) -> bool {
    resources.resource_ids == plan.request.resource_ids
        && resources.adapter_versions == plan.request.adapter_versions
        && resources.ai_profile == plan.request.ai_profile
        && resources.resource_hashes.len() == resources.resource_ids.len()
        && resources.adapter_hashes.len() == resources.adapter_versions.len()
        && resources
            .resource_hashes
            .iter()
            .chain(&resources.adapter_hashes)
            .all(|h| valid_hash(h))
        && match &resources.ai_profile {
            AiProfile::Disabled => resources.model_hash.is_none(),
            AiProfile::Configured {
                available: true, ..
            } => resources.model_hash.as_deref().is_some_and(valid_hash),
            _ => false,
        }
}
pub fn prepare_repair_scope(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    resources: &Resources,
    cancel: &AtomicBool,
) -> Result<RepairScope> {
    supported_repair_plan(plan)?;
    if !resources_match(plan, resources) {
        return Err(E::InvalidBinding);
    }
    let root = storage.stat(&plan.request.source).map_err(|_| E::Stale)?;
    let root_path = path(&root.reference)?;
    let mut sources = Vec::new();
    let mut seen = HashSet::new();
    for op in &plan.operations {
        checked(cancel)?;
        let OperationKind::FixFile(fix) = &op.kind else {
            return Err(E::Unsupported);
        };
        let source = &fix.source;
        if !path(&source.reference)?.starts_with(&root_path)
            || !seen.insert(source.reference.clone())
            || storage.stat(&source.reference).map_err(|_| E::Stale)? != *source
        {
            return Err(E::Stale);
        }
        let mut input = storage.open_read(&source.reference).map_err(|_| E::Stale)?;
        let evidence = input.binding().ok_or(E::Stale)?;
        if Some(&evidence.identity) != source.identity.as_ref()
            || Some(evidence.size) != source.size
            || Some(&evidence.modified) != source.modified.as_ref()
        {
            return Err(E::Stale);
        }
        let mut hash = Sha256::new();
        let mut length = 0u64;
        let mut buffer = [0; 65536];
        loop {
            checked(cancel)?;
            let n = input.read(&mut buffer).map_err(|_| E::Stale)?;
            if n == 0 {
                break;
            }
            length = length.checked_add(n as u64).ok_or(E::Stale)?;
            if Some(length) > source.size {
                return Err(E::Stale);
            }
            hash.update(&buffer[..n]);
        }
        if Some(length) != source.size
            || storage.stat(&source.reference).map_err(|_| E::Stale)? != *source
        {
            return Err(E::Stale);
        }
        sources.push(SourceEvidence {
            observation: source.clone(),
            sha256: format!("{:x}", hash.finalize()),
        });
    }
    Ok(RepairScope {
        inspected_plan: plan.binding_sha256.clone(),
        source_root: root.identity.ok_or(E::Scope)?,
        sources,
        resources: resources.clone(),
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairStatus {
    Complete,
    UnsupportedTags,
    MetadataFailure,
    Failed,
    Cancelled,
    NeedsRecovery,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepairItem {
    pub operation: usize,
    pub original: StorageRef,
    pub status: RepairStatus,
    pub detail: String,
    pub renamed: bool,
    pub repaired: bool,
    pub unchanged: bool,
    pub processed: bool,
    pub current: Option<MutationObservation>,
    pub backup: Option<MutationObservation>,
    pub staged: Option<MutationObservation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairJobStatus {
    Complete,
    Partial,
    Cancelled,
    NeedsRecovery,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepairResult {
    pub status: RepairJobStatus,
    pub total_files: usize,
    pub processed_files: usize,
    pub renamed_files: usize,
    pub repaired_files: usize,
    pub unchanged_files: usize,
    pub unsupported_files: usize,
    pub failed_files: usize,
    pub retained_backup_bytes: u64,
    pub items: Vec<RepairItem>,
}
impl RepairResult {
    fn build(total: usize, items: Vec<RepairItem>, cancelled: bool) -> Self {
        let status = if items
            .iter()
            .any(|i| i.status == RepairStatus::NeedsRecovery)
        {
            RepairJobStatus::NeedsRecovery
        } else if cancelled || items.iter().any(|i| i.status == RepairStatus::Cancelled) {
            RepairJobStatus::Cancelled
        } else if items.len() == total && items.iter().all(|i| i.status == RepairStatus::Complete) {
            RepairJobStatus::Complete
        } else {
            RepairJobStatus::Partial
        };
        Self {
            status,
            total_files: total,
            processed_files: items.iter().filter(|i| i.processed).count(),
            renamed_files: items.iter().filter(|i| i.renamed).count(),
            repaired_files: items.iter().filter(|i| i.repaired).count(),
            unchanged_files: items.iter().filter(|i| i.unchanged).count(),
            unsupported_files: items
                .iter()
                .filter(|i| i.status == RepairStatus::UnsupportedTags)
                .count(),
            failed_files: items
                .iter()
                .filter(|i| {
                    matches!(
                        i.status,
                        RepairStatus::Failed | RepairStatus::MetadataFailure
                    )
                })
                .count(),
            retained_backup_bytes: items
                .iter()
                .filter_map(|i| i.backup.as_ref().and_then(|b| b.size))
                .fold(0u64, u64::saturating_add),
            items,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct RepairProgress {
    pub operation: usize,
    pub total_files: usize,
    pub phase: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepairField {
    field: LegacyField,
    original: String,
    replacement: String,
}
impl From<&FieldRepair> for RepairField {
    fn from(f: &FieldRepair) -> Self {
        Self {
            field: f.field,
            original: f.original.clone(),
            replacement: f.replacement.clone(),
        }
    }
}
impl RepairField {
    fn native(&self) -> FieldRepair {
        FieldRepair {
            field: self.field,
            original: self.original.clone(),
            replacement: self.replacement.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RepairOperation {
    id: usize,
    depends_on: Vec<usize>,
    source: SourceEvidence,
    intended_name: String,
    rename: bool,
    save: bool,
    repairs: Vec<RepairField>,
    metadata_failure: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TempPreparedV2 {
    source: SourceEvidence,
    replacement_length: u64,
    replacement_sha256: String,
    media_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Header {
    schema: u16,
    job_id: String,
    plan_sha256: String,
    source: StorageRef,
    source_identity: ObjectIdentity,
    resources: Resources,
    operations: Vec<RepairOperation>,
    /// Recovery-local evidence only; never read from or written to a journal.
    #[serde(skip)]
    observed_source: Option<StorageRef>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Phase {
    Header,
    OperationPrepared,
    RenamePrepared,
    RenameObserved,
    TempPrepared,
    TempObserved,
    StagedVerified,
    BackupPrepared,
    BackupObserved,
    PublishPrepared,
    PublishedVerified,
    Committed,
    Stopped,
    Finished,
}
#[derive(Serialize, Deserialize)]
struct Record {
    sequence: u64,
    previous: String,
    job_id: String,
    operation: Option<usize>,
    phase: Phase,
    payload: serde_json::Value,
}
#[derive(Serialize, Deserialize)]
struct Envelope {
    body: Record,
    sha256: String,
}
struct Journal {
    sink: WindowsJournalStore,
    job_id: String,
    sequence: u64,
    previous: String,
    poisoned: bool,
    bytes_written: usize,
}
fn journal_budget(used: usize, additional: usize, limit: usize) -> Option<usize> {
    used.checked_add(additional).filter(|total| *total <= limit)
}
impl Journal {
    fn new(sink: WindowsJournalStore, header: &Header) -> Result<Self> {
        let mut value = Self {
            sink,
            job_id: header.job_id.clone(),
            sequence: 0,
            previous: String::new(),
            poisoned: false,
            bytes_written: 0,
        };
        value.append(None, Phase::Header, header)?;
        Ok(value)
    }
    fn append(
        &mut self,
        operation: Option<usize>,
        phase: Phase,
        payload: &impl Serialize,
    ) -> Result<()> {
        if self.poisoned {
            return Err(E::NeedsRecovery);
        }
        self.poisoned = true;
        let body = Record {
            sequence: self.sequence,
            previous: self.previous.clone(),
            job_id: self.job_id.clone(),
            operation,
            phase,
            payload: serde_json::to_value(payload).map_err(|_| E::Io)?,
        };
        let sha256 = crate::hash(&body)?;
        let mut bytes = serde_json::to_vec(&Envelope {
            body,
            sha256: sha256.clone(),
        })
        .map_err(|_| E::Io)?;
        bytes.push(b'\n');
        self.bytes_written =
            journal_budget(self.bytes_written, bytes.len(), MAX_JOURNAL).ok_or(E::NeedsRecovery)?;
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

pub struct SelectedWindowsRepairJob {
    storage: WindowsRepairStorage,
    header: Header,
    attempted: bool,
    locations: Vec<WindowsJournalLocation>,
    retained: Vec<WindowsPublished>,
    temps: Vec<WindowsOwnedTemp>,
}
impl SelectedWindowsRepairJob {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        roots: WindowsMutationStorage,
        plan: &JobPlanV2,
        metadata: &mut dyn MetadataProvider,
        recognition: &mut dyn RecognitionProvider,
        resources: &Resources,
        scope: &RepairScope,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        checked(cancel)?;
        supported_repair_plan(plan)?;
        if roots.domain() != WindowsMutationDomain::SelectedRepair
            || roots.selected_repair_reference().map_err(|_| E::Scope)? != plan.request.source
            || roots
                .selected_repair_root()
                .map_err(|_| E::Scope)?
                .identity()
                != scope.source_root()
            || scope.inspected_plan != plan.binding_sha256
            || scope.resources() != resources
        {
            return Err(E::Scope);
        }
        let mut reads = WindowsPlanningStorage::for_repair(&roots)?;
        let fresh = plan_job_controlled(
            &plan.request,
            &mut reads,
            metadata,
            recognition,
            &Control(cancel),
        )
        .map_err(|_| {
            if cancel.load(Ordering::Acquire) {
                E::Cancelled
            } else {
                E::Stale
            }
        })?;
        if fresh.binding_sha256 != plan.binding_sha256
            || prepare_repair_scope(&fresh, &mut reads, resources, cancel)? != *scope
        {
            return Err(E::Stale);
        }
        let mut operations = Vec::new();
        for op in &plan.operations {
            let OperationKind::FixFile(fix) = &op.kind else {
                return Err(E::Unsupported);
            };
            let source = scope
                .sources
                .iter()
                .find(|s| s.observation == fix.source)
                .ok_or(E::Scope)?
                .clone();
            operations.push(RepairOperation {
                id: op.id,
                depends_on: op.depends_on.clone(),
                source,
                intended_name: fix.intended_name.clone().ok_or(E::Unsupported)?,
                rename: fix.rename_intended,
                save: fix.save_intended,
                repairs: fix.repairs.iter().map(RepairField::from).collect(),
                metadata_failure: fix.failure.as_ref().map(|f| format!("{f:?}")),
            });
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| E::Io)?
            .as_nanos();
        let job_id = format!(
            "repair-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let header = Header {
            schema: 2,
            job_id,
            plan_sha256: plan.binding_sha256.clone(),
            source: plan.request.source.clone(),
            source_identity: scope.source_root.clone(),
            resources: resources.clone(),
            operations,
            observed_source: None,
        };
        let header_record = Record {
            sequence: 0,
            previous: String::new(),
            job_id: header.job_id.clone(),
            operation: None,
            phase: Phase::Header,
            payload: serde_json::to_value(&header).map_err(|_| E::Io)?,
        };
        let header_bytes = serde_json::to_vec(&Envelope {
            body: header_record,
            sha256: "0".repeat(64),
        })
        .map_err(|_| E::Io)?
        .len();
        if journal_budget(0, header_bytes.saturating_add(1), MAX_JOURNAL).is_none() {
            return Err(E::Unsupported);
        }
        Ok(Self {
            storage: WindowsRepairStorage::new(roots).map_err(|_| E::Scope)?,
            header,
            attempted: false,
            locations: vec![],
            retained: vec![],
            temps: vec![],
        })
    }
    pub fn job_id(&self) -> &str {
        &self.header.job_id
    }
    pub fn journal_locations(&self) -> &[WindowsJournalLocation] {
        &self.locations
    }
    pub fn run(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(RepairProgress),
    ) -> Result<RepairResult> {
        if self.attempted {
            return Err(E::NeedsRecovery);
        }
        self.attempted = true;
        checked(cancel)?;
        let parent = self
            .storage
            .roots()
            .selected_journal_root()
            .map_err(|_| E::Scope)?;
        let file = self
            .storage
            .roots()
            .create_journal(&parent, &name(&format!("{}.repair.jsonl", self.job_id())))
            .map_err(|_| E::Io)?;
        self.locations.push(file.location());
        let mut journal = Journal::new(WindowsJournalStore::new(file), &self.header)?;
        let mut items = Vec::new();
        for operation in self.header.operations.clone() {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            progress(RepairProgress {
                operation: operation.id,
                total_files: self.header.operations.len(),
                phase: "prepared".into(),
            });
            let mut item = RepairItem {
                operation: operation.id,
                original: operation.source.observation.reference.clone(),
                status: RepairStatus::Failed,
                detail: String::new(),
                renamed: false,
                repaired: false,
                unchanged: false,
                processed: false,
                current: None,
                backup: None,
                staged: None,
            };
            let result = self.run_item(&operation, &mut item, &mut journal, cancel, progress);
            if let Err(error) = result {
                item.status = if error == E::Cancelled && item.backup.is_none() {
                    RepairStatus::Cancelled
                } else {
                    RepairStatus::NeedsRecovery
                };
                item.detail = format!("{error}");
                if !journal.poisoned {
                    journal.append(Some(operation.id), Phase::Stopped, &item)?;
                }
                items.push(item);
                break;
            }
            let stop = matches!(
                item.status,
                RepairStatus::Failed | RepairStatus::Cancelled | RepairStatus::NeedsRecovery
            );
            items.push(item);
            if stop {
                break;
            }
        }
        let result = RepairResult::build(
            self.header.operations.len(),
            items,
            cancel.load(Ordering::Acquire),
        );
        if !journal.poisoned {
            journal.append(None, Phase::Finished, &result)?;
        }
        Ok(result)
    }
    fn run_item(
        &mut self,
        op: &RepairOperation,
        item: &mut RepairItem,
        journal: &mut Journal,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(RepairProgress),
    ) -> Result<()> {
        journal.append(Some(op.id), Phase::OperationPrepared, op)?;
        checked(cancel)?;
        let source_path = path(&op.source.observation.reference)?;
        let parent_path = source_path.parent().ok_or(E::Scope)?;
        let parent_ref = StorageRef::WindowsPath {
            utf16: parent_path.as_os_str().encode_wide().collect(),
        };
        let parent = self
            .storage
            .roots()
            .parent_reference(&parent_ref)
            .map_err(|_| E::Stale)?;
        let expected = ExpectedSource {
            identity: op.source.observation.identity.clone().ok_or(E::Scope)?,
            parent_identity: parent.identity().clone(),
            name: MutationName::Windows(
                source_path
                    .file_name()
                    .ok_or(E::Scope)?
                    .encode_wide()
                    .collect(),
            ),
            size: op.source.observation.size,
            modified: op.source.observation.modified.clone(),
            digest: digest_value(
                &op.source.sha256,
                op.source.observation.size.ok_or(E::Scope)?,
            )?,
        };
        let mut source = match self.storage.acquire_source(&parent, &expected) {
            Ok(v) => v,
            Err(error) => return stop(journal, item, error, false),
        };
        item.current = Some(source.observation().clone());
        if op.rename {
            journal.append(Some(op.id), Phase::RenamePrepared, &op.intended_name)?;
            match self
                .storage
                .rename_original(&mut source, &name(&op.intended_name), cancel)
            {
                MutationOutcome::Applied(renamed) => {
                    source = renamed;
                    item.renamed = true;
                    item.current = Some(source.observation().clone());
                    journal.append(Some(op.id), Phase::RenameObserved, &item.current)?
                }
                MutationOutcome::NoEffect(error) => return stop(journal, item, error, false),
                MutationOutcome::PendingUnknown(error) => return stop(journal, item, error, true),
            }
        }
        item.processed = true;
        if !op.save {
            item.status = RepairStatus::MetadataFailure;
            item.detail = op
                .metadata_failure
                .clone()
                .unwrap_or_else(|| "Original policy did not reach metadata save".into());
            return journal.append(Some(op.id), Phase::Stopped, item);
        }
        if op.repairs.is_empty() {
            item.status = RepairStatus::Complete;
            item.unchanged = true;
            item.detail = "No tag bytes needed changing".into();
            return journal.append(Some(op.id), Phase::Committed, item);
        }
        let repairs = op
            .repairs
            .iter()
            .map(RepairField::native)
            .collect::<Vec<_>>();
        let extension = op.intended_name.to_ascii_lowercase();
        if ![".mp3", ".flac", ".ogg", ".opus"]
            .iter()
            .any(|suffix| extension.ends_with(suffix))
        {
            item.status = RepairStatus::UnsupportedTags;
            item.detail="Physical tag writing for this format is not yet supported; filename outcome is retained".into();
            return journal.append(Some(op.id), Phase::Stopped, item);
        }
        if let Err(error) = self.storage.prepare_metadata_preservation(&mut source) {
            if !may_continue_after_metadata_refusal(error.kind) {
                return stop(journal, item, error, false);
            }
            item.status = RepairStatus::UnsupportedTags;
            item.detail = error.detail;
            return journal.append(Some(op.id), Phase::Stopped, item);
        }
        let staged_plan = match StagedRepairPlan::inspect(
            &op.intended_name,
            &mut source,
            expected.digest.length,
            &repairs,
            cancel,
        ) {
            Ok(v) => v,
            Err(TagWriteError::Unsupported(reason)) => {
                item.status = RepairStatus::UnsupportedTags;
                item.detail = reason.into();
                return journal.append(Some(op.id), Phase::Stopped, item);
            }
            Err(TagWriteError::Cancelled) => return Err(E::Cancelled),
            Err(_) => return Err(E::Stale),
        };
        // Compute immutable replacement evidence without creating any file.
        // This also rechecks the complete original before the staging intent.
        let planned = match staged_plan.write_staged(
            &mut source,
            &mut std::io::sink(),
            &op.source.sha256,
            cancel,
        ) {
            Ok(value) if value.staged_length == staged_plan.staged_length() => value,
            Err(TagWriteError::Cancelled) => return Err(E::Cancelled),
            _ => return Err(E::Stale),
        };
        journal.append(
            Some(op.id),
            Phase::TempPrepared,
            &TempPreparedV2 {
                source: op.source.clone(),
                replacement_length: planned.staged_length,
                replacement_sha256: planned.staged_sha256.clone(),
                media_sha256: planned.media_sha256.clone(),
            },
        )?;
        let mut temp = match self.storage.create_staged(
            &source,
            &format!("{}-{}", self.job_id(), op.id),
            cancel,
        ) {
            MutationOutcome::Applied(v) => v,
            MutationOutcome::NoEffect(error) => return stop(journal, item, error, false),
            MutationOutcome::PendingUnknown(error) => return stop(journal, item, error, true),
        };
        item.staged = Some(temp.observation().clone());
        if let Err(error) = journal.append(Some(op.id), Phase::TempObserved, &item.staged) {
            self.temps.push(temp);
            return Err(error);
        }
        progress(RepairProgress {
            operation: op.id,
            total_files: self.header.operations.len(),
            phase: "staging_tags".into(),
        });
        let staged =
            match staged_plan.write_staged(&mut source, &mut temp, &op.source.sha256, cancel) {
                Ok(v) if v == planned => v,
                Ok(_) => {
                    self.temps.push(temp);
                    return Err(E::Stale);
                }
                Err(error) => {
                    item.staged = self
                        .storage
                        .observe_staged(&temp)
                        .ok()
                        .or(item.staged.take());
                    self.temps.push(temp);
                    item.status = if error == TagWriteError::Cancelled {
                        RepairStatus::Cancelled
                    } else {
                        RepairStatus::Failed
                    };
                    item.detail = format!("Tag staging failed; original retained: {error}");
                    return journal.append(Some(op.id), Phase::Stopped, item);
                }
            };
        let staged_digest = digest_value(&staged.staged_sha256, staged.staged_length)?;
        let verified = (|| {
            temp.sync_data()?;
            let observation = temp.verify_content(&staged_digest)?;
            self.storage
                .verify_staged_fields(&temp, &op.intended_name, &repairs)?;
            self.storage.finalize_staged(&source, &mut temp)?;
            Ok::<_, MutationError>(observation)
        })();
        match verified {
            Ok(observation) => item.staged = Some(observation),
            Err(error) => {
                self.temps.push(temp);
                return stop(journal, item, error, false);
            }
        }
        if let Err(error)=journal.append(Some(op.id),Phase::StagedVerified,&serde_json::json!({"observation":item.staged,"sha256":staged.staged_sha256,"length":staged.staged_length,"media_sha256":staged.media_sha256})){self.temps.push(temp);return Err(error)}
        checked(cancel)?;
        let backup_name = format!(".sorter-{}-{}.backup", self.job_id(), op.id);
        if let Err(error)=journal.append(Some(op.id),Phase::BackupPrepared,&serde_json::json!({"name":backup_name,"source":item.current,"sha256":op.source.sha256,"original_length":expected.digest.length})){self.temps.push(temp);return Err(error)}
        let backup = match self
            .storage
            .preserve_original(&mut source, &name(&backup_name), cancel)
        {
            MutationOutcome::Applied(v) => v,
            MutationOutcome::NoEffect(error) => {
                self.temps.push(temp);
                return stop(journal, item, error, false);
            }
            MutationOutcome::PendingUnknown(error) => {
                self.temps.push(temp);
                return stop(journal, item, error, true);
            }
        };
        item.backup = Some(backup.observation().clone());
        item.current = None;
        if let Err(error) = journal.append(Some(op.id), Phase::BackupObserved, &item.backup) {
            self.retained.push(backup);
            self.temps.push(temp);
            return Err(error);
        }
        progress(RepairProgress {
            operation: op.id,
            total_files: self.header.operations.len(),
            phase: "original_backed_up".into(),
        });
        if cancel.load(Ordering::Acquire) {
            self.retained.push(backup);
            self.temps.push(temp);
            item.status = RepairStatus::NeedsRecovery;
            item.detail =
                "Cancelled after original was retained as backup; no automatic replay".into();
            return journal.append(Some(op.id), Phase::Stopped, item);
        }
        if let Err(error)=journal.append(Some(op.id),Phase::PublishPrepared,&serde_json::json!({"name":op.intended_name,"staged":item.staged,"sha256":staged.staged_sha256,"replacement_length":planned.staged_length})){self.retained.push(backup);self.temps.push(temp);return Err(error)}
        let mut published = match self.storage.publish_staged(
            &mut temp,
            &backup,
            &parent,
            &name(&op.intended_name),
            cancel,
        ) {
            MutationOutcome::Applied(v) => v,
            MutationOutcome::NoEffect(error) | MutationOutcome::PendingUnknown(error) => {
                self.retained.push(backup);
                self.temps.push(temp);
                return stop(journal, item, error, true);
            }
        };
        item.current = Some(published.observation().clone());
        item.staged = None;
        let verified = self
            .storage
            .sync_published(&mut published, &staged_digest, cancel)
            .and_then(|_| {
                self.storage
                    .verify_published(&mut published, &staged_digest)
            });
        match verified {
            Ok(observed) => item.current = Some(observed),
            Err(error) => {
                self.retained.push(backup);
                self.retained.push(published);
                return stop(journal, item, error, true);
            }
        }
        if let Err(error)=journal.append(Some(op.id),Phase::PublishedVerified,&serde_json::json!({"observation":item.current,"sha256":staged.staged_sha256,"backup":item.backup,"original_sha256":op.source.sha256})){self.retained.push(backup);self.retained.push(published);return Err(error)}
        item.status = RepairStatus::Complete;
        item.repaired = true;
        item.detail = "Tags repaired; original backup retained".into();
        if let Err(error) = journal.append(Some(op.id), Phase::Committed, item) {
            self.retained.push(backup);
            self.retained.push(published);
            return Err(error);
        }
        self.retained.push(backup);
        self.retained.push(published);
        progress(RepairProgress {
            operation: op.id,
            total_files: self.header.operations.len(),
            phase: "committed".into(),
        });
        Ok(())
    }
}
fn may_continue_after_metadata_refusal(kind: MutationErrorKind) -> bool {
    kind == MutationErrorKind::Unsupported
}
fn stop(
    journal: &mut Journal,
    item: &mut RepairItem,
    error: MutationError,
    unknown: bool,
) -> Result<()> {
    if let Some(observed) = error.observed {
        let leaf = match &observed.actual_name {
            MutationName::Windows(units) => String::from_utf16_lossy(units),
            _ => String::new(),
        };
        if leaf.ends_with(".backup") {
            item.backup = Some(*observed);
            item.current = None
        } else if leaf.ends_with(".tmp") {
            item.staged = Some(*observed)
        } else {
            item.current = Some(*observed)
        }
    }
    item.status = if unknown {
        RepairStatus::NeedsRecovery
    } else if error.kind == MutationErrorKind::Cancelled {
        RepairStatus::Cancelled
    } else {
        RepairStatus::Failed
    };
    item.detail = error.detail;
    journal.append(Some(item.operation), Phase::Stopped, item)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairEvidenceStatus {
    Verified,
    Missing,
    Changed,
    Unverified,
}
#[derive(Clone, Debug, Serialize)]
pub struct RepairRecoveryEvidence {
    pub role: String,
    pub reference: StorageRef,
    pub expected_identity: ObjectIdentity,
    pub observed_identity: Option<ObjectIdentity>,
    pub status: RepairEvidenceStatus,
}
#[derive(Clone, Debug, Serialize)]
pub struct RepairRecoveryItem {
    pub operation: usize,
    pub last_phase: String,
    pub recorded: Option<RepairItem>,
    pub evidence: Vec<RepairRecoveryEvidence>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RepairRecoveryReport {
    pub job_id: String,
    pub recorded_finished: bool,
    pub incomplete_tail: bool,
    pub operations: Vec<RepairRecoveryItem>,
}
#[derive(Default)]
struct RecoveryState {
    phase: Option<Phase>,
    last_effect_phase: Option<Phase>,
    current: Option<MutationObservation>,
    backup: Option<MutationObservation>,
    staged: Option<MutationObservation>,
    staged_hash: Option<String>,
    replacement_length: Option<u64>,
    planned_hash: Option<String>,
    planned_media_hash: Option<String>,
    replacement_identity: Option<ObjectIdentity>,
    recorded: Option<RepairItem>,
}
fn decode<T: serde::de::DeserializeOwned>(value: &serde_json::Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|_| E::CorruptJournal)
}
fn observed(value: &serde_json::Value) -> Result<MutationObservation> {
    decode::<Option<MutationObservation>>(value)?.ok_or(E::CorruptJournal)
}
fn allowed_transition(previous: Option<Phase>, next: Phase) -> bool {
    match next {
        Phase::OperationPrepared => previous.is_none(),
        Phase::RenamePrepared => previous == Some(Phase::OperationPrepared),
        Phase::RenameObserved => previous == Some(Phase::RenamePrepared),
        Phase::TempPrepared => matches!(
            previous,
            Some(Phase::OperationPrepared | Phase::RenameObserved)
        ),
        Phase::TempObserved => previous == Some(Phase::TempPrepared),
        Phase::StagedVerified => previous == Some(Phase::TempObserved),
        Phase::BackupPrepared => previous == Some(Phase::StagedVerified),
        Phase::BackupObserved => previous == Some(Phase::BackupPrepared),
        Phase::PublishPrepared => previous == Some(Phase::BackupObserved),
        Phase::PublishedVerified => previous == Some(Phase::PublishPrepared),
        Phase::Committed => matches!(
            previous,
            Some(Phase::OperationPrepared | Phase::RenameObserved | Phase::PublishedVerified)
        ),
        Phase::Stopped => {
            previous.is_some() && !matches!(previous, Some(Phase::Committed | Phase::Stopped))
        }
        _ => false,
    }
}
#[derive(Clone, Copy)]
enum ObservationLength {
    Exact(u64),
    Partial(u64),
}
impl ObservationLength {
    fn matches(self, observed: Option<u64>) -> bool {
        match self {
            Self::Exact(expected) => observed == Some(expected),
            Self::Partial(limit) => observed.is_some_and(|size| size <= limit),
        }
    }
}
fn original_length(op: &RepairOperation) -> Result<u64> {
    op.source.observation.size.ok_or(E::CorruptJournal)
}
fn replacement_length(state: &RecoveryState) -> Result<u64> {
    state.replacement_length.ok_or(E::CorruptJournal)
}
fn check_observation(
    header: &Header,
    op: &RepairOperation,
    observation: &MutationObservation,
    leaf: &str,
    identity: Option<&ObjectIdentity>,
    length: ObservationLength,
) -> Result<()> {
    let source = comparable_path(&op.source.observation.reference)?;
    let expected = source.parent().ok_or(E::CorruptJournal)?.join(leaf);
    let aligned = aligned_recovery_reference(
        &header.source,
        header.observed_source.as_ref(),
        &observation.reference,
    )
    .map_err(|_| E::CorruptJournal)?;
    if comparable_path(&aligned)? != expected
        || !expected.starts_with(comparable_path(&header.source)?)
        || observation.actual_name != name(leaf)
        || identity.is_some_and(|id| id != &observation.identity)
        || Some(&observation.parent_identity) != op.source.observation.parent_identity.as_ref()
        || !length.matches(observation.size)
    {
        return Err(E::CorruptJournal);
    }
    Ok(())
}
fn validate_terminal(
    header: &Header,
    op: &RepairOperation,
    state: &RecoveryState,
    item: &RepairItem,
) -> Result<()> {
    let source_id = op
        .source
        .observation
        .identity
        .as_ref()
        .ok_or(E::CorruptJournal)?;
    let after_backup = matches!(
        state.phase,
        Some(
            Phase::BackupPrepared
                | Phase::BackupObserved
                | Phase::PublishPrepared
                | Phase::PublishedVerified
        )
    );
    let after_publish = matches!(
        state.phase,
        Some(Phase::PublishPrepared | Phase::PublishedVerified)
    );
    if item.repaired && state.phase != Some(Phase::PublishedVerified)
        || item.unchanged
            && (!op.save || !op.repairs.is_empty() || item.status != RepairStatus::Complete)
    {
        return Err(E::CorruptJournal);
    }
    if let Some(current) = &item.current {
        if &current.identity == source_id {
            let leaf = if op.rename
                && state.phase != Some(Phase::OperationPrepared)
                && !(state.phase == Some(Phase::RenamePrepared)
                    && current.actual_name == name(&op.source.observation.name))
            {
                &op.intended_name
            } else {
                &op.source.observation.name
            };
            check_observation(
                header,
                op,
                current,
                leaf,
                Some(source_id),
                ObservationLength::Exact(original_length(op)?),
            )?;
        } else if after_publish && state.replacement_identity.as_ref() == Some(&current.identity) {
            check_observation(
                header,
                op,
                current,
                &op.intended_name,
                state.replacement_identity.as_ref(),
                ObservationLength::Exact(replacement_length(state)?),
            )?;
        } else {
            return Err(E::CorruptJournal);
        }
    }
    if let Some(backup) = &item.backup {
        if !after_backup {
            return Err(E::CorruptJournal);
        }
        check_observation(
            header,
            op,
            backup,
            &format!(".sorter-{}-{}.backup", header.job_id, op.id),
            Some(source_id),
            ObservationLength::Exact(original_length(op)?),
        )?;
    }
    if let Some(staged) = &item.staged {
        if !matches!(
            state.phase,
            Some(
                Phase::TempPrepared
                    | Phase::TempObserved
                    | Phase::StagedVerified
                    | Phase::BackupPrepared
                    | Phase::BackupObserved
                    | Phase::PublishPrepared
            )
        ) {
            return Err(E::CorruptJournal);
        }
        check_observation(
            header,
            op,
            staged,
            &format!(".sorter-{}-{}.tmp", header.job_id, op.id),
            state.staged.as_ref().map(|s| &s.identity),
            if state.staged_hash.is_some() {
                ObservationLength::Exact(replacement_length(state)?)
            } else {
                ObservationLength::Partial(replacement_length(state)?)
            },
        )?;
        if &staged.identity == source_id {
            return Err(E::CorruptJournal);
        }
    }
    if state
        .backup
        .as_ref()
        .is_some_and(|prior| item.backup.as_ref() != Some(prior))
    {
        return Err(E::CorruptJournal);
    }
    if state.current.is_some() && item.current.is_none() && item.backup.is_none() {
        return Err(E::CorruptJournal);
    }
    if state.staged.is_some()
        && item.staged.is_none()
        && !item
            .current
            .as_ref()
            .is_some_and(|current| state.replacement_identity.as_ref() == Some(&current.identity))
    {
        return Err(E::CorruptJournal);
    }
    if item.status == RepairStatus::Complete && item.current.is_none() {
        return Err(E::CorruptJournal);
    }
    Ok(())
}
impl SelectedWindowsRepairJob {
    /// Close data guards after the one-shot attempt, then freshly inspect bytes.
    /// Closing these handles performs no unlink, replay or backup cleanup.
    pub fn inspect_recovery(&mut self, cancel: &AtomicBool) -> Result<RepairRecoveryReport> {
        if !self.attempted {
            return Err(E::Unsupported);
        }
        self.retained.clear();
        self.temps.clear();
        let location = self.locations.first().ok_or(E::Unsupported)?;
        let bytes = self
            .storage
            .roots()
            .read_selected_journal(location.name(), location.identity())
            .map_err(|_| E::Io)?;
        recover_repair_journal(self.storage.roots(), &bytes, cancel)
    }
}
/// Display-only header/framing evidence. Physical phase replay remains pending
/// until fresh root identity supplies the observed (never serialized) root alias.
#[derive(Clone, Debug, Serialize)]
pub struct RecordedRepairJob {
    pub job_id: String,
    pub source: StorageRef,
    pub source_identity: ObjectIdentity,
    pub operations: Vec<RecordedRepairOperation>,
    pub incomplete_tail: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecordedRepairOperation {
    pub operation: usize,
    pub source: StorageRef,
    pub intended_name: String,
}
pub fn inspect_recorded_repair_job(bytes: &[u8], control: &InspectionControl) -> Result<RecordedRepairJob> {
    control.stage("journals")?;
    check_inspection_input(bytes, control)?;
    let mut offset = 0usize;
    let mut sequence = 0u64;
    let mut previous = String::new();
    let mut header: Option<Header> = None;
    let mut incomplete_tail = false;
    while offset < bytes.len() {
        control.checkpoint()?;
        let Some(end) = bytes[offset..].iter().position(|byte| *byte == b'\n') else { incomplete_tail = true; break; };
        let envelope: Envelope = serde_json::from_slice(&bytes[offset..offset + end]).map_err(|_| E::CorruptJournal)?;
        offset += end + 1;
        if envelope.body.sequence != sequence || envelope.body.previous != previous || crate::hash(&envelope.body)? != envelope.sha256 { return Err(E::CorruptJournal); }
        let record = envelope.body;
        if sequence == 0 {
            if record.phase != Phase::Header || record.operation.is_some() { return Err(E::CorruptJournal); }
            let value: Header = decode(&record.payload)?;
            if value.operations.len() > MAX_RESTART_OPERATIONS { return Err(E::Unsupported); }
            if !matches!(value.schema, 1 | 2) || value.job_id != record.job_id || value.job_id.is_empty()
                || value.job_id.len() > 100 || !value.job_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                || !valid_hash(&value.plan_sha256) || value.operations.is_empty()
                || !matches!(value.source_identity, ObjectIdentity::Windows { .. })
            { return Err(E::CorruptJournal); }
            let root = comparable_path(&value.source)?;
            if !root.is_absolute() { return Err(E::Scope); }
            let mut seen = HashSet::new();
            for (index, operation) in value.operations.iter().enumerate() {
                control.checkpoint()?;
                if operation.id != index || operation.depends_on.iter().any(|id| *id >= index)
                    || !valid_hash(&operation.source.sha256) || operation.source.observation.size.is_none()
                    || !matches!(operation.source.observation.identity, Some(ObjectIdentity::Windows { .. }))
                    || !matches!(operation.source.observation.parent_identity, Some(ObjectIdentity::Windows { .. }))
                    || !comparable_path(&operation.source.observation.reference)?.starts_with(&root)
                    || !seen.insert(operation.source.observation.reference.clone())
                    || operation.intended_name.is_empty() || operation.intended_name.contains(['/', '\\', ':', '\0'])
                    || operation.intended_name == "." || operation.intended_name == ".."
                { return Err(E::CorruptJournal); }
            }
            header = Some(value);
        } else {
            let header = header.as_ref().ok_or(E::CorruptJournal)?;
            if record.job_id != header.job_id || record.phase == Phase::Header
                || record.operation.is_some_and(|id| id >= header.operations.len())
            { return Err(E::CorruptJournal); }
        }
        sequence = sequence.checked_add(1).ok_or(E::Unsupported)?;
        previous = envelope.sha256;
        control.record()?;
    }
    let header = header.ok_or(E::CorruptJournal)?;
    let mut operations = Vec::new();
    for operation in header.operations {
        control.checkpoint()?;
        operations.push(RecordedRepairOperation { operation: operation.id, source: operation.source.observation.reference, intended_name: operation.intended_name });
    }
    control.checkpoint()?;
    Ok(RecordedRepairJob { job_id: header.job_id, source: header.source, source_identity: header.source_identity, operations, incomplete_tail })
}
/// Fresh explicitly granted repair roots are mandatory; serialized evidence
/// reconstructs only read-only reconciliation, never an executable job.
pub fn recover_repair_journal(
    roots: &WindowsMutationStorage,
    bytes: &[u8],
    cancel: &AtomicBool,
) -> Result<RepairRecoveryReport> {
    recover_repair_journal_inner(roots, bytes, cancel, None, 0)
}
/// Read-only interval observations. Cursor reorders fresh reads, never IDs or
/// strict phase replay, and never imports prior hash verification as current.
pub fn recover_repair_journal_controlled(
    roots: &WindowsMutationStorage, bytes: &[u8], cancel: &AtomicBool,
    control: &InspectionControl, start_operation: usize,
) -> Result<RepairRecoveryReport> {
    control.stage("journals")?;
    check_inspection_input(bytes, control)?;
    recover_repair_journal_inner(roots, bytes, cancel, Some(control), start_operation)
}
fn recover_repair_journal_inner(
    roots: &WindowsMutationStorage, bytes: &[u8], cancel: &AtomicBool,
    control: Option<&InspectionControl>, start_operation: usize,
) -> Result<RepairRecoveryReport> {
    checked_inspection(cancel, control)?;
    if bytes.len() > MAX_JOURNAL {
        return Err(E::Unsupported);
    }
    if roots.domain() != WindowsMutationDomain::SelectedRepair {
        return Err(E::Scope);
    }
    let mut header: Option<Header> = None;
    let mut states: Vec<RecoveryState> = Vec::new();
    let mut offset = 0;
    let mut sequence = 0;
    let mut previous = String::new();
    let mut finished = false;
    let mut incomplete_tail = false;
    let mut active = 0usize;
    let mut stopped = false;
    while offset < bytes.len() {
        checked_inspection(cancel, control)?;
        let Some(end) = bytes[offset..].iter().position(|b| *b == b'\n') else {
            incomplete_tail = true;
            break;
        };
        let envelope: Envelope =
            serde_json::from_slice(&bytes[offset..offset + end]).map_err(|_| E::CorruptJournal)?;
        offset += end + 1;
        if finished
            || envelope.body.sequence != sequence
            || envelope.body.previous != previous
            || crate::hash(&envelope.body)? != envelope.sha256
        {
            return Err(E::CorruptJournal);
        }
        sequence += 1;
        if let Some(control) = control { control.record()?; }
        previous = envelope.sha256;
        let record = envelope.body;
        if record.phase == Phase::Header {
            if sequence != 1 || record.operation.is_some() {
                return Err(E::CorruptJournal);
            }
            let mut value: Header = decode(&record.payload)?;
            if control.is_some() && value.operations.len() > MAX_RESTART_OPERATIONS { return Err(E::Unsupported); }
            if !matches!(value.schema, 1 | 2)
                || value.job_id != record.job_id
                || value.operations.is_empty()
                || !valid_hash(&value.plan_sha256)
                || value.source != inspection_probe(cancel, control, || roots.selected_repair_reference().map_err(|_| E::Scope))?
                || &value.source_identity
                    != inspection_probe(cancel, control, || roots.selected_repair_root().map_err(|_| E::Scope))?.identity()
            {
                return Err(E::Scope);
            }
            value.observed_source = Some(
                inspection_probe(cancel, control, || roots.selected_repair_observed_reference().map_err(|_| E::Scope))?,
            );
            let root = comparable_path(&value.source)?;
            let mut seen = HashSet::new();
            for (index, op) in value.operations.iter().enumerate() {
                checked_inspection(cancel, control)?;
                if op.id != index
                    || !valid_hash(&op.source.sha256)
                    || op.source.observation.size.is_none()
                    || op.source.observation.identity.is_none()
                    || !comparable_path(&op.source.observation.reference)?.starts_with(&root)
                    || !seen.insert(op.source.observation.reference.clone())
                    || op.depends_on.iter().any(|d| *d >= index)
                    || op.intended_name.is_empty()
                    || op.intended_name.contains(['/', '\\', ':', '\0'])
                    || op.intended_name == "."
                    || op.intended_name == ".."
                {
                    return Err(E::CorruptJournal);
                }
            }
            states = (0..value.operations.len())
                .map(|_| RecoveryState::default())
                .collect();
            header = Some(value);
            continue;
        }
        let header = header.as_ref().ok_or(E::CorruptJournal)?;
        if record.job_id != header.job_id {
            return Err(E::CorruptJournal);
        }
        if record.phase == Phase::Finished {
            if record.operation.is_some() {
                return Err(E::CorruptJournal);
            }
            let result: RepairResult = decode(&record.payload)?;
            let mut recorded = Vec::new();
            for state in &states {
                checked_inspection(cancel, control)?;
                if let Some(item) = &state.recorded { recorded.push(item.clone()); }
            }
            if result.total_files != header.operations.len()
                || result.items.len() != recorded.len()
                || result.items.len() > header.operations.len()
                || serde_json::to_value(&result.items).map_err(|_| E::Io)?
                    != serde_json::to_value(&recorded).map_err(|_| E::Io)?
            {
                return Err(E::CorruptJournal);
            }
            let rebuilt = RepairResult::build(
                result.total_files,
                recorded,
                result.status == RepairJobStatus::Cancelled,
            );
            if serde_json::to_value(&rebuilt).map_err(|_| E::Io)? != record.payload {
                return Err(E::CorruptJournal);
            }
            finished = true;
            continue;
        }
        let index = record.operation.ok_or(E::CorruptJournal)?;
        if stopped || index >= states.len() || index != active {
            return Err(E::CorruptJournal);
        }
        let op = &header.operations[index];
        let state = &mut states[index];
        if !allowed_transition(state.phase, record.phase)
            || op.rename
                && state.phase == Some(Phase::OperationPrepared)
                && matches!(record.phase, Phase::TempPrepared | Phase::Committed)
        {
            return Err(E::CorruptJournal);
        }
        match record.phase {
            Phase::OperationPrepared => {
                if serde_json::to_value(op).map_err(|_| E::Io)? != record.payload {
                    return Err(E::CorruptJournal);
                }
            }
            Phase::RenamePrepared => {
                if !op.rename
                    || record.payload != serde_json::Value::String(op.intended_name.clone())
                {
                    return Err(E::CorruptJournal);
                }
            }
            Phase::RenameObserved => {
                let observation = observed(&record.payload)?;
                check_observation(
                    header,
                    op,
                    &observation,
                    &op.intended_name,
                    op.source.observation.identity.as_ref(),
                    ObservationLength::Exact(original_length(op)?),
                )?;
                state.current = Some(observation)
            }
            Phase::TempPrepared => {
                if !op.save || op.repairs.is_empty() {
                    return Err(E::CorruptJournal);
                }
                let original = serde_json::to_value(&op.source).map_err(|_| E::Io)?;
                if header.schema == 1 {
                    if original != record.payload {
                        return Err(E::CorruptJournal);
                    }
                    state.replacement_length = Some(original_length(op)?);
                } else {
                    let intent: TempPreparedV2 = decode(&record.payload)?;
                    let length = intent.replacement_length;
                    if !valid_hash(&intent.replacement_sha256) || !valid_hash(&intent.media_sha256)
                    {
                        return Err(E::CorruptJournal);
                    }
                    let extension = op.intended_name.to_ascii_lowercase();
                    // MP3 is fixed-length; FLAC may collapse duplicate comments.
                    // Ogg can add at most one 27-byte header page when separating
                    // Vorbis comment/setup. The exact planned length and hashes
                    // are independently bound to every later replacement record.
                    if serde_json::to_value(&intent.source).map_err(|_| E::Io)? != original
                        || length == 0
                        || if extension.ends_with(".mp3") {
                            length != original_length(op)?
                        } else if extension.ends_with(".flac") {
                            length > original_length(op)?
                        } else if extension.ends_with(".ogg") || extension.ends_with(".opus") {
                            original_length(op)?
                                .checked_add(27)
                                .is_none_or(|maximum| length > maximum)
                        } else {
                            true
                        }
                    {
                        return Err(E::CorruptJournal);
                    }
                    state.replacement_length = Some(length);
                    state.planned_hash = Some(intent.replacement_sha256);
                    state.planned_media_hash = Some(intent.media_sha256);
                }
            }
            Phase::TempObserved => {
                let observation = observed(&record.payload)?;
                check_observation(
                    header,
                    op,
                    &observation,
                    &format!(".sorter-{}-{}.tmp", header.job_id, index),
                    None,
                    ObservationLength::Exact(0),
                )?;
                if Some(&observation.identity) == op.source.observation.identity.as_ref() {
                    return Err(E::CorruptJournal);
                }
                state.staged = Some(observation)
            }
            Phase::StagedVerified => {
                let observation = observed(&record.payload["observation"])?;
                let sha = record.payload["sha256"].as_str().ok_or(E::CorruptJournal)?;
                if !valid_hash(sha)
                    || !record.payload["media_sha256"]
                        .as_str()
                        .is_some_and(valid_hash)
                    || Some(&observation.identity) != state.staged.as_ref().map(|o| &o.identity)
                    || observation.size != record.payload["length"].as_u64()
                    || observation.size != state.replacement_length
                    || state
                        .planned_hash
                        .as_deref()
                        .is_some_and(|planned| planned != sha)
                    || state.planned_media_hash.as_deref().is_some_and(|planned| {
                        record.payload["media_sha256"].as_str() != Some(planned)
                    })
                {
                    return Err(E::CorruptJournal);
                }
                check_observation(
                    header,
                    op,
                    &observation,
                    &format!(".sorter-{}-{}.tmp", header.job_id, index),
                    None,
                    ObservationLength::Exact(replacement_length(state)?),
                )?;
                state.replacement_identity = Some(observation.identity.clone());
                state.staged = Some(observation);
                state.staged_hash = Some(sha.into())
            }
            Phase::BackupPrepared => {
                if record.payload["name"] != format!(".sorter-{}-{}.backup", header.job_id, index)
                    || record.payload["sha256"] != op.source.sha256
                    || header.schema == 2
                        && record.payload["original_length"].as_u64() != op.source.observation.size
                {
                    return Err(E::CorruptJournal);
                }
                if header.schema == 2 {
                    let source = observed(&record.payload["source"])?;
                    check_observation(
                        header,
                        op,
                        &source,
                        &op.intended_name,
                        op.source.observation.identity.as_ref(),
                        ObservationLength::Exact(original_length(op)?),
                    )?;
                }
            }
            Phase::BackupObserved => {
                let observation = observed(&record.payload)?;
                check_observation(
                    header,
                    op,
                    &observation,
                    &format!(".sorter-{}-{}.backup", header.job_id, index),
                    op.source.observation.identity.as_ref(),
                    ObservationLength::Exact(original_length(op)?),
                )?;
                state.backup = Some(observation);
                state.current = None
            }
            Phase::PublishPrepared => {
                if record.payload["name"] != op.intended_name
                    || record.payload["sha256"].as_str() != state.staged_hash.as_deref()
                    || header.schema == 2
                        && record.payload["replacement_length"].as_u64() != state.replacement_length
                    || header.schema == 2
                        && record.payload["staged"]
                            != serde_json::to_value(&state.staged).map_err(|_| E::Io)?
                {
                    return Err(E::CorruptJournal);
                }
            }
            Phase::PublishedVerified => {
                let observation = observed(&record.payload["observation"])?;
                check_observation(
                    header,
                    op,
                    &observation,
                    &op.intended_name,
                    state.staged.as_ref().map(|o| &o.identity),
                    ObservationLength::Exact(replacement_length(state)?),
                )?;
                if record.payload["sha256"].as_str() != state.staged_hash.as_deref()
                    || record.payload["original_sha256"] != op.source.sha256
                    || header.schema == 2
                        && record.payload["backup"]
                            != serde_json::to_value(&state.backup).map_err(|_| E::Io)?
                {
                    return Err(E::CorruptJournal);
                }
                state.current = Some(observation);
                state.staged = None
            }
            Phase::Committed | Phase::Stopped => {
                let item: RepairItem = decode(&record.payload)?;
                if item.operation != index
                    || item.original != op.source.observation.reference
                    || item.renamed && !op.rename
                {
                    return Err(E::CorruptJournal);
                }
                if record.phase == Phase::Committed
                    && (!op.save
                        || item.renamed != op.rename
                        || item.status != RepairStatus::Complete
                        || !item.processed
                        || item.repaired != (state.phase == Some(Phase::PublishedVerified))
                        || item.unchanged != op.repairs.is_empty())
                {
                    return Err(E::CorruptJournal);
                }
                if record.phase == Phase::Stopped && item.status == RepairStatus::Complete {
                    return Err(E::CorruptJournal);
                }
                validate_terminal(header, op, state, &item)?;
                state.last_effect_phase = state.phase;
                state.current = item.current.clone();
                state.backup = item.backup.clone();
                state.staged = item.staged.clone();
                stopped = matches!(
                    item.status,
                    RepairStatus::Failed | RepairStatus::Cancelled | RepairStatus::NeedsRecovery
                );
                state.recorded = Some(item);
                active += 1
            }
            _ => return Err(E::CorruptJournal),
        }
        state.phase = Some(record.phase);
    }
    let header = header.ok_or(E::CorruptJournal)?;
    let mut reads = inspection_probe(cancel, control, || WindowsPlanningStorage::for_repair(roots))?;
    let total = header.operations.len();
    if start_operation > total { return Err(E::Scope); }
    if let Some(control) = control { control.operations(total)?; }
    let mut operations = vec![None; total];
    for index in (start_operation..total).chain(0..start_operation) {
        checked_inspection(cancel, control)?;
        let op = &header.operations[index];
        let state = &states[index];
        if let Some(control) = control { control.begin_operation(op.id)?; }
        let mut evidence = Vec::new();
        if let Some(current) = &state.current {
            let expected = if state.replacement_identity.as_ref() == Some(&current.identity) {
                state.staged_hash.as_deref()
            } else {
                Some(op.source.sha256.as_str())
            };
            evidence.push(inspect_evidence(
                &mut reads, &header, "current", current, expected, cancel, control,
            )?)
        }
        if let Some(backup) = &state.backup {
            evidence.push(inspect_evidence(
                &mut reads,
                &header,
                "backup",
                backup,
                Some(&op.source.sha256),
                cancel,
                control,
            )?)
        }
        if let Some(staged) = &state.staged {
            evidence.push(inspect_evidence(
                &mut reads,
                &header,
                "staged",
                staged,
                state.staged_hash.as_deref(),
                cancel,
                control,
            )?)
        }
        if state.current.is_none() && state.backup.is_none() {
            let source = &op.source.observation;
            let original = MutationObservation {
                reference: source.reference.clone(),
                identity: source.identity.clone().ok_or(E::CorruptJournal)?,
                parent_identity: source.parent_identity.clone().ok_or(E::CorruptJournal)?,
                actual_name: name(&source.name),
                size: source.size,
            };
            evidence.push(inspect_evidence(
                &mut reads,
                &header,
                "original",
                &original,
                Some(&op.source.sha256),
                cancel,
                control,
            )?)
        }
        // Journal intents name candidates even if dispatch response was lost.
        // These are expected identities to compare, never observed success facts.
        if state
            .recorded
            .as_ref()
            .is_none_or(|item| item.status == RepairStatus::NeedsRecovery)
        {
            let effective_phase = state.last_effect_phase.or(state.phase);
            let original = &op.source.observation;
            let source_identity = original.identity.clone().ok_or(E::CorruptJournal)?;
            let mut candidates = Vec::new();
            if op.rename && effective_phase.is_some_and(|p| p != Phase::OperationPrepared) {
                candidates.push((
                    "planned_name",
                    op.intended_name.clone(),
                    source_identity.clone(),
                    Some(op.source.sha256.as_str()),
                    original_length(op)?,
                ));
            }
            if matches!(
                effective_phase,
                Some(
                    Phase::BackupPrepared
                        | Phase::BackupObserved
                        | Phase::PublishPrepared
                        | Phase::PublishedVerified
                )
            ) {
                candidates.push((
                    "backup_candidate",
                    format!(".sorter-{}-{}.backup", header.job_id, op.id),
                    source_identity,
                    Some(op.source.sha256.as_str()),
                    original_length(op)?,
                ));
            }
            if effective_phase == Some(Phase::PublishPrepared)
                && let Some(identity) = &state.replacement_identity
            {
                candidates.push((
                    "published_candidate",
                    op.intended_name.clone(),
                    identity.clone(),
                    state.staged_hash.as_deref(),
                    replacement_length(state)?,
                ));
            }
            for (role, leaf, identity, hash, length) in candidates {
                checked_inspection(cancel, control)?;
                let candidate = path(&original.reference)?
                    .parent()
                    .ok_or(E::CorruptJournal)?
                    .join(&leaf);
                let expected = MutationObservation {
                    reference: StorageRef::WindowsPath {
                        utf16: candidate.as_os_str().encode_wide().collect(),
                    },
                    identity,
                    parent_identity: original.parent_identity.clone().ok_or(E::CorruptJournal)?,
                    actual_name: name(&leaf),
                    size: Some(length),
                };
                if !evidence.iter().any(|e| {
                    e.reference == expected.reference && e.expected_identity == expected.identity
                }) {
                    evidence.push(inspect_evidence(
                        &mut reads, &header, role, &expected, hash, cancel, control,
                    )?);
                }
            }
        }
        operations[index] = Some(RepairRecoveryItem {
            operation: op.id,
            last_phase: format!("{:?}", state.phase),
            recorded: state.recorded.clone(),
            evidence,
        });
        if let Some(control) = control { control.complete_operation()?; }
    }
    checked_inspection(cancel, control)?;
    let operations = operations.into_iter().collect::<Option<Vec<_>>>().ok_or(E::NeedsRecovery)?;
    Ok(RepairRecoveryReport {
        job_id: header.job_id,
        recorded_finished: finished,
        incomplete_tail,
        operations,
    })
}
fn inspect_evidence(
    reads: &mut dyn ReadStorage,
    header: &Header,
    role: &str,
    expected: &MutationObservation,
    sha: Option<&str>,
    cancel: &AtomicBool,
    control: Option<&InspectionControl>,
) -> Result<RepairRecoveryEvidence> {
    checked_inspection(cancel, control)?;
    let mut value = RepairRecoveryEvidence {
        role: role.into(),
        reference: expected.reference.clone(),
        expected_identity: expected.identity.clone(),
        observed_identity: None,
        status: RepairEvidenceStatus::Unverified,
    };
    let read_reference = aligned_recovery_reference(
        &header.source,
        header.observed_source.as_ref(),
        &expected.reference,
    )?;
    let observed = inspection_probe(cancel, control, || match reads.stat(&read_reference) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind == StorageErrorKind::NotFound => Ok(None),
        Err(_) => Err(E::Io),
    });
    let observed = match observed {
        Ok(Some(observed)) => observed,
        Ok(None) => { value.status = RepairEvidenceStatus::Missing; return Ok(value); }
        Err(E::Cancelled) => return Err(E::Cancelled),
        Err(_) => return Ok(value),
    };
    value.observed_identity = observed.identity.clone();
    if observed.identity.as_ref() != Some(&expected.identity)
        || observed.kind != EntryKind::File
        || observed.size != expected.size
        || observed.parent_identity.as_ref() != Some(&expected.parent_identity)
    {
        value.status = RepairEvidenceStatus::Changed;
        return Ok(value);
    }
    let Some(sha) = sha else { return Ok(value) };
    let mut input = match inspection_probe(cancel, control, || reads.open_read(&read_reference).map_err(|_| E::Io)) {
        Ok(input) => input,
        Err(E::Cancelled) => return Err(E::Cancelled),
        Err(_) => return Ok(value),
    };
    if input
        .binding()
        .is_none_or(|b| b.identity != expected.identity || Some(b.size) != expected.size)
    {
        return Ok(value);
    }
    let mut hash = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0; 65536];
    loop {
        checked_inspection(cancel, control)?;
        let amount = if control.is_some() { (observed.size.ok_or(E::Stale)? - length).min(buffer.len() as u64) as usize } else { buffer.len() };
        if amount == 0 { break; }
        if let Some(control) = control { control.observation()?; }
        checked_inspection(cancel, control)?;
        let read = input.read(&mut buffer[..amount]);
        if let (Some(control), Ok(count)) = (control, &read) { control.bytes(*count)?; }
        checked_inspection(cancel, control)?;
        let n = match read { Ok(count) => count, Err(_) => return Ok(value) };
        if n == 0 {
            break;
        }
        length = length.checked_add(n as u64).ok_or(E::Io)?;
        if Some(length) > observed.size {
            return Ok(value);
        }
        hash.update(&buffer[..n]);
    }
    if control.is_some() {
        checked_inspection(cancel, control)?;
        if let Some(control) = control { control.observation()?; }
        let mut extra = [0u8; 1];
        checked_inspection(cancel, control)?;
        let read = input.read(&mut extra);
        if let (Some(control), Ok(count)) = (control, &read) { control.bytes(*count)?; }
        checked_inspection(cancel, control)?;
        if !matches!(read, Ok(0)) { return Ok(value); }
    }
    let unchanged = if Some(length) == observed.size && format!("{:x}", hash.finalize()) == sha {
        match inspection_probe(cancel, control, || reads.stat(&read_reference).map_err(|_| E::Io)) {
            Ok(again) => again == observed,
            Err(E::Cancelled) => return Err(E::Cancelled),
            Err(_) => false,
        }
    } else { false };
    value.status = if unchanged { RepairEvidenceStatus::Verified } else { RepairEvidenceStatus::Changed };
    checked_inspection(cancel, control)?;
    Ok(value)
}

#[cfg(test)]
mod journal_budget_tests {
    #[test]
    fn local_drive_prefix_alignment_does_not_accept_device_or_other_volume_aliases() {
        use sorter_core::discovery::StorageRef;
        let reference = |value: &str| StorageRef::WindowsPath {
            utf16: value.encode_utf16().collect(),
        };
        let root = reference(r"C:\music");
        assert_eq!(
            super::aligned_read_reference(&root, &reference(r"\\?\C:\music\song.mp3")).unwrap(),
            reference(r"C:\music\song.mp3")
        );
        for candidate in [
            r"\\?\D:\music\song.mp3",
            r"\\?\UNC\server\music\song.mp3",
            r"\\.\C:\music\song.mp3",
            r"\\?\Volume{other}\music\song.mp3",
        ] {
            assert!(super::aligned_read_reference(&root, &reference(candidate)).is_err());
        }
    }
    #[test]
    fn observed_root_alias_never_normalizes_child_components_or_other_roots() {
        use sorter_core::discovery::StorageRef;
        let reference = |value: &str| StorageRef::WindowsPath {
            utf16: value.encode_utf16().collect(),
        };
        let selected = reference(r"C:\Users\RUNNER~1\music");
        let observed = reference(r"\\?\C:\Users\runneradmin\music");
        let child = reference(r"\\?\C:\Users\runneradmin\music\album\song.mp3");
        assert_eq!(
            super::aligned_recovery_reference(&selected, Some(&observed), &child).unwrap(),
            reference(r"C:\Users\RUNNER~1\music\album\song.mp3")
        );
        assert!(super::aligned_recovery_reference(&selected, None, &child).is_err());
        for wrong in [
            r"\\?\C:\Users\another\music\album\song.mp3",
            r"\\?\D:\Users\runneradmin\music\album\song.mp3",
            r"\\?\C:\Users\runneradmin\music-other\album\song.mp3",
            r"\\?\C:\Users\runneradmin\music\..\outside.mp3",
        ] {
            assert!(
                super::aligned_recovery_reference(&selected, Some(&observed), &reference(wrong))
                    .is_err()
            );
        }
        // Even under the valid root, an 8.3 child alias is left exact. It cannot
        // match the planned long child path in check_observation.
        assert_ne!(
            super::aligned_recovery_reference(
                &selected,
                Some(&observed),
                &reference(r"\\?\C:\Users\runneradmin\music\ALBUM~1\song.mp3")
            )
            .unwrap(),
            reference(r"C:\Users\RUNNER~1\music\album-long\song.mp3")
        );
    }
    #[test]
    fn metadata_staleness_and_io_failures_never_continue_as_format_limits() {
        use sorter_storage_contracts::MutationErrorKind as K;
        assert!(super::may_continue_after_metadata_refusal(K::Unsupported));
        for kind in [
            K::StalePlan,
            K::Io,
            K::PermissionDenied,
            K::VerificationFailed,
        ] {
            assert!(!super::may_continue_after_metadata_refusal(kind));
        }
    }
    #[test]
    fn bounded_append_keeps_previous_prefix_readable() {
        assert_eq!(super::journal_budget(7, 3, 10), Some(10));
        assert_eq!(super::journal_budget(7, 4, 10), None);
        assert_eq!(super::journal_budget(usize::MAX, 1, usize::MAX), None);
    }
}
