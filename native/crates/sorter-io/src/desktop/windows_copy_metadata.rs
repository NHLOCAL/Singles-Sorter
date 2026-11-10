//! Opt-in Python 3.10-style ordinary Copy metadata: observed access/write times
//! and readonly, while preserving the created destination's other metadata.
//! The source stays held and untouched. This is not an ACL/ADS copy operation.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsCopyMetadataEvidence {
    pub source_identity: ObjectIdentity,
    pub destination_identity: ObjectIdentity,
    /// Native FILETIME ticks (100 ns since 1601), observed after source reads.
    pub access_time: i64,
    pub write_time: i64,
    pub readonly: bool,
    pub retained_creation_time: i64,
    /// Destination attributes except READONLY and the nonsemantic NORMAL bit.
    pub retained_attributes: u32,
}

/// In-process source directory metadata captured before child retirement.
/// No Deserialize: a journal cannot synthesize this operation capability.
pub struct WindowsAlbumMoveDirectoryMetadata {
    source_identity: ObjectIdentity,
    access_time: i64,
    write_time: i64,
    readonly: bool,
}

/// Read-only native facts for comparison with a persisted Copy receipt. This
/// does not infer the source object or reconstruct a mutation/publication guard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsCopyMetadataObservation {
    pub observation: MutationObservation,
    pub kind: EntryKind,
    pub creation_time: i64,
    pub access_time: i64,
    pub write_time: i64,
    pub readonly: bool,
    pub retained_attributes: u32,
}

pub(super) struct CopyMetadataState {
    evidence: WindowsCopyMetadataEvidence,
    digest: ContentDigest,
    poisoned: bool,
}
impl CopyMetadataState {
    pub(super) fn check(&self, file: &File) -> Result<(), MutationError> {
        if self.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "copy metadata is poisoned",
            ));
        }
        check_copy_metadata(file, &self.evidence)
    }
}

