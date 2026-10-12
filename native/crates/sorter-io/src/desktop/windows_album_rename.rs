//! Separate whole-object album rename under an explicit no-external-writer
//! contract. Repeated tree snapshots are evidence, not membership exclusion.
//! This never changes ordinary source-file lease sharing or deletes a source.
use super::*;
use std::collections::BTreeMap;

type RelativeKey = Vec<Vec<u16>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsAlbumTreeEntry {
    /// Nonempty sequence of lossless, single-component descendant names.
    pub relative: Vec<MutationName>,
    pub identity: ObjectIdentity,
    pub kind: EntryKind,
    /// Required for files, absent for directories. Includes exact byte length.
    pub digest: Option<ContentDigest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsAlbumTreeObservation {
    pub relative: Vec<MutationName>,
    pub observation: MutationObservation,
}

struct AlbumTree {
    entries: Vec<WindowsAlbumTreeEntry>,
    children: BTreeMap<RelativeKey, Vec<usize>>,
}

enum AlbumSourceScope {
    Child(WindowsParent),
    SelectedRoot {
        ancestors: Vec<File>,
        parent_identity: ObjectIdentity,
    },
}
impl AlbumSourceScope {
    fn parent_handle(&self) -> &File {
        match self {
            Self::Child(parent) => parent.handle(),
            Self::SelectedRoot { ancestors, .. } => {
                ancestors.last().expect("validated source ancestry")
            }
        }
    }
    fn parent_identity(&self) -> &ObjectIdentity {
        match self {
            Self::Child(parent) => &parent.identity,
            Self::SelectedRoot {
                parent_identity, ..
            } => parent_identity,
        }
    }
    fn check(
        &self,
        storage: &WindowsMutationStorage,
        root_session: bool,
    ) -> Result<(), MutationError> {
        match self {
            Self::Child(parent) if !root_session => storage.check_role(parent, RootRole::Source),
            Self::SelectedRoot {
                ancestors,
                parent_identity,
            } if root_session => {
                for ancestor in ancestors {
                    ensure_directory(ancestor)?;
                }
                if ancestors
                    .last()
                    .map(identity)
                    .transpose()
                    .map_err(read_error)?
                    .as_ref()
                    != Some(parent_identity)
                {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "selected album root parent changed",
                    ));
                }
                Ok(())
            }
            _ => Err(err(
                MutationErrorKind::Unsupported,
                "album source authority domains do not match",
            )),
        }
    }
}

/// Dedicated whole-directory lease, not a MutationStorage source lease. The
/// host must authorize ordinary use with no concurrent external album writers.
/// Descendant guards from planning must already have been released.
pub struct WindowsAlbumDirectoryLease {
    file: Option<File>,
    scope: AlbumSourceScope,
    original: MutationObservation,
    tree: AlbumTree,
}
impl WindowsAlbumDirectoryLease {
    pub fn observation(&self) -> &MutationObservation {
        &self.original
    }
}

