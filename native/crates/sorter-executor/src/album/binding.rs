use crate::{ExecutionError as E, Result, binding::*, hash};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sorter_core::{discovery::*, job_plan::*, metadata::MetadataProvider};
use std::{collections::HashSet, io::Read};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryRequirement {
    /// Complete path from the target root; empty path is the target root itself.
    pub components: Vec<String>,
    /// None requires exclusive creation. Some requires this exact existing parent.
    pub existing: Option<EntryObservation>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AlbumData {
    pub schema: u16,
    pub source: EntryObservation,
    pub directories: Vec<DirectoryRequirement>,
    pub transfers: BindingData,
}
/// No Deserialize and no public fields: serialized evidence cannot mint a grant.
#[derive(Clone, Debug)]
pub struct AlbumBinding {
    pub(super) data: AlbumData,
    pub(super) recovery_only: bool,
}
impl AlbumBinding {
    pub fn sha256(&self) -> Result<String> {
        hash(&self.data)
    }
    pub fn source(&self) -> &EntryObservation {
        &self.data.source
    }
    pub fn directories(&self) -> &[DirectoryRequirement] {
        &self.data.directories
    }
    pub fn children(&self) -> impl Iterator<Item = &SourceEvidence> {
        self.data.transfers.operations.iter().map(|o| &o.source)
    }
    pub(super) fn transfers(&self) -> ExecutionBinding {
        ExecutionBinding {
            data: self.data.transfers.clone(),
            recovery_only: self.recovery_only,
        }
    }
}
pub(super) fn synthetic(entry: &EntryObservation) -> bool {
    matches!(entry.reference, StorageRef::TestOpaque { .. })
        && matches!(entry.identity, Some(ObjectIdentity::Test { .. }))
}
/// Conservative executor limitation, not a change to planner sanitization policy.
fn supported_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':', '\0', '<', '>', '"', '|', '?', '*'])
        && !name.ends_with(['.', ' '])
        && !name.chars().any(char::is_control)
        && !matches!(
            name.split('.')
                .next()
                .unwrap()
                .to_ascii_uppercase()
                .as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}