fn check_copy_metadata(
    file: &File,
    expected: &WindowsCopyMetadataEvidence,
) -> Result<(), MutationError> {
    let basic = repair_basic(file)?;
    if identity(file).map_err(read_error)? != expected.destination_identity
        || basic.CreationTime != expected.retained_creation_time
        || basic.LastAccessTime != expected.access_time
        || basic.LastWriteTime != expected.write_time
        || (basic.FileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0) != expected.readonly
        || retained_attributes(basic.FileAttributes) != expected.retained_attributes
    {
        return Err(err(
            MutationErrorKind::VerificationFailed,
            "copy timestamp/readonly or retained destination metadata differs",
        ));
    }
    Ok(())
}
pub(super) fn retained_attributes(attributes: u32) -> u32 {
    attributes & !(FILE_ATTRIBUTE_NORMAL.0 | FILE_ATTRIBUTE_READONLY.0)
}
fn capture_copy_metadata(
    source: &File,
    destination: &File,
) -> Result<WindowsCopyMetadataEvidence, MutationError> {
    let from = repair_basic(source)?;
    let to = repair_basic(destination)?;
    // Zero and negative sentinel encodings cannot be faithfully requested with
    // SetFileTime; reject before touching the destination instead of rounding.
    if from.LastAccessTime <= 0 || from.LastWriteTime <= 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "copy timestamp is outside the positive explicit FILETIME domain",
        ));
    }
    Ok(WindowsCopyMetadataEvidence {
        source_identity: identity(source).map_err(read_error)?,
        destination_identity: identity(destination).map_err(read_error)?,
        access_time: from.LastAccessTime,
        write_time: from.LastWriteTime,
        readonly: from.FileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0,
        retained_creation_time: to.CreationTime,
        retained_attributes: retained_attributes(to.FileAttributes),
    })
}
fn file_time(ticks: i64) -> FILETIME {
    let ticks = ticks as u64;
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}
fn set_copy_metadata(
    file: &File,
    evidence: &WindowsCopyMetadataEvidence,
) -> Result<(), MutationError> {
    let access = file_time(evidence.access_time);
    let write = file_time(evidence.write_time);
    let freeze = FILETIME {
        dwLowDateTime: u32::MAX,
        dwHighDateTime: u32::MAX,
    };
    // Freeze this handle's later verification reads/writes, then explicitly set
    // the desired values. Actual post-close behavior is a native acceptance gate.
    unsafe {
        SetFileTime(
            HANDLE(file.as_raw_handle()),
            None,
            Some(&freeze),
            Some(&freeze),
        )
    }
    .map_err(win_error)?;
    unsafe {
        SetFileTime(
            HANDLE(file.as_raw_handle()),
            None,
            Some(&access),
            Some(&write),
        )
    }
    .map_err(win_error)?;
    let attributes = evidence.retained_attributes
        | if evidence.readonly {
            FILE_ATTRIBUTE_READONLY.0
        } else {
            0
        };
    let basic = FILE_BASIC_INFO {
        FileAttributes: if attributes == 0 {
            FILE_ATTRIBUTE_NORMAL.0
        } else {
            attributes
        },
        ..Default::default()
    };
    unsafe {
        SetFileInformationByHandle(
            HANDLE(file.as_raw_handle()),
            FileBasicInfo,
            (&basic as *const FILE_BASIC_INFO).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    }
    .map_err(win_error)?;
    // Reassert suppression after the explicit timestamp update. No source
    // handle timestamp, destination creation time or security data is written.
    unsafe {
        SetFileTime(
            HANDLE(file.as_raw_handle()),
            None,
            Some(&freeze),
            Some(&freeze),
        )
    }
    .map_err(win_error)
}
fn metadata_pending(error: MutationError, observation: &MutationObservation) -> MutationError {
    err(MutationErrorKind::UnknownOutcome, &format!("copy metadata dispatched; retained source/temp require reconciliation without replay: {error:?}"))
        .with_observation(observation.clone())
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyMetadataCheckpoint {
    BeforeDispatch,
    AfterMetadata,
    AfterFlush,
    AfterVerification,
}

impl WindowsMutationStorage {
    /// Exact target-leaf inspection under a live target capability. A missing
    /// name is not authority to recreate it; a foreign object is never adopted.
    pub fn inspect_selected_copy_metadata(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        expected_identity: &ObjectIdentity,
        expected_kind: EntryKind,
    ) -> Result<Option<WindowsCopyMetadataObservation>, MutationError> {
        if !matches!(
            self.domain,
            WindowsMutationDomain::SelectedRoots | WindowsMutationDomain::OwnedSyntheticTree
        ) || !matches!(expected_kind, EntryKind::File | EntryKind::Directory)
        {
            return Err(err(
                MutationErrorKind::Unsupported,
                "ordinary selected Copy target metadata inspection required",
            ));
        }
        self.check_role(parent, RootRole::Target)?;
        let file = match super::super::open_child(parent.handle(), units(name)?, false) {
            Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => {
                self.check_role(parent, RootRole::Target)?;
                return Ok(None);
            }
            Err(error) => return Err(read_error(error)),
            Ok(file) => file,
        };
        let actual_kind = kind(&file.metadata().map_err(io_error)?);
        if actual_kind == EntryKind::LinkOrReparse {
            return Err(err(
                MutationErrorKind::UnsafeReparse,
                "Copy metadata target is a reparse point",
            ));
        }
        let observation = observe(&file, parent)?;
        if observation.identity != *expected_identity
            || observation.actual_name != *name
            || actual_kind != expected_kind
        {
            return Err(err(
                MutationErrorKind::StalePlan,
                "Copy metadata target identity/name/kind differs",
            )
            .with_observation(observation));
        }
        if actual_kind == EntryKind::File {
            ensure_regular(&file)?;
        } else {
            ensure_directory(&file)?;
        }
        let basic = repair_basic(&file)?;
        self.check_role(parent, RootRole::Target)?;
        let after = observe(&file, parent)?;
        if after.identity != observation.identity
            || after.actual_name != observation.actual_name
            || kind(&file.metadata().map_err(io_error)?) != actual_kind
        {
            return Err(err(
                MutationErrorKind::StalePlan,
                "Copy metadata target changed during inspection",
            )
            .with_observation(after));
        }
        Ok(Some(WindowsCopyMetadataObservation {
            observation,
            kind: actual_kind,
            creation_time: basic.CreationTime,
            access_time: basic.LastAccessTime,
            write_time: basic.LastWriteTime,
            readonly: basic.FileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0,
            retained_attributes: retained_attributes(basic.FileAttributes),
        }))
    }

    pub fn finalize_selected_copy_metadata(
        &self,
        source: &WindowsSourceLease,
        temp: &mut WindowsOwnedTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        self.finalize_selected_copy_metadata_with(source, temp, cancel, |_| Ok(()))
    }
    pub(super) fn finalize_selected_copy_metadata_controlled(
        &self,
        source: &WindowsSourceLease,
        temp: &mut WindowsOwnedTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        self.finalize_copy_metadata_inner(source, temp, cancel, true, |_| Ok(()))
    }
    fn finalize_selected_copy_metadata_with(
        &self,
        source: &WindowsSourceLease,
        temp: &mut WindowsOwnedTemp,
        cancel: &AtomicBool,
        checkpoint: impl FnMut(CopyMetadataCheckpoint) -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        self.finalize_copy_metadata_inner(source, temp, cancel, false, checkpoint)
    }
    fn finalize_copy_metadata_inner(
        &self,
        source: &WindowsSourceLease,
        temp: &mut WindowsOwnedTemp,
        cancel: &AtomicBool,
        controlled: bool,
        mut checkpoint: impl FnMut(CopyMetadataCheckpoint) -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        let preflight = (|| {
            cancelled(cancel)?;
            if !matches!(
                self.domain,
                WindowsMutationDomain::SelectedRoots | WindowsMutationDomain::OwnedSyntheticTree
            ) || source.mode != SourceLeaseMode::Ordinary
                || source.repair.is_some()
                || temp.repair.is_some()
            {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "ordinary selected Copy metadata requires its own source/temp domain",
                ));
            }
            if temp.copy_metadata.is_some() {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "copy metadata already finalized or poisoned; no replay",
                ));
            }
            self.check_role(&temp.parent, RootRole::Target)?;
            if controlled {
                self.validate_source_controlled(source, cancel)?;
                temp.verify_content_controlled(&source.expected.digest, cancel)?;
            } else {
                self.validate_source(source)?;
                temp.verify_content(&source.expected.digest)?;
            }
            cancelled(cancel)?;
            let file = temp.file.as_ref().expect("verified temp");
            let evidence =
                capture_copy_metadata(source.file.as_ref().expect("verified source"), file)?;
            if evidence.source_identity != source.expected.identity
                || evidence.destination_identity != temp.identity
            {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "copy metadata source/temp identity differs",
                ));
            }
            checkpoint(CopyMetadataCheckpoint::BeforeDispatch)?;
            cancelled(cancel)?;
            Ok(evidence)
        })();
        let evidence = match preflight {
            Ok(value) => value,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        temp.copy_metadata = Some(CopyMetadataState {
            evidence: evidence.clone(),
            digest: source.expected.digest.clone(),
            poisoned: true,
        });
        let after = (|| {
            let file = temp.file.as_ref().expect("retained temp");
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
            set_copy_metadata(file, &evidence)?;
            checkpoint(CopyMetadataCheckpoint::AfterMetadata)?;
            cancelled(cancel)?;
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
            checkpoint(CopyMetadataCheckpoint::AfterFlush)?;
            cancelled(cancel)?;
            check_copy_metadata(file, &evidence)?;
            self.check_role(&temp.parent, RootRole::Target)?;
            self.check_role(&source.parent, RootRole::Source)?;
            checkpoint(CopyMetadataCheckpoint::AfterVerification)?;
            cancelled(cancel)
        })();
        match after {
            Ok(()) => {
                temp.copy_metadata.as_mut().expect("bound state").poisoned = false;
                MutationOutcome::Applied(evidence)
            }
            Err(error) => {
                MutationOutcome::PendingUnknown(metadata_pending(error, &temp.observation))
            }
        }
    }

    pub(super) fn publish_finalized_copy_controlled(
        &mut self,
        temp: &mut WindowsOwnedTemp,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        self.publish_finalized_copy_inner(temp, parent, name, cancel, true)
    }
    pub(super) fn publish_finalized_copy(
        &mut self,
        temp: &mut WindowsOwnedTemp,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        self.publish_finalized_copy_inner(temp, parent, name, cancel, false)
    }
    fn publish_finalized_copy_inner(
        &mut self,
        temp: &mut WindowsOwnedTemp,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
        controlled: bool,
    ) -> MutationOutcome<WindowsPublished> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.check_role(&temp.parent, RootRole::Target)?;
            self.check_role(parent, RootRole::Target)?;
            let state = temp.copy_metadata.as_ref().ok_or_else(|| {
                err(
                    MutationErrorKind::Unsupported,
                    "copy metadata binding missing",
                )
            })?;
            if controlled {
                temp.verify_content_controlled(&state.digest, cancel)?;
            } else {
                temp.verify_content(&state.digest)?;
            }
            units(name)?;
            ensure_same_volume(&temp.identity, &parent.identity)?;
            cancelled(cancel)?;
            Ok(())
        })();
        if let Err(error) = preflight {
            return MutationOutcome::NoEffect(error);
        }
        let file = temp.file.as_ref().expect("verified temp");
        if let Err(error) = super::album_rename::rename_album_handle(file, parent, name) {
            if error.kind == MutationErrorKind::UnknownOutcome {
                drop(temp.file.take());
                temp.copy_metadata.as_mut().expect("bound state").poisoned = true;
                return MutationOutcome::PendingUnknown(metadata_pending(error, &temp.observation));
            }
            return MutationOutcome::NoEffect(error);
        }
        let file = temp.file.take().expect("dispatched temp");
        let state = temp.copy_metadata.take().expect("bound state");
        match finish_publish(file, parent, name, &temp.identity) {
            MutationOutcome::Applied(mut published) => {
                published.copy_metadata = Some(state);
                let checked = (|| {
                    cancelled(cancel)?;
                    let state = published
                        .copy_metadata
                        .as_ref()
                        .expect("transferred copy state");
                    if controlled {
                        self.verify_published_controlled(&published, &state.digest, cancel)?;
                    } else {
                        self.verify(&published, &state.digest)?;
                    }
                    Ok(())
                })();
                match checked {
                    Ok(()) => MutationOutcome::Applied(published),
                    Err(error) => MutationOutcome::PendingUnknown(metadata_pending(
                        error,
                        &published.observation,
                    )),
                }
            }
            MutationOutcome::PendingUnknown(error) | MutationOutcome::NoEffect(error) => {
                MutationOutcome::PendingUnknown(metadata_pending(error, &temp.observation))
            }
        }
    }
}