/// Keeps the exact renamed root and target ancestry pinned after observed
/// whole-tree verification. This is not a generic durable-flush acknowledgment.
pub struct WindowsAlbumPublication {
    file: File,
    parent: WindowsParent,
    root: MutationObservation,
    entries: Vec<WindowsAlbumTreeObservation>,
    tree: AlbumTree,
}
impl WindowsAlbumPublication {
    pub fn observation(&self) -> &MutationObservation {
        &self.root
    }
    pub fn entries(&self) -> &[WindowsAlbumTreeObservation] {
        &self.entries
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlbumRenameCheckpoint {
    BeforeFinalVerification,
    BeforeRename,
    AfterRename,
    AfterVerification,
}

impl WindowsMutationStorage {
    /// Acquire one ordinary album child with its eventual DELETE rights. No
    /// source/root or child-file capability is silently weakened or converted.
    pub fn acquire_album_directory(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        expected: &ObjectIdentity,
        entries: Vec<WindowsAlbumTreeEntry>,
        cancel: &AtomicBool,
    ) -> Result<WindowsAlbumDirectoryLease, MutationError> {
        cancelled(cancel)?;
        if self.domain == WindowsMutationDomain::SelectedRepair {
            return Err(err(
                MutationErrorKind::Unsupported,
                "repair authority cannot move album directories",
            ));
        }
        self.check_role(parent, RootRole::Source)?;
        let tree = validate_album_tree(expected, entries)?;
        let file = open_album_directory(parent.handle(), units(name)?, true)?;
        ensure_directory(&file)?;
        let original = album_observe(&file, &parent.identity)?;
        if original.identity != *expected || original.actual_name != *name {
            return Err(err(
                MutationErrorKind::StalePlan,
                "album root identity/name changed",
            )
            .with_observation(original));
        }
        verify_album_tree(&file, &tree, cancel)?;
        self.check_role(parent, RootRole::Source)?;
        cancelled(cancel)?;
        Ok(WindowsAlbumDirectoryLease {
            file: Some(file),
            scope: AlbumSourceScope::Child(parent.clone()),
            original,
            tree,
        })
    }

    /// Move the same directory object, preserving all child objects, named
    /// streams, EAs and security metadata. The executor admits this only when
    /// its current verified manifest matches the policy destination names and
    /// target is absent. Ordered child sanitization has a separate scope.
    pub fn rename_album_directory(
        &self,
        source: &mut WindowsAlbumDirectoryLease,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumPublication> {
        self.rename_album_directory_with(source, parent, name, cancel, (false, |_| Ok(())))
    }

    fn rename_album_directory_with(
        &self,
        source: &mut WindowsAlbumDirectoryLease,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
        execution: (
            bool,
            impl FnMut(AlbumRenameCheckpoint) -> Result<(), MutationError>,
        ),
    ) -> MutationOutcome<WindowsAlbumPublication> {
        let (root_session, mut checkpoint) = execution;
        let preflight = (|| {
            cancelled(cancel)?;
            if self.domain == WindowsMutationDomain::SelectedRepair {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "repair cannot dispatch album moves",
                ));
            }
            source.scope.check(self, root_session)?;
            self.check_role(parent, RootRole::Target)?;
            units(name)?;
            ensure_same_volume(&source.original.identity, &parent.identity)?;
            if parent.contains_ancestor(&source.original.identity)? {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "album destination is inside its source",
                ));
            }
            let file = source
                .file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "album lease consumed"))?;
            checkpoint(AlbumRenameCheckpoint::BeforeFinalVerification)?;
            let current = album_observe(file, source.scope.parent_identity())?;
            if current.identity != source.original.identity
                || current.actual_name != source.original.actual_name
            {
                return Err(
                    err(MutationErrorKind::StalePlan, "held album source changed")
                        .with_observation(current),
                );
            }
            verify_album_tree(file, &source.tree, cancel)?;
            source.scope.check(self, root_session)?;
            self.check_role(parent, RootRole::Target)?;
            checkpoint(AlbumRenameCheckpoint::BeforeRename)?;
            cancelled(cancel)?;
            rename_album_handle(file, parent, name)
        })();
        if let Err(error) = preflight {
            if error.kind == MutationErrorKind::UnknownOutcome {
                // Unexpected completion status is not a license to retry a
                // possibly dispatched native rename through this lease.
                drop(source.file.take());
                return MutationOutcome::PendingUnknown(
                    error.with_observation(source.original.clone()),
                );
            }
            return MutationOutcome::NoEffect(error);
        }
        let file = source.file.take().expect("validated album lease");
        let mut observed = None;
        let after = (|| {
            checkpoint(AlbumRenameCheckpoint::AfterRename)?;
            cancelled(cancel)?;
            self.check_role(parent, RootRole::Target)?;
            let root = album_observe(&file, &parent.identity)?;
            observed = Some(root.clone());
            if root.identity != source.original.identity || root.actual_name != *name {
                return Err(err(
                    MutationErrorKind::VerificationFailed,
                    "renamed album root differs",
                ));
            }
            let entries = verify_album_tree(&file, &source.tree, cancel)?;
            checkpoint(AlbumRenameCheckpoint::AfterVerification)?;
            cancelled(cancel)?;
            self.check_role(parent, RootRole::Target)?;
            Ok((root, entries))
        })();
        match after {
            Ok((root, entries)) => MutationOutcome::Applied(WindowsAlbumPublication {
                file,
                parent: parent.clone(),
                root,
                entries,
                tree: AlbumTree {
                    entries: source.tree.entries.clone(),
                    children: source.tree.children.clone(),
                },
            }),
            Err(error) => {
                let observation = album_observe(&file, &parent.identity)
                    .ok()
                    .or(observed)
                    .unwrap_or_else(|| source.original.clone());
                MutationOutcome::PendingUnknown(
                    err(
                        MutationErrorKind::UnknownOutcome,
                        &format!(
                            "whole-album rename dispatched; reconcile without replay: {error:?}"
                        ),
                    )
                    .with_observation(observation),
                )
            }
        }
    }

    /// Read only the source's originally bound leaf beneath its retained parent.
    /// An absent old link is not source authority, and a replacement is evidence
    /// only. This never initializes a broad storage root at a missing path.
    pub fn inspect_original_album_location(
        &self,
        source: &WindowsAlbumDirectoryLease,
    ) -> Result<Option<MutationObservation>, MutationError> {
        source.scope.check(self, false)?;
        inspect_album_original_location(source)
    }

    pub fn verify_album_publication(
        &self,
        published: &WindowsAlbumPublication,
        cancel: &AtomicBool,
    ) -> Result<Vec<WindowsAlbumTreeObservation>, MutationError> {
        self.check_role(&published.parent, RootRole::Target)?;
        let root = album_observe(&published.file, &published.parent.identity)?;
        if root.identity != published.root.identity
            || root.actual_name != published.root.actual_name
        {
            return Err(err(
                MutationErrorKind::VerificationFailed,
                "published album root changed",
            ));
        }
        verify_album_tree(&published.file, &published.tree, cancel)
    }
}