/// Validate exactly one flat album Copy operation against the unchanged planner.
/// Mixed jobs, album moves and recursive children remain unsupported capabilities.
/// Every source child (including non-audio sidecars) must be represented in order.
pub fn validate_album_copy(
    plan: &JobPlanV2,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
) -> Result<AlbumBinding> {
    let (root, target) =
        validate_plan_scope(plan, storage, metadata, recognition, resources, scope)?;
    if plan.operations.len() != 1
        || !plan.request.settings.copy_mode
        || !scope.allowed_modes.contains(&TransferMode::Copy)
    {
        return Err(E::Unsupported);
    }
    if root.kind != EntryKind::Directory
        || target.kind != EntryKind::Directory
        || !synthetic(&root)
        || !synthetic(&target)
        || root.identity == target.identity
    {
        return Err(E::Unsupported);
    }
    let op = &plan.operations[0];
    let OperationKind::AlbumTransfer {
        source,
        destination,
        children,
        remove_empty_source,
    } = &op.kind
    else {
        return Err(E::Unsupported);
    };
    if op.id != 0
        || !op.depends_on.is_empty()
        || !op.stop_and_replan_on_effect_mismatch
        || *remove_empty_source
        || source.kind != EntryKind::Directory
        || !synthetic(source)
        || destination.root != target.reference
        || destination.components.is_empty()
        || destination.components.iter().any(|s| !supported_name(s))
        || children.is_empty()
    {
        return Err(E::Unsupported);
    }
    if storage.stat(&source.reference).map_err(|_| E::Stale)? != *source {
        return Err(E::Stale);
    }
    let actual = storage.children(&source.reference).map_err(|_| E::Stale)?;
    if actual
        != children
            .iter()
            .map(|c| c.source.clone())
            .collect::<Vec<_>>()
    {
        return Err(E::Stale);
    }
    let mut names = HashSet::new();
    let mut identities = HashSet::new();
    let mut references = HashSet::new();
    let mut operations = vec![];
    for (id, child) in children.iter().enumerate() {
        let e = &child.source;
        if !child.copy
            || !child.predicted_success
            || !child.blockers.is_empty()
            || e.kind != EntryKind::File
            || !synthetic(e)
            || e.size.is_none()
            || e.modified.is_none()
            || e.identity == source.identity
            || e.identity == root.identity
            || e.parent_identity != source.identity
            || !supported_name(&e.name)
            || child.destination.root != target.reference
            || child.destination.components.len() != destination.components.len() + 1
            || !child
                .destination
                .components
                .starts_with(&destination.components)
            || !supported_name(child.destination.components.last().unwrap())
            || !names.insert(child.destination.components.last().unwrap().to_lowercase())
            || !identities.insert(e.identity.clone())
            || !references.insert(e.reference.clone())
        {
            return Err(E::Unsupported);
        }
        let inspected: Vec<_> = scope
            .inspected_sources
            .iter()
            .filter(|s| s.observation == *e)
            .collect();
        if inspected.len() != 1 {
            return Err(E::Scope);
        }
        let inspected = inspected[0];
        let mut reader = storage.open_read(&e.reference).map_err(|_| E::Stale)?;
        let rb = reader.binding().ok_or(E::Stale)?;
        if Some(&rb.identity) != e.identity.as_ref()
            || Some(rb.size) != e.size
            || Some(&rb.modified) != e.modified.as_ref()
        {
            return Err(E::Stale);
        }
        let mut digest = Sha256::new();
        let mut length = 0u64;
        let mut bytes = [0; 8192];
        loop {
            let n = reader.read(&mut bytes).map_err(|_| E::Stale)?;
            if n == 0 {
                break;
            }
            length = length.checked_add(n as u64).ok_or(E::Stale)?;
            if Some(length) > e.size {
                return Err(E::Stale);
            }
            digest.update(&bytes[..n]);
        }
        if Some(length) != e.size
            || format!("{:x}", digest.finalize()) != inspected.sha256
            || storage.stat(&e.reference).map_err(|_| E::Stale)? != *e
        {
            return Err(E::Stale);
        }
        operations.push(BoundOperation {
            id,
            depends_on: if id == 0 { vec![] } else { vec![id - 1] },
            source: inspected.clone(),
            destination: DestinationEvidence {
                id: hash(&child.destination)?,
                root: child.destination.root.clone(),
                components: child.destination.components.clone(),
            },
            mode: TransferMode::Copy,
            copy_effect: true,
            duet_cohort: None,
        });
    }
    if scope.inspected_sources.len() != children.len() {
        return Err(E::Scope);
    }
    if storage.children(&source.reference).map_err(|_| E::Stale)? != actual {
        return Err(E::Stale);
    }
    let mut directories = vec![DirectoryRequirement {
        components: vec![],
        existing: Some(target.clone()),
    }];
    let mut current = Some(target.clone());
    for n in 1..=destination.components.len() {
        let components = destination.components[..n].to_vec();
        let found = if let Some(parent) = &current {
            let entries = storage.children(&parent.reference).map_err(|_| E::Stale)?;
            let same: Vec<_> = entries
                .iter()
                .filter(|e| e.name.to_lowercase() == components[n - 1].to_lowercase())
                .collect();
            if same.len() > 1 {
                return Err(E::Stale);
            }
            same.first().map(|e| (*e).clone())
        } else {
            None
        };
        if let Some(e) = &found
            && (n == destination.components.len()
                || e.kind != EntryKind::Directory
                || !synthetic(e)
                || e.name != components[n - 1]
                || e.parent_identity != current.as_ref().unwrap().identity)
        {
            return Err(E::Stale);
        }
        directories.push(DirectoryRequirement {
            components,
            existing: found.clone(),
        });
        current = found;
    }
    let mut parents = HashSet::new();
    for directory in &directories {
        if let Some(existing) = &directory.existing
            && (!parents.insert(existing.identity.clone())
                || existing.identity == source.identity
                || existing.identity == root.identity
                || operations
                    .iter()
                    .any(|o| o.source.observation.identity == existing.identity))
        {
            return Err(E::Unsupported);
        }
    }
    let context = hash(&(source, &directories, &operations))?;
    Ok(AlbumBinding {
        recovery_only: false,
        data: AlbumData {
            schema: 1,
            source: source.clone(),
            directories,
            transfers: BindingData {
                copy_metadata_version:None,
                schema: 2,
                album_context: Some(context),
                plan_sha256: plan.binding_sha256.clone(),
                source_root: root,
                target_root: target,
                resources: resources.clone(),
                operations,
                scope_sha256: scope_digest(scope)?,
                scope_evidence: ScopeEvidence {
                    inspected_plan: scope.inspected_plan.clone(),
                    source_root: scope.source_root.clone(),
                    target_root: scope.target_root.clone(),
                    inspected_sources: scope.inspected_sources.clone(),
                    resources: scope.resources.clone(),
                    allowed_modes: scope.allowed_modes.clone(),
                },
                synthetic_only: true,
                owned_windows_copy: false,
                owned_windows_rename: false,
                selected_windows_copy: false,
                selected_windows_rename: false,
            },
        },
    })
}
