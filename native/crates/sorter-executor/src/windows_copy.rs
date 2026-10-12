//! Experimental single-file Copy on a consumed, caller-owned NTFS fixture tree.
//! No path constructor grants mutation ownership; no Move, albums or tag writes.
use crate::{ExecutionError as E, Result, binding::*, transfer::TransferStorage};
use sorter_core::{discovery::*, job_plan::*, metadata::MetadataProvider};
use sorter_io::desktop::{DesktopStorage, WindowsDirectoryReadLimits, windows_mutation::*};
use sorter_storage_contracts::*;
use std::{
    ffi::OsString,
    io::{self, Read, Seek, SeekFrom, Write},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

fn error(detail: &str) -> MutationError {
    MutationError::new(MutationErrorKind::StalePlan, detail)
}
fn path(reference: &StorageRef) -> std::result::Result<PathBuf, MutationError> {
    match reference {
        StorageRef::WindowsPath { utf16 }
            if !utf16.contains(&0)
                && !utf16
                    .split(|u| *u == 47 || *u == 92)
                    .any(|part| part == [46] || part == [46, 46]) =>
        {
            Ok(OsString::from_wide(utf16).into())
        }
        _ => Err(error("lossless Windows reference required")),
    }
}
fn within_roots(
    reference: &StorageRef,
    roots: &[PathBuf; 2],
) -> std::result::Result<(), StorageError> {
    let path = path(reference).map_err(|_| {
        StorageError::new(StorageErrorKind::OutsideRoots, "invalid owned reference")
    })?;
    if !path.is_absolute() || !roots.iter().any(|root| path.starts_with(root)) {
        return Err(StorageError::new(
            StorageErrorKind::OutsideRoots,
            "reference outside checked owned roots",
        ));
    }
    Ok(())
}
fn reference(path: &Path) -> StorageRef {
    StorageRef::WindowsPath {
        utf16: path.as_os_str().encode_wide().collect(),
    }
}
/// Revalidate against real read handles, then bind only a single Copy beneath
/// the already-owned mutation capability. Serialized paths cannot mint that capability.
pub fn validate_owned_copy(
    owned: &WindowsMutationStorage,
    plan: &JobPlanV2,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
) -> Result<ExecutionBinding> {
    validate_owned_domain(
        owned,
        plan,
        metadata,
        recognition,
        resources,
        scope,
        ExecutionDomain::OwnedWindowsCopy,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_owned_domain(
    owned: &WindowsMutationStorage,
    plan: &JobPlanV2,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
    domain: ExecutionDomain,
) -> Result<ExecutionBinding> {
    if owned.domain() != WindowsMutationDomain::OwnedSyntheticTree {
        return Err(E::Scope);
    }
    let copy = domain == ExecutionDomain::OwnedWindowsCopy;
    if plan.request.settings.copy_mode != copy
        || !plan.request.settings.main_folder_only
        || plan.request.settings.duet_mode
        || plan.operations.len() != 1
        || scope.allowed_modes
            != [if copy {
                TransferMode::Copy
            } else {
                TransferMode::Move
            }]
    {
        return Err(E::Unsupported);
    }
    let target = plan.request.target.as_ref().ok_or(E::Scope)?;
    let source_path = path(&plan.request.source).map_err(|_| E::Scope)?;
    let target_path = path(target).map_err(|_| E::Scope)?;
    if source_path.starts_with(&target_path) || target_path.starts_with(&source_path) {
        return Err(E::Scope);
    }
    let source_parent = owned
        .parent_reference(&plan.request.source)
        .map_err(|_| E::Scope)?;
    let target_parent = owned.parent_reference(target).map_err(|_| E::Scope)?;
    if source_parent
        .contains_ancestor(target_parent.identity())
        .map_err(|_| E::Scope)?
        || target_parent
            .contains_ancestor(source_parent.identity())
            .map_err(|_| E::Scope)?
    {
        return Err(E::Scope);
    }
    if source_parent.identity() != &scope.source_root
        || target_parent.identity() != &scope.target_root
    {
        return Err(E::Scope);
    }
    // Reject external/mixed observations before any planner, metadata or content reads.
    for observation in &plan.observations {
        let observed_path = path(&observation.reference).map_err(|_| E::Scope)?;
        if !observed_path.starts_with(&source_path) && !observed_path.starts_with(&target_path) {
            return Err(E::Scope);
        }
        if !matches!(observation.identity, Some(ObjectIdentity::Windows { .. })) {
            return Err(E::Scope);
        }
    }
    let OperationKind::SingleTransfer {
        source, transfer, ..
    } = &plan.operations[0].kind
    else {
        return Err(E::Unsupported);
    };
    let source_file_path = path(&source.reference).map_err(|_| E::Scope)?;
    let leaf = source_file_path
        .file_name()
        .ok_or(E::Scope)?
        .encode_wide()
        .collect::<Vec<_>>();
    // The planner's display string cannot represent unpaired surrogates. Refuse
    // that unsupported policy boundary instead of publishing a lossy replacement.
    if String::from_utf16(&leaf).map_err(|_| E::Unsupported)? != source.name {
        return Err(E::Unsupported);
    }
    if source_file_path.parent() != Some(source_path.as_path())
        || transfer.destination.root != *target
        || transfer.copy != copy
    {
        return Err(E::Scope);
    }
    // This slice cannot create directories. Resolve all destination ancestors now,
    // and again under the held scope immediately before effects.
    let destination = DestinationEvidence {
        id: String::new(),
        root: target.clone(),
        components: transfer.destination.components.clone(),
    };
    let actual_destination =
        destination_parent(owned, &destination, &scope.target_root).map_err(|_| E::Scope)?;
    if !copy
        && !matches!((&scope.source_root, actual_destination.identity()),
        (ObjectIdentity::Windows { volume: a, .. }, ObjectIdentity::Windows { volume: b, .. }) if a == b)
    {
        return Err(E::Unsupported);
    }
    let mut reads = OwnedCopyReadStorage::new(owned, &plan.request.source, target)?;
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
    Ok(binding)
}
pub(crate) fn destination_parent(
    owned: &WindowsMutationStorage,
    destination: &DestinationEvidence,
    root: &ObjectIdentity,
) -> std::result::Result<WindowsParent, MutationError> {
    let anchor = owned.parent_reference(&destination.root)?;
    if anchor.identity() != root || destination.components.is_empty() {
        return Err(error("target root differs"));
    }
    let mut directory = path(&destination.root)?;
    for component in &destination.components[..destination.components.len() - 1] {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.contains(['/', '\\', ':', '\0'])
        {
            return Err(error("unsafe destination component"));
        }
        directory.push(component);
    }
    let parent = owned.parent_reference(&reference(&directory))?;
    if !parent.contains_ancestor(root)? {
        return Err(error("target ancestor differs"));
    }
    Ok(parent)
}
impl TransferStorage for WindowsMutationStorage {
    #[cfg(feature="selected-windows-execution")]
    fn finalize_selected_copy_metadata(&mut self,source:&WindowsSourceLease,temp:&mut WindowsOwnedTemp,cancel:&std::sync::atomic::AtomicBool)->MutationOutcome<crate::physical_journal::CopyMetadataEvidence>{
        match WindowsMutationStorage::finalize_selected_copy_metadata(self,source,temp,cancel){
            MutationOutcome::Applied(value)=>MutationOutcome::Applied(crate::physical_journal::CopyMetadataEvidence{source_identity:value.source_identity,destination_identity:value.destination_identity,access_time:value.access_time,write_time:value.write_time,readonly:value.readonly,retained_creation_time:value.retained_creation_time,retained_attributes:value.retained_attributes}),
            MutationOutcome::NoEffect(error)=>MutationOutcome::NoEffect(error),MutationOutcome::PendingUnknown(error)=>MutationOutcome::PendingUnknown(error),
        }
    }

    fn source_parent(
        &mut self,
        source: &SourceEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<WindowsParent, MutationError> {
        let path = path(&source.observation.reference)?;
        let parent = self.parent_reference(&reference(
            path.parent()
                .ok_or_else(|| error("missing source parent"))?,
        ))?;
        if !parent.contains_ancestor(root)? {
            return Err(error("source root differs"));
        }
        Ok(parent)
    }
    fn destination_parent(
        &mut self,
        destination: &DestinationEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<WindowsParent, MutationError> {
        destination_parent(self, destination, root)
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
        source.rewind()
    }
    fn read_source(
        &mut self,
        source: &mut WindowsSourceLease,
        buffer: &mut [u8],
    ) -> io::Result<usize> {
        source.read(buffer)
    }
    fn write_temp(&mut self, temp: &mut WindowsOwnedTemp, buffer: &[u8]) -> io::Result<usize> {
        temp.write(buffer)
    }
    fn sync_temp(&mut self, temp: &mut WindowsOwnedTemp) -> std::result::Result<(), MutationError> {
        temp.sync_data()
    }
    fn verify_temp(
        &mut self,
        temp: &WindowsOwnedTemp,
        digest: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        temp.verify_content(digest)
    }
    fn rename_eligible(&self, _: &WindowsSourceLease, _: &WindowsParent) -> bool {
        false
    }
    fn sync_owned_windows_publication(
        &mut self,
        published: &mut WindowsPublished,
        digest: &ContentDigest,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> std::result::Result<MutationObservation, MutationError> {
        let ack = self.sync_published_file(published, digest, cancel)?;
        Ok(ack.observation().clone())
    }
}

/// Fresh owned-root identity verification followed by read-only reconciliation.
/// Retained in-process handles model close/reopen; no cross-process grant is inferred.
pub fn open_owned_copy_recovery(
    owned: &WindowsMutationStorage,
    scope: &RecoveryScope,
    job_id: &str,
    bytes: &[u8],
) -> Result<crate::physical_journal::PhysicalJournalV2> {
    open_owned_recovery_domain(
        owned,
        scope,
        job_id,
        bytes,
        ExecutionDomain::OwnedWindowsCopy,
    )
}
pub(crate) fn open_owned_recovery_domain(
    owned: &WindowsMutationStorage,
    scope: &RecoveryScope,
    job_id: &str,
    bytes: &[u8],
    domain: ExecutionDomain,
) -> Result<crate::physical_journal::PhysicalJournalV2> {
    let selected = matches!(
        domain,
        ExecutionDomain::SelectedWindowsCopy | ExecutionDomain::SelectedWindowsRename
    );
    if owned.domain()
        != if selected {
            WindowsMutationDomain::SelectedRoots
        } else {
            WindowsMutationDomain::OwnedSyntheticTree
        }
    {
        return Err(E::Scope);
    }
    let journal = crate::physical_journal::PhysicalJournalV2::open_for_recovery_domain(
        scope, job_id, bytes, domain,
    )?;
    let binding = journal.binding();
    for observed in [&binding.data.source_root, &binding.data.target_root] {
        let parent = owned
            .parent_reference(&observed.reference)
            .map_err(|_| E::Scope)?;
        if Some(parent.identity()) != observed.identity.as_ref() {
            return Err(E::Scope);
        }
    }
    Ok(journal)
}

/// Read-only observations limited to one recovered binding and its freshly checked
/// owned roots. It cannot resume, publish, delete, append or adopt a found temp.
pub struct WindowsCopyRecovery<'a> {
    owned:&'a WindowsMutationStorage,
    binding: ExecutionBinding,
    reads: DesktopStorage,
    roots: [PathBuf; 2],
    _source: Option<WindowsParent>,
    _target: Option<WindowsParent>,
}
impl<'a> WindowsCopyRecovery<'a> {
    pub fn new(owned: &'a WindowsMutationStorage, binding: &ExecutionBinding) -> Result<Self> {
        Self::new_domain(owned, binding, ExecutionDomain::OwnedWindowsCopy)
    }
    pub(crate) fn new_domain(
        owned: &'a WindowsMutationStorage,
        binding: &ExecutionBinding,
        domain: ExecutionDomain,
    ) -> Result<Self> {
        let selected = matches!(
            domain,
            ExecutionDomain::SelectedWindowsCopy | ExecutionDomain::SelectedWindowsRename
        );
        if owned.domain()
            != if selected {
                WindowsMutationDomain::SelectedRoots
            } else {
                WindowsMutationDomain::OwnedSyntheticTree
            }
        {
            return Err(E::Scope);
        }
        if !binding.recovery_only || binding.data.domain() != domain || !binding.data.valid_domain()
        {
            return Err(E::Scope);
        }
        let source = owned
            .parent_reference(&binding.data.source_root.reference)
            .map_err(|_| E::Scope)?;
        let target = owned
            .parent_reference(&binding.data.target_root.reference)
            .map_err(|_| E::Scope)?;
        if source
            .contains_ancestor(target.identity())
            .map_err(|_| E::Scope)?
            || target
                .contains_ancestor(source.identity())
                .map_err(|_| E::Scope)?
        {
            return Err(E::Scope);
        }
        if Some(source.identity()) != binding.data.source_root.identity.as_ref()
            || Some(target.identity()) != binding.data.target_root.identity.as_ref()
        {
            return Err(E::Scope);
        }
        let roots = [
            path(&binding.data.source_root.reference).map_err(|_| E::Scope)?,
            path(&binding.data.target_root.reference).map_err(|_| E::Scope)?,
        ];
        let reads = DesktopStorage::new(&roots).map_err(|_| E::Scope)?;
        Ok(Self {
            owned,
            binding: binding.clone(),
            reads,
            roots,
            _source: Some(source),
            _target: Some(target),
        })
    }
    fn observe(
        &mut self,
        reference: &StorageRef,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        within_roots(reference, &self.roots).map_err(|_| E::Scope)?;
        let before = match self.reads.stat(reference) {
            Ok(e) => e,
            Err(e) if e.kind == StorageErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(E::Io),
        };
        let mut input = self.reads.open_read(reference).map_err(|_| E::Io)?;
        let bound = input.binding().ok_or(E::Stale)?;
        if Some(&bound.identity) != before.identity.as_ref()
            || Some(bound.size) != before.size
            || Some(&bound.modified) != before.modified.as_ref()
        {
            return Err(E::Stale);
        }
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        let mut buffer = [0; 65536];
        let mut length = 0u64;
        loop {
            let n = input.read(&mut buffer).map_err(|_| E::Io)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
            length += n as u64;
            if Some(length) > before.size {
                return Err(E::Stale);
            }
        }
        if Some(length) != before.size
            || self.reads.stat(reference).map_err(|_| E::Stale)? != before
        {
            return Err(E::Stale);
        }
        Ok(Some(crate::recovery::ObservedObject {
            identity: before.identity.ok_or(E::Stale)?,
            sha256: format!("{:x}", hash.finalize()),
        }))
    }
}
impl crate::recovery::RecoveryStorage for WindowsCopyRecovery<'_> {
    fn verify_copy_metadata(&mut self,destination:&DestinationEvidence,expected:&crate::physical_journal::CopyMetadataEvidence)->Result<bool>{
        if !self.binding.operations().iter().any(|op|op.destination==*destination&&op.source.observation.identity.as_ref()==Some(&expected.source_identity)){return Err(E::Scope)}
        let parent=destination_parent(self.owned,destination,self.binding.data.target_root.identity.as_ref().ok_or(E::Scope)?).map_err(|_|E::Stale)?;
        let leaf=MutationName::Windows(destination.components.last().ok_or(E::Scope)?.encode_utf16().collect());
        let actual=self.owned.inspect_selected_copy_metadata(&parent,&leaf,&expected.destination_identity,EntryKind::File).map_err(|_|E::Stale)?;
        Ok(actual.is_some_and(|actual|actual.creation_time==expected.retained_creation_time&&actual.access_time==expected.access_time&&actual.write_time==expected.write_time&&actual.readonly==expected.readonly&&actual.retained_attributes==expected.retained_attributes))
    }

    fn source(
        &mut self,
        reference: &StorageRef,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        if !self
            .binding
            .operations()
            .iter()
            .any(|op| op.source.observation.reference == *reference)
        {
            return Err(E::Scope);
        }
        self.observe(reference)
    }
    fn destination(
        &mut self,
        destination: &DestinationEvidence,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        if !self
            .binding
            .operations()
            .iter()
            .any(|op| op.destination == *destination)
        {
            return Err(E::Scope);
        }
        let mut destination_path = path(&destination.root).map_err(|_| E::Scope)?;
        for component in &destination.components {
            destination_path.push(component);
        }
        self.observe(&reference(&destination_path))
    }
}

/// Read-only fixture planning adapter. Capabilities are observed by real I/O,
/// never inferred from a path or promoted in the ordinary preview backend.
pub struct OwnedCopyReadStorage {
    reads: DesktopStorage,
    roots: [PathBuf; 2],
    _source: Option<WindowsParent>,
    _target: Option<WindowsParent>,
}
/// Read-only capability-probing adapter for a selected Windows preview.
pub type WindowsPlanningStorage = OwnedCopyReadStorage;
impl OwnedCopyReadStorage {
    /// Explicit capped/cancellable route for album preview and revalidation.
    pub fn for_album_preview<'a>(
        source: &Path,
        target: &Path,
        limits: WindowsDirectoryReadLimits,
        cancel: &'a AtomicBool,
    ) -> Result<WindowsAlbumPlanningStorage<'a>> {
        limits.validate().map_err(|_| E::Unsupported)?;
        if cancel.load(Ordering::Acquire) { return Err(E::Cancelled); }
        Ok(WindowsAlbumPlanningStorage { inner: Self::for_preview(source, target)?, limits, cancel })
    }
    pub fn for_album<'a>(
        owned: &WindowsMutationStorage,
        source: &StorageRef,
        target: &StorageRef,
        limits: WindowsDirectoryReadLimits,
        cancel: &'a AtomicBool,
    ) -> Result<WindowsAlbumPlanningStorage<'a>> {
        limits.validate().map_err(|_| E::Unsupported)?;
        if cancel.load(Ordering::Acquire) { return Err(E::Cancelled); }
        Ok(WindowsAlbumPlanningStorage { inner: Self::new(owned, source, target)?, limits, cancel })
    }
    /// Probes read/list/seek availability; grants no mutation authority.
    pub fn for_preview(source: &Path, target: &Path) -> Result<Self> {
        if !source.is_absolute()
            || !target.is_absolute()
            || source.starts_with(target)
            || target.starts_with(source)
        {
            return Err(E::Scope);
        }
        let roots = [source.to_path_buf(), target.to_path_buf()];
        let reads = DesktopStorage::new(&roots).map_err(|_| E::Scope)?;
        Ok(Self {
            reads,
            roots,
            _source: None,
            _target: None,
        })
    }
    pub fn new(
        owned: &WindowsMutationStorage,
        source: &StorageRef,
        target: &StorageRef,
    ) -> Result<Self> {
        let source_path = path(source).map_err(|_| E::Scope)?;
        let target_path = path(target).map_err(|_| E::Scope)?;
        if source_path.starts_with(&target_path) || target_path.starts_with(&source_path) {
            return Err(E::Scope);
        }
        let source = owned.parent_reference(source).map_err(|_| E::Scope)?;
        let target = owned.parent_reference(target).map_err(|_| E::Scope)?;
        if source
            .contains_ancestor(target.identity())
            .map_err(|_| E::Scope)?
            || target
                .contains_ancestor(source.identity())
                .map_err(|_| E::Scope)?
        {
            return Err(E::Scope);
        }
        let roots = [source_path, target_path];
        let reads = DesktopStorage::new(&roots).map_err(|_| E::Scope)?;
        Ok(Self {
            reads,
            roots,
            _source: Some(source),
            _target: Some(target),
        })
    }
    /// Single-root capability-probing read adapter for original FixNames policy.
    pub fn for_repair_preview(source: &Path) -> Result<Self> {
        if !source.is_absolute() {
            return Err(E::Scope);
        }
        let roots = [source.to_path_buf(), source.to_path_buf()];
        let reads = DesktopStorage::new(&[source.to_path_buf()]).map_err(|_| E::Scope)?;
        Ok(Self {
            reads,
            roots,
            _source: None,
            _target: None,
        })
    }
    pub fn for_repair(owned: &WindowsMutationStorage) -> Result<Self> {
        if owned.domain() != WindowsMutationDomain::SelectedRepair {
            return Err(E::Scope);
        }
        let source = owned.selected_repair_root().map_err(|_| E::Scope)?;
        let reference = owned.selected_repair_reference().map_err(|_| E::Scope)?;
        let mut result = Self::for_repair_preview(&path(&reference).map_err(|_| E::Scope)?)?;
        result._source = Some(source);
        Ok(result)
    }
    pub fn reference(&mut self, path: &Path) -> std::result::Result<StorageRef, StorageError> {
        within_roots(&reference(path), &self.roots)?;
        self.reads.reference(path)
    }
    fn establish(
        &mut self,
        observed: EntryObservation,
    ) -> std::result::Result<EntryObservation, StorageError> {
        self.establish_with_limits(observed, None)
    }
    fn children_with_limits(
        &mut self,
        reference: &StorageRef,
        limits: Option<(WindowsDirectoryReadLimits, &AtomicBool)>,
    ) -> std::result::Result<Vec<EntryObservation>, StorageError> {
        match limits {
            Some((limits, cancel)) => self.reads.children_bounded(reference, limits, cancel),
            None => self.reads.children(reference),
        }
    }
    fn establish_with_limits(
        &mut self,
        mut observed: EntryObservation,
        limits: Option<(WindowsDirectoryReadLimits, &AtomicBool)>,
    ) -> std::result::Result<EntryObservation, StorageError> {
        check_album_read_cancel(limits)?;
        within_roots(&observed.reference, &self.roots)?;
        let caps = match observed.kind {
            EntryKind::Directory => {
                self.children_with_limits(&observed.reference, limits)?;
                Capabilities {
                    read: Some(false),
                    list: Some(true),
                    seek: Some(false),
                }
            }
            EntryKind::File => {
                let mut input = self.reads.open_read(&observed.reference)?;
                let bound = input.binding().ok_or_else(|| {
                    StorageError::new(StorageErrorKind::Changed, "missing read binding")
                })?;
                if Some(&bound.identity) != observed.identity.as_ref()
                    || Some(bound.size) != observed.size
                    || Some(&bound.modified) != observed.modified.as_ref()
                {
                    return Err(StorageError::new(
                        StorageErrorKind::Changed,
                        "capability probe identity differs",
                    ));
                }
                let mut probe = [0; 1];
                let expected = usize::from(observed.size.is_some_and(|size| size != 0));
                if input.read(&mut probe)? != expected || input.seek(SeekFrom::Start(0))? != 0 {
                    return Err(StorageError::new(
                        StorageErrorKind::Changed,
                        "read/seek capability probe failed",
                    ));
                }
                Capabilities {
                    read: Some(true),
                    list: Some(false),
                    seek: Some(true),
                }
            }
            _ => {
                return Err(StorageError::new(
                    StorageErrorKind::UnsupportedCapability,
                    "only regular owned fixture entries",
                ));
            }
        };
        check_album_read_cancel(limits)?;
        if self.reads.stat(&observed.reference)? != observed {
            return Err(StorageError::new(
                StorageErrorKind::Changed,
                "capability probe changed observation",
            ));
        }
        check_album_read_cancel(limits)?;
        observed.capabilities = caps;
        Ok(observed)
    }
}
impl ReadStorage for OwnedCopyReadStorage {
    fn stat(
        &mut self,
        reference: &StorageRef,
    ) -> std::result::Result<EntryObservation, StorageError> {
        within_roots(reference, &self.roots)?;
        let observed = self.reads.stat(reference)?;
        self.establish(observed)
    }
    fn children(
        &mut self,
        reference: &StorageRef,
    ) -> std::result::Result<Vec<EntryObservation>, StorageError> {
        within_roots(reference, &self.roots)?;
        let entries = self.reads.children(reference)?;
        entries.into_iter().map(|e| self.establish(e)).collect()
    }
    fn open_read(
        &mut self,
        reference: &StorageRef,
    ) -> std::result::Result<ReadInput, StorageError> {
        within_roots(reference, &self.roots)?;
        self.reads.open_read(reference)
    }
}