fn validate_album_tree(
    root: &ObjectIdentity,
    entries: Vec<WindowsAlbumTreeEntry>,
) -> Result<AlbumTree, MutationError> {
    if !matches!(root, ObjectIdentity::Windows { .. }) || entries.len() > 100_000 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "bounded Windows album manifest required",
        ));
    }
    let mut keyed = BTreeMap::new();
    let mut name_units = 0usize;
    for entry in entries {
        if entry.relative.is_empty() || entry.relative.len() > 128 || entry.identity == *root {
            return Err(err(
                MutationErrorKind::StalePlan,
                "album manifest has invalid depth or root alias",
            ));
        }
        ensure_same_volume(root, &entry.identity)?;
        if !matches!(
            (entry.kind, &entry.digest),
            (EntryKind::File, Some(_)) | (EntryKind::Directory, None)
        ) {
            return Err(err(
                MutationErrorKind::Unsupported,
                "album manifest must identify ordinary files with digests and directories",
            ));
        }
        let mut key = Vec::with_capacity(entry.relative.len());
        for part in &entry.relative {
            let part = units(part)?;
            name_units = name_units.checked_add(part.len()).ok_or_else(|| {
                err(
                    MutationErrorKind::Unsupported,
                    "album manifest name budget overflow",
                )
            })?;
            if name_units > 8 * 1024 * 1024 {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "album manifest exceeds 16 MiB UTF-16 budget",
                ));
            }
            key.push(part.to_vec());
        }
        if keyed.insert(key, entry).is_some() {
            return Err(err(
                MutationErrorKind::StalePlan,
                "album manifest repeats a descendant path",
            ));
        }
    }
    for key in keyed.keys().filter(|key| key.len() > 1) {
        if !matches!(keyed.get(&key[..key.len() - 1]), Some(entry) if entry.kind == EntryKind::Directory)
        {
            return Err(err(
                MutationErrorKind::StalePlan,
                "album manifest is missing a directory parent",
            ));
        }
    }
    let mut result = AlbumTree {
        entries: Vec::with_capacity(keyed.len()),
        children: BTreeMap::new(),
    };
    for (key, entry) in keyed {
        result
            .children
            .entry(key[..key.len() - 1].to_vec())
            .or_default()
            .push(result.entries.len());
        result.entries.push(entry);
    }
    Ok(result)
}