impl WindowsOwnedTemp {
    pub fn copy_metadata_evidence(&self) -> Result<&WindowsCopyMetadataEvidence, MutationError> {
        let state = self.copy_metadata.as_ref().ok_or_else(|| {
            err(
                MutationErrorKind::Unsupported,
                "temp has no selected Copy metadata",
            )
        })?;
        state.check(
            self.file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?,
        )?;
        Ok(&state.evidence)
    }
}
impl WindowsPublished {
    pub fn copy_metadata_evidence(&self) -> Result<&WindowsCopyMetadataEvidence, MutationError> {
        let state = self.copy_metadata.as_ref().ok_or_else(|| {
            err(
                MutationErrorKind::Unsupported,
                "publication has no selected Copy metadata",
            )
        })?;
        state.check(&self.file)?;
        Ok(&state.evidence)
    }
}

/// Only successful FILE_CREATE of a directory can produce this token. A normal
/// WindowsParent, even one at the same path, cannot be adopted for copystat.
pub struct WindowsCreatedCopyDirectory {
    directory: WindowsParent,
    original_parent: WindowsParent,
    observation: MutationObservation,
    metadata: Option<CopyDirectoryState>,
}
struct CopyDirectoryState {
    evidence: WindowsCopyMetadataEvidence,
    poisoned: bool,
}
impl WindowsCreatedCopyDirectory {
    /// Destination capability for building descendants before postorder finalize.
    /// The executor must not perform later namespace writes through clones.
    pub fn parent(&self) -> &WindowsParent {
        &self.directory
    }
    pub fn observation(&self) -> &MutationObservation {
        &self.observation
    }
}

