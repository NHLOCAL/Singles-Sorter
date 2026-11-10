//! Single-file same-volume rename beneath an owned NTFS fixture capability.
//! No copy/retire fallback, production roots, directories, albums or apply UI.
use crate::{Result, binding::*, transfer::TransferStorage, windows_copy::*};
use sorter_core::{discovery::*, job_plan::*, metadata::MetadataProvider};
use sorter_io::desktop::windows_mutation::*;
use sorter_storage_contracts::*;
use std::io::{self, Read};

pub fn validate_owned_rename(
    owned: &WindowsRenameStorage,
    plan: &JobPlanV2,
    metadata: &mut dyn MetadataProvider,
    recognition: &mut dyn RecognitionProvider,
    resources: &Resources,
    scope: &ExecutionScope,
) -> Result<ExecutionBinding> {
    validate_owned_domain(
        owned.owned(),
        plan,
        metadata,
        recognition,
        resources,
        scope,
        ExecutionDomain::OwnedWindowsRename,
    )
}
impl TransferStorage for WindowsRenameStorage {
    fn source_parent(
        &mut self,
        source: &SourceEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<WindowsParent, MutationError> {
        // Resolve the lossless parent from the bound file reference. The native
        // owned capability rejects paths outside its tree and unsafe segments.
        let StorageRef::WindowsPath { utf16 } = &source.observation.reference else {
            return Err(MutationError::new(
                MutationErrorKind::StalePlan,
                "Windows source required",
            ));
        };
        let split = utf16
            .iter()
            .rposition(|u| *u == 92 || *u == 47)
            .ok_or_else(|| {
                MutationError::new(MutationErrorKind::InvalidName, "missing source parent")
            })?;
        let parent = self.owned().parent_reference(&StorageRef::WindowsPath {
            utf16: utf16[..split].to_vec(),
        })?;
        if !parent.contains_ancestor(root)? {
            return Err(MutationError::new(
                MutationErrorKind::StalePlan,
                "source root differs",
            ));
        }
        Ok(parent)
    }
    fn destination_parent(
        &mut self,
        destination: &DestinationEvidence,
        root: &ObjectIdentity,
    ) -> std::result::Result<WindowsParent, MutationError> {
        destination_parent(self.owned(), destination, root)
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
    fn write_temp(&mut self, _: &mut WindowsOwnedTemp, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("rename-only storage cannot write temps"))
    }
    fn sync_temp(&mut self, _: &mut WindowsOwnedTemp) -> std::result::Result<(), MutationError> {
        Err(unsupported())
    }
    fn verify_temp(
        &mut self,
        _: &WindowsOwnedTemp,
        _: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        Err(unsupported())
    }
    fn rename_eligible(&self, source: &WindowsSourceLease, parent: &WindowsParent) -> bool {
        matches!((&source.observation().identity, parent.identity()),
            (ObjectIdentity::Windows { volume: a, .. }, ObjectIdentity::Windows { volume: b, .. }) if a == b)
    }
    fn owned_windows_rename_contract(&self) -> bool {
        true
    }
}
fn unsupported() -> MutationError {
    MutationError::new(
        MutationErrorKind::Unsupported,
        "rename-only storage cannot copy/retire",
    )
}
pub fn open_owned_rename_recovery(
    owned: &WindowsRenameStorage,
    scope: &RecoveryScope,
    job_id: &str,
    bytes: &[u8],
) -> Result<crate::physical_journal::PhysicalJournalV2> {
    open_owned_recovery_domain(
        owned.owned(),
        scope,
        job_id,
        bytes,
        ExecutionDomain::OwnedWindowsRename,
    )
}
/// Read-only binding/root checks shared with Copy, with an exclusive rename domain.
pub struct WindowsRenameRecovery<'a>(WindowsCopyRecovery<'a>);
impl<'a> WindowsRenameRecovery<'a> {
    pub fn new(owned: &'a WindowsRenameStorage, binding: &ExecutionBinding) -> Result<Self> {
        Ok(Self(WindowsCopyRecovery::new_domain(
            owned.owned(),
            binding,
            ExecutionDomain::OwnedWindowsRename,
        )?))
    }
}
impl crate::recovery::RecoveryStorage for WindowsRenameRecovery<'_> {
    fn source(
        &mut self,
        reference: &StorageRef,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        crate::recovery::RecoveryStorage::source(&mut self.0, reference)
    }
    fn destination(
        &mut self,
        destination: &DestinationEvidence,
    ) -> Result<Option<crate::recovery::ObservedObject>> {
        crate::recovery::RecoveryStorage::destination(&mut self.0, destination)
    }
}