fn verify_album_tree(
    root: &File,
    tree: &AlbumTree,
    cancel: &AtomicBool,
) -> Result<Vec<WindowsAlbumTreeObservation>, MutationError> {
    fn walk(
        directory: &File,
        relative: &mut RelativeKey,
        tree: &AlbumTree,
        cancel: &AtomicBool,
        result: &mut Vec<WindowsAlbumTreeObservation>,
    ) -> Result<(), MutationError> {
        cancelled(cancel)?;
        ensure_directory(directory)?;
        let parent_id = identity(directory).map_err(read_error)?;
        let expected = tree
            .children
            .get(relative)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut members = directory_members_with_limits(directory, cancel, expected.len(), 1024)?;
        for member in &members {
            units(&member.name)?;
        }
        members.sort_by(|a, b| {
            units(&a.name)
                .expect("validated name")
                .cmp(units(&b.name).expect("validated name"))
        });
        if members.len() != expected.len() {
            return Err(err(
                MutationErrorKind::StalePlan,
                "album membership changed",
            ));
        }
        for (member, index) in members.into_iter().zip(expected) {
            let entry = &tree.entries[*index];
            if member.name != *entry.relative.last().expect("validated path")
                || member.identity != entry.identity
                || member.kind != entry.kind
            {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "album member name/identity/kind changed",
                ));
            }
            let leaf = units(&member.name)?;
            let child = if entry.kind == EntryKind::Directory {
                open_album_directory(directory, leaf, false)?
            } else {
                open_album_file(directory, leaf)?
            };
            let observation = album_observe(&child, &parent_id)?;
            if observation.identity != entry.identity || observation.actual_name != member.name {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "album descendant changed during relative open",
                )
                .with_observation(observation));
            }
            if let Some(expected_digest) = &entry.digest {
                ensure_regular(&child)?;
                if observation.size != Some(expected_digest.length)
                    || album_digest(&child, expected_digest.length, cancel)? != *expected_digest
                {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "album file digest/length changed",
                    )
                    .with_observation(observation));
                }
            } else {
                relative.push(leaf.to_vec());
                walk(&child, relative, tree, cancel, result)?;
                let _ = relative.pop();
            }
            result.push(WindowsAlbumTreeObservation {
                relative: entry.relative.clone(),
                observation,
            });
            // Each temporary descendant handle is dropped before the root rename.
        }
        ensure_directory(directory)?;
        cancelled(cancel)
    }
    let mut result = Vec::with_capacity(tree.entries.len());
    walk(root, &mut Vec::new(), tree, cancel, &mut result)?;
    result.sort_by(|a, b| {
        let a = a
            .relative
            .iter()
            .map(|part| units(part).expect("validated name"));
        let b = b
            .relative
            .iter()
            .map(|part| units(part).expect("validated name"));
        a.cmp(b)
    });
    Ok(result)
}

fn album_digest(
    mut file: &File,
    length: u64,
    cancel: &AtomicBool,
) -> Result<ContentDigest, MutationError> {
    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let mut hash = Sha256::new();
    let mut remaining = length;
    let mut buffer = [0u8; 64 * 1024];
    while remaining != 0 {
        cancelled(cancel)?;
        let amount = remaining.min(buffer.len() as u64) as usize;
        let n = file.read(&mut buffer[..amount]).map_err(io_error)?;
        if n == 0 {
            return Err(err(
                MutationErrorKind::StalePlan,
                "album file shortened while reading",
            ));
        }
        hash.update(&buffer[..n]);
        remaining -= n as u64;
    }
    cancelled(cancel)?;
    if file.read(&mut buffer[..1]).map_err(io_error)? != 0 {
        return Err(err(
            MutationErrorKind::StalePlan,
            "album file grew while reading",
        ));
    }
    Ok(ContentDigest {
        sha256: hash.finalize().into(),
        length,
    })
}