/// Opt-in bounded Windows planning. Per-enumeration limits include directory
/// capability probes; the album collector separately bounds the complete tree.
/// Borrowing cancellation avoids changing legacy adapter lifetimes/semantics.
pub struct WindowsAlbumPlanningStorage<'a> {
    inner: OwnedCopyReadStorage,
    limits: WindowsDirectoryReadLimits,
    cancel: &'a AtomicBool,
}
fn check_album_read_cancel(limits: Option<(WindowsDirectoryReadLimits, &AtomicBool)>) -> std::result::Result<(), StorageError> {
    if limits.is_some_and(|(_, cancel)| cancel.load(Ordering::Acquire)) {
        Err(StorageError::new(StorageErrorKind::Cancelled, "album planning read cancelled"))
    } else { Ok(()) }
}
impl WindowsAlbumPlanningStorage<'_> {
    pub fn reference(&mut self, path: &Path) -> std::result::Result<StorageRef, StorageError> {
        check_album_read_cancel(Some((self.limits, self.cancel)))?;
        self.inner.reference(path)
    }
}
impl ReadStorage for WindowsAlbumPlanningStorage<'_> {
    fn stat(&mut self, reference: &StorageRef) -> std::result::Result<EntryObservation, StorageError> {
        let bounds = Some((self.limits, self.cancel)); check_album_read_cancel(bounds)?;
        within_roots(reference, &self.inner.roots)?;
        let observed = self.inner.reads.stat(reference)?;
        self.inner.establish_with_limits(observed, bounds)
    }
    fn children(&mut self, reference: &StorageRef) -> std::result::Result<Vec<EntryObservation>, StorageError> {
        let bounds = Some((self.limits, self.cancel)); check_album_read_cancel(bounds)?;
        within_roots(reference, &self.inner.roots)?;
        let entries = self.inner.children_with_limits(reference, bounds)?;
        entries.into_iter().map(|entry| self.inner.establish_with_limits(entry, bounds)).collect()
    }
    fn open_read(&mut self, reference: &StorageRef) -> std::result::Result<ReadInput, StorageError> {
        let bounds = Some((self.limits, self.cancel)); check_album_read_cancel(bounds)?;
        within_roots(reference, &self.inner.roots)?;
        let result = self.inner.reads.open_read(reference)?;
        check_album_read_cancel(bounds)?;
        Ok(result)
    }
}
