use crate::{ExecutionError as E, Result, digest, hash};
use serde::{Deserialize, Serialize};
use sorter_core::{discovery::*, job_plan::*, metadata::MetadataProvider};
use std::{collections::HashSet, io::Read};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub resource_ids: Vec<String>,
    pub adapter_versions: Vec<String>,
    pub ai_profile: AiProfile,
    /// Actual bytes hashed by the host, in the same order as the identities above.
    pub resource_hashes: Vec<String>,
    pub adapter_hashes: Vec<String>,
    pub model_hash: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidence {
    pub observation: EntryObservation,
    pub sha256: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferMode {
    Copy,
    Move,
}
/// Host-only grant. Deliberately not Deserialize: a plan/journal cannot mint consent.
/// This evidence alone grants no native path access. Native Copy validation also
/// requires the consumed owned-TempDir capability; SAF execution remains disabled.
#[derive(Clone, Debug)]
pub struct ExecutionScope {
    pub inspected_plan: String,
    pub source_root: ObjectIdentity,
    pub target_root: ObjectIdentity,
    pub inspected_sources: Vec<SourceEvidence>,
    pub resources: Resources,
    pub allowed_modes: Vec<TransferMode>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScopeEvidence {
    pub inspected_plan: String,
    pub source_root: ObjectIdentity,
    pub target_root: ObjectIdentity,
    pub inspected_sources: Vec<SourceEvidence>,
    pub resources: Resources,
    pub allowed_modes: Vec<TransferMode>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundOperation {
    pub id: usize,
    pub depends_on: Vec<usize>,
    pub source: SourceEvidence,
    pub destination: DestinationEvidence,
    pub mode: TransferMode,
    /// Original per-transfer effect; duet copies retain their source until group retirement.
    pub copy_effect: bool,
    /// Exact ordered cohort from the plan's DuetDeleteIntention, never inferred from identity alone.
    pub duet_cohort: Option<Vec<usize>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DestinationEvidence {
    pub id: String,
    pub root: StorageRef,
    pub components: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BindingData {
    pub schema: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub album_context: Option<String>,
    pub plan_sha256: String,
    pub source_root: EntryObservation,
    pub target_root: EntryObservation,
    pub resources: Resources,
    pub operations: Vec<BoundOperation>,
    pub scope_sha256: String,
    pub scope_evidence: ScopeEvidence,
    pub synthetic_only: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub owned_windows_copy: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub owned_windows_rename: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub selected_windows_copy: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub selected_windows_rename: bool,
}
/// Internal domain choice; serialized booleans do not grant a live binding.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionDomain {
    Synthetic,
    OwnedWindowsCopy,
    OwnedWindowsRename,
    SelectedWindowsCopy,
    SelectedWindowsRename,
}
/// Only validation constructs a usable binding. A serialized header is evidence,
/// never an ExecutionBinding or authorization token.
#[derive(Clone, Debug)]
pub struct ExecutionBinding {
    pub(crate) data: BindingData,
    pub(crate) recovery_only: bool,
}
/// New host grant for read-only reconciliation after a process restart. The
/// inspected binding digest identifies the complete evidence, not consent by itself.
#[derive(Clone, Debug)]
pub struct RecoveryScope {
    pub original_scope: ExecutionScope,
    pub inspected_binding_sha256: String,
}
pub(crate) fn scope_digest(scope: &ExecutionScope) -> Result<String> {
    hash(&(
        &scope.inspected_plan,
        &scope.source_root,
        &scope.target_root,
        &scope.inspected_sources,
        &scope.resources,
        &scope.allowed_modes,
    ))
}
impl ExecutionBinding {
    pub fn operations(&self) -> &[BoundOperation] {
        &self.data.operations
    }
    pub fn plan_sha256(&self) -> &str {
        &self.data.plan_sha256
    }
    pub fn sha256(&self) -> Result<String> {
        hash(&self.data)
    }
    pub fn required_destinations(&self, source: &StorageRef) -> Vec<&str> {
        self.operations()
            .iter()
            .filter(|o| &o.source.observation.reference == source)
            .map(|o| o.destination.id.as_str())
            .collect()
    }
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn synthetic(e: &EntryObservation) -> bool {
    matches!(e.reference, StorageRef::TestOpaque { .. })
        && matches!(e.identity, Some(ObjectIdentity::Test { .. }))
}
/// Re-run the unchanged v14 policy against fresh storage/metadata, verify the
/// entire observed plan hash, then strengthen source evidence against host-inspected
/// digests. Caller retains the real host authorization boundary; this API is test-only
/// in capability (all references must be explicitly synthetic), not native apply.
pub fn validate_execution(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
) -> Result<ExecutionBinding> {
    validate_execution_domain(
        plan,
        storage,
        metadata,
        recognition,
        resources,
        scope,
        ExecutionDomain::Synthetic,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_execution_domain(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
    domain: ExecutionDomain,
) -> Result<ExecutionBinding> {
    let (root, target) = validate_plan_scope_domain(
        plan,
        storage,
        metadata,
        recognition,
        resources,
        scope,
        domain,
    )?;
    let mut operations = vec![];
    let mut destinations = HashSet::new();
    let mut sources = HashSet::new();
    for (index, op) in plan.operations.iter().enumerate() {
        if op.id != index
            || !op.stop_and_replan_on_effect_mismatch
            || op.depends_on.iter().any(|d| *d >= op.id)
            || op.depends_on.iter().collect::<HashSet<_>>().len() != op.depends_on.len()
        {
            return Err(E::InvalidBinding);
        }
        let OperationKind::SingleTransfer {
            source, transfer, ..
        } = &op.kind
        else {
            return Err(E::Unsupported);
        };
        if source != &transfer.source
            || !transfer.predicted_success
            || !transfer.blockers.is_empty()
            || source.kind != EntryKind::File
            || source.size.is_none()
            || source.modified.is_none()
            || transfer.destination.root != target.reference
            || transfer.destination.components.is_empty()
            || transfer.destination.components.iter().any(|p| {
                p.is_empty() || p == "." || p == ".." || p.contains(['/', '\\', ':', '\0'])
            })
        {
            return Err(E::InvalidBinding);
        }
        let id = hash(&transfer.destination)?;
        if !destinations.insert(id.clone()) {
            return Err(E::InvalidBinding);
        }
        let matches: Vec<_> = scope
            .inspected_sources
            .iter()
            .filter(|s| s.observation.reference == source.reference)
            .collect();
        if matches.len() != 1
            || matches[0].observation != *source
            || !valid_hash(&matches[0].sha256)
        {
            return Err(E::Scope);
        }
        let expected = matches[0];
        if storage.stat(&source.reference).map_err(|_| E::Stale)? != *source {
            return Err(E::Stale);
        }
        let mut input = storage.open_read(&source.reference).map_err(|_| E::Stale)?;
        let Some(read_binding) = input.binding() else {
            return Err(E::Stale);
        };
        if Some(&read_binding.identity) != source.identity.as_ref()
            || Some(read_binding.size) != source.size
            || Some(&read_binding.modified) != source.modified.as_ref()
        {
            return Err(E::Stale);
        }
        // Streaming hash; never allocate an entire recording.
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        let mut size = 0u64;
        loop {
            let n = input.read(&mut buffer).map_err(|_| E::Stale)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
            size += n as u64;
        }
        if format!("{:x}", hasher.finalize()) != expected.sha256
            || Some(size) != source.size
            || storage.stat(&source.reference).map_err(|_| E::Stale)? != *source
        {
            return Err(E::Stale);
        }
        sources.insert(source.reference.clone());
        let mode = if plan.request.settings.copy_mode {
            TransferMode::Copy
        } else {
            TransferMode::Move
        };
        if !scope.allowed_modes.contains(&mode) {
            return Err(E::Scope);
        }
        operations.push(BoundOperation {
            id: op.id,
            depends_on: op.depends_on.clone(),
            source: expected.clone(),
            destination: DestinationEvidence {
                id,
                root: transfer.destination.root.clone(),
                components: transfer.destination.components.clone(),
            },
            mode,
            copy_effect: transfer.copy,
            duet_cohort: None,
        });
    }
    for effect in &plan.predicted_effects {
        if let PredictedEffect::DuetDeleteIntention {
            source,
            after_operations,
            ..
        } = effect
        {
            let expected: Vec<_> = operations
                .iter()
                .filter(|o| o.source.observation.reference == source.reference)
                .map(|o| o.id)
                .collect();
            if expected != *after_operations
                || expected.len() < 2
                || expected.iter().any(|i| {
                    let o = &operations[*i];
                    o.source.observation != *source
                        || !o.copy_effect
                        || o.mode != TransferMode::Move
                        || o.duet_cohort.is_some()
                })
            {
                return Err(E::InvalidBinding);
            }
            for i in &expected {
                operations[*i].duet_cohort = Some(expected.clone());
            }
        }
    }
    if operations
        .iter()
        .any(|o| o.mode == TransferMode::Move && o.copy_effect && o.duet_cohort.is_none())
    {
        return Err(E::InvalidBinding);
    }
    if sources.len() != scope.inspected_sources.len() {
        return Err(E::Scope);
    }
    let scope_sha256 = scope_digest(scope)?;
    Ok(ExecutionBinding {
        recovery_only: false,
        data: BindingData {
            schema: 2,
            album_context: None,
            plan_sha256: plan.binding_sha256.clone(),
            source_root: root,
            target_root: target,
            resources: resources.clone(),
            operations,
            scope_sha256,
            scope_evidence: ScopeEvidence {
                inspected_plan: scope.inspected_plan.clone(),
                source_root: scope.source_root.clone(),
                target_root: scope.target_root.clone(),
                inspected_sources: scope.inspected_sources.clone(),
                resources: scope.resources.clone(),
                allowed_modes: scope.allowed_modes.clone(),
            },
            synthetic_only: domain == ExecutionDomain::Synthetic,
            owned_windows_copy: domain == ExecutionDomain::OwnedWindowsCopy,
            owned_windows_rename: domain == ExecutionDomain::OwnedWindowsRename,
            selected_windows_copy: domain == ExecutionDomain::SelectedWindowsCopy,
            selected_windows_rename: domain == ExecutionDomain::SelectedWindowsRename,
        },
    })
}
// Keep the digest helper available to a host inspecting synthetic source bytes.
pub fn content_sha256(bytes: &[u8]) -> String {
    digest(bytes)
}

struct CheckedRecognition<'a> {
    inner: &'a mut dyn RecognitionProvider,
    failed: bool,
}
impl RecognitionProvider for CheckedRecognition<'_> {
    fn profile(&self) -> AiProfile {
        self.inner.profile()
    }
    fn inference_executed(&self) -> bool {
        self.inner.inference_executed()
    }
    fn diagnostics(&self) -> Vec<String> {
        self.inner.diagnostics()
    }
    fn classify(
        &mut self,
        text: &str,
    ) -> std::result::Result<sorter_core::ClassifierFact, RecognitionError> {
        let result = self.inner.classify(text);
        self.failed |= result.is_err();
        result
    }
    fn ner(&mut self, text: &str) -> std::result::Result<Vec<String>, RecognitionError> {
        let result = self.inner.ner(text);
        self.failed |= result.is_err();
        result
    }
}

/// Both IDs must belong to the same complete, validated planned duet cohort.
pub(crate) fn same_duet_cohort(prior: &BoundOperation, next: &BoundOperation) -> bool {
    prior.copy_effect
        && next.copy_effect
        && prior.source == next.source
        && prior.duet_cohort.as_ref().is_some_and(|ids| {
            ids.len() > 1
                && ids.contains(&prior.id)
                && ids.contains(&next.id)
                && next.duet_cohort.as_ref() == Some(ids)
        })
}

pub(crate) fn validate_plan_scope(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
) -> Result<(EntryObservation, EntryObservation)> {
    validate_plan_scope_domain(
        plan,
        storage,
        metadata,
        recognition,
        resources,
        scope,
        ExecutionDomain::Synthetic,
    )
}
#[allow(clippy::too_many_arguments)]
fn validate_plan_scope_domain(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
    domain: ExecutionDomain,
) -> Result<(EntryObservation, EntryObservation)> {
    verify_job_plan_binding(plan).map_err(|_| E::InvalidBinding)?;
    if plan.request.mode != JobMode::Sort || plan.operations.is_empty() {
        return Err(E::Unsupported);
    }
    if plan.blockers.iter().any(|b| {
        b != "PHYSICAL_APPLY_UNAVAILABLE"
            && b != "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS"
    }) {
        return Err(E::Unsupported);
    }
    if resources != &scope.resources
        || resources.resource_ids != plan.request.resource_ids
        || resources.adapter_versions != plan.request.adapter_versions
        || resources.ai_profile != plan.request.ai_profile
        || resources.resource_hashes.len() != resources.resource_ids.len()
        || resources.adapter_hashes.len() != resources.adapter_versions.len()
        || !resources
            .resource_hashes
            .iter()
            .chain(&resources.adapter_hashes)
            .all(|h| valid_hash(h))
    {
        return Err(E::InvalidBinding);
    }
    match &resources.ai_profile {
        AiProfile::Disabled if resources.model_hash.is_none() => {}
        AiProfile::Configured {
            available: true, ..
        } if resources.model_hash.as_deref().is_some_and(valid_hash) => {}
        _ => return Err(E::Unsupported),
    }
    if scope.inspected_plan != plan.binding_sha256 {
        return Err(E::Scope);
    }
    if domain != ExecutionDomain::Synthetic {
        let (enabled, copy_mode, mode) = match domain {
            ExecutionDomain::OwnedWindowsCopy => (
                cfg!(all(windows, feature = "experimental-windows-copy")),
                true,
                TransferMode::Copy,
            ),
            ExecutionDomain::OwnedWindowsRename => (
                cfg!(all(windows, feature = "experimental-windows-rename")),
                false,
                TransferMode::Move,
            ),
            ExecutionDomain::SelectedWindowsCopy => (
                cfg!(all(windows, feature = "selected-windows-execution")),
                true,
                TransferMode::Copy,
            ),
            ExecutionDomain::SelectedWindowsRename => (
                cfg!(all(windows, feature = "selected-windows-execution")),
                false,
                TransferMode::Move,
            ),
            ExecutionDomain::Synthetic => unreachable!(),
        };
        if !enabled
            || plan.request.settings.copy_mode != copy_mode
            || (!matches!(
                domain,
                ExecutionDomain::SelectedWindowsCopy | ExecutionDomain::SelectedWindowsRename
            ) && plan.operations.len() != 1)
            || scope.allowed_modes != [mode]
            || plan.observations.iter().any(|e| !windows(e))
        {
            return Err(E::Unsupported);
        }
    } else if plan.observations.iter().any(|e| !synthetic(e)) {
        return Err(E::Unsupported);
    }
    let mut checked = CheckedRecognition {
        inner: recognition,
        failed: false,
    };
    let fresh = plan_job(&plan.request, storage, metadata, &mut checked).map_err(|_| E::Stale)?;
    if checked.failed {
        return Err(E::Unsupported);
    }
    if fresh.binding_sha256 != plan.binding_sha256 {
        return Err(E::Stale);
    }
    let root = storage.stat(&plan.request.source).map_err(|_| E::Stale)?;
    let target = storage
        .stat(plan.request.target.as_ref().ok_or(E::InvalidBinding)?)
        .map_err(|_| E::Stale)?;
    if root.identity.as_ref() != Some(&scope.source_root)
        || target.identity.as_ref() != Some(&scope.target_root)
    {
        return Err(E::Scope);
    }
    Ok((root, target))
}

fn windows(e: &EntryObservation) -> bool {
    matches!(e.reference, StorageRef::WindowsPath { .. })
        && matches!(e.identity, Some(ObjectIdentity::Windows { .. }))
}
impl BindingData {
    pub(crate) fn windows_copy(&self) -> bool {
        self.owned_windows_copy || self.selected_windows_copy
    }
    pub(crate) fn windows_rename(&self) -> bool {
        self.owned_windows_rename || self.selected_windows_rename
    }
    /// Domain evidence is necessary but never sufficient authority to execute.
    pub(crate) fn valid_domain(&self) -> bool {
        if self.synthetic_only
            && !self.owned_windows_copy
            && !self.owned_windows_rename
            && !self.selected_windows_copy
            && !self.selected_windows_rename
        {
            return synthetic(&self.source_root)
                && synthetic(&self.target_root)
                && self.operations.iter().all(|o| {
                    synthetic(&o.source.observation)
                        && matches!(o.destination.root, StorageRef::TestOpaque { .. })
                });
        }
        let (enabled, mode, copy_effect) = match (
            self.owned_windows_copy,
            self.owned_windows_rename,
            self.selected_windows_copy,
            self.selected_windows_rename,
        ) {
            (true, false, false, false) => (
                cfg!(all(windows, feature = "experimental-windows-copy")),
                TransferMode::Copy,
                true,
            ),
            (false, true, false, false) => (
                cfg!(all(windows, feature = "experimental-windows-rename")),
                TransferMode::Move,
                false,
            ),
            (false, false, true, false) => (
                cfg!(all(windows, feature = "selected-windows-execution")),
                TransferMode::Copy,
                true,
            ),
            (false, false, false, true) => (
                cfg!(all(windows, feature = "selected-windows-execution")),
                TransferMode::Move,
                false,
            ),
            _ => return false,
        };
        !self.synthetic_only
            && enabled
            && self.album_context.is_none()
            && (self.operations.len() == 1
                || ((self.selected_windows_copy || self.selected_windows_rename)
                    && !self.operations.is_empty()))
            && windows(&self.source_root)
            && windows(&self.target_root)
            && self.scope_evidence.allowed_modes == [mode]
            && self.operations.iter().all(|o| {
                o.mode == mode
                    && o.copy_effect == copy_effect
                    && o.duet_cohort.is_none()
                    && windows(&o.source.observation)
                    && o.destination.root == self.target_root.reference
            })
    }
    pub(crate) fn domain(&self) -> ExecutionDomain {
        if self.selected_windows_rename {
            ExecutionDomain::SelectedWindowsRename
        } else if self.selected_windows_copy {
            ExecutionDomain::SelectedWindowsCopy
        } else if self.owned_windows_rename {
            ExecutionDomain::OwnedWindowsRename
        } else if self.owned_windows_copy {
            ExecutionDomain::OwnedWindowsCopy
        } else {
            ExecutionDomain::Synthetic
        }
    }
}

/// Capture content evidence at preview time. The host must retain this scope and
/// pass it unchanged at apply; recomputing it then would bless changed contents.
/// This read-only helper grants no native access and never trusts path text as consent.
pub fn prepare_scope(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    resources: &Resources,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<ExecutionScope> {
    use sha2::{Digest, Sha256};
    use std::sync::atomic::Ordering;
    let source_root = storage.stat(&plan.request.source).map_err(|_| E::Stale)?;
    let target_root = storage
        .stat(plan.request.target.as_ref().ok_or(E::Scope)?)
        .map_err(|_| E::Stale)?;
    let mut inspected_sources = Vec::new();
    let mut seen = HashSet::new();
    for operation in &plan.operations {
        if cancel.load(Ordering::Acquire) {
            return Err(E::Cancelled);
        }
        let OperationKind::SingleTransfer { source, .. } = &operation.kind else {
            return Err(E::Unsupported);
        };
        if !seen.insert(source.reference.clone()) {
            continue;
        }
        if storage.stat(&source.reference).map_err(|_| E::Stale)? != *source {
            return Err(E::Stale);
        }
        let mut input = storage.open_read(&source.reference).map_err(|_| E::Stale)?;
        let observed = input.binding().ok_or(E::Stale)?;
        if Some(&observed.identity) != source.identity.as_ref()
            || Some(observed.size) != source.size
            || Some(&observed.modified) != source.modified.as_ref()
        {
            return Err(E::Stale);
        }
        let length = source.size.ok_or(E::Stale)?;
        let mut read = 0u64;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err(E::Cancelled);
            }
            let n = input.read(&mut buffer).map_err(|_| E::Stale)?;
            if n == 0 {
                break;
            }
            read = read.checked_add(n as u64).ok_or(E::Stale)?;
            if read > length {
                return Err(E::Stale);
            }
            hash.update(&buffer[..n]);
        }
        if read != length || storage.stat(&source.reference).map_err(|_| E::Stale)? != *source {
            return Err(E::Stale);
        }
        inspected_sources.push(SourceEvidence {
            observation: source.clone(),
            sha256: format!("{:x}", hash.finalize()),
        });
    }
    Ok(ExecutionScope {
        inspected_plan: plan.binding_sha256.clone(),
        source_root: source_root.identity.ok_or(E::Scope)?,
        target_root: target_root.identity.ok_or(E::Scope)?,
        inspected_sources,
        resources: resources.clone(),
        allowed_modes: vec![if plan.request.settings.copy_mode {
            TransferMode::Copy
        } else {
            TransferMode::Move
        }],
    })
}