pub(super) fn open_album_directory(
    parent: &File,
    name: &[u16],
    rename_root: bool,
) -> Result<File, MutationError> {
    open_album_relative(
        parent,
        name,
        FILE_LIST_DIRECTORY
            | FILE_READ_ATTRIBUTES
            | SYNCHRONIZE
            | if rename_root { DELETE } else { FILE_TRAVERSE },
        FILE_DIRECTORY_FILE
            | if rename_root {
                FILE_WRITE_THROUGH
            } else {
                NTCREATEFILE_CREATE_OPTIONS(0)
            },
        if rename_root {
            FILE_SHARE_MODE(0)
        } else {
            FILE_SHARE_READ | FILE_SHARE_WRITE
        },
    )
}
fn open_album_file(parent: &File, name: &[u16]) -> Result<File, MutationError> {
    open_album_relative(
        parent,
        name,
        FILE_GENERIC_READ,
        FILE_NON_DIRECTORY_FILE,
        FILE_SHARE_READ,
    )
}
fn open_album_relative(
    parent: &File,
    name: &[u16],
    access: FILE_ACCESS_RIGHTS,
    options: NTCREATEFILE_CREATE_OPTIONS,
    sharing: FILE_SHARE_MODE,
) -> Result<File, MutationError> {
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
            access,
            &attributes,
            &mut status_block,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            sharing,
            FILE_OPEN,
            options | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            None,
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "album relative open encountered reparse point",
        ));
    }
    if status.0 < 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(err(
            MutationErrorKind::Unsupported,
            "album open returned no handle",
        ));
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    if status.0 != 0 || unsafe { status_block.Anonymous.Status }.0 != 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "album open did not complete synchronously",
        ));
    }
    Ok(file)
}

pub(super) fn rename_album_handle(
    file: &File,
    parent: &WindowsParent,
    name: &MutationName,
) -> Result<(), MutationError> {
    rename_album_handle_relative(file, Some(parent.handle()), name)
}

fn rename_album_handle_relative(
    file: &File,
    parent: Option<&File>,
    name: &MutationName,
) -> Result<(), MutationError> {
    let name = units(name)?;
    let offset = std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName);
    let bytes = size_of::<FILE_RENAME_INFORMATION>() + name.len() * 2;
    let mut allocation = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let pointer = allocation.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    unsafe {
        (*pointer).Anonymous.ReplaceIfExists = false;
        (*pointer).RootDirectory = parent
            .map(|parent| HANDLE(parent.as_raw_handle()))
            .unwrap_or_default();
        (*pointer).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            pointer.cast::<u8>().add(offset).cast::<u16>(),
            name.len(),
        );
    }
    let mut status_block = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtSetInformationFile(
            HANDLE(file.as_raw_handle()),
            &mut status_block,
            pointer.cast(),
            bytes as u32,
            FileRenameInformation,
        )
    };
    if status.0 < 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if status.0 != 0 || unsafe { status_block.Anonymous.Status }.0 != 0 {
        return Err(err(
            MutationErrorKind::UnknownOutcome,
            "album rename completion is uncertain",
        ));
    }
    Ok(())
}

fn inspect_album_original_location(
    source: &WindowsAlbumDirectoryLease,
) -> Result<Option<MutationObservation>, MutationError> {
    if let Some(file) = &source.file {
        ensure_directory(file)?;
        let observation = album_observe(file, source.scope.parent_identity())?;
        if observation.identity != source.original.identity
            || observation.actual_name != source.original.actual_name
        {
            return Err(err(
                MutationErrorKind::StalePlan,
                "bound original album location changed",
            )
            .with_observation(observation));
        }
        return Ok(Some(observation));
    }
    match super::super::open_child(
        source.scope.parent_handle(),
        units(&source.original.actual_name)?,
        false,
    ) {
        Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(None),
        Err(error) => Err(read_error(error)),
        Ok(file) => {
            if kind(&file.metadata().map_err(io_error)?) == EntryKind::LinkOrReparse {
                return Err(err(
                    MutationErrorKind::UnsafeReparse,
                    "original album location is now reparse",
                ));
            }
            album_observe(&file, source.scope.parent_identity()).map(Some)
        }
    }
}