impl WindowsMutationStorage {
    pub fn create_selected_copy_directory(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCreatedCopyDirectory> {
        self.create_selected_copy_directory_with(parent, name, cancel, || Ok(()))
    }
    fn create_selected_copy_directory_with(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
        after_create: impl FnOnce() -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsCreatedCopyDirectory> {
        let preflight = (|| {
            cancelled(cancel)?;
            if !matches!(
                self.domain,
                WindowsMutationDomain::SelectedRoots | WindowsMutationDomain::OwnedSyntheticTree
            ) {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "created Copy directories require selected Copy authority",
                ));
            }
            self.check_role(parent, RootRole::Target)?;
            units(name)?;
            let ancestors = parent
                .ancestors
                .iter()
                .map(File::try_clone)
                .collect::<io::Result<Vec<_>>>()
                .map_err(io_error)?;
            cancelled(cancel)?;
            Ok(ancestors)
        })();
        let mut ancestors = match preflight {
            Ok(value) => value,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        let file = match create_copy_directory_handle(
            parent.handle(),
            units(name).expect("validated name"),
        ) {
            Ok(value) => value,
            Err(error) if error.kind == MutationErrorKind::UnknownOutcome => {
                return MutationOutcome::PendingUnknown(error);
            }
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        let verified = (|| {
            after_create()?;
            cancelled(cancel)?;
            ensure_directory(&file)?;
            let observation = observe(&file, parent)?;
            if observation.actual_name != *name {
                return Err(err(
                    MutationErrorKind::VerificationFailed,
                    "created Copy directory name differs",
                ));
            }
            self.check_role(parent, RootRole::Target)?;
            Ok(observation)
        })();
        let observation = match verified {
            Ok(value) => value,
            Err(error) => {
                let mut pending = err(
                    MutationErrorKind::UnknownOutcome,
                    &format!("Copy directory created; retain without replay: {error:?}"),
                );
                if let Ok(observation) = observe(&file, parent) {
                    pending = pending.with_observation(observation);
                }
                return MutationOutcome::PendingUnknown(pending);
            }
        };
        ancestors.push(file);
        MutationOutcome::Applied(WindowsCreatedCopyDirectory {
            directory: WindowsParent {
                ancestors: Arc::new(ancestors),
                identity: observation.identity.clone(),
                root: parent.root.clone(),
            },
            original_parent: parent.clone(),
            observation,
            metadata: None,
        })
    }

    pub fn capture_album_move_directory_metadata(
        &self,
        source: &WindowsParent,
        cancel: &AtomicBool,
    ) -> Result<WindowsAlbumMoveDirectoryMetadata, MutationError> {
        cancelled(cancel)?;
        self.check_role(source, RootRole::Source)?;
        if !source.root.album_cleanup {
            return Err(err(
                MutationErrorKind::Unsupported,
                "album metadata capture requires explicit Move scope",
            ));
        }
        ensure_directory(source.handle())?;
        let basic = repair_basic(source.handle())?;
        if basic.LastAccessTime <= 0 || basic.LastWriteTime <= 0 {
            return Err(err(
                MutationErrorKind::Unsupported,
                "album directory timestamps require explicit positive FILETIME",
            ));
        }
        let value = WindowsAlbumMoveDirectoryMetadata {
            source_identity: source.identity.clone(),
            access_time: basic.LastAccessTime,
            write_time: basic.LastWriteTime,
            readonly: basic.FileAttributes & FILE_ATTRIBUTE_READONLY.0 != 0,
        };
        cancelled(cancel)?;
        Ok(value)
    }
    pub fn finalize_album_move_directory_metadata(
        &self,
        source: &WindowsParent,
        captured: &WindowsAlbumMoveDirectoryMetadata,
        created: &mut WindowsCreatedCopyDirectory,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        if !source.root.album_cleanup || captured.source_identity != source.identity {
            return MutationOutcome::NoEffect(err(
                MutationErrorKind::StalePlan,
                "album metadata source capture differs",
            ));
        }
        self.finalize_copy_directory_metadata_inner(
            source,
            created,
            cancel,
            |_| Ok(()),
            Some(captured),
        )
    }

    /// Postorder only: every child namespace/content operation must already be
    /// complete. A later external or caller edit is a verification failure.
    pub fn finalize_selected_copy_directory_metadata(
        &self,
        source: &WindowsParent,
        created: &mut WindowsCreatedCopyDirectory,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        self.finalize_selected_copy_directory_metadata_with(source, created, cancel, |_| Ok(()))
    }
    fn finalize_selected_copy_directory_metadata_with(
        &self,
        source: &WindowsParent,
        created: &mut WindowsCreatedCopyDirectory,
        cancel: &AtomicBool,
        mut checkpoint: impl FnMut(CopyMetadataCheckpoint) -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        self.finalize_copy_directory_metadata_inner(source, created, cancel, &mut checkpoint, None)
    }
    fn finalize_copy_directory_metadata_inner(
        &self,
        source: &WindowsParent,
        created: &mut WindowsCreatedCopyDirectory,
        cancel: &AtomicBool,
        mut checkpoint: impl FnMut(CopyMetadataCheckpoint) -> Result<(), MutationError>,
        captured: Option<&WindowsAlbumMoveDirectoryMetadata>,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        let preflight = (|| {
            cancelled(cancel)?;
            if !matches!(
                self.domain,
                WindowsMutationDomain::SelectedRoots | WindowsMutationDomain::OwnedSyntheticTree
            ) {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "Copy directory metadata authority required",
                ));
            }
            self.check_role(source, RootRole::Source)?;
            self.check_role(&created.directory, RootRole::Target)?;
            self.check_role(&created.original_parent, RootRole::Target)?;
            if created.metadata.is_some() {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "Copy directory metadata finalized or poisoned; no replay",
                ));
            }
            ensure_directory(source.handle())?;
            ensure_directory(created.directory.handle())?;
            let observed = observe(created.directory.handle(), &created.original_parent)?;
            if observed.identity != created.observation.identity
                || observed.actual_name != created.observation.actual_name
            {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "created Copy directory binding differs",
                ));
            }
            let mut evidence = capture_copy_metadata(source.handle(), created.directory.handle())?;
            if let Some(captured) = captured {
                if captured.source_identity != evidence.source_identity {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "captured album directory changed",
                    ));
                }
                evidence.access_time = captured.access_time;
                evidence.write_time = captured.write_time;
                evidence.readonly = captured.readonly;
            }
            if evidence.source_identity != source.identity
                || evidence.destination_identity != created.directory.identity
            {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "Copy directory source/destination identity differs",
                ));
            }
            checkpoint(CopyMetadataCheckpoint::BeforeDispatch)?;
            cancelled(cancel)?;
            Ok(evidence)
        })();
        let evidence = match preflight {
            Ok(value) => value,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        created.metadata = Some(CopyDirectoryState {
            evidence: evidence.clone(),
            poisoned: true,
        });
        let after = (|| {
            set_copy_metadata(created.directory.handle(), &evidence)?;
            checkpoint(CopyMetadataCheckpoint::AfterMetadata)?;
            cancelled(cancel)?;
            // No invented FlushFileBuffers/directory durability acknowledgment.
            check_copy_metadata(created.directory.handle(), &evidence)?;
            self.check_role(source, RootRole::Source)?;
            self.check_role(&created.directory, RootRole::Target)?;
            checkpoint(CopyMetadataCheckpoint::AfterVerification)?;
            cancelled(cancel)
        })();
        match after {
            Ok(()) => {
                created.metadata.as_mut().expect("bound metadata").poisoned = false;
                MutationOutcome::Applied(evidence)
            }
            Err(error) => {
                MutationOutcome::PendingUnknown(metadata_pending(error, &created.observation))
            }
        }
    }

    pub fn verify_selected_copy_directory_metadata<'a>(
        &self,
        created: &'a WindowsCreatedCopyDirectory,
    ) -> Result<&'a WindowsCopyMetadataEvidence, MutationError> {
        self.check_role(&created.directory, RootRole::Target)?;
        self.check_role(&created.original_parent, RootRole::Target)?;
        let state = created.metadata.as_ref().ok_or_else(|| {
            err(
                MutationErrorKind::StalePlan,
                "created Copy directory metadata not finalized",
            )
        })?;
        if state.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "Copy directory metadata poisoned",
            ));
        }
        ensure_directory(created.directory.handle())?;
        let observed = observe(created.directory.handle(), &created.original_parent)?;
        if observed.identity != created.observation.identity
            || observed.actual_name != created.observation.actual_name
        {
            return Err(err(
                MutationErrorKind::StalePlan,
                "created Copy directory location differs",
            ));
        }
        check_copy_metadata(created.directory.handle(), &state.evidence)?;
        Ok(&state.evidence)
    }
}

