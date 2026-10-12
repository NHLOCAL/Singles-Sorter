//! Read-only reconciliation after this live job consumed its selected album root.
//! The original parent capability is private native evidence, never journal data.
use super::*;
use crate::inspection_control::{InspectionControl, probe};
use crate::recovery::{ObservedObject, RecoveryStorage};

pub(super) struct RemovedRootRecovery<'a> {
    roots: &'a WindowsMutationStorage,
    original: &'a WindowsAlbumRootCleanup,
    binding: &'a ExecutionBinding,
    reads: DesktopStorage,
    control: &'a InspectionControl<'a>,
    _target: WindowsParent,
}
use sorter_io::desktop::DesktopStorage;
impl<'a> RemovedRootRecovery<'a> {
    pub(super) fn new(
        roots: &'a WindowsMutationStorage,
        original: &'a WindowsAlbumRootCleanup,
        binding: &'a ExecutionBinding,
        observed_source: &StorageRef,
        control: &'a InspectionControl<'a>,
    ) -> Result<Self> {
        control.checkpoint()?;
        if !binding.recovery_only
            || !binding.data.valid_domain()
            || roots.domain() != WindowsMutationDomain::SelectedRoots
            || binding.data.source_root.identity.as_ref() != Some(&original.observation().identity)
            || binding.data.source_root.parent_identity.as_ref()
                != Some(&original.observation().parent_identity)
            || observed_source != &original.observation().reference
            || !matches!(
                binding.data.domain(),
                ExecutionDomain::SelectedWindowsRename | ExecutionDomain::SelectedWindowsCopiedMove
            )
        {
            return Err(E::Scope);
        }
        let target = probe(Some(control), || {
            roots.selected_target_root().map_err(|_| E::Scope)
        })?;
        if binding.data.target_root.identity.as_ref() != Some(target.identity()) {
            return Err(E::Scope);
        }
        let target_path = path(&binding.data.target_root.reference)?;
        let mut reads = probe(Some(control), || {
            DesktopStorage::new(std::slice::from_ref(&target_path)).map_err(|_| E::Scope)
        })?;
        if probe(Some(control), || {
            reads
                .stat(&binding.data.target_root.reference)
                .map_err(|_| E::Scope)
        })?
        .identity
        .as_ref()
            != Some(target.identity())
        {
            return Err(E::Scope);
        }
        control.checkpoint()?;
        Ok(Self {
            roots,
            original,
            binding,
            reads,
            control,
            _target: target,
        })
    }
    fn destination_parent(
        &self,
        destination: &DestinationEvidence,
    ) -> Result<(WindowsParent, MutationName, StorageRef)> {
        self.control.checkpoint()?;
        if !self
            .binding
            .operations()
            .iter()
            .any(|operation| operation.destination == *destination)
        {
            return Err(E::Scope);
        }
        let mut full = path(&destination.root)?;
        for component in &destination.components {
            full.push(component);
        }
        let parent = probe(Some(self.control), || {
            self.roots
                .parent_reference(&reference(full.parent().ok_or(E::Scope)?))
                .map_err(|_| E::Stale)
        })?;
        let leaf = MutationName::Windows(full.file_name().ok_or(E::Scope)?.encode_wide().collect());
        Ok((parent, leaf, reference(&full)))
    }
}
impl RecoveryStorage for RemovedRootRecovery<'_> {
    fn source(&mut self, source: &StorageRef) -> Result<Option<ObservedObject>> {
        self.control.checkpoint()?;
        if !self
            .binding
            .operations()
            .iter()
            .any(|operation| operation.source.observation.reference == *source)
        {
            return Err(E::Scope);
        }
        let source_path = path(source)?;
        let source_root = path(&self.binding.data.source_root.reference)?;
        if source_path
            .strip_prefix(&source_root)
            .map_or(true, |relative| relative.as_os_str().is_empty())
        {
            return Err(E::Scope);
        }
        if probe(Some(self.control), || {
            self.original
                .inspect_original(self.control.cancellation_flag())
                .map_err(album_native_error)
        })?
        .is_some()
        {
            return Err(E::NeedsRecovery);
        }
        Ok(None)
    }
    fn destination(&mut self, destination: &DestinationEvidence) -> Result<Option<ObservedObject>> {
        let (_parent, _leaf, reference) = self.destination_parent(destination)?;
        self.control.observation()?;
        let before = self.reads.stat(&reference);
        self.control.checkpoint()?;
        let before = match before {
            Ok(value) => value,
            Err(error) if error.kind == StorageErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(E::Io),
        };
        if before.kind != EntryKind::File {
            return Err(E::Stale);
        }
        let mut input = probe(Some(self.control), || {
            self.reads.open_read(&reference).map_err(|_| E::Io)
        })?;
        self.control.checkpoint()?;
        let held = input.binding().ok_or(E::Stale)?;
        if Some(&held.identity) != before.identity.as_ref()
            || Some(held.size) != before.size
            || Some(&held.modified) != before.modified.as_ref()
        {
            return Err(E::Stale);
        }
        let mut hash = Sha256::new();
        let mut bytes = 0u64;
        let mut buffer = [0; 65536];
        let expected = before.size.ok_or(E::Stale)?;
        loop {
            self.control.checkpoint()?;
            let remaining = expected.checked_sub(bytes).ok_or(E::Stale)?;
            let want = if remaining == 0 {
                1
            } else {
                remaining.min(buffer.len() as u64) as usize
            };
            let result = input.read(&mut buffer[..want]);
            if let Ok(count) = result.as_ref() {
                self.control.bytes(*count)?;
            }
            self.control.checkpoint()?;
            let count = result.map_err(|_| E::Io)?;
            if count == 0 {
                break;
            }
            bytes = bytes.checked_add(count as u64).ok_or(E::Io)?;
            if bytes > expected {
                return Err(E::Stale);
            }
            hash.update(&buffer[..count]);
        }
        let after = probe(Some(self.control), || {
            self.reads.stat(&reference).map_err(|_| E::Stale)
        })?;
        if Some(bytes) != before.size || after != before {
            return Err(E::Stale);
        }
        Ok(Some(ObservedObject {
            identity: before.identity.ok_or(E::Stale)?,
            sha256: format!("{:x}", hash.finalize()),
        }))
    }
    fn verify_copy_metadata(
        &mut self,
        destination: &DestinationEvidence,
        expected: &crate::physical_journal::CopyMetadataEvidence,
    ) -> Result<bool> {
        let (parent, leaf, _) = self.destination_parent(destination)?;
        if !self.binding.operations().iter().any(|operation| {
            operation.destination == *destination
                && operation.source.observation.identity.as_ref() == Some(&expected.source_identity)
        }) {
            return Err(E::Scope);
        }
        let actual = probe(Some(self.control), || {
            self.roots
                .inspect_selected_copy_metadata(
                    &parent,
                    &leaf,
                    &expected.destination_identity,
                    EntryKind::File,
                )
                .map_err(album_native_error)
        })?;
        self.control.checkpoint()?;
        Ok(actual.is_some_and(|actual| {
            actual.creation_time == expected.retained_creation_time
                && actual.access_time == expected.access_time
                && actual.write_time == expected.write_time
                && actual.readonly == expected.readonly
                && actual.retained_attributes == expected.retained_attributes
        }))
    }
    fn verify_move_destination_streams(
        &mut self,
        destination: &DestinationEvidence,
        identity: &ObjectIdentity,
        expected: &CopiedMoveSourceEvidence,
    ) -> Result<bool> {
        if !self
            .binding
            .data
            .copied_move_sources
            .as_ref()
            .is_some_and(|sources| sources.contains(expected))
            || !self.binding.operations().iter().any(|operation| {
                operation.destination == *destination
                    && operation.source.observation.reference == expected.source
            })
        {
            return Err(E::Scope);
        }
        let (parent, leaf, _) = self.destination_parent(destination)?;
        let actual = probe(Some(self.control), || {
            self.roots
                .inspect_copied_move_streams_observed(
                    &parent,
                    &leaf,
                    identity,
                    self.control.cancellation_flag(),
                    &mut |event| crate::windows_copy::inspection_event(self.control, event),
                )
                .map_err(crate::windows_copy::inspection_error)
        })?;
        self.control.checkpoint()?;
        Ok(actual.streams.len() == expected.streams.len()
            && actual
                .streams
                .iter()
                .zip(&expected.streams)
                .all(|(actual, expected)| {
                    actual.name == expected.name
                        && actual.length == expected.length
                        && actual.sha256 == expected.sha256
                }))
    }
}