pub(super) fn album_observe(
    file: &File,
    parent_identity: &ObjectIdentity,
) -> Result<MutationObservation, MutationError> {
    let mut path = vec![0u16; 32768];
    let n = unsafe {
        GetFinalPathNameByHandleW(
            HANDLE(file.as_raw_handle()),
            &mut path,
            GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
        )
    } as usize;
    if n == 0 {
        return Err(io_error(io::Error::last_os_error()));
    }
    if n >= path.len() {
        return Err(err(
            MutationErrorKind::Unsupported,
            "album observed path exceeds bound",
        ));
    }
    path.truncate(n);
    let name = path
        .rsplit(|unit| *unit == 92)
        .next()
        .ok_or_else(|| err(MutationErrorKind::Io, "album observed leaf missing"))?
        .to_vec();
    let metadata = file.metadata().map_err(io_error)?;
    Ok(MutationObservation {
        identity: identity(file).map_err(read_error)?,
        parent_identity: parent_identity.clone(),
        actual_name: MutationName::Windows(name),
        reference: StorageRef::WindowsPath { utf16: path },
        size: (!metadata.is_dir()).then_some(metadata.len()),
    })
}

/// One-shot selected-root album authority. Source ancestors never become an
/// exposed general parent grant, and the source is never put in generic roots.
pub struct WindowsAlbumRootRenameSession {
    target_and_journal: WindowsMutationStorage,
    source_reference: StorageRef,
    source: WindowsAlbumDirectoryLease,
}
impl WindowsAlbumRootRenameSession {
    pub fn selected_source_reference(&self) -> &StorageRef {
        &self.source_reference
    }
    pub fn selected_source_identity(&self) -> &ObjectIdentity {
        &self.source.original.identity
    }
    pub fn source_observation(&self) -> &MutationObservation {
        &self.source.original
    }
    pub fn target_and_journal(&self) -> &WindowsMutationStorage {
        &self.target_and_journal
    }
    pub fn rename_album_directory(
        &mut self,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsAlbumPublication> {
        self.target_and_journal.rename_album_directory_with(
            &mut self.source,
            parent,
            name,
            cancel,
            (true, |_| Ok(())),
        )
    }
    pub fn inspect_original_location(&self) -> Result<Option<MutationObservation>, MutationError> {
        self.source.scope.check(&self.target_and_journal, true)?;
        inspect_album_original_location(&self.source)
    }
    pub fn verify_album_publication(
        &self,
        published: &WindowsAlbumPublication,
        cancel: &AtomicBool,
    ) -> Result<Vec<WindowsAlbumTreeObservation>, MutationError> {
        self.target_and_journal
            .verify_album_publication(published, cancel)
    }
}

impl WindowsMutationStorage {
    /// Choose this explicit domain before acquiring ordinary selected-root pins.
    /// The source grant supplies full identity independently from the manifest.
    /// No preexisting generic source authority is weakened or silently reused.
    pub fn for_selected_album_root_rename(
        source: WindowsSelectedRootGrant,
        target: WindowsSelectedRootGrant,
        journal: WindowsSelectedRootGrant,
        entries: Vec<WindowsAlbumTreeEntry>,
        cancel: &AtomicBool,
    ) -> Result<WindowsAlbumRootRenameSession, MutationError> {
        cancelled(cancel)?;
        validate_root_path(&source.path)?;
        let target = pin_root(
            target.path,
            Some(&target.expected_identity),
            RootRole::Target,
            RootOwnership::Selected,
        )?;
        let journal = pin_root(
            journal.path,
            Some(&journal.expected_identity),
            RootRole::Journal,
            RootOwnership::Selected,
        )?;
        ensure_disjoint_roots(&target, &journal)?;
        let parent_path = source.path.parent().ok_or_else(|| {
            err(
                MutationErrorKind::Unsupported,
                "selected album root needs parent",
            )
        })?;
        let ancestors = chain(parent_path).map_err(read_error)?;
        for ancestor in &ancestors {
            ensure_directory(ancestor)?;
        }
        let parent = ancestors.last().ok_or_else(|| {
            err(
                MutationErrorKind::Unsupported,
                "selected album root has no parent pin",
            )
        })?;
        let parent_identity = identity(parent).map_err(read_error)?;
        let leaf = MutationName::Windows(
            source
                .path
                .file_name()
                .ok_or_else(|| {
                    err(
                        MutationErrorKind::InvalidName,
                        "selected album root has no leaf",
                    )
                })?
                .encode_wide()
                .collect(),
        );
        let tree = validate_album_tree(&source.expected_identity, entries)?;
        let file = open_album_directory(parent, units(&leaf)?, true)?;
        ensure_directory(&file)?;
        let original = album_observe(&file, &parent_identity)?;
        if original.identity != source.expected_identity {
            return Err(err(
                MutationErrorKind::StalePlan,
                "selected album root changed from grant",
            )
            .with_observation(original));
        }
        // Establish the same local-NTFS domain as ordinary selected roots using
        // this exact source handle, without reopening or pinning its old path.
        require_album_ntfs(&file)?;
        for other in [&target, &journal] {
            let other_id = identity(&other.directory).map_err(read_error)?;
            for ancestor in ancestors.iter().chain(std::iter::once(&file)) {
                if identity(ancestor).map_err(read_error)? == other_id {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "selected album roots overlap",
                    ));
                }
            }
            for ancestor in other
                ._ancestors
                .iter()
                .chain(std::iter::once(&other.directory))
            {
                if identity(ancestor).map_err(read_error)? == original.identity {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "selected album roots overlap",
                    ));
                }
            }
        }
        verify_album_tree(&file, &tree, cancel)?;
        cancelled(cancel)?;
        Ok(WindowsAlbumRootRenameSession {
            target_and_journal: WindowsMutationStorage {
                roots: vec![target, journal],
                domain: WindowsMutationDomain::SelectedRoots,
            },
            source_reference: StorageRef::WindowsPath {
                utf16: source.path.as_os_str().encode_wide().collect(),
            },
            source: WindowsAlbumDirectoryLease {
                file: Some(file),
                scope: AlbumSourceScope::SelectedRoot {
                    ancestors,
                    parent_identity,
                },
                original,
                tree,
            },
        })
    }
}