fn create_copy_directory_handle(parent: &File, name: &[u16]) -> Result<File, MutationError> {
    ensure_directory(parent)?;
    let mut name = name.to_vec();
    let string = UNICODE_STRING {
        Length: (name.len() * 2) as u16,
        MaximumLength: (name.len() * 2) as u16,
        Buffer: PWSTR(name.as_mut_ptr()),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &string,
        Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE,
        ..Default::default()
    };
    let mut handle = HANDLE::default();
    let mut status_block = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_TRAVERSE | FILE_READ_ATTRIBUTES | FILE_WRITE_ATTRIBUTES | SYNCHRONIZE,
            &attributes,
            &mut status_block,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_CREATE,
            FILE_DIRECTORY_FILE
                | FILE_OPEN_REPARSE_POINT
                | FILE_SYNCHRONOUS_IO_NONALERT
                | FILE_WRITE_THROUGH,
            None,
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "Copy directory create encountered reparse",
        ));
    }
    if status.0 < 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(err(
            MutationErrorKind::UnknownOutcome,
            "Copy directory create returned no handle",
        ));
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    if status.0 != 0 || unsafe { status_block.Anonymous.Status }.0 != 0 {
        return Err(err(
            MutationErrorKind::UnknownOutcome,
            "Copy directory create completion uncertain",
        ));
    }
    Ok(file)
}

#[cfg(test)]
#[path = "windows_copy_metadata_tests.rs"]
mod tests;
