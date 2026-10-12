//! Selected Windows album jobs consume the unchanged planner's operations.
//! Membership is a checked planned-content snapshot, never an exclusion claim.
use crate::{
    ExecutionError as E, Result,
    binding::*,
    physical_journal::{JournalStore, PhysicalJournalV2},
    results::*,
    transfer::{TransferProgress, TransferStorage},
    windows_copy::WindowsPlanningStorage,
    windows_journal::WindowsJournalStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sorter_core::{discovery::*, job_plan::*, metadata::MetadataProvider};
use sorter_io::desktop::windows_mutation::*;
use sorter_storage_contracts::*;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    io::{self, Read},
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
            .split(|u| *u == 47 || *u == 92)
            .any(|p| p == [46] || p == [46, 46])
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
fn name(value: &str) -> MutationName {
    MutationName::Windows(value.encode_utf16().collect())
}
fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Acquire) {
        Err(E::Cancelled)
    } else {
        Ok(())
    }
}
fn failure(detail: &str) -> MutationError {
    MutationError::new(MutationErrorKind::StalePlan, detail)
}
fn leaf(entry: &EntryObservation) -> Result<MutationName> {
    Ok(MutationName::Windows(
        path(&entry.reference)?
            .file_name()
            .ok_or(E::Scope)?
            .encode_wide()
            .collect(),
    ))
}
fn expected(source: &SourceEvidence) -> Result<ExpectedSource> {
    let mut sha256 = [0; 32];
    if source.sha256.len() != 64 {
        return Err(E::InvalidBinding);
    }
    for (i, b) in sha256.iter_mut().enumerate() {
        *b = u8::from_str_radix(&source.sha256[i * 2..i * 2 + 2], 16)
            .map_err(|_| E::InvalidBinding)?;
    }
    Ok(ExpectedSource {
        identity: source.observation.identity.clone().ok_or(E::Scope)?,
        parent_identity: source.observation.parent_identity.clone().ok_or(E::Scope)?,
        name: leaf(&source.observation)?,
        size: source.observation.size,
        modified: source.observation.modified.clone(),
        digest: ContentDigest {
            sha256,
            length: source.observation.size.ok_or(E::Scope)?,
        },
    })
}
fn parent_reference(entry: &EntryObservation) -> Result<StorageRef> {
    Ok(reference(path(&entry.reference)?.parent().ok_or(E::Scope)?))
}
fn destination_path(destination: &Destination) -> Result<PathBuf> {
    let mut value = path(&destination.root)?;
    for component in &destination.components {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.contains(['/', '\\', ':', '\0'])
        {
            return Err(E::Unsupported);
        }
        value.push(component);
    }
    Ok(value)
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct DirectorySnapshot {
    source: EntryObservation,
    children: Vec<EntryObservation>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct DirectoryRequirement {
    destination: Destination,
    existing: Option<EntryObservation>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct FileIntent {
    original_operation: usize,
    source: EntryObservation,
    destination: Destination,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct AlbumChildPlan {
    source: EntryObservation,
    destination_name: String,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct AlbumGroup {
    children: Vec<AlbumChildPlan>,
    operation: usize,
    artist: String,
    source: EntryObservation,
    destination: Destination,
    files: Vec<usize>,
    cleanup: Vec<EntryObservation>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct CopiedDirectory {
    operation: usize,
    source: EntryObservation,
    destination: Destination,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Snapshot {
    copied_directories: Vec<CopiedDirectory>,
    directories: Vec<DirectorySnapshot>,
    destinations: Vec<DirectoryRequirement>,
    files: Vec<FileIntent>,
    albums: Vec<AlbumGroup>,
}
/// Retained preview evidence. Deliberately no Deserialize or mutation authority.
#[derive(Clone, Debug)]
pub struct AlbumScope {
    execution: ExecutionScope,
    snapshot: Snapshot,
}
impl AlbumScope {
    pub fn execution_scope(&self) -> &ExecutionScope {
        &self.execution
    }
    pub fn resources(&self) -> &Resources {
        &self.execution.resources
    }
}

pub fn supported_album_plan(plan: &JobPlanV2) -> Result<()> {
    verify_job_plan_binding(plan).map_err(|_| E::InvalidBinding)?;
    if plan.observations.len() > 100_000 {
        return Err(E::Unsupported);
    }
    let source_depth = path(&plan.request.source)?.components().count();
    let mut observed_units = 0usize;
    for entry in &plan.observations {
        let entry_path = path(&entry.reference)?;
        if entry_path.starts_with(path(&plan.request.source)?)
            && entry_path.components().count().saturating_sub(source_depth) > 128
        {
            return Err(E::Unsupported);
        }
        observed_units = observed_units
            .checked_add(entry_path.as_os_str().encode_wide().count())
            .filter(|n| *n <= 16 * 1024 * 1024)
            .ok_or(E::Unsupported)?;
    }
    if plan.request.mode != JobMode::Sort
        || plan.request.target.is_none()
        || !plan
            .operations
            .iter()
            .any(|op| matches!(op.kind, OperationKind::AlbumTransfer { .. }))
    {
        return Err(E::Unsupported);
    }
    if plan.blockers.iter().any(|b| {
        b != "PHYSICAL_APPLY_UNAVAILABLE"
            && b != "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS"
    }) {
        return Err(E::Unsupported);
    }
    for (index, op) in plan.operations.iter().enumerate() {
        if op.id != index
            || !op.stop_and_replan_on_effect_mismatch
            || op.depends_on.iter().any(|d| *d >= index)
        {
            return Err(E::InvalidBinding);
        }
        let children: Vec<&TransferIntent> = match &op.kind {
            OperationKind::SingleTransfer {
                source, transfer, ..
            } if source == &transfer.source => vec![transfer],
            OperationKind::AlbumTransfer {
                source, children, ..
            } if source.kind == EntryKind::Directory => children.iter().collect(),
            _ => return Err(E::Unsupported),
        };
        for child in children {
            if !child.predicted_success
                || !child.blockers.is_empty()
                || child.copy != plan.request.settings.copy_mode
                || !matches!(child.source.kind, EntryKind::File | EntryKind::Directory)
            {
                return Err(E::Unsupported);
            }
            destination_path(&child.destination)?;
        }
    }
    Ok(())
}
struct Collector<'a> {
    plan: &'a JobPlanV2,
    reads: &'a mut dyn ReadStorage,
    cancel: &'a AtomicBool,
    snapshot: Snapshot,
    watched: HashSet<StorageRef>,
    destinations: HashSet<StorageRef>,
    entries: usize,
    path_units: usize,
}
impl Collector<'_> {
    fn directory(&mut self, source: &EntryObservation) -> Result<Vec<EntryObservation>> {
        cancelled(self.cancel)?;
        if self.reads.stat(&source.reference).map_err(|_| E::Stale)? != *source {
            return Err(E::Stale);
        }
        let children = self
            .reads
            .children(&source.reference)
            .map_err(|_| E::Stale)?;
        if !self.watched.contains(&source.reference) {
            self.entries = self
                .entries
                .checked_add(children.len())
                .filter(|n| *n <= 100_000)
                .ok_or(E::Unsupported)?;
            for child in &children {
                self.path_units = self
                    .path_units
                    .checked_add(path(&child.reference)?.as_os_str().encode_wide().count())
                    .filter(|n| *n <= 16 * 1024 * 1024)
                    .ok_or(E::Unsupported)?;
            }
        }
        for child in &children {
            if child.parent_identity != source.identity
                || !matches!(child.kind, EntryKind::Directory | EntryKind::File)
                || !path(&child.reference)?.starts_with(path(&self.plan.request.source)?)
            {
                return Err(E::Unsupported);
            }
        }
        if self.watched.insert(source.reference.clone()) {
            self.snapshot.directories.push(DirectorySnapshot {
                source: source.clone(),
                children: children.clone(),
            });
        }
        Ok(children)
    }
    fn target_directory(&mut self, destination: Destination) -> Result<()> {
        for count in 1..=destination.components.len() {
            let partial = Destination {
                root: destination.root.clone(),
                components: destination.components[..count].to_vec(),
            };
            let r = reference(&destination_path(&partial)?);
            if !self.destinations.insert(r.clone()) {
                continue;
            }
            let existing = self
                .plan
                .observations
                .iter()
                .find(|o| o.reference == r)
                .cloned();
            if existing
                .as_ref()
                .is_some_and(|o| o.kind != EntryKind::Directory)
            {
                return Err(E::Stale);
            }
            self.snapshot.destinations.push(DirectoryRequirement {
                destination: partial,
                existing,
            });
        }
        Ok(())
    }
    fn expand(
        &mut self,
        source: EntryObservation,
        destination: Destination,
        operation: usize,
        cleanup: &mut Vec<EntryObservation>,
    ) -> Result<()> {
        cancelled(self.cancel)?;
        if path(&source.reference)?
            .components()
            .count()
            .saturating_sub(path(&self.plan.request.source)?.components().count())
            > 128
            || destination.components.len() > 128
        {
            return Err(E::Unsupported);
        }
        if destination.root != *self.plan.request.target.as_ref().ok_or(E::Scope)? {
            return Err(E::Scope);
        }
        destination_path(&destination)?;
        match source.kind {
            EntryKind::File => {
                if source.size.is_none() || source.modified.is_none() || source.identity.is_none() {
                    return Err(E::Scope);
                }
                let parent = Destination {
                    root: destination.root.clone(),
                    components: destination.components[..destination
                        .components
                        .len()
                        .checked_sub(1)
                        .ok_or(E::Scope)?]
                        .to_vec(),
                };
                self.target_directory(parent)?;
                self.snapshot.files.push(FileIntent {
                    original_operation: operation,
                    source,
                    destination,
                });
            }
            EntryKind::Directory => {
                self.target_directory(destination.clone())?;
                for child in self.directory(&source)? {
                    let exact = leaf(&child)?;
                    if exact != name(&child.name) {
                        return Err(E::Unsupported);
                    }
                    let mut dest = destination.clone();
                    dest.components.push(child.name.clone());
                    self.expand(child, dest, operation, cleanup)?;
                }
                self.snapshot.copied_directories.push(CopiedDirectory {
                    operation,
                    source: source.clone(),
                    destination,
                });
                cleanup.push(source);
            }
            _ => return Err(E::Unsupported),
        }
        Ok(())
    }
}
pub fn prepare_album_scope(
    plan: &JobPlanV2,
    reads: &mut dyn ReadStorage,
    resources: &Resources,
    cancel: &AtomicBool,
) -> Result<AlbumScope> {
    supported_album_plan(plan)?;
    let root = reads.stat(&plan.request.source).map_err(|_| E::Stale)?;
    let target = reads
        .stat(plan.request.target.as_ref().ok_or(E::Scope)?)
        .map_err(|_| E::Stale)?;
    let mut collector = Collector {
        plan,
        reads,
        cancel,
        snapshot: Snapshot {
            copied_directories: vec![],
            directories: vec![],
            destinations: vec![],
            files: vec![],
            albums: vec![],
        },
        watched: HashSet::new(),
        destinations: HashSet::new(),
        entries: 0,
        path_units: 0,
    };
    for op in &plan.operations {
        match &op.kind {
            OperationKind::SingleTransfer {
                source, transfer, ..
            } => collector.expand(
                source.clone(),
                transfer.destination.clone(),
                op.id,
                &mut vec![],
            )?,
            OperationKind::AlbumTransfer {
                source,
                destination,
                children,
                remove_empty_source,
            } => {
                let all = collector.directory(source)?;
                let mut expected_children = all
                    .iter()
                    .filter(|child| sorter_core::sanitize(&child.name).is_some())
                    .collect::<Vec<_>>();
                if expected_children.len() != children.len() {
                    return Err(E::Stale);
                }
                for (child, expected) in children.iter().zip(expected_children.drain(..)) {
                    if child.source != *expected {
                        return Err(E::Stale);
                    }
                }
                collector.target_directory(destination.clone())?;
                let start = collector.snapshot.files.len();
                let mut cleanup = vec![];
                for child in children {
                    collector.expand(
                        child.source.clone(),
                        child.destination.clone(),
                        op.id,
                        &mut cleanup,
                    )?;
                }
                if *remove_empty_source {
                    cleanup.push(source.clone());
                }
                let artist = plan
                    .decisions
                    .iter()
                    .find_map(|decision| match decision {
                        DecisionV2::Album { read, decision }
                            if read.source.reference == source.reference =>
                        {
                            decision.artist_name.clone()
                        }
                        _ => None,
                    })
                    .ok_or(E::InvalidBinding)?;
                collector.snapshot.albums.push(AlbumGroup {
                    children: children
                        .iter()
                        .map(|child| {
                            Ok(AlbumChildPlan {
                                source: child.source.clone(),
                                destination_name: child
                                    .destination
                                    .components
                                    .last()
                                    .cloned()
                                    .ok_or(E::Scope)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                    operation: op.id,
                    artist,
                    source: source.clone(),
                    destination: destination.clone(),
                    files: (start..collector.snapshot.files.len()).collect(),
                    cleanup,
                });
            }
            _ => return Err(E::Unsupported),
        }
    }
    let snapshot = collector.snapshot;
    if snapshot.files.is_empty() {
        return Err(E::Unsupported);
    }
    let mut inspected_sources = vec![];
    let mut seen = HashSet::new();
    let mut destinations = HashSet::new();
    for file in &snapshot.files {
        cancelled(cancel)?;
        if !destinations.insert(crate::hash(&file.destination)?) {
            return Err(E::Unsupported);
        }
        if !seen.insert(file.source.reference.clone()) {
            continue;
        }
        if reads.stat(&file.source.reference).map_err(|_| E::Stale)? != file.source {
            return Err(E::Stale);
        }
        let mut input = reads
            .open_read(&file.source.reference)
            .map_err(|_| E::Stale)?;
        let binding = input.binding().ok_or(E::Stale)?;
        if Some(&binding.identity) != file.source.identity.as_ref()
            || Some(binding.size) != file.source.size
            || Some(&binding.modified) != file.source.modified.as_ref()
        {
            return Err(E::Stale);
        }
        let mut hash = Sha256::new();
        let mut length = 0u64;
        let mut buffer = [0; 65536];
        loop {
            cancelled(cancel)?;
            let n = input.read(&mut buffer).map_err(|_| E::Stale)?;
            if n == 0 {
                break;
            }
            length = length.checked_add(n as u64).ok_or(E::Stale)?;
            if Some(length) > file.source.size {
                return Err(E::Stale);
            }
            hash.update(&buffer[..n]);
        }
        if Some(length) != file.source.size
            || reads.stat(&file.source.reference).map_err(|_| E::Stale)? != file.source
        {
            return Err(E::Stale);
        }
        inspected_sources.push(SourceEvidence {
            observation: file.source.clone(),
            sha256: format!("{:x}", hash.finalize()),
        });
    }
    Ok(AlbumScope {
        execution: ExecutionScope {
            inspected_plan: plan.binding_sha256.clone(),
            source_root: root.identity.ok_or(E::Scope)?,
            target_root: target.identity.ok_or(E::Scope)?,
            inspected_sources,
            resources: resources.clone(),
            allowed_modes: vec![if plan.request.settings.copy_mode {
                TransferMode::Copy
            } else {
                TransferMode::Move
            }],
        },
        snapshot,
    })
}

enum Inner {
    Copy(WindowsMutationStorage),
    Rename(WindowsRenameStorage),
}
impl Inner {
    fn roots(&self) -> &WindowsMutationStorage {
        match self {
            Self::Copy(s) => s,
            Self::Rename(s) => s.owned(),
        }
    }
}
macro_rules! native_mut {($s:expr,$method:ident $(,$arg:expr)*)=>{match &mut $s.inner{Inner::Copy(storage)=>storage.$method($($arg),*),Inner::Rename(storage)=>storage.$method($($arg),*)}};}
macro_rules! native_ref {($s:expr,$method:ident $(,$arg:expr)*)=>{match &$s.inner{Inner::Copy(storage)=>storage.$method($($arg),*),Inner::Rename(storage)=>storage.$method($($arg),*)}};}
fn same_members(
    mut actual: Vec<WindowsDirectoryMember>,
    mut expected: Vec<WindowsDirectoryMember>,
) -> bool {
    fn key(member: &WindowsDirectoryMember) -> &[u16] {
        match &member.name {
            MutationName::Windows(value) => value,
            MutationName::Provider(_) => &[],
        }
    }
    if actual
        .iter()
        .chain(&expected)
        .any(|member| !matches!(member.name, MutationName::Windows(_)))
    {
        return false;
    }
    actual.sort_by(|a, b| key(a).cmp(key(b)));
    expected.sort_by(|a, b| key(a).cmp(key(b)));
    actual == expected && !actual.windows(2).any(|pair| pair[0].name == pair[1].name)
}
struct Watch {
    parent: WindowsParent,
    expected: Vec<WindowsDirectoryMember>,
}
struct AlbumStorage {
    inner: Inner,
    pool: HashMap<ObjectIdentity, WindowsSourceLease>,
    watches: Vec<Watch>,
    removed: HashSet<ObjectIdentity>,
}
impl AlbumStorage {
    fn roots(&self) -> &WindowsMutationStorage {
        self.inner.roots()
    }
    fn check_all(&self) -> std::result::Result<(), MutationError> {
        for watch in &self.watches {
            let expected = watch
                .expected
                .iter()
                .filter(|child| !self.removed.contains(&child.identity))
                .cloned()
                .collect::<Vec<_>>();
            if !same_members(
                self.roots().source_directory_members(&watch.parent)?,
                expected,
            ) {
                return Err(failure("planned album membership changed; stop and replan"));
            }
        }
        Ok(())
    }
    fn prepare_sources(&mut self, snapshot: &Snapshot, binding: &ExecutionBinding) -> Result<()> {
        for directory in &snapshot.directories {
            let parent = self
                .roots()
                .parent_reference(&directory.source.reference)
                .map_err(|_| E::Stale)?;
            if Some(parent.identity()) != directory.source.identity.as_ref() {
                return Err(E::Stale);
            }
            let expected = directory
                .children
                .iter()
                .map(|child| {
                    Ok(WindowsDirectoryMember {
                        name: leaf(child)?,
                        identity: child.identity.clone().ok_or(E::Scope)?,
                        kind: child.kind,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            self.watches.push(Watch { parent, expected });
        }
        self.check_all().map_err(|_| E::Stale)?;
        for source in &binding.data.scope_evidence.inspected_sources {
            let parent = self
                .roots()
                .parent_reference(&parent_reference(&source.observation)?)
                .map_err(|_| E::Stale)?;
            let expected = expected(source)?;
            let lease =
                native_mut!(self, acquire_source, &parent, &expected).map_err(|_| E::Stale)?;
            if self.pool.insert(expected.identity, lease).is_some() {
                return Err(E::Scope);
            }
        }
        self.check_all().map_err(|_| E::Stale)
    }
    fn after_publication(
        &self,
        outcome: MutationOutcome<WindowsPublished>,
    ) -> MutationOutcome<WindowsPublished> {
        match outcome {
            MutationOutcome::Applied(published) => match self.check_all() {
                Ok(()) => MutationOutcome::Applied(published),
                Err(error) => MutationOutcome::PendingUnknown(
                    error.with_observation(published.observation().clone()),
                ),
            },
            other => other,
        }
    }
}
impl MutationStorage for AlbumStorage {
    type Parent = WindowsParent;
    type SourceLease = WindowsSourceLease;
    type OwnedTemp = WindowsOwnedTemp;
    type Published = WindowsPublished;
    fn capabilities(&self) -> MutationCapabilities {
        native_ref!(self, capabilities)
    }
    fn acquire_source(
        &mut self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
    ) -> std::result::Result<WindowsSourceLease, MutationError> {
        self.check_all()?;
        if parent.identity() != &expected.parent_identity {
            return Err(failure("album source parent mismatch"));
        }
        let lease = self
            .pool
            .remove(&expected.identity)
            .ok_or_else(|| failure("source is not in the preleased album cohort"))?;
        if lease.observation().actual_name != expected.name
            || lease.observation().parent_identity != expected.parent_identity
        {
            return Err(failure("preleased album source mismatch"));
        }
        Ok(lease)
    }
    fn create_temp(
        &mut self,
        parent: &WindowsParent,
        job_id: &str,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsOwnedTemp> {
        if let Err(error) = self.check_all() {
            return MutationOutcome::NoEffect(error);
        }
        native_mut!(self, create_temp, parent, job_id, cancel)
    }
    fn publish_no_replace(
        &mut self,
        temp: &mut WindowsOwnedTemp,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        if let Err(error) = self.check_all() {
            return MutationOutcome::NoEffect(error);
        }
        let outcome = native_mut!(self, publish_no_replace, temp, parent, name, cancel);
        self.after_publication(outcome)
    }
    fn rename_no_replace(
        &mut self,
        source: &mut WindowsSourceLease,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        if let Err(error) = self.check_all() {
            return MutationOutcome::NoEffect(error);
        }
        let id = source.observation().identity.clone();
        let outcome = native_mut!(self, rename_no_replace, source, parent, name, cancel);
        if matches!(&outcome, MutationOutcome::Applied(_)) {
            self.removed.insert(id);
        }
        self.after_publication(outcome)
    }
    fn verify(
        &mut self,
        published: &WindowsPublished,
        digest: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.check_all()?;
        native_mut!(self, verify, published, digest)
    }
    fn retire_source(
        &mut self,
        _: &mut WindowsSourceLease,
        _: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        MutationOutcome::NoEffect(MutationError::new(
            MutationErrorKind::Unsupported,
            "album copied retirement is not supported",
        ))
    }
    fn inspect(
        &mut self,
        published: &WindowsPublished,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.check_all()?;
        native_mut!(self, inspect, published)
    }
}
impl TransferStorage for AlbumStorage {
    fn finalize_selected_copy_metadata(
        &mut self,
        source: &WindowsSourceLease,
        temp: &mut WindowsOwnedTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<crate::physical_journal::CopyMetadataEvidence> {
        if let Err(error) = self.check_all() {
            return MutationOutcome::NoEffect(error);
        }
        match &mut self.inner {
            Inner::Copy(storage) => {
                <WindowsMutationStorage as TransferStorage>::finalize_selected_copy_metadata(
                    storage, source, temp, cancel,
                )
            }
            Inner::Rename(_) => MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Unsupported,
                "rename has no Copy metadata phase",
            )),
        }
    }

    fn source_parent(
        &mut self,
        source: &SourceEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<WindowsParent, MutationError> {
        native_mut!(self, source_parent, source, root)
    }
    fn destination_parent(
        &mut self,
        destination: &DestinationEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<WindowsParent, MutationError> {
        native_mut!(self, destination_parent, destination, root)
    }
    fn parent_identity(&self, parent: &WindowsParent) -> ObjectIdentity {
        parent.identity().clone()
    }
    fn temp_observation(&self, temp: &WindowsOwnedTemp) -> MutationObservation {
        temp.observation().clone()
    }
    fn rewind_source(
        &mut self,
        source: &mut WindowsSourceLease,
    ) -> std::result::Result<(), MutationError> {
        native_mut!(self, rewind_source, source)
    }
    fn read_source(
        &mut self,
        source: &mut WindowsSourceLease,
        buffer: &mut [u8],
    ) -> io::Result<usize> {
        native_mut!(self, read_source, source, buffer)
    }
    fn write_temp(&mut self, temp: &mut WindowsOwnedTemp, buffer: &[u8]) -> io::Result<usize> {
        native_mut!(self, write_temp, temp, buffer)
    }
    fn sync_temp(&mut self, temp: &mut WindowsOwnedTemp) -> std::result::Result<(), MutationError> {
        native_mut!(self, sync_temp, temp)
    }
    fn verify_temp(
        &mut self,
        temp: &WindowsOwnedTemp,
        digest: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        native_mut!(self, verify_temp, temp, digest)
    }
    fn rename_eligible(&self, source: &WindowsSourceLease, parent: &WindowsParent) -> bool {
        native_ref!(self, rename_eligible, source, parent)
    }
    fn owned_windows_rename_contract(&self) -> bool {
        matches!(self.inner, Inner::Rename(_))
    }
    fn sync_owned_windows_publication(
        &mut self,
        published: &mut WindowsPublished,
        digest: &ContentDigest,
        cancel: &AtomicBool,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.check_all()?;
        let observed = native_mut!(
            self,
            sync_owned_windows_publication,
            published,
            digest,
            cancel
        )?;
        self.check_all()?;
        Ok(observed)
    }
}

const MAX_MANIFEST: usize = 64 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct ManifestRecord {
    sequence: u64,
    previous: String,
    job_id: String,
    phase: String,
    payload: serde_json::Value,
}
#[derive(Serialize, Deserialize)]
struct ManifestEnvelope {
    body: ManifestRecord,
    sha256: String,
}
struct Manifest {
    sink: WindowsJournalStore,
    job_id: String,
    sequence: u64,
    previous: String,
    bytes: usize,
    poisoned: bool,
}
impl Manifest {
    fn append(&mut self, phase: &str, payload: &impl Serialize) -> Result<()> {
        if self.poisoned {
            return Err(E::NeedsRecovery);
        }
        self.poisoned = true;
        let body = ManifestRecord {
            sequence: self.sequence,
            previous: self.previous.clone(),
            job_id: self.job_id.clone(),
            phase: phase.into(),
            payload: serde_json::to_value(payload).map_err(|_| E::Io)?,
        };
        let sha256 = crate::hash(&body)?;
        let mut bytes = serde_json::to_vec(&ManifestEnvelope {
            body,
            sha256: sha256.clone(),
        })
        .map_err(|_| E::Io)?;
        bytes.push(b'\n');
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= MAX_MANIFEST)
            .ok_or(E::NeedsRecovery)?;
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetainedDirectory {
    pub reference: StorageRef,
    pub identity: ObjectIdentity,
    pub reason: String,
}
#[derive(Clone, Debug)]
pub struct SelectedAlbumResult {
    pub child_renames: Vec<whole::AlbumChildRenameReceipt>,
    pub artist_files: Vec<(String, usize)>,
    pub whole_albums: Vec<WholeAlbumOutcome>,
    pub status: JobStatus,
    pub transfers: Option<JobResult>,
    pub effect_receipts: Vec<crate::results::ObservedEffectReceipt>,
    pub albums_completed: usize,
    pub created_directories: Vec<MutationObservation>,
    pub removed_directories: Vec<MutationObservation>,
    pub retained_directories: Vec<RetainedDirectory>,
    pub detail: String,
}
/// One attempt only. Roots, source cohort and created directories stay owned by
/// this job through journal completion. There is no replay or guessed cleanup.
struct SelectedAlbumFileJob {
    copy_directories: Vec<WindowsCreatedCopyDirectory>,
    completed_albums: Vec<usize>,
    storage: AlbumStorage,
    binding: ExecutionBinding,
    snapshot: Snapshot,
    job_id: String,
    attempted: bool,
    locations: Vec<WindowsJournalLocation>,
    directories: Vec<WindowsParent>,
    created: Vec<MutationObservation>,
    removed: Vec<MutationObservation>,
    retained: Vec<RetainedDirectory>,
    guards: Option<crate::transfer::HeldTransferGuards<AlbumStorage>>,
}
impl SelectedAlbumFileJob {
    fn prepare(
        roots: WindowsMutationStorage,
        validated: &ValidatedAlbumExecution,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let plan = &validated.plan;
        let scope = &validated.scope;
        let resources = scope.resources();
        cancelled(cancel)?;
        if roots.domain() != WindowsMutationDomain::SelectedRoots
            || roots.selected_source_reference().map_err(|_| E::Scope)? != plan.request.source
            || Some(roots.selected_target_reference().map_err(|_| E::Scope)?) != plan.request.target
        {
            return Err(E::Scope);
        }
        let source = roots.selected_source_root().map_err(|_| E::Scope)?;
        let target = roots.selected_target_root().map_err(|_| E::Scope)?;
        if source.identity() != &scope.execution.source_root
            || target.identity() != &scope.execution.target_root
        {
            return Err(E::Scope);
        }
        let copy = plan.request.settings.copy_mode;
        // Once a copied child tree receives postorder metadata no later operation may
        // change its namespace. This check uses only the already planned destinations.
        for directory in &scope.snapshot.copied_directories {
            if scope.snapshot.files.iter().any(|file| {
                file.original_operation != directory.operation
                    && file.destination.root == directory.destination.root
                    && file
                        .destination
                        .components
                        .starts_with(&directory.destination.components)
            }) {
                return Err(E::Unsupported);
            }
        }
        if !copy
            && !matches!((source.identity(),target.identity()),(ObjectIdentity::Windows{volume:a,..},ObjectIdentity::Windows{volume:b,..})if a==b)
        {
            return Err(E::Unsupported);
        }
        let root = validated.source.clone();
        let target = validated.target.clone();
        let mut operations = Vec::new();
        for (id, file) in scope.snapshot.files.iter().enumerate() {
            let source = scope
                .execution
                .inspected_sources
                .iter()
                .find(|s| s.observation == file.source)
                .ok_or(E::Scope)?
                .clone();
            operations.push(BoundOperation {
                id,
                depends_on: if id == 0 { vec![] } else { vec![id - 1] },
                source,
                destination: DestinationEvidence {
                    id: crate::hash(&file.destination)?,
                    root: file.destination.root.clone(),
                    components: file.destination.components.clone(),
                },
                mode: if copy {
                    TransferMode::Copy
                } else {
                    TransferMode::Move
                },
                copy_effect: copy,
                duet_cohort: None,
            });
        }
        let binding = ExecutionBinding {
            recovery_only: false,
            data: BindingData {
                schema: 2,
                album_context: None,
                copy_metadata_version: if copy { Some(1) } else { None },
                plan_sha256: plan.binding_sha256.clone(),
                source_root: root,
                target_root: target,
                resources: resources.clone(),
                operations,
                scope_sha256: scope_digest(&scope.execution)?,
                scope_evidence: ScopeEvidence {
                    inspected_plan: scope.execution.inspected_plan.clone(),
                    source_root: scope.execution.source_root.clone(),
                    target_root: scope.execution.target_root.clone(),
                    inspected_sources: scope.execution.inspected_sources.clone(),
                    resources: resources.clone(),
                    allowed_modes: scope.execution.allowed_modes.clone(),
                },
                synthetic_only: false,
                owned_windows_copy: false,
                owned_windows_rename: false,
                selected_windows_copy: copy,
                selected_windows_rename: !copy,
            },
        };
        if !binding.data.valid_domain() {
            return Err(E::InvalidBinding);
        }
        if serde_json::to_vec(&(&binding.data, &scope.snapshot))
            .map_err(|_| E::Io)?
            .len()
            > MAX_MANIFEST - 1024
        {
            return Err(E::Unsupported);
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| E::Io)?
            .as_nanos();
        let job_id = format!(
            "album-{}-{now}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        Ok(Self {
            copy_directories: vec![],
            completed_albums: vec![],
            storage: AlbumStorage {
                inner: if copy {
                    Inner::Copy(roots)
                } else {
                    Inner::Rename(WindowsRenameStorage::new(roots))
                },
                pool: HashMap::new(),
                watches: vec![],
                removed: HashSet::new(),
            },
            binding,
            snapshot: scope.snapshot.clone(),
            job_id,
            attempted: false,
            locations: vec![],
            directories: vec![],
            created: vec![],
            removed: vec![],
            retained: vec![],
            guards: None,
        })
    }
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    pub fn journal_locations(&self) -> &[WindowsJournalLocation] {
        &self.locations
    }
    pub fn binding_sha256(&self) -> Result<String> {
        self.binding.sha256()
    }
    pub fn run(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(TransferProgress),
    ) -> Result<SelectedAlbumResult> {
        if self.attempted {
            return Err(E::NeedsRecovery);
        }
        self.attempted = true;
        cancelled(cancel)?;
        self.storage
            .prepare_sources(&self.snapshot, &self.binding)?;
        cancelled(cancel)?;
        let journal_parent = self
            .storage
            .roots()
            .selected_journal_root()
            .map_err(|_| E::Scope)?;
        let file = self
            .storage
            .roots()
            .create_journal(
                &journal_parent,
                &name(&format!("{}.album.jsonl", self.job_id)),
            )
            .map_err(|_| E::Io)?;
        self.locations.push(file.location());
        let mut manifest = Manifest {
            sink: WindowsJournalStore::new(file),
            job_id: self.job_id.clone(),
            sequence: 0,
            previous: String::new(),
            bytes: 0,
            poisoned: false,
        };
        manifest.append(
            "Header",
            &serde_json::json!({"schema":1,"binding":self.binding.data,"snapshot":self.snapshot}),
        )?;
        if let Err(error) = self.create_directories(&mut manifest, cancel) {
            if !manifest.poisoned {
                manifest.append("ExecutionStopped",&serde_json::json!({"stage":"directories","status":format!("{:?}",error_status(&error))}))?;
            }
            return self.finish(
                &mut manifest,
                None,
                error_status(&error),
                format!("directory preparation stopped: {error}"),
            );
        }
        let file = match self.storage.roots().create_journal(
            &journal_parent,
            &name(&format!("{}.transfers.jsonl", self.job_id)),
        ) {
            Ok(file) => file,
            Err(error) => {
                manifest.append(
                    "ExecutionStopped",
                    &serde_json::json!({"stage":"transfer_journal","status":"NeedsRecovery"}),
                )?;
                return self.finish(&mut manifest, None, JobStatus::NeedsRecovery, error.detail);
            }
        };
        self.locations.push(file.location());
        let mut sink = BoundedTransferJournal {
            inner: WindowsJournalStore::new(file),
            bytes: 0,
            poisoned: false,
        };
        let mut journal = PhysicalJournalV2::create(&self.binding, &self.job_id, &mut sink)?;
        manifest.append(
            "TransfersPrepared",
            &serde_json::json!({"binding_sha256":self.binding.sha256()?}),
        )?;
        let files = &self.snapshot.files;
        let mut report = |mut value: TransferProgress| {
            value.operation = files[value.operation].original_operation;
            progress(value)
        };
        let snapshot = &self.snapshot;
        let created = &self.created;
        let copy_directories = &mut self.copy_directories;
        let completed = &mut self.completed_albums;
        let copy = self.binding.data.selected_windows_copy;
        let mut after_commit = |operation, storage: &mut AlbumStorage| {
            if !copy {
                return MutationOutcome::Applied(());
            }
            let Some(album) = snapshot
                .albums
                .iter()
                .find(|album| album.files.last() == Some(&operation))
            else {
                return MutationOutcome::Applied(());
            };
            match complete_copy_album(
                storage,
                snapshot,
                album,
                created,
                copy_directories,
                &mut manifest,
                cancel,
            ) {
                MutationOutcome::Applied(()) => {
                    completed.push(album.operation);
                    MutationOutcome::Applied(())
                }
                other => other,
            }
        };
        let (result, guards) = crate::transfer::run_selected_album_transfers(
            &self.binding,
            &mut self.storage,
            &mut journal,
            &mut sink,
            cancel,
            &mut report,
            &mut after_commit,
        )?;
        self.guards = Some(guards);
        let mut status = result.status;
        let mut detail = String::new();
        manifest.append("TransfersObserved",&serde_json::json!({"status":format!("{status:?}"),"committed_destinations":result.committed_destinations,"moved_sources":result.moved_sources}))?;
        if status == JobStatus::Complete
            && let Err(error) = self.storage.check_all()
        {
            status = JobStatus::Partial;
            detail = error.detail;
            manifest.append(
                "ExecutionStopped",
                &serde_json::json!({"stage":"membership","status":"Partial"}),
            )?;
        }
        if status == JobStatus::Complete && !self.binding.data.selected_windows_copy {
            // Every known file has an acknowledged verified rename. Drop data/source
            // directory pins only after that journal sync; cleanup opens an exact empty
            // child under a still-pinned parent and cannot delete newly added files.
            self.guards = None;
            self.storage.pool.clear();
            self.storage.watches.clear();
            if let Err(error) = self.cleanup_directories(&mut manifest, cancel) {
                status = error_status(&error);
                detail = format!("file transfers completed; directory cleanup stopped: {error}");
                if !manifest.poisoned {
                    manifest.append(
                        "ExecutionStopped",
                        &serde_json::json!({"stage":"cleanup","status":format!("{status:?}")}),
                    )?;
                }
            }
            if !self.retained.is_empty() && status == JobStatus::Complete {
                status = JobStatus::Partial;
                detail = "file transfers completed; some original directories are retained".into();
            }
        }
        let mut result = result;
        for item in &mut result.items {
            item.operation = self.snapshot.files[item.operation].original_operation;
        }
        for operation in &mut result.committed_operations {
            *operation = self.snapshot.files[*operation].original_operation;
        }
        for receipt in &mut result.effect_receipts {
            receipt.operation = self.snapshot.files[receipt.operation].original_operation;
        }
        self.finish(&mut manifest, Some(result), status, detail)
    }
    fn create_directories(&mut self, manifest: &mut Manifest, cancel: &AtomicBool) -> Result<()> {
        for requirement in &self.snapshot.destinations {
            cancelled(cancel)?;
            self.storage.check_all().map_err(|_| E::Stale)?;
            let full = destination_path(&requirement.destination)?;
            let root = path(&requirement.destination.root)?;
            let parent_ref = reference(full.parent().ok_or(E::Scope)?);
            if !path(&parent_ref)?.starts_with(&root) {
                return Err(E::Scope);
            }
            let parent = self
                .storage
                .roots()
                .parent_reference(&parent_ref)
                .map_err(|_| E::Stale)?;
            if let Some(existing) = &requirement.existing {
                let held = self
                    .storage
                    .roots()
                    .parent_reference(&existing.reference)
                    .map_err(|_| E::Stale)?;
                if Some(held.identity()) != existing.identity.as_ref() {
                    return Err(E::Stale);
                }
                self.directories.push(held);
                continue;
            }
            let leaf = requirement.destination.components.last().ok_or(E::Scope)?;
            manifest.append("DirectoryPrepared",&serde_json::json!({"reference":reference(&full),"parent":parent.identity(),"name":leaf}))?;
            cancelled(cancel)?;
            self.storage.check_all().map_err(|_| E::Stale)?;
            let created = if self.binding.data.selected_windows_copy {
                match self.storage.roots().create_selected_copy_directory(
                    &parent,
                    &name(leaf),
                    cancel,
                ) {
                    MutationOutcome::Applied(token) => {
                        let parent = token.parent().clone();
                        self.copy_directories.push(token);
                        MutationOutcome::Applied(parent)
                    }
                    MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
                    MutationOutcome::PendingUnknown(error) => {
                        MutationOutcome::PendingUnknown(error)
                    }
                }
            } else {
                self.storage
                    .roots()
                    .create_destination_directory(&parent, &name(leaf), cancel)
            };
            match created {
                MutationOutcome::Applied(directory) => {
                    let observed = MutationObservation {
                        reference: reference(&full),
                        identity: directory.identity().clone(),
                        parent_identity: parent.identity().clone(),
                        actual_name: name(leaf),
                        size: None,
                    };
                    self.created.push(observed.clone());
                    self.directories.push(directory);
                    manifest.append("DirectoryObserved", &observed)?;
                }
                MutationOutcome::NoEffect(error) => {
                    return Err(if error.kind == MutationErrorKind::Cancelled {
                        E::Cancelled
                    } else {
                        E::Stale
                    });
                }
                MutationOutcome::PendingUnknown(_) => return Err(E::NeedsRecovery),
            }
        }
        Ok(())
    }
    fn check_remaining_directories(&self) -> Result<()> {
        for directory in &self.snapshot.directories {
            if directory
                .source
                .identity
                .as_ref()
                .is_some_and(|id| self.storage.removed.contains(id))
            {
                continue;
            }
            let parent = self
                .storage
                .roots()
                .parent_reference(&directory.source.reference)
                .map_err(|_| E::Stale)?;
            if Some(parent.identity()) != directory.source.identity.as_ref() {
                return Err(E::Stale);
            }
            let expected = directory
                .children
                .iter()
                .filter(|c| {
                    !c.identity
                        .as_ref()
                        .is_some_and(|id| self.storage.removed.contains(id))
                })
                .map(|c| {
                    Ok(WindowsDirectoryMember {
                        name: leaf(c)?,
                        identity: c.identity.clone().ok_or(E::Scope)?,
                        kind: c.kind,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            if !same_members(
                self.storage
                    .roots()
                    .source_directory_members(&parent)
                    .map_err(|_| E::Stale)?,
                expected,
            ) {
                return Err(E::Stale);
            }
        }
        Ok(())
    }
    fn cleanup_directories(&mut self, manifest: &mut Manifest, cancel: &AtomicBool) -> Result<()> {
        let mut directories = self
            .snapshot
            .albums
            .iter()
            .flat_map(|a| a.cleanup.iter().cloned())
            .collect::<Vec<_>>();
        directories.sort_by_key(|e| {
            std::cmp::Reverse(
                path(&e.reference)
                    .map(|p| p.components().count())
                    .unwrap_or(0),
            )
        });
        let mut seen = HashSet::new();
        for directory in directories {
            cancelled(cancel)?;
            if !seen.insert(directory.reference.clone()) {
                continue;
            }
            let id = directory.identity.clone().ok_or(E::Scope)?;
            self.check_remaining_directories()?;
            if id == self.binding.data.scope_evidence.source_root {
                let retained = RetainedDirectory {
                    reference: directory.reference.clone(),
                    identity: id,
                    reason: "selected source root remains pinned; root cleanup is not implemented"
                        .into(),
                };
                manifest.append("DirectoryRetained", &retained)?;
                self.retained.push(retained);
                continue;
            }
            let parent = self
                .storage
                .roots()
                .parent_reference(&parent_reference(&directory)?)
                .map_err(|_| E::Stale)?;
            manifest.append("CleanupPrepared", &directory)?;
            match self.storage.roots().remove_empty_source_directory(
                &parent,
                &leaf(&directory)?,
                &id,
                cancel,
            ) {
                MutationOutcome::Applied(observation) => {
                    self.storage.removed.insert(id);
                    self.removed.push(observation.clone());
                    manifest.append("CleanupObserved", &observation)?;
                }
                MutationOutcome::NoEffect(error) => {
                    let retained = RetainedDirectory {
                        reference: directory.reference,
                        identity: id,
                        reason: error.detail,
                    };
                    manifest.append("DirectoryRetained", &retained)?;
                    self.retained.push(retained);
                    return Err(if error.kind == MutationErrorKind::Cancelled {
                        E::Cancelled
                    } else {
                        E::Stale
                    });
                }
                MutationOutcome::PendingUnknown(error) => {
                    if let Some(observed) = error.observed {
                        manifest.append("CleanupUnknown", observed.as_ref())?;
                    }
                    return Err(E::NeedsRecovery);
                }
            }
        }
        Ok(())
    }
    fn finish(
        &mut self,
        manifest: &mut Manifest,
        transfers: Option<JobResult>,
        status: JobStatus,
        detail: String,
    ) -> Result<SelectedAlbumResult> {
        let albums_completed = self.completed_albums.len();
        let artist_files =
            album_artist_counts(&self.snapshot, self.completed_albums.iter().copied());
        let result = SelectedAlbumResult {
            child_renames: vec![],
            artist_files,
            whole_albums: vec![],
            status,
            effect_receipts: transfers.as_ref().map(|result| result.effect_receipts.clone()).unwrap_or_default(),
            transfers,
            albums_completed,
            created_directories: self.created.clone(),
            removed_directories: self.removed.clone(),
            retained_directories: self.retained.clone(),
            detail,
        };
        if !manifest.poisoned {
            manifest.append("Finished",&serde_json::json!({"status":format!("{:?}",result.status),"albums_completed":result.albums_completed,"created":result.created_directories,"removed":result.removed_directories,"retained":result.retained_directories,"detail":result.detail}))?;
        }
        Ok(result)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlbumDirectoryMetadataReceipt {
    pub destination: StorageRef,
    pub evidence: crate::physical_journal::CopyMetadataEvidence,
}
fn copy_metadata_evidence(
    value: WindowsCopyMetadataEvidence,
) -> crate::physical_journal::CopyMetadataEvidence {
    crate::physical_journal::CopyMetadataEvidence {
        source_identity: value.source_identity,
        destination_identity: value.destination_identity,
        access_time: value.access_time,
        write_time: value.write_time,
        readonly: value.readonly,
        retained_creation_time: value.retained_creation_time,
        retained_attributes: value.retained_attributes,
    }
}
fn complete_copy_album(
    storage: &AlbumStorage,
    snapshot: &Snapshot,
    album: &AlbumGroup,
    created: &[MutationObservation],
    tokens: &mut [WindowsCreatedCopyDirectory],
    manifest: &mut Manifest,
    cancel: &AtomicBool,
) -> MutationOutcome<()> {
    let before = || -> Result<()> {
        cancelled(cancel)?;
        storage.check_all().map_err(|_| E::Stale)
    };
    if let Err(error) = before() {
        return MutationOutcome::NoEffect(MutationError::new(
            if error == E::Cancelled {
                MutationErrorKind::Cancelled
            } else {
                MutationErrorKind::StalePlan
            },
            format!("album completion precheck: {error}"),
        ));
    }
    let result = (|| -> Result<()> {
        // Collector emits copied child directories in postorder. No later operation
        // may write within one of these finalized trees; album roots are not copystat.
        for directory in snapshot
            .copied_directories
            .iter()
            .filter(|directory| directory.operation == album.operation)
        {
            cancelled(cancel)?;
            let destination = reference(&destination_path(&directory.destination)?);
            let observed = created
                .iter()
                .find(|entry| entry.reference == destination)
                .ok_or(E::Scope)?;
            let token = tokens
                .iter_mut()
                .find(|token| token.observation().identity == observed.identity)
                .ok_or(E::Scope)?;
            let source = storage
                .roots()
                .parent_reference(&directory.source.reference)
                .map_err(|_| E::Stale)?;
            if Some(source.identity()) != directory.source.identity.as_ref() {
                return Err(E::Stale);
            }
            manifest.append("DirectoryMetadataPrepared",&serde_json::json!({"source":directory.source,"destination":directory.destination,"created":observed}))?;
            let evidence = match storage
                .roots()
                .finalize_selected_copy_directory_metadata(&source, token, cancel)
            {
                MutationOutcome::Applied(evidence) => evidence,
                MutationOutcome::NoEffect(error) => {
                    manifest.append("DirectoryMetadataStopped",&serde_json::json!({"destination":destination,"status":if error.kind==MutationErrorKind::Cancelled{"Cancelled"}else{"Partial"}}))?;
                    return Err(if error.kind == MutationErrorKind::Cancelled {
                        E::Cancelled
                    } else {
                        E::Stale
                    });
                }
                MutationOutcome::PendingUnknown(_) => {
                    manifest.append(
                        "DirectoryMetadataStopped",
                        &serde_json::json!({"destination":destination,"status":"NeedsRecovery"}),
                    )?;
                    return Err(E::NeedsRecovery);
                }
            };
            manifest.append(
                "DirectoryMetadataObserved",
                &AlbumDirectoryMetadataReceipt {
                    destination,
                    evidence: copy_metadata_evidence(evidence),
                },
            )?;
        }
        storage.check_all().map_err(|_| E::Stale)?;
        manifest.append(
            "AlbumCompleted",
            &serde_json::json!({"operation":album.operation,"files":album.files}),
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => MutationOutcome::Applied(()),
        Err(error) => {
            let native = MutationError::new(
                if error == E::Cancelled {
                    MutationErrorKind::Cancelled
                } else if error == E::NeedsRecovery {
                    MutationErrorKind::UnknownOutcome
                } else {
                    MutationErrorKind::StalePlan
                },
                format!("album Copy metadata completion: {error}"),
            );
            if error == E::NeedsRecovery {
                MutationOutcome::PendingUnknown(native)
            } else {
                MutationOutcome::NoEffect(native)
            }
        }
    }
}
fn error_status(error: &E) -> JobStatus {
    match error {
        E::Cancelled => JobStatus::Cancelled,
        E::NeedsRecovery => JobStatus::NeedsRecovery,
        _ => JobStatus::Partial,
    }
}

struct BoundedTransferJournal {
    inner: WindowsJournalStore,
    bytes: usize,
    poisoned: bool,
}
impl JournalStore for BoundedTransferJournal {
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        if self.poisoned {
            return Err(E::NeedsRecovery);
        }
        self.poisoned = true;
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= MAX_MANIFEST)
            .ok_or(E::NeedsRecovery)?;
        self.inner.append(bytes)?;
        self.poisoned = false;
        Ok(())
    }
    fn sync(&mut self) -> Result<()> {
        if self.poisoned {
            return Err(E::NeedsRecovery);
        }
        self.poisoned = true;
        self.inner.sync()?;
        self.poisoned = false;
        Ok(())
    }
    fn read_all(&mut self) -> Result<Vec<u8>> {
        self.inner.read_all()
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlbumDirectoryEvidenceStatus {
    SameIdentity,
    Absent,
    Replaced,
    Unverified,
}
#[derive(Clone, Debug, Serialize)]
pub struct AlbumDirectoryEvidence {
    pub reference: StorageRef,
    pub expected_identity: ObjectIdentity,
    pub observed_identity: Option<ObjectIdentity>,
    pub status: AlbumDirectoryEvidenceStatus,
}
#[derive(Clone, Debug)]
pub struct SelectedAlbumRecovery {
    pub directory_metadata: Vec<AlbumDirectoryMetadataReceipt>,
    pub directory_metadata_status: Vec<(StorageRef, String)>,
    pub pending_metadata: Vec<StorageRef>,
    completed_albums: Vec<usize>,
    recorded_status: Option<String>,
    recorded_transfers: Option<(String, usize, usize)>,
    pub whole: Option<WholeAlbumRecovery>,
    pub manifest_finished: bool,
    pub incomplete_tail: bool,
    pub pending_directories: Vec<StorageRef>,
    pub pending_cleanup: Vec<StorageRef>,
    pub directories: Vec<AlbumDirectoryEvidence>,
    pub transfers: Option<crate::recovery::RecoveryReport>,
}
impl SelectedAlbumFileJob {
    /// Inspect only the retained job's exact journals. Releasing guards performs no
    /// cleanup; this method neither reconstructs a writable binding nor replays work.
    pub fn inspect_recovery(&mut self, cancel: &AtomicBool) -> Result<SelectedAlbumRecovery> {
        if !self.attempted {
            return Err(E::Unsupported);
        }
        cancelled(cancel)?;
        self.guards = None;
        self.storage.pool.clear();
        self.storage.watches.clear();
        self.copy_directories.clear();
        self.directories.clear();
        let location = self.locations.first().ok_or(E::Unsupported)?;
        let bytes = self
            .storage
            .roots()
            .read_selected_journal(location.name(), location.identity())
            .map_err(|_| E::Io)?;
        let (mut report, observed) = self.read_manifest(&bytes, cancel)?;
        for receipt in &report.directory_metadata {
            cancelled(cancel)?;
            let full = path(&receipt.destination)?;
            let parent_ref = reference(full.parent().ok_or(E::CorruptJournal)?);
            let parent = self
                .storage
                .roots()
                .parent_reference(&parent_ref)
                .map_err(|_| E::Stale)?;
            let leaf = MutationName::Windows(
                full.file_name()
                    .ok_or(E::CorruptJournal)?
                    .encode_wide()
                    .collect(),
            );
            let expected = &receipt.evidence;
            let status = match self.storage.roots().inspect_selected_copy_metadata(
                &parent,
                &leaf,
                &expected.destination_identity,
                EntryKind::Directory,
            ) {
                Ok(Some(actual)) => {
                    if actual.creation_time == expected.retained_creation_time
                        && actual.access_time == expected.access_time
                        && actual.write_time == expected.write_time
                        && actual.readonly == expected.readonly
                        && actual.retained_attributes == expected.retained_attributes
                    {
                        "verified"
                    } else {
                        "changed"
                    }
                }
                Ok(None) => "missing",
                Err(_) => "unverified",
            };
            report
                .directory_metadata_status
                .push((receipt.destination.clone(), status.into()));
        }
        let mut reads = sorter_io::desktop::DesktopStorage::new(&[
            path(&self.binding.data.source_root.reference)?,
            path(&self.binding.data.target_root.reference)?,
        ])
        .map_err(|_| E::Scope)?;
        for (reference, expected_identity) in observed {
            cancelled(cancel)?;
            let mut evidence = AlbumDirectoryEvidence {
                reference: reference.clone(),
                expected_identity: expected_identity.clone(),
                observed_identity: None,
                status: AlbumDirectoryEvidenceStatus::Unverified,
            };
            match reads.stat(&reference) {
                Ok(current) => {
                    evidence.observed_identity = current.identity.clone();
                    evidence.status = if current.kind == EntryKind::Directory
                        && current.identity.as_ref() == Some(&expected_identity)
                    {
                        AlbumDirectoryEvidenceStatus::SameIdentity
                    } else {
                        AlbumDirectoryEvidenceStatus::Replaced
                    };
                }
                Err(error) if error.kind == StorageErrorKind::NotFound => {
                    evidence.status = AlbumDirectoryEvidenceStatus::Absent
                }
                Err(_) => {}
            }
            report.directories.push(evidence);
        }
        if let Some(location) = self.locations.get(1) {
            let bytes = self
                .storage
                .roots()
                .read_selected_journal(location.name(), location.identity())
                .map_err(|_| E::Io)?;
            let binding = ExecutionBinding {
                data: self.binding.data.clone(),
                recovery_only: true,
            };
            let journal = PhysicalJournalV2::open(&binding, &self.job_id, &bytes)?;
            if let Some((status, committed, moved)) = &report.recorded_transfers {
                let counts = JobResult::from_journal(&journal, vec![], false);
                if *committed != counts.committed_destinations
                    || *moved != counts.moved_sources
                    || (journal.needs_recovery() && status != "NeedsRecovery")
                    || (status == "Complete" && *committed != binding.operations().len())
                {
                    return Err(E::CorruptJournal);
                }
            }
            for operation in &report.completed_albums {
                let album = self
                    .snapshot
                    .albums
                    .iter()
                    .find(|album| album.operation == *operation)
                    .ok_or(E::CorruptJournal)?;
                if album.files.iter().any(|id| {
                    !journal.events().iter().any(|event| {
                        event.operation == *id
                            && event.phase == crate::physical_journal::Phase::Committed
                    })
                }) {
                    return Err(E::CorruptJournal);
                }
            }
            let domain = if self.binding.data.selected_windows_copy {
                ExecutionDomain::SelectedWindowsCopy
            } else {
                ExecutionDomain::SelectedWindowsRename
            };
            let mut storage = crate::windows_copy::WindowsCopyRecovery::new_domain(
                self.storage.roots(),
                &binding,
                domain,
            )?;
            let mut transfers = crate::recovery::recover_job(&binding, &journal, &mut storage)?;
            for operation in &mut transfers.operations {
                operation.operation = self.snapshot.files[operation.operation].original_operation;
            }
            report.transfers = Some(transfers);
        } else if report.recorded_transfers.is_some() {
            return Err(E::CorruptJournal);
        }
        Ok(report)
    }
    fn read_manifest(
        &self,
        bytes: &[u8],
        cancel: &AtomicBool,
    ) -> Result<(SelectedAlbumRecovery, Vec<(StorageRef, ObjectIdentity)>)> {
        if bytes.len() > MAX_MANIFEST {
            return Err(E::Unsupported);
        }
        let mut report = SelectedAlbumRecovery {
            directory_metadata: vec![],
            directory_metadata_status: vec![],
            pending_metadata: vec![],
            completed_albums: vec![],
            recorded_status: None,
            recorded_transfers: None,
            whole: None,
            manifest_finished: false,
            incomplete_tail: false,
            pending_directories: vec![],
            pending_cleanup: vec![],
            directories: vec![],
            transfers: None,
        };
        let mut observed = Vec::new();
        let mut sequence = 0;
        let mut previous = String::new();
        let mut offset = 0;
        let mut header = false;
        let mut pending_create: Option<(StorageRef, ObjectIdentity, String)> = None;
        let mut pending_cleanup: Option<EntryObservation> = None;
        let mut transfers_prepared = false;
        let mut transfers_observed = false;
        let mut created = HashSet::new();
        let mut created_evidence = Vec::<MutationObservation>::new();
        let mut removed_evidence = Vec::<MutationObservation>::new();
        let mut retained_evidence = Vec::<RetainedDirectory>::new();
        let mut transfer_status = None::<String>;
        let mut cleaned = HashSet::new();
        let mut stopped_status = None::<String>;
        let mut pending_metadata = None::<(usize, ObjectIdentity)>;
        let mut metadata_stopped = None::<String>;
        while offset < bytes.len() {
            cancelled(cancel)?;
            let Some(end) = bytes[offset..].iter().position(|b| *b == b'\n') else {
                report.incomplete_tail = true;
                break;
            };
            let record: ManifestEnvelope = serde_json::from_slice(&bytes[offset..offset + end])
                .map_err(|_| E::CorruptJournal)?;
            offset += end + 1;
            if report.manifest_finished
                || record.body.sequence != sequence
                || record.body.previous != previous
                || record.body.job_id != self.job_id
                || crate::hash(&record.body)? != record.sha256
            {
                return Err(E::CorruptJournal);
            }
            sequence += 1;
            previous = record.sha256;
            let record = record.body;
            if stopped_status.is_some() && record.phase != "Finished" {
                return Err(E::CorruptJournal);
            }
            match record.phase.as_str() {
                "Header" => {
                    if header
                        || sequence != 1
                        || record.payload
                            != serde_json::json!({"schema":1,"binding":self.binding.data,"snapshot":self.snapshot})
                    {
                        return Err(E::CorruptJournal);
                    }
                    header = true;
                }
                "DirectoryPrepared" => {
                    if !header || pending_create.is_some() || transfers_prepared {
                        return Err(E::CorruptJournal);
                    }
                    let reference: StorageRef =
                        serde_json::from_value(record.payload["reference"].clone())
                            .map_err(|_| E::CorruptJournal)?;
                    let parent: ObjectIdentity =
                        serde_json::from_value(record.payload["parent"].clone())
                            .map_err(|_| E::CorruptJournal)?;
                    let leaf = record.payload["name"]
                        .as_str()
                        .ok_or(E::CorruptJournal)?
                        .to_owned();
                    if !self.snapshot.destinations.iter().any(|d| {
                        d.existing.is_none()
                            && destination_path(&d.destination)
                                .is_ok_and(|p| self::reference(&p) == reference)
                            && d.destination.components.last() == Some(&leaf)
                    }) || created.contains(&reference)
                    {
                        return Err(E::CorruptJournal);
                    }
                    if directory_parent_identity(
                        &self.snapshot,
                        &self.binding.data.target_root.reference,
                        &self.binding.data.scope_evidence.target_root,
                        &reference,
                        &created_evidence,
                    )? != parent
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_create = Some((reference, parent, leaf));
                }
                "DirectoryObserved" => {
                    let observation: MutationObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    let (reference, parent, leaf) =
                        pending_create.take().ok_or(E::CorruptJournal)?;
                    if observation.reference != reference
                        || observation.parent_identity != parent
                        || observation.actual_name != name(&leaf)
                        || observation.size.is_some()
                        || !new_directory_identity(
                            &self.snapshot,
                            &self.binding.data.scope_evidence.source_root,
                            &self.binding.data.scope_evidence.target_root,
                            &observation.identity,
                            &created_evidence,
                        )
                    {
                        return Err(E::CorruptJournal);
                    }
                    created.insert(reference.clone());
                    created_evidence.push(observation.clone());
                    observed.push((reference, observation.identity));
                }
                "TransfersPrepared" => {
                    if !header
                        || pending_create.is_some()
                        || transfers_prepared
                        || record.payload["binding_sha256"] != self.binding.sha256()?
                        || self
                            .snapshot
                            .destinations
                            .iter()
                            .filter(|d| d.existing.is_none())
                            .any(|d| {
                                destination_path(&d.destination)
                                    .map_or(true, |p| !created.contains(&reference(&p)))
                            })
                    {
                        return Err(E::CorruptJournal);
                    }
                    transfers_prepared = true;
                }
                "TransfersObserved" => {
                    if !transfers_prepared || transfers_observed || pending_metadata.is_some() {
                        return Err(E::CorruptJournal);
                    }
                    if record.payload["committed_destinations"]
                        .as_u64()
                        .is_none_or(|n| n > self.binding.operations().len() as u64)
                        || record.payload["moved_sources"].as_u64().is_none_or(|n| {
                            n > self.binding.data.scope_evidence.inspected_sources.len() as u64
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(
                        status,
                        "Complete" | "Partial" | "Cancelled" | "NeedsRecovery"
                    ) || (status == "Complete"
                        && (record.payload["committed_destinations"].as_u64()
                            != Some(self.binding.operations().len() as u64)
                            || record.payload["moved_sources"].as_u64()
                                != Some(if self.binding.data.selected_windows_copy {
                                    0
                                } else {
                                    self.binding.data.scope_evidence.inspected_sources.len() as u64
                                })))
                    {
                        return Err(E::CorruptJournal);
                    }
                    if metadata_stopped
                        .as_deref()
                        .is_some_and(|expected| expected != status)
                    {
                        return Err(E::CorruptJournal);
                    }
                    report.recorded_transfers = Some((
                        status.to_owned(),
                        record.payload["committed_destinations"]
                            .as_u64()
                            .ok_or(E::CorruptJournal)? as usize,
                        record.payload["moved_sources"]
                            .as_u64()
                            .ok_or(E::CorruptJournal)? as usize,
                    ));
                    transfer_status = Some(status.to_owned());
                    transfers_observed = true;
                }
                "DirectoryMetadataPrepared" => {
                    if !transfers_prepared
                        || transfers_observed
                        || !self.binding.data.selected_windows_copy
                        || pending_metadata.is_some()
                        || metadata_stopped.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let index = self
                        .snapshot
                        .copied_directories
                        .iter()
                        .position(|directory| {
                            record.payload["source"]
                                == serde_json::to_value(&directory.source).unwrap_or_default()
                                && record.payload["destination"]
                                    == serde_json::to_value(&directory.destination)
                                        .unwrap_or_default()
                        })
                        .ok_or(E::CorruptJournal)?;
                    let directory = &self.snapshot.copied_directories[index];
                    let destination = reference(&destination_path(&directory.destination)?);
                    let created = created_evidence
                        .iter()
                        .find(|entry| entry.reference == destination)
                        .ok_or(E::CorruptJournal)?;
                    if record.payload["created"]
                        != serde_json::to_value(created).map_err(|_| E::Io)?
                        || report
                            .directory_metadata
                            .iter()
                            .any(|receipt| receipt.destination == destination)
                        || self.snapshot.copied_directories[..index]
                            .iter()
                            .filter(|prior| prior.operation == directory.operation)
                            .any(|prior| {
                                destination_path(&prior.destination).map_or(true, |p| {
                                    !report
                                        .directory_metadata
                                        .iter()
                                        .any(|receipt| receipt.destination == reference(&p))
                                })
                            })
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_metadata = Some((index, created.identity.clone()));
                }
                "DirectoryMetadataObserved" => {
                    let (index, id) = pending_metadata.take().ok_or(E::CorruptJournal)?;
                    let directory = &self.snapshot.copied_directories[index];
                    let receipt: AlbumDirectoryMetadataReceipt =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if receipt.destination != reference(&destination_path(&directory.destination)?)
                        || receipt.evidence.destination_identity != id
                        || Some(&receipt.evidence.source_identity)
                            != directory.source.identity.as_ref()
                        || receipt.evidence.access_time <= 0
                        || receipt.evidence.write_time <= 0
                        || receipt.evidence.retained_attributes & 0x81 != 0
                    {
                        return Err(E::CorruptJournal);
                    }
                    report.directory_metadata.push(receipt);
                }
                "DirectoryMetadataStopped" => {
                    let (index, _) = pending_metadata.take().ok_or(E::CorruptJournal)?;
                    let destination = reference(&destination_path(
                        &self.snapshot.copied_directories[index].destination,
                    )?);
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(status, "Partial" | "Cancelled" | "NeedsRecovery")
                        || record.payload["destination"]
                            != serde_json::to_value(&destination).map_err(|_| E::Io)?
                    {
                        return Err(E::CorruptJournal);
                    }
                    if status == "NeedsRecovery" {
                        report.pending_metadata.push(destination);
                    }
                    metadata_stopped = Some(status.to_owned());
                }
                "AlbumCompleted" => {
                    if !transfers_prepared
                        || transfers_observed
                        || pending_metadata.is_some()
                        || metadata_stopped.is_some()
                        || !self.binding.data.selected_windows_copy
                    {
                        return Err(E::CorruptJournal);
                    }
                    let operation = record.payload["operation"]
                        .as_u64()
                        .and_then(|id| usize::try_from(id).ok())
                        .ok_or(E::CorruptJournal)?;
                    let album = self
                        .snapshot
                        .albums
                        .iter()
                        .find(|album| album.operation == operation)
                        .ok_or(E::CorruptJournal)?;
                    if report.completed_albums.contains(&operation)
                        || record.payload
                            != serde_json::json!({"operation":operation,"files":album.files})
                    {
                        return Err(E::CorruptJournal);
                    }
                    if self
                        .snapshot
                        .copied_directories
                        .iter()
                        .filter(|directory| directory.operation == operation)
                        .any(|directory| {
                            destination_path(&directory.destination).map_or(true, |p| {
                                !report
                                    .directory_metadata
                                    .iter()
                                    .any(|receipt| receipt.destination == reference(&p))
                            })
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    report.completed_albums.push(operation);
                }
                "CleanupPrepared" => {
                    if transfer_status.as_deref() != Some("Complete")
                        || self.binding.data.selected_windows_copy
                        || pending_cleanup.is_some()
                    {
                        return Err(E::CorruptJournal);
                    }
                    let directory: EntryObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if !self
                        .snapshot
                        .albums
                        .iter()
                        .any(|a| a.cleanup.contains(&directory))
                        || cleaned.contains(&directory.reference)
                    {
                        return Err(E::CorruptJournal);
                    }
                    pending_cleanup = Some(directory);
                }
                "CleanupObserved" | "CleanupUnknown" => {
                    let observation: MutationObservation =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    let directory = pending_cleanup.take().ok_or(E::CorruptJournal)?;
                    if Some(&observation.identity) != directory.identity.as_ref()
                        || Some(&observation.parent_identity) != directory.parent_identity.as_ref()
                        || observation.actual_name != leaf(&directory)?
                    {
                        return Err(E::CorruptJournal);
                    }
                    if record.phase == "CleanupObserved" {
                        cleaned.insert(directory.reference.clone());
                        removed_evidence.push(observation.clone());
                    } else {
                        report.pending_cleanup.push(directory.reference.clone());
                    }
                    observed.push((directory.reference, observation.identity));
                }
                "DirectoryRetained" => {
                    if !transfers_observed {
                        return Err(E::CorruptJournal);
                    }
                    let retained: RetainedDirectory =
                        serde_json::from_value(record.payload).map_err(|_| E::CorruptJournal)?;
                    if !self
                        .snapshot
                        .albums
                        .iter()
                        .flat_map(|a| &a.cleanup)
                        .any(|d| {
                            d.reference == retained.reference
                                && d.identity.as_ref() == Some(&retained.identity)
                        })
                    {
                        return Err(E::CorruptJournal);
                    }
                    if let Some(pending) = pending_cleanup.take()
                        && pending.reference != retained.reference
                    {
                        return Err(E::CorruptJournal);
                    }
                    if retained_evidence
                        .iter()
                        .any(|r| r.reference == retained.reference)
                    {
                        return Err(E::CorruptJournal);
                    }
                    retained_evidence.push(retained.clone());
                    observed.push((retained.reference, retained.identity));
                }
                "ExecutionStopped" => {
                    if !header || stopped_status.is_some() {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(status, "Partial" | "Cancelled" | "NeedsRecovery") {
                        return Err(E::CorruptJournal);
                    }
                    let valid = match record.payload["stage"].as_str() {
                        Some("directories") => !transfers_prepared,
                        Some("transfer_journal") => {
                            !transfers_prepared
                                && pending_create.is_none()
                                && status == "NeedsRecovery"
                        }
                        Some("membership") => {
                            transfer_status.as_deref() == Some("Complete") && status == "Partial"
                        }
                        Some("cleanup") => {
                            transfer_status.as_deref() == Some("Complete")
                                && !self.binding.data.selected_windows_copy
                        }
                        _ => false,
                    };
                    if !valid {
                        return Err(E::CorruptJournal);
                    }
                    stopped_status = Some(status.to_owned());
                }
                "Finished" => {
                    if !header || pending_metadata.is_some() {
                        return Err(E::CorruptJournal);
                    }
                    let status = record.payload["status"].as_str().ok_or(E::CorruptJournal)?;
                    if !matches!(
                        status,
                        "Complete" | "Partial" | "Cancelled" | "NeedsRecovery"
                    ) || record.payload["created"]
                        != serde_json::to_value(&created_evidence).map_err(|_| E::Io)?
                        || record.payload["removed"]
                            != serde_json::to_value(&removed_evidence).map_err(|_| E::Io)?
                        || record.payload["retained"]
                            != serde_json::to_value(&retained_evidence).map_err(|_| E::Io)?
                        || record.payload["albums_completed"].as_u64()
                            != Some(report.completed_albums.len() as u64)
                    {
                        return Err(E::CorruptJournal);
                    }
                    if status == "Complete"
                        && (pending_metadata.is_some()
                            || !report.pending_metadata.is_empty()
                            || report.completed_albums.len() != self.snapshot.albums.len()
                            || transfer_status.as_deref() != Some("Complete")
                            || pending_create.is_some()
                            || pending_cleanup.is_some()
                            || !report.pending_cleanup.is_empty()
                            || !retained_evidence.is_empty())
                    {
                        return Err(E::CorruptJournal);
                    }
                    if (transfer_status.as_deref() == Some("NeedsRecovery")
                        || !report.pending_cleanup.is_empty())
                        && status != "NeedsRecovery"
                    {
                        return Err(E::CorruptJournal);
                    }
                    let expected = stopped_status
                        .as_deref()
                        .or({
                            if !retained_evidence.is_empty() {
                                Some("Partial")
                            } else {
                                transfer_status.as_deref()
                            }
                        })
                        .ok_or(E::CorruptJournal)?;
                    if status != expected {
                        return Err(E::CorruptJournal);
                    }
                    report.recorded_status = Some(status.to_owned());
                    report.manifest_finished = true;
                }
                _ => return Err(E::CorruptJournal),
            }
        }
        if !header {
            return Err(E::CorruptJournal);
        }
        if let Some((index, _)) = pending_metadata {
            report.pending_metadata.push(reference(&destination_path(
                &self.snapshot.copied_directories[index].destination,
            )?));
        }
        if let Some((reference, ..)) = pending_create {
            report.pending_directories.push(reference);
        }
        if let Some(directory) = pending_cleanup {
            report.pending_cleanup.push(directory.reference);
        }
        Ok((report, observed))
    }
}

#[path = "windows_album_move.rs"]
mod whole;
pub use whole::{
    AlbumChildLocationEvidence, AlbumChildRenameReceipt, AlbumFailure, AlbumFileEvidence,
    WholeAlbumOutcome, WholeAlbumRecovery,
};
/// Product album dispatcher. Fresh host grants select the explicit whole-root
/// authority before any generic source pin is constructed.
pub struct SelectedWindowsAlbumJob {
    inner: AlbumJobKind,
}
enum AlbumJobKind {
    Files(Box<SelectedAlbumFileJob>),
    Whole(Box<whole::WholeAlbumJob>),
}
impl SelectedWindowsAlbumJob {
    pub fn prepare(
        source: WindowsSelectedRootGrant,
        target: WindowsSelectedRootGrant,
        journal: WindowsSelectedRootGrant,
        validated: ValidatedAlbumExecution,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let inner = if whole::eligible(&validated.plan, &validated.scope) {
            AlbumJobKind::Whole(Box::new(whole::WholeAlbumJob::prepare(
                source, target, journal, validated, cancel,
            )?))
        } else {
            let roots = WindowsMutationStorage::for_selected_roots(source, target, journal)
                .map_err(|_| E::Scope)?;
            AlbumJobKind::Files(Box::new(SelectedAlbumFileJob::prepare(
                roots, &validated, cancel,
            )?))
        };
        Ok(Self { inner })
    }
    pub fn job_id(&self) -> &str {
        match &self.inner {
            AlbumJobKind::Files(job) => job.job_id(),
            AlbumJobKind::Whole(job) => job.job_id(),
        }
    }
    pub fn journal_locations(&self) -> &[WindowsJournalLocation] {
        match &self.inner {
            AlbumJobKind::Files(job) => job.journal_locations(),
            AlbumJobKind::Whole(job) => job.journal_locations(),
        }
    }
    pub fn binding_sha256(&self) -> Result<String> {
        match &self.inner {
            AlbumJobKind::Files(job) => job.binding_sha256(),
            AlbumJobKind::Whole(job) => job.binding_sha256(),
        }
    }
    pub fn run(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(TransferProgress),
    ) -> Result<SelectedAlbumResult> {
        match &mut self.inner {
            AlbumJobKind::Files(job) => job.run(cancel, progress),
            AlbumJobKind::Whole(job) => job.run(cancel, progress),
        }
    }
    pub fn inspect_recovery(&mut self, cancel: &AtomicBool) -> Result<SelectedAlbumRecovery> {
        match &mut self.inner {
            AlbumJobKind::Files(job) => job.inspect_recovery(cancel),
            AlbumJobKind::Whole(job) => {
                let report = job.inspect_recovery(cancel)?;
                Ok(SelectedAlbumRecovery {
                    directory_metadata: vec![],
                    directory_metadata_status: vec![],
                    pending_metadata: vec![],
                    completed_albums: vec![],
                    recorded_status: None,
                    recorded_transfers: None,
                    manifest_finished: report.recorded_finished,
                    incomplete_tail: report.incomplete_tail,
                    pending_directories: report.pending_directories.clone(),
                    pending_cleanup: vec![],
                    directories: vec![],
                    transfers: None,
                    whole: Some(report),
                })
            }
        }
    }
}

fn album_artist_counts(
    snapshot: &Snapshot,
    completed: impl Iterator<Item = usize>,
) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = vec![];
    for operation in completed {
        let Some(album) = snapshot.albums.iter().find(|a| a.operation == operation) else {
            continue;
        };
        let count = album
            .files
            .iter()
            .filter(|id| {
                snapshot.files[**id].destination.components.len()
                    == album.destination.components.len() + 1
            })
            .count();
        if let Some((_, total)) = counts
            .iter_mut()
            .find(|(artist, _)| artist == &album.artist)
        {
            *total += count;
        } else {
            counts.push((album.artist.clone(), count));
        }
    }
    counts
}

/// Opaque validation result; contains no filesystem grant and cannot be decoded
/// from a renderer, journal, or external request. Native acquisition rechecks
/// the exact granted identities and planned content before any effect.
pub struct ValidatedAlbumExecution {
    plan: JobPlanV2,
    scope: AlbumScope,
    source: EntryObservation,
    target: EntryObservation,
}
/// Call while planning adapters exist, then drop every metadata/read adapter
/// before passing the result to SelectedWindowsAlbumJob::prepare. In particular,
/// a root album cannot be opened for DELETE while a preview pins that root.
pub fn validate_album_execution(
    plan: &JobPlanV2,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &AlbumScope,
    cancel: &AtomicBool,
) -> Result<ValidatedAlbumExecution> {
    supported_album_plan(plan)?;
    cancelled(cancel)?;
    let mut reads = WindowsPlanningStorage::for_album_preview(
        &path(&plan.request.source)?,
        &path(plan.request.target.as_ref().ok_or(E::Scope)?)?,
        sorter_io::desktop::WindowsDirectoryReadLimits::ALBUM_DEFAULT,
        cancel,
    )?;
    let domain = if plan.request.settings.copy_mode {
        ExecutionDomain::SelectedWindowsCopy
    } else {
        ExecutionDomain::SelectedWindowsRename
    };
    let (source, target) = validate_plan_scope_domain(
        plan,
        &mut reads,
        metadata,
        recognition,
        resources,
        &scope.execution,
        domain,
    )?;
    let fresh = prepare_album_scope(plan, &mut reads, resources, cancel)?;
    if fresh.snapshot != scope.snapshot
        || scope_digest(&fresh.execution)? != scope_digest(&scope.execution)?
    {
        return Err(E::Stale);
    }
    cancelled(cancel)?;
    Ok(ValidatedAlbumExecution {
        plan: plan.clone(),
        scope: scope.clone(),
        source,
        target,
    })
}

fn new_directory_identity(
    snapshot: &Snapshot,
    source: &ObjectIdentity,
    target: &ObjectIdentity,
    id: &ObjectIdentity,
    created: &[MutationObservation],
) -> bool {
    matches!((id,target),(ObjectIdentity::Windows{volume:a,..},ObjectIdentity::Windows{volume:b,..})if a==b)
        && id != source
        && id != target
        && !snapshot.directories.iter().any(|d| {
            d.source.identity.as_ref() == Some(id)
                || d.children.iter().any(|c| c.identity.as_ref() == Some(id))
        })
        && !snapshot.destinations.iter().any(|d| {
            d.existing
                .as_ref()
                .is_some_and(|e| e.identity.as_ref() == Some(id))
        })
        && !created.iter().any(|o| &o.identity == id)
}
fn directory_parent_identity(
    snapshot: &Snapshot,
    target_reference: &StorageRef,
    target_id: &ObjectIdentity,
    child: &StorageRef,
    created: &[MutationObservation],
) -> Result<ObjectIdentity> {
    let child = path(child)?;
    let parent = reference(child.parent().ok_or(E::CorruptJournal)?);
    if &parent == target_reference {
        return Ok(target_id.clone());
    }
    if let Some(created) = created.iter().find(|o| o.reference == parent) {
        return Ok(created.identity.clone());
    }
    snapshot
        .destinations
        .iter()
        .find_map(|d| {
            d.existing
                .as_ref()
                .filter(|e| e.reference == parent)
                .and_then(|e| e.identity.clone())
        })
        .ok_or(E::CorruptJournal)
}