pub(super) fn require_album_ntfs(directory: &File) -> Result<(), MutationError> {
    let mut local = [0u16; 32768];
    let n = unsafe {
        GetFinalPathNameByHandleW(
            HANDLE(directory.as_raw_handle()),
            &mut local,
            VOLUME_NAME_GUID,
        )
    } as usize;
    let prefix: Vec<_> = r"\\?\Volume{".encode_utf16().collect();
    if n == 0 || n >= local.len() || !local[..n].starts_with(&prefix) {
        return Err(err(
            MutationErrorKind::Unsupported,
            "album root has no local volume GUID",
        ));
    }
    let mut filesystem = [0u16; 32];
    unsafe {
        GetVolumeInformationByHandleW(
            HANDLE(directory.as_raw_handle()),
            None,
            None,
            None,
            None,
            Some(&mut filesystem),
        )
    }
    .map_err(win_error)?;
    if filesystem
        .iter()
        .copied()
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>()
        != "NTFS".encode_utf16().collect::<Vec<_>>()
    {
        return Err(err(
            MutationErrorKind::Unsupported,
            "whole-album rename requires local NTFS",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "windows_album_rename_tests.rs"]
mod tests;

#[path = "windows_album_child_rename.rs"]
mod child_rename;
pub use child_rename::{
    WindowsAlbumChildRename, WindowsAlbumChildRenameObservation, WindowsAlbumChildRenameScope,
    WindowsAlbumRootChildRenameSession,
};
