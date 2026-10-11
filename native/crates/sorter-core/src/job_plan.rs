//! V2 ordered, read-only planning. All effects are predictions, never execution.
use crate::album::{AlbumDecision, FolderObservation, analyze_album};
use crate::discovery::*;
use crate::metadata::MetadataProvider;
use crate::{ClassifierFact, Settings};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
mod singles;
pub use singles::single_decision;
mod timeline;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobMode {
    Sort,
    FixNames,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProfile {
    Disabled,
    Configured { bundle_id: String, available: bool },
}
pub trait RecognitionProvider {
    fn profile(&self) -> AiProfile;
    fn classify(&mut self, text: &str) -> Result<ClassifierFact, String>;
    fn ner(&mut self, text: &str) -> Result<Vec<String>, String>;
}
pub struct DisabledRecognition;
impl RecognitionProvider for DisabledRecognition {
    fn profile(&self) -> AiProfile {
        AiProfile::Disabled
    }
    fn classify(&mut self, _: &str) -> Result<ClassifierFact, String> {
        Err("AI intentionally disabled".into())
    }
    fn ner(&mut self, _: &str) -> Result<Vec<String>, String> {
        Err("AI intentionally disabled".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobRequestV2 {
    pub mode: JobMode,
    pub source: StorageRef,
    pub target: Option<StorageRef>,
    pub settings: Settings,
    /// Lossless CSV rows, bundled first. Width errors remain lazy lookup errors.
    pub alias_rows: Vec<Vec<String>>,
    pub resource_ids: Vec<String>,
    pub adapter_versions: Vec<String>,
    pub ai_profile: AiProfile,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct JobError {
    pub code: String,
    pub detail: String,
}
impl From<StorageError> for JobError {
    fn from(e: StorageError) -> Self {
        Self {
            code: format!("STORAGE_{:?}", e.kind),
            detail: e.detail,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ReadPoint {
    pub source: EntryObservation,
    pub after_operations: usize,
    pub depends_on: Vec<usize>,
    pub virtually_present: bool,
}
#[derive(Clone, Debug, Serialize)]
pub enum DecisionV2 {
    Album {
        read: ReadPoint,
        decision: AlbumDecision,
    },
    Single {
        read: ReadPoint,
        artists: Vec<String>,
    },
    ItemError {
        read: ReadPoint,
        error: String,
    },
    Vanished {
        read: ReadPoint,
    },
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Destination {
    pub root: StorageRef,
    pub components: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct TransferIntent {
    pub source: EntryObservation,
    pub destination: Destination,
    pub copy: bool,
    pub predicted_success: bool,
    pub blockers: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub enum OperationKind {
    AlbumTransfer {
        source: EntryObservation,
        destination: Destination,
        children: Vec<TransferIntent>,
        remove_empty_source: bool,
    },
    SingleTransfer {
        source: EntryObservation,
        artist: String,
        transfer: Box<TransferIntent>,
    },
    FixFile(crate::fix_plan::FixFilePlan),
}
#[derive(Clone, Debug, Serialize)]
pub struct OperationV2 {
    pub id: usize,
    pub kind: OperationKind,
    pub depends_on: Vec<usize>,
    /// Any difference from a predicted effect invalidates later dependent work.
    pub stop_and_replan_on_effect_mismatch: bool,
}
#[derive(Clone, Debug, Serialize)]
pub enum PredictedEffect {
    TargetCreated {
        operation: usize,
        destination: Destination,
        kind: EntryKind,
    },
    SourceRenamed {
        operation: usize,
        source: EntryObservation,
        intended_name: String,
    },
    SourceUnavailable {
        operation: usize,
        source: StorageRef,
    },
    /// Historical intent only. Never grants source-delete eligibility.
    DuetDeleteIntention {
        source: EntryObservation,
        after_operations: Vec<usize>,
        blocked: bool,
    },
}
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct PredictedCounters {
    pub songs_sorted: usize,
    pub albums_processed: usize,
    pub artist_folders_created: usize,
    pub top_artists: Vec<(String, usize)>,
}
#[derive(Clone, Debug, Serialize)]
pub struct JobPlanV2 {
    pub schema_version: u16,
    pub ruleset_id: String,
    /// Integrity of this observed plan, not a claim that audio bytes are unchanged.
    pub binding_sha256: String,
    pub content_binding: String,
    pub executable: bool,
    pub request: JobRequestV2,
    pub observations: Vec<EntryObservation>,
    pub metadata_observations: Vec<crate::metadata::MetadataObservation>,
    pub precollected_visits: Vec<StorageRef>,
    pub decisions: Vec<DecisionV2>,
    pub operations: Vec<OperationV2>,
    pub predicted_effects: Vec<PredictedEffect>,
    pub predicted_legacy_counters: PredictedCounters,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}
#[derive(Clone)]
struct Node {
    entry: EntryObservation,
    path: Vec<String>,
    children: Vec<usize>,
    listed: bool,
}
#[derive(Default)]
struct Tree {
    nodes: Vec<Node>,
    visits: Vec<usize>,
}
impl Tree {
    fn collect(
        storage: &mut dyn ReadStorage,
        root: EntryObservation,
        recurse: bool,
    ) -> Result<Self, JobError> {
        let mut tree = Self::default();
        tree.add(storage, root, vec![], recurse, &mut HashSet::new())?;
        Ok(tree)
    }
    fn add(
        &mut self,
        storage: &mut dyn ReadStorage,
        entry: EntryObservation,
        path: Vec<String>,
        recurse: bool,
        seen: &mut HashSet<StorageRef>,
    ) -> Result<usize, JobError> {
        if !seen.insert(entry.reference.clone()) {
            return Err(JobError {
                code: "CYCLIC_ENUMERATION".into(),
                detail: "Repeated directory reference".into(),
            });
        }
        let i = self.nodes.len();
        self.nodes.push(Node {
            entry: entry.clone(),
            path: path.clone(),
            children: vec![],
            listed: true,
        });
        if entry.kind == EntryKind::Directory {
            self.visits.push(i);
            let children = storage.children(&entry.reference)?;
            // Preserve provider encounter order, including files amongst directories.
            for child in children {
                let mut p = path.clone();
                p.push(child.name.clone());
                let c = self.nodes.len();
                self.nodes.push(Node {
                    entry: child,
                    path: p,
                    children: vec![],
                    listed: false,
                });
                self.nodes[i].children.push(c);
            }
            if recurse {
                let children = self.nodes[i].children.clone();
                for c in children {
                    if self.nodes[c].entry.kind == EntryKind::Directory {
                        self.expand(storage, c, seen)?;
                    }
                }
            }
        }
        Ok(i)
    }
    fn expand(
        &mut self,
        storage: &mut dyn ReadStorage,
        i: usize,
        seen: &mut HashSet<StorageRef>,
    ) -> Result<(), JobError> {
        let node = self.nodes[i].clone();
        if !seen.insert(node.entry.reference.clone()) {
            return Err(JobError {
                code: "CYCLIC_ENUMERATION".into(),
                detail: "Repeated directory reference".into(),
            });
        }
        self.visits.push(i);
        self.nodes[i].listed = true;
        for child in storage.children(&node.entry.reference)? {
            let mut p = node.path.clone();
            p.push(child.name.clone());
            let c = self.nodes.len();
            self.nodes.push(Node {
                entry: child,
                path: p,
                children: vec![],
                listed: false,
            });
            self.nodes[i].children.push(c);
        }
        for c in self.nodes[i].children.clone() {
            if self.nodes[c].entry.kind == EntryKind::Directory {
                self.expand(storage, c, seen)?;
            }
        }
        Ok(())
    }
}
struct State {
    plan: JobPlanV2,
    source: Tree,
    removed: BTreeMap<usize, usize>,
    target: BTreeMap<Vec<String>, (EntryKind, Option<usize>)>,
    artist_counts: Vec<(String, usize)>,
    created_artists: HashSet<String>,
    metadata_clock: std::rc::Rc<std::cell::Cell<usize>>,
}
impl State {
    fn read(&self, index: usize) -> ReadPoint {
        ReadPoint {
            source: self.source.nodes[index].entry.clone(),
            after_operations: self.plan.operations.len(),
            depends_on: (0..self.plan.operations.len()).collect(),
            virtually_present: !self.removed.contains_key(&index),
        }
    }
    fn count(&mut self, artist: &str, count: usize) {
        if let Some((_, n)) = self.artist_counts.iter_mut().find(|(a, _)| a == artist) {
            *n += count;
        } else {
            self.artist_counts.push((artist.into(), count));
        }
    }
    fn destination(&self, components: Vec<String>) -> Destination {
        Destination {
            root: self
                .plan
                .request
                .target
                .clone()
                .expect("sort validated target"),
            components,
        }
    }
    fn op(&mut self, kind: OperationKind) -> usize {
        let id = self.plan.operations.len();
        self.plan.operations.push(OperationV2 {
            id,
            kind,
            depends_on: (0..id).collect(),
            stop_and_replan_on_effect_mismatch: true,
        });
        self.metadata_clock.set(id + 1);
        id
    }
    fn destination_blockers(&self, path: &[String]) -> Vec<String> {
        let mut blockers = path_blockers(path);
        if (1..=path.len()).any(|n| {
            self.target.keys().any(|old| {
                old.len() == n
                    && old != &path[..n]
                    && old
                        .iter()
                        .zip(&path[..n])
                        .all(|(a, b)| a.to_lowercase() == b.to_lowercase())
            })
        }) {
            blockers.push("CASE_COLLISION_REQUIRES_PROVIDER_RESOLUTION".into());
        }
        blockers
    }
    fn can_mkdir(&self, path: &[String]) -> bool {
        (1..=path.len()).all(|n| {
            self.target
                .get(&path[..n])
                .is_none_or(|(kind, _)| *kind == EntryKind::Directory)
        })
    }
    fn mkdir(&mut self, path: &[String], id: usize) {
        for len in 1..=path.len() {
            let p = path[..len].to_vec();
            if !self.target.contains_key(&p) {
                self.target
                    .insert(p.clone(), (EntryKind::Directory, Some(id)));
                self.plan
                    .predicted_effects
                    .push(PredictedEffect::TargetCreated {
                        operation: id,
                        destination: self.destination(p),
                        kind: EntryKind::Directory,
                    });
            }
        }
    }
    fn remove(&mut self, index: usize, id: usize) {
        if self.removed.contains_key(&index) {
            return;
        }
        self.removed.insert(index, id);
        self.plan
            .predicted_effects
            .push(PredictedEffect::SourceUnavailable {
                operation: id,
                source: self.source.nodes[index].entry.reference.clone(),
            });
        for c in self.source.nodes[index].children.clone() {
            self.remove(c, id);
        }
    }
}
fn audio(name: &str) -> bool {
    crate::album::is_audio(name)
}
fn path_blockers(path: &[String]) -> Vec<String> {
    let mut result = vec![];
    if path
        .iter()
        .any(|p| p.is_empty() || p == "." || p == ".." || p.contains(['/', '\\', ':', '\0']))
    {
        result.push("UNSAFE_RAW_DESTINATION".into());
    }
    if path.iter().any(|p| crate::fix_plan::unsafe_name(p)) {
        result.push("UNSAFE_WINDOWS_NAME".into());
    }
    result
}
/// Read-only v2 entry point. References remain lossless; display names are used
/// only for logical destination intent, never to manufacture a storage reference.
pub fn plan_job(
    request: &JobRequestV2,
    storage: &mut dyn ReadStorage,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
) -> Result<JobPlanV2, JobError> {
    if recognition.profile() != request.ai_profile {
        return Err(JobError {
            code: "RECOGNITION_PROFILE_MISMATCH".into(),
            detail: "Requested and provider AI identities differ".into(),
        });
    }
    if matches!(
        request.ai_profile,
        AiProfile::Configured {
            available: false,
            ..
        }
    ) {
        return Err(JobError {
            code: "RECOGNITION_UNAVAILABLE".into(),
            detail: "Configured AI is unavailable; it is not disabled".into(),
        });
    }
    let root = storage.stat(&request.source)?;
    if root.kind != EntryKind::Directory {
        return Err(JobError {
            code: "SOURCE_NOT_DIRECTORY".into(),
            detail: "Source must be a directory".into(),
        });
    }
    let target_root = if request.mode == JobMode::Sort {
        let r = request.target.as_ref().ok_or_else(|| JobError {
            code: "TARGET_REQUIRED".into(),
            detail: "Sort requires a target".into(),
        })?;
        let target = storage.stat(r)?;
        if target.kind != EntryKind::Directory {
            return Err(JobError {
                code: "TARGET_NOT_DIRECTORY".into(),
                detail: "Target must be a directory".into(),
            });
        }
        if root.reference == target.reference
            || (root.identity.is_some() && root.identity == target.identity)
        {
            return Err(JobError {
                code: "SAME_ROOT".into(),
                detail: "Source and target identify the same object".into(),
            });
        }
        Some(target)
    } else {
        None
    };
    let source = Tree::collect(storage, root, !request.settings.main_folder_only)?;
    if source.nodes[0].children.is_empty() && request.mode == JobMode::Sort {
        return Err(JobError {
            code: "SOURCE_EMPTY".into(),
            detail: "Released sort rejects empty source".into(),
        });
    }
    let target_tree = target_root
        .map(|r| Tree::collect(storage, r, true))
        .transpose()?;
    let mut observations = source
        .nodes
        .iter()
        .map(|n| n.entry.clone())
        .collect::<Vec<_>>();
    let mut target = BTreeMap::new();
    if let Some(t) = target_tree {
        for n in t.nodes {
            target.insert(n.path, (n.entry.kind, None));
            observations.push(n.entry);
        }
    }
    let overlap = observed_overlap(&observations, &request.source, request.target.as_ref());
    let visits = if request.settings.main_folder_only {
        source.nodes[0].children.clone()
    } else {
        source.visits.clone()
    };
    let mut blockers = vec!["PHYSICAL_APPLY_UNAVAILABLE".into()];
    if overlap {
        blockers.push("SOURCE_TARGET_OVERLAP".into());
    }
    if request.alias_rows.iter().any(|r| r.len() != 2) {
        blockers.push("MALFORMED_ALIAS_ROWS".into());
    }
    blockers.extend(audit_observations(&observations));
    blockers.sort();
    blockers.dedup();
    let clock = std::rc::Rc::new(std::cell::Cell::new(0));
    let recorded = std::rc::Rc::new(std::cell::RefCell::new(vec![]));
    let mut metadata = crate::metadata::RecordingProvider {
        inner: metadata,
        clock: clock.clone(),
        observations: recorded.clone(),
    };
    let mut state = State {
        plan: JobPlanV2 {
            schema_version: 2,
            ruleset_id: crate::RULESET_ID.into(),
            binding_sha256: String::new(),
            content_binding:
                "identity-size-modified-and-metadata-observations; audio-byte-digests-not-observed"
                    .into(),
            executable: false,
            request: request.clone(),
            observations,
            metadata_observations: vec![],
            precollected_visits: visits
                .iter()
                .map(|&i| source.nodes[i].entry.reference.clone())
                .collect(),
            decisions: vec![],
            operations: vec![],
            predicted_effects: vec![],
            predicted_legacy_counters: PredictedCounters::default(),
            warnings: vec![
                "Effects and counters are predicted; no files or tags were changed".into(),
            ],
            blockers,
        },
        source,
        removed: BTreeMap::new(),
        target,
        artist_counts: vec![],
        created_artists: HashSet::new(),
        metadata_clock: clock,
    };
    if matches!(request.ai_profile, AiProfile::Disabled) {
        state
            .plan
            .warnings
            .push("AI intentionally disabled; no native inference bundle is used".into());
    }
    if request.mode == JobMode::FixNames {
        timeline::fix(&mut state, &mut metadata)?;
    } else {
        timeline::sort(&mut state, visits, storage, &mut metadata, recognition)?;
    }
    state.artist_counts.sort_by_key(|a| std::cmp::Reverse(a.1));
    state.artist_counts.truncate(5);
    state.plan.predicted_legacy_counters.top_artists = state.artist_counts;
    state.plan.predicted_legacy_counters.artist_folders_created = state.created_artists.len();
    state.plan.metadata_observations = recorded.take();
    state
        .plan
        .blockers
        .extend(audit_observations(&state.plan.observations));
    if observed_overlap(
        &state.plan.observations,
        &request.source,
        request.target.as_ref(),
    ) {
        state.plan.blockers.push("SOURCE_TARGET_OVERLAP".into());
    }
    if state.plan.metadata_observations.iter().any(|o|matches!(o,crate::metadata::MetadataObservation::Open{result:Err(e),..} if !matches!(e,crate::metadata::MetadataError::LegacyOpen{..}))){state.plan.blockers.push("METADATA_ADAPTER_LIMITATION".into());}
    state.plan.blockers.sort();
    state.plan.blockers.dedup();
    state.plan.binding_sha256 = binding_digest(&state.plan)?;
    Ok(state.plan)
}

fn observed_overlap(
    observations: &[EntryObservation],
    source: &StorageRef,
    target: Option<&StorageRef>,
) -> bool {
    let Some(target) = target else {
        return false;
    };
    let Some(a) = observations.iter().find(|e| &e.reference == source) else {
        return false;
    };
    let Some(b) = observations.iter().find(|e| &e.reference == target) else {
        return false;
    };
    let reaches = |child: &EntryObservation, ancestor: &EntryObservation| {
        let Some(goal) = &ancestor.identity else {
            return false;
        };
        let mut next = child.parent_identity.as_ref();
        let mut seen = HashSet::new();
        while let Some(identity) = next {
            if identity == goal {
                return true;
            }
            if !seen.insert(identity) {
                return false;
            }
            next = observations
                .iter()
                .find(|e| e.identity.as_ref() == Some(identity))
                .and_then(|e| e.parent_identity.as_ref());
        }
        false
    };
    reaches(a, b) || reaches(b, a)
}
fn audit_observations(observations: &[EntryObservation]) -> Vec<String> {
    let mut blockers = vec![];
    let mut names = std::collections::HashMap::new();
    let mut cases = std::collections::HashMap::new();
    let mut identities = std::collections::HashMap::new();
    for e in observations {
        if e.identity.is_none() {
            blockers.push("MISSING_IDENTITY".into());
        }
        if e.kind == EntryKind::LinkOrReparse {
            blockers.push("LINK_OR_REPARSE".into());
        }
        if e.capabilities.read.is_none()
            || e.capabilities.list.is_none()
            || e.capabilities.seek.is_none()
        {
            blockers.push("UNKNOWN_CAPABILITY".into());
        }
        if names
            .insert(
                (e.parent_identity.clone(), e.name.clone()),
                e.reference.clone(),
            )
            .is_some_and(|old| old != e.reference)
        {
            blockers.push("AMBIGUOUS_DISPLAY_NAME".into());
        }
        if cases
            .insert(
                (e.parent_identity.clone(), e.name.to_lowercase()),
                e.name.clone(),
            )
            .is_some_and(|old| old != e.name)
        {
            blockers.push("CASE_COLLISION_REQUIRES_PROVIDER_RESOLUTION".into());
        }
        if e.kind == EntryKind::File
            && let Some(identity) = &e.identity
            && identities
                .insert(identity.clone(), e.reference.clone())
                .is_some_and(|old| old != e.reference)
        {
            blockers.push("HARD_LINK_IDENTITY".into());
        }
    }
    blockers
}

fn binding_digest(plan: &JobPlanV2) -> Result<String, JobError> {
    use sha2::{Digest, Sha256};
    let mut value = serde_json::to_value(plan).map_err(|e| JobError {
        code: "BINDING_SERIALIZATION".into(),
        detail: e.to_string(),
    })?;
    value["binding_sha256"] = serde_json::Value::String(String::new());
    let bytes = serde_json::to_vec(&value).map_err(|e| JobError {
        code: "BINDING_SERIALIZATION".into(),
        detail: e.to_string(),
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
/// Integrity verification only. Re-reading sources and validating effect
/// preconditions remain mandatory in any future executor; this never enables apply.
pub fn verify_job_plan_binding(plan: &JobPlanV2) -> Result<(), JobError> {
    if plan.executable || plan.schema_version != 2 || binding_digest(plan)? != plan.binding_sha256 {
        return Err(JobError {
            code: "JOB_BINDING_MISMATCH".into(),
            detail: "Observed plan content was changed or execution eligibility was altered".into(),
        });
    }
    Ok(())
}
