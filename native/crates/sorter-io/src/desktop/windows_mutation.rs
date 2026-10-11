//! Experimental primitives restricted to an owned TempDir. No production constructor.
//! A source lease denies read/write/delete sharing; ancestors remain pinned.
//! See docs/verification/windows-mutation-task2.md for API evidence and native gates.
use super::{chain, identity, kind};
use crate::{
    mutation::*,
    storage::{EntryKind, ObjectIdentity, StorageRef},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use windows::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
            FILE_OPEN_REPARSE_POINT, FILE_RENAME_INFORMATION, FILE_SYNCHRONOUS_IO_NONALERT,
            FILE_WRITE_THROUGH, FileRenameInformation, NTCREATEFILE_CREATE_OPTIONS, NtCreateFile,
            NtSetInformationFile,
        },
    },
    Win32::{
        Foundation::{
            HANDLE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, RtlNtStatusToDosError,
            STATUS_REPARSE_POINT_ENCOUNTERED, UNICODE_STRING,
        },
        Storage::FileSystem::*,
        System::IO::IO_STATUS_BLOCK,
    },
    core::PWSTR,
};

fn err(kind: MutationErrorKind, text: &str) -> MutationError {
    MutationError::new(kind, text)
}
fn io_error(e: io::Error) -> MutationError {
    let kind = match e.raw_os_error() {
        Some(32 | 33) => MutationErrorKind::Locked,
        Some(80 | 183) => MutationErrorKind::StalePlan,
        Some(5) => MutationErrorKind::PermissionDenied,
        Some(39 | 112) => MutationErrorKind::DiskFull,
        Some(17 | 50 | 1) => MutationErrorKind::Unsupported,
        _ => MutationErrorKind::Io,
    };
    MutationError::new(kind, e.to_string())
}
fn win_error(e: windows::core::Error) -> MutationError {
    io_error(io::Error::from_raw_os_error(
        (e.code().0 as u32 & 0xffff) as i32,
    ))
}
fn read_error(e: crate::storage::StorageError) -> MutationError {
    use crate::storage::StorageErrorKind as K;
    let kind = match e.kind {
        K::UnsafeLink => MutationErrorKind::UnsafeReparse,
        K::Changed => MutationErrorKind::StalePlan,
        K::PermissionDenied => MutationErrorKind::PermissionDenied,
        _ => MutationErrorKind::Io,
    };
    err(kind, &format!("{e:?}"))
}
fn cancelled(cancel: &AtomicBool) -> Result<(), MutationError> {
    if cancel.load(Ordering::Acquire) {
        Err(err(
            MutationErrorKind::Cancelled,
            "cancelled before dispatch",
        ))
    } else {
        Ok(())
    }
}
fn units(name: &MutationName) -> Result<&[u16], MutationError> {
    let MutationName::Windows(name) = name else {
        return Err(err(
            MutationErrorKind::InvalidName,
            "Windows name must be lossless UTF-16",
        ));
    };
    if name.is_empty()
        || name.len() > 255
        || name == &[46]
        || name == &[46, 46]
        || name
            .iter()
            .any(|u| *u < 32 || matches!(*u, 34 | 42 | 47 | 58 | 60 | 62 | 63 | 92 | 124))
        || matches!(name.last(), Some(32 | 46))
    {
        return Err(err(
            MutationErrorKind::InvalidName,
            "invalid or unsafe Windows child name",
        ));
    }
    // Only ASCII device tokens are normalized; this is never an identity comparison.
    let stem: Vec<u16> = name
        .iter()
        .take_while(|u| **u != 46)
        .map(|u| if (97..=122).contains(u) { u - 32 } else { *u })
        .collect();
    let ascii = String::from_utf16_lossy(&stem);
    if matches!(
        ascii.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ((ascii.starts_with("COM") || ascii.starts_with("LPT"))
        && stem.len() == 4
        && (matches!(stem[3], 49..=57 | 0xb9 | 0xb2 | 0xb3)))
    {
        return Err(err(
            MutationErrorKind::InvalidName,
            "reserved DOS device name",
        ));
    }
    Ok(name)
}
struct Root {
    _ancestors: Vec<File>,
    directory: File,
    _tree: tempfile::TempDir,
}
#[derive(Clone)]
pub struct WindowsParent {
    ancestors: Arc<Vec<File>>,
    identity: ObjectIdentity,
    root: Arc<Root>,
}
impl WindowsParent {
    fn handle(&self) -> &File {
        self.ancestors.last().unwrap_or(&self.root.directory)
    }
    pub fn contains_ancestor(&self, expected: &ObjectIdentity) -> Result<bool, MutationError> {
        for file in std::iter::once(&self.root.directory).chain(self.ancestors.iter()) {
            if &identity(file).map_err(read_error)? == expected {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn identity(&self) -> &ObjectIdentity {
        &self.identity
    }
}
pub struct WindowsMutationStorage {
    root: Arc<Root>,
}
impl WindowsMutationStorage {
    /// The only constructor consumes a fresh caller-owned synthetic TempDir.
    /// It is deliberately unsuitable for authorizing a user-selected library.
    pub fn for_owned_synthetic_tree(tree: tempfile::TempDir) -> Result<Self, MutationError> {
        use std::path::Prefix;
        if !matches!(tree.path().components().next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
        {
            return Err(err(
                MutationErrorKind::Unsupported,
                "only local drive-backed synthetic trees",
            ));
        }
        let mut ancestors = chain(tree.path()).map_err(read_error)?;
        let observed = ancestors
            .pop()
            .ok_or_else(|| err(MutationErrorKind::Io, "no root"))?;
        let root_name = tree
            .path()
            .file_name()
            .ok_or_else(|| err(MutationErrorKind::InvalidName, "root leaf missing"))?
            .encode_wide()
            .collect::<Vec<_>>();
        let directory = open_directory(
            ancestors.last().ok_or_else(|| {
                err(
                    MutationErrorKind::Unsupported,
                    "root anchor cannot be a synthetic tree",
                )
            })?,
            &root_name,
        )?;
        if identity(&directory).map_err(read_error)? != identity(&observed).map_err(read_error)? {
            return Err(err(MutationErrorKind::StalePlan, "root changed"));
        }
        ensure_directory(&directory)?;
        // A handle-derived volume GUID path is unavailable for SMB; do not infer locality from a drive letter.
        let mut local = [0u16; 32768];
        let n = unsafe {
            GetFinalPathNameByHandleW(
                HANDLE(directory.as_raw_handle()),
                &mut local,
                VOLUME_NAME_GUID,
            )
        } as usize;
        let prefix = r"\\?\Volume{".encode_utf16().collect::<Vec<_>>();
        if n == 0 || n >= local.len() || !local[..n].starts_with(&prefix) {
            return Err(err(
                MutationErrorKind::Unsupported,
                "no local volume GUID from held root",
            ));
        }
        let mut fs = [0u16; 32];
        unsafe {
            GetVolumeInformationByHandleW(
                HANDLE(directory.as_raw_handle()),
                None,
                None,
                None,
                None,
                Some(&mut fs),
            )
        }
        .map_err(win_error)?;
        if fs
            .iter()
            .copied()
            .take_while(|x| *x != 0)
            .collect::<Vec<_>>()
            != "NTFS".encode_utf16().collect::<Vec<_>>()
        {
            return Err(err(
                MutationErrorKind::Unsupported,
                "only local NTFS research primitives are implemented",
            ));
        }
        Ok(Self {
            root: Arc::new(Root {
                _tree: tree,
                _ancestors: ancestors,
                directory,
            }),
        })
    }
    pub fn parent(&self, relative: &Path) -> Result<WindowsParent, MutationError> {
        let mut ancestors = Vec::new();
        for c in relative.components() {
            let Component::Normal(n) = c else {
                return Err(err(
                    MutationErrorKind::InvalidName,
                    "only relative child components",
                ));
            };
            let name = MutationName::Windows(n.encode_wide().collect());
            let name = units(&name)?;
            let parent = ancestors.last().unwrap_or(&self.root.directory);
            let child = open_directory(parent, name)?;
            ensure_directory(&child)?;
            ancestors.push(child);
        }
        let id = identity(ancestors.last().unwrap_or(&self.root.directory)).map_err(read_error)?;
        Ok(WindowsParent {
            root: self.root.clone(),
            ancestors: Arc::new(ancestors),
            identity: id,
        })
    }
    /// Resolve only a directory beneath the consumed TempDir, using held ancestry.
    /// A serialized Windows path is evidence, never an ownership grant.
    pub fn parent_reference(&self, reference: &StorageRef) -> Result<WindowsParent, MutationError> {
        let StorageRef::WindowsPath { utf16 } = reference else {
            return Err(err(
                MutationErrorKind::Unsupported,
                "Windows reference required",
            ));
        };
        if utf16.contains(&0) {
            return Err(err(MutationErrorKind::InvalidName, "NUL in reference"));
        }
        let path = std::path::PathBuf::from(std::ffi::OsString::from_wide(utf16));
        let relative = path
            .strip_prefix(self.root._tree.path())
            .map_err(|_| err(MutationErrorKind::StalePlan, "reference outside owned tree"))?;
        self.parent(relative)
    }
    fn check_parent(&self, parent: &WindowsParent) -> Result<(), MutationError> {
        if !Arc::ptr_eq(&self.root, &parent.root) {
            return Err(err(
                MutationErrorKind::StalePlan,
                "parent belongs to another scope",
            ));
        }
        ensure_directory(&self.root.directory)?;
        for f in self.root._ancestors.iter().chain(parent.ancestors.iter()) {
            ensure_directory(f)?;
        }
        if identity(parent.handle()).map_err(read_error)? != parent.identity {
            return Err(err(
                MutationErrorKind::StalePlan,
                "parent identity mismatch",
            ));
        }
        Ok(())
    }
    /// Read-only synthetic fixture observation. The returned evidence is not consent.
    pub fn observe_source(
        &mut self,
        parent: &WindowsParent,
        name: &MutationName,
    ) -> Result<ExpectedSource, MutationError> {
        self.check_parent(parent)?;
        let file = open_file(parent.handle(), units(name)?, false)?;
        ensure_regular(&file)?;
        let observation = observe(&file, parent)?;
        let digest = digest(&file)?;
        Ok(ExpectedSource {
            identity: observation.identity,
            parent_identity: parent.identity.clone(),
            name: observation.actual_name,
            size: observation.size,
            modified: None,
            digest,
        })
    }
}
/// A location capability produced only by successful exclusive journal creation.
/// It holds the root and ancestry alive and identifies the exact file to read.
#[derive(Clone)]
pub struct WindowsJournalLocation {
    parent: WindowsParent,
    name: MutationName,
    identity: ObjectIdentity,
}

/// Evidence only that the OS accepted a file-buffer flush request.
/// This is not a namespace barrier or a mutation durability capability.
#[derive(Debug)]
#[must_use]
pub struct FileBufferFlushAcknowledgment {
    _private: (),
}

/// Private append-only journal bytes on an owned synthetic NTFS tree.
/// No writable reopen, truncate, raw handle, or arbitrary-path constructor exists.
/// `flush_file_buffers` acknowledges the OS request, not verified power-loss durability.
pub struct WindowsJournalFile {
    file: File,
    location: WindowsJournalLocation,
    poisoned: bool,
}
impl WindowsMutationStorage {
    pub fn create_journal(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
    ) -> Result<WindowsJournalFile, MutationError> {
        self.check_parent(parent)?;
        let file = open_relative_file(
            parent.handle(),
            units(name)?,
            true,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        )?;
        ensure_regular(&file)?;
        let location = WindowsJournalLocation {
            parent: parent.clone(),
            name: name.clone(),
            identity: identity(&file).map_err(read_error)?,
        };
        Ok(WindowsJournalFile {
            file,
            location,
            poisoned: false,
        })
    }

    /// Read-only close/reopen inspection, using previously captured identity and
    /// held ancestry. A supplied filename is never sufficient recovery authority.
    /// This is not a production restart-root authorization API.
    pub fn read_journal(
        &self,
        location: &WindowsJournalLocation,
    ) -> Result<Vec<u8>, MutationError> {
        self.check_parent(&location.parent)?;
        let mut file = open_relative_file(
            location.parent.handle(),
            units(&location.name)?,
            false,
            FILE_GENERIC_READ,
        )?;
        ensure_regular(&file)?;
        if identity(&file).map_err(read_error)? != location.identity {
            return Err(err(
                MutationErrorKind::StalePlan,
                "journal identity changed",
            ));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(io_error)?;
        Ok(bytes)
    }
}
impl WindowsJournalFile {
    pub fn location(&self) -> WindowsJournalLocation {
        self.location.clone()
    }

    pub fn read_all(&mut self) -> Result<Vec<u8>, MutationError> {
        let result = (|| {
            self.file.seek(SeekFrom::Start(0)).map_err(io_error)?;
            let mut bytes = Vec::new();
            self.file.read_to_end(&mut bytes).map_err(io_error)?;
            Ok(bytes)
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    pub fn append(&mut self, bytes: &[u8]) -> Result<(), MutationError> {
        if self.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "journal writer poisoned",
            ));
        }
        self.poisoned = true;
        self.file.seek(SeekFrom::End(0)).map_err(io_error)?;
        self.file.write_all(bytes).map_err(io_error)?;
        self.poisoned = false;
        Ok(())
    }
    pub fn flush_file_buffers(&mut self) -> Result<FileBufferFlushAcknowledgment, MutationError> {
        if self.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "journal writer poisoned",
            ));
        }
        self.poisoned = true;
        unsafe { FlushFileBuffers(HANDLE(self.file.as_raw_handle())) }.map_err(win_error)?;
        self.poisoned = false;
        Ok(FileBufferFlushAcknowledgment { _private: () })
    }
}

fn ensure_directory(file: &File) -> Result<(), MutationError> {
    match kind(&file.metadata().map_err(io_error)?) {
        EntryKind::Directory => ensure_case_insensitive(file),
        EntryKind::LinkOrReparse => Err(err(
            MutationErrorKind::UnsafeReparse,
            "reparse ancestor rejected",
        )),
        _ => Err(err(MutationErrorKind::Unsupported, "not a directory")),
    }
}
fn ensure_regular(file: &File) -> Result<(), MutationError> {
    if kind(&file.metadata().map_err(io_error)?) != EntryKind::File {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "only regular non-reparse files",
        ));
    }
    let mut info = FILE_STANDARD_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileStandardInfo,
            (&mut info as *mut FILE_STANDARD_INFO).cast(),
            size_of::<FILE_STANDARD_INFO>() as u32,
        )
    }
    .map_err(win_error)?;
    if info.NumberOfLinks != 1 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "hard-linked sources are unsupported",
        ));
    }
    Ok(())
}

fn ensure_case_insensitive(parent: &File) -> Result<(), MutationError> {
    // Case-sensitive directories are unsupported rather than silently case-folded.
    let mut case = FILE_CASE_SENSITIVE_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(parent.as_raw_handle()),
            FileCaseSensitiveInfo,
            (&mut case as *mut FILE_CASE_SENSITIVE_INFO).cast(),
            size_of::<FILE_CASE_SENSITIVE_INFO>() as u32,
        )
    }
    .map_err(win_error)?;
    if case.Flags != 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "case-sensitive directory requires separate native acceptance",
        ));
    }
    Ok(())
}

fn open_directory(parent: &File, name: &[u16]) -> Result<File, MutationError> {
    ensure_directory(parent)?;
    let mut owned = name.to_vec();
    let length = (owned.len() * 2) as u16;
    let string = UNICODE_STRING {
        Length: length,
        MaximumLength: length,
        Buffer: PWSTR(owned.as_mut_ptr()),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &string,
        Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE,
        ..Default::default()
    };
    let mut handle = HANDLE::default();
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            &attributes,
            &mut ios,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            None,
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "native relative operation encountered reparse point",
        ));
    }
    if status.0 < 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(err(
            MutationErrorKind::Io,
            "directory open returned no handle",
        ));
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    ensure_directory(&file)?;
    Ok(file)
}

fn open_file(parent: &File, name: &[u16], create: bool) -> Result<File, MutationError> {
    open_relative_file(
        parent,
        name,
        create,
        FILE_GENERIC_READ
            | DELETE
            | if create {
                FILE_GENERIC_WRITE
            } else {
                FILE_ACCESS_RIGHTS(0)
            },
    )
}
fn open_relative_file(
    parent: &File,
    name: &[u16],
    create: bool,
    access: FILE_ACCESS_RIGHTS,
) -> Result<File, MutationError> {
    open_relative_file_with_options(parent, name, create, access, NTCREATEFILE_CREATE_OPTIONS(0))
}
fn open_relative_file_with_options(
    parent: &File,
    name: &[u16],
    create: bool,
    access: FILE_ACCESS_RIGHTS,
    extra_options: NTCREATEFILE_CREATE_OPTIONS,
) -> Result<File, MutationError> {
    let mut owned = name.to_vec();
    let length = (owned.len() * 2) as u16;
    let string = UNICODE_STRING {
        Length: length,
        MaximumLength: length,
        Buffer: PWSTR(owned.as_mut_ptr()),
    };
    ensure_case_insensitive(parent)?;
    let attr = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &string,
        Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE,
        ..Default::default()
    };
    let mut handle = HANDLE::default();
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &attr,
            &mut ios,
            None,
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_MODE(0),
            if create { FILE_CREATE } else { FILE_OPEN },
            FILE_NON_DIRECTORY_FILE
                | FILE_OPEN_REPARSE_POINT
                | FILE_SYNCHRONOUS_IO_NONALERT
                | extra_options,
            None,
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "native relative operation encountered reparse point",
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
            "native open returned no handle",
        ));
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}
fn digest(mut file: &File) -> Result<ContentDigest, MutationError> {
    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 65536];
    let mut length = 0;
    loop {
        let n = file.read(&mut bytes).map_err(io_error)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
        length += n as u64;
    }
    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
    Ok(ContentDigest {
        sha256: hash.finalize().into(),
        length,
    })
}
fn observe(file: &File, parent: &WindowsParent) -> Result<MutationObservation, MutationError> {
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
        return Err(err(MutationErrorKind::Io, "final path exceeds buffer"));
    }
    path.truncate(n);
    let actual = path
        .rsplit(|u| *u == 92)
        .next()
        .ok_or_else(|| err(MutationErrorKind::Io, "missing final name"))?
        .to_vec();
    Ok(MutationObservation {
        identity: identity(file).map_err(read_error)?,
        parent_identity: parent.identity.clone(),
        actual_name: MutationName::Windows(actual),
        reference: StorageRef::WindowsPath { utf16: path },
        size: Some(file.metadata().map_err(io_error)?.len()),
    })
}
pub struct WindowsSourceLease {
    file: Option<File>,
    parent: WindowsParent,
    expected: ExpectedSource,
    observation: MutationObservation,
    mode: SourceLeaseMode,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceLeaseMode {
    Ordinary,
    WriteThroughRename,
}
pub struct WindowsOwnedTemp {
    file: Option<File>,
    parent: WindowsParent,
    identity: ObjectIdentity,
    observation: MutationObservation,
}
pub struct WindowsPublished {
    file: File,
    parent: WindowsParent,
    observation: MutationObservation,
    sync_poisoned: bool,
}
/// Checked file-buffer synchronization of the exact held, verified publication.
/// Not a namespace durability capability, source retirement, or replay authority.
#[derive(Debug)]
#[must_use]
pub struct PublishedFileSyncAcknowledgment {
    observation: MutationObservation,
}
impl PublishedFileSyncAcknowledgment {
    pub fn observation(&self) -> &MutationObservation {
        &self.observation
    }
}
impl WindowsMutationStorage {
    /// Verify, flush, and reverify without closing or reopening the publication.
    /// Any failure after dispatch permanently disables further sync on this guard;
    /// inspection remains read-only and possible. No source is retired here.
    pub fn sync_published_file(
        &mut self,
        published: &mut WindowsPublished,
        expected: &ContentDigest,
        cancel: &AtomicBool,
    ) -> Result<PublishedFileSyncAcknowledgment, MutationError> {
        self.sync_published_file_with(published, expected, cancel, |file| {
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)
        })
    }
    fn sync_published_file_with(
        &mut self,
        published: &mut WindowsPublished,
        expected: &ContentDigest,
        cancel: &AtomicBool,
        flush: impl FnOnce(&File) -> Result<(), MutationError>,
    ) -> Result<PublishedFileSyncAcknowledgment, MutationError> {
        self.check_parent(&published.parent)?;
        if published.sync_poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "published sync poisoned",
            ));
        }
        cancelled(cancel)?;
        self.verify(published, expected)?;
        cancelled(cancel)?;
        published.sync_poisoned = true;
        flush(&published.file).map_err(|e| {
            err(
                MutationErrorKind::UnknownOutcome,
                &format!("published file sync: {e:?}"),
            )
            .with_observation(published.observation.clone())
        })?;
        let observation = self.verify(published, expected).map_err(|e| {
            err(
                MutationErrorKind::UnknownOutcome,
                &format!("post-sync verification: {e:?}"),
            )
            .with_observation(published.observation.clone())
        })?;
        published.sync_poisoned = false;
        Ok(PublishedFileSyncAcknowledgment { observation })
    }
}
impl WindowsSourceLease {
    pub fn rewind(&mut self) -> Result<(), MutationError> {
        self.file
            .as_mut()
            .ok_or_else(|| err(MutationErrorKind::StalePlan, "lease consumed"))?
            .seek(SeekFrom::Start(0))
            .map_err(io_error)?;
        Ok(())
    }
    pub fn observation(&self) -> &MutationObservation {
        &self.observation
    }
}
impl Read for WindowsSourceLease {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("lease consumed"))?
            .read(buf)
    }
}
impl Write for WindowsOwnedTemp {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("temp consumed"))?
            .write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("temp consumed"))?
            .flush()
    }
}
impl WindowsOwnedTemp {
    pub fn verify_content(
        &self,
        expected: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
        ensure_regular(file)?;
        let observed = observe(file, &self.parent)?;
        if observed.identity != self.identity
            || observed.actual_name != self.observation.actual_name
            || digest(file)? != *expected
        {
            return Err(err(
                MutationErrorKind::VerificationFailed,
                "temp identity/name/content changed",
            ));
        }
        Ok(observed)
    }
    pub fn observation(&self) -> &MutationObservation {
        &self.observation
    }
    pub fn sync_data(&self) -> Result<(), MutationError> {
        let file = self
            .file
            .as_ref()
            .ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
        unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)
    }
}
fn rename(file: &File, parent: &WindowsParent, name: &MutationName) -> Result<(), MutationError> {
    let name = units(name)?;
    let from = identity(file).map_err(read_error)?;
    ensure_same_volume(&from, &parent.identity)?;
    let offset = std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName);
    let bytes = (offset + name.len() * 2).max(size_of::<FILE_RENAME_INFORMATION>());
    // usize storage provides alignment for the native header; checked bounded names.
    let mut allocation = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let ptr = allocation.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    unsafe {
        (*ptr).Anonymous.ReplaceIfExists = false;
        (*ptr).RootDirectory = HANDLE(parent.handle().as_raw_handle());
        (*ptr).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            (ptr as *mut u8).add(offset).cast::<u16>(),
            name.len(),
        );
    }
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtSetInformationFile(
            HANDLE(file.as_raw_handle()),
            &mut ios,
            ptr.cast(),
            bytes as u32,
            FileRenameInformation,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "native relative operation encountered reparse point",
        ));
    }
    if status.0 < 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    Ok(())
}
fn ensure_same_volume(from: &ObjectIdentity, to: &ObjectIdentity) -> Result<(), MutationError> {
    if !matches!((from, to), (ObjectIdentity::Windows { volume: a, .. }, ObjectIdentity::Windows { volume: b, .. }) if a == b)
    {
        return Err(err(
            MutationErrorKind::Unsupported,
            "cross-volume rename is unsupported; no copy/delete fallback",
        ));
    }
    Ok(())
}
impl MutationStorage for WindowsMutationStorage {
    type Parent = WindowsParent;
    type SourceLease = WindowsSourceLease;
    type OwnedTemp = WindowsOwnedTemp;
    type Published = WindowsPublished;
    fn capabilities(&self) -> MutationCapabilities {
        MutationCapabilities {
            seek: Capability::Supported,
            exclusive_create: Capability::Supported,
            no_replace_publish: Capability::Supported,
            same_volume_no_replace_rename: Capability::Supported,
            content_verification: Capability::Supported,
            durable_flush: Capability::Unknown,
            leased_source_retirement: Capability::Supported,
            staged_replacement: Capability::Unsupported,
        }
    }
    fn acquire_source(
        &mut self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
    ) -> Result<WindowsSourceLease, MutationError> {
        self.acquire_source_with_mode(parent, expected, SourceLeaseMode::Ordinary)
    }
    fn create_temp(
        &mut self,
        parent: &WindowsParent,
        job_id: &str,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsOwnedTemp> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.check_parent(parent)?;
            if job_id.is_empty()
                || job_id.len() > 100
                || !job_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err(err(
                    MutationErrorKind::InvalidName,
                    "job correlation must be 1-100 ASCII letters/digits/hyphens",
                ));
            }
            Ok(())
        })();
        if let Err(e) = preflight {
            return MutationOutcome::NoEffect(e);
        }
        if let Err(e) = cancelled(cancel) {
            return MutationOutcome::NoEffect(e);
        }
        let name = MutationName::Windows(format!(".sorter-{job_id}.tmp").encode_utf16().collect());
        let file = match open_file(
            parent.handle(),
            units(&name).expect("validated internal name"),
            true,
        ) {
            Ok(f) => f,
            Err(e) if e.kind == MutationErrorKind::UnknownOutcome => {
                return MutationOutcome::PendingUnknown(e);
            }
            Err(e) => return MutationOutcome::NoEffect(e),
        };
        // The create has happened: failures from here must not claim no effect.
        let observation = match observe(&file, parent) {
            Ok(observation) => observation,
            Err(e) => return MutationOutcome::PendingUnknown(e),
        };
        MutationOutcome::Applied(WindowsOwnedTemp {
            file: Some(file),
            parent: parent.clone(),
            identity: observation.identity.clone(),
            observation,
        })
    }
    fn publish_no_replace(
        &mut self,
        temp: &mut WindowsOwnedTemp,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.check_parent(&temp.parent)?;
            self.check_parent(parent)?;
            units(name)?;
            let file = temp
                .file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
            ensure_regular(file)?;
            if identity(file).map_err(read_error)? != temp.identity {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "temporary identity changed",
                ));
            }
            cancelled(cancel)?;
            rename(file, parent, name)
        })();
        if let Err(e) = preflight {
            return MutationOutcome::NoEffect(e);
        }
        let file = temp.file.take().expect("preflight retained handle");
        finish_publish(file, parent, name, &temp.identity)
    }
    fn rename_no_replace(
        &mut self,
        source: &mut WindowsSourceLease,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.check_parent(parent)?;
            self.validate_source(source)?;
            units(name)?;
            cancelled(cancel)?;
            rename(source.file.as_ref().expect("validated lease"), parent, name)
        })();
        if let Err(e) = preflight {
            return MutationOutcome::NoEffect(e);
        }
        let file = source.file.take().expect("validated lease");
        finish_publish(file, parent, name, &source.expected.identity)
    }
    fn verify(
        &mut self,
        published: &WindowsPublished,
        expected: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        let obs = self.inspect(published)?;
        if digest(&published.file)? != *expected {
            return Err(err(
                MutationErrorKind::VerificationFailed,
                "published digest/EOF mismatch",
            )
            .with_observation(obs));
        }
        Ok(obs)
    }
    fn retire_source(
        &mut self,
        source: &mut WindowsSourceLease,
        cancel: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.validate_source(source)?;
            observe(
                source.file.as_ref().expect("validated lease"),
                &source.parent,
            )
        })();
        let obs = match preflight {
            Ok(o) => o,
            Err(e) => return MutationOutcome::NoEffect(e),
        };
        if let Err(e) = cancelled(cancel) {
            return MutationOutcome::NoEffect(e);
        }
        let file = source.file.as_ref().expect("validated lease");
        let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
        if let Err(e) = unsafe {
            SetFileInformationByHandle(
                HANDLE(file.as_raw_handle()),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                size_of::<FILE_DISPOSITION_INFO>() as u32,
            )
        } {
            return MutationOutcome::NoEffect(win_error(e));
        }
        // Never reopen/delete a path. Other attribute handles may delay removal.
        // Keep the same lease until disposition is accepted; consume it afterward.
        drop(source.file.take());
        MutationOutcome::Applied(RetirementOutcome::DispositionRequested(obs))
    }
    fn inspect(
        &mut self,
        published: &WindowsPublished,
    ) -> Result<MutationObservation, MutationError> {
        self.check_parent(&published.parent)?;
        ensure_regular(&published.file)?;
        let obs = observe(&published.file, &published.parent)?;
        if obs.identity != published.observation.identity
            || obs.actual_name != published.observation.actual_name
        {
            return Err(
                err(MutationErrorKind::StalePlan, "published object changed").with_observation(obs),
            );
        }
        Ok(obs)
    }
}
impl WindowsMutationStorage {
    fn acquire_source_with_mode(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        mode: SourceLeaseMode,
    ) -> Result<WindowsSourceLease, MutationError> {
        self.check_parent(parent)?;
        if expected.parent_identity != parent.identity {
            return Err(err(MutationErrorKind::StalePlan, "source parent mismatch"));
        }
        let file = match mode {
            SourceLeaseMode::Ordinary => open_file(parent.handle(), units(&expected.name)?, false)?,
            SourceLeaseMode::WriteThroughRename => open_relative_file_with_options(
                parent.handle(),
                units(&expected.name)?,
                false,
                FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE,
                // Native CreateOptions, NOT Win32 FILE_FLAG_WRITE_THROUGH attributes.
                FILE_WRITE_THROUGH,
            )?,
        };
        let observation = verify_expected_source(&file, parent, expected)?;
        Ok(WindowsSourceLease {
            file: Some(file),
            parent: parent.clone(),
            expected: expected.clone(),
            observation,
            mode,
        })
    }
    fn validate_source(&self, source: &WindowsSourceLease) -> Result<(), MutationError> {
        self.check_parent(&source.parent)?;
        let file = source
            .file
            .as_ref()
            .ok_or_else(|| err(MutationErrorKind::StalePlan, "source lease consumed"))?;
        ensure_regular(file)?;
        let obs = observe(file, &source.parent)?;
        if obs.identity != source.expected.identity
            || obs.actual_name != source.expected.name
            || digest(file)? != source.expected.digest
        {
            return Err(err(MutationErrorKind::StalePlan, "leased source changed"));
        }
        Ok(())
    }
}
fn verify_expected_source(
    file: &File,
    parent: &WindowsParent,
    expected: &ExpectedSource,
) -> Result<MutationObservation, MutationError> {
    ensure_regular(file)?;
    let observed = observe(file, parent)?;
    if observed.identity != expected.identity
        || observed.actual_name != expected.name
        || expected.size.is_some_and(|s| observed.size != Some(s))
        || digest(file)? != expected.digest
        || expected.modified.as_ref().is_some_and(|expected| {
            file.metadata()
                .and_then(|m| m.modified())
                .map(|t| format!("{t:?}"))
                .as_ref()
                .ok()
                != Some(expected)
        })
    {
        return Err(err(
            MutationErrorKind::StalePlan,
            "source identity/name/content changed",
        )
        .with_observation(observed));
    }
    Ok(observed)
}
fn finish_publish(
    file: File,
    parent: &WindowsParent,
    name: &MutationName,
    expected: &ObjectIdentity,
) -> MutationOutcome<WindowsPublished> {
    let observation = match observe(&file, parent) {
        Ok(o) => o,
        Err(e) => return MutationOutcome::PendingUnknown(e),
    };
    if observation.identity != *expected || observation.actual_name != *name {
        return MutationOutcome::PendingUnknown(
            err(
                MutationErrorKind::UnknownOutcome,
                "rename completed but observed identity/name differs",
            )
            .with_observation(observation),
        );
    }
    MutationOutcome::Applied(WindowsPublished {
        file,
        parent: parent.clone(),
        observation,
        sync_poisoned: false,
    })
}

/// Rename-only guard for a consumed, owned synthetic NTFS tree.
///
/// Its narrower contract is a native write-through source open, checked preflush,
/// same-volume no-replace rename, and checked flush/full verification on the very
/// same held object. It does not assert generic namespace or power-loss durability.
/// In particular, `durable_flush` remains Unknown and this type cannot publish
/// copies, retire sources, or authorize a production/user-library mutation.
pub struct WindowsRenameStorage {
    owned: WindowsMutationStorage,
}

// Private deterministic failure/cancellation boundaries. The public API exposes
// neither callbacks nor arbitrary handles/paths. Successful native calls are
// always real, including when tests inject a lost response at the next boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenameCheckpoint {
    BeforePreflush,
    AfterPreflush,
    BeforeRename,
    AfterRename,
    BeforePostflush,
    AfterPostflush,
}

impl WindowsRenameStorage {
    pub fn new(owned: WindowsMutationStorage) -> Self {
        Self { owned }
    }

    pub fn owned(&self) -> &WindowsMutationStorage {
        &self.owned
    }

    fn rename_no_replace_with(
        &mut self,
        source: &mut WindowsSourceLease,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
        mut checkpoint: impl FnMut(RenameCheckpoint) -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsPublished> {
        let before_dispatch = (|| {
            cancelled(cancel)?;
            self.owned.check_parent(&source.parent)?;
            self.owned.check_parent(parent)?;
            if source.mode != SourceLeaseMode::WriteThroughRename {
                return Err(err(
                    MutationErrorKind::Unsupported,
                    "rename guard requires its own write-through source lease",
                ));
            }
            let file = source
                .file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "source lease consumed"))?;
            verify_expected_source(file, &source.parent, &source.expected)?;
            units(name)?;
            ensure_same_volume(&source.expected.identity, &parent.identity)?;
            checkpoint(RenameCheckpoint::BeforePreflush)?;
            cancelled(cancel)?;
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
            checkpoint(RenameCheckpoint::AfterPreflush)?;
            cancelled(cancel)?;
            // Recheck every supplied source precondition on the held handle.
            verify_expected_source(file, &source.parent, &source.expected)?;
            checkpoint(RenameCheckpoint::BeforeRename)?;
            cancelled(cancel)?;
            rename(file, parent, name)
        })();
        if let Err(error) = before_dispatch {
            return MutationOutcome::NoEffect(error);
        }

        // Namespace dispatch succeeded. Consume the lease before any fallible
        // observation, cancellation, callback or sync. There is no replay route.
        let file = source.file.take().expect("validated rename lease");
        let mut published = match finish_publish(file, parent, name, &source.expected.identity) {
            MutationOutcome::Applied(published) => published,
            MutationOutcome::PendingUnknown(error) => {
                return MutationOutcome::PendingUnknown(error);
            }
            MutationOutcome::NoEffect(_) => unreachable!("rename was already dispatched"),
        };
        let after_dispatch: Result<(), MutationError> = (|| {
            checkpoint(RenameCheckpoint::AfterRename)?;
            cancelled(cancel)?;
            checkpoint(RenameCheckpoint::BeforePostflush)?;
            cancelled(cancel)?;
            // This method checks the exact held publication before and after
            // its real FlushFileBuffers call, without closing/reopening by name.
            let _ack =
                self.owned
                    .sync_published_file(&mut published, &source.expected.digest, cancel)?;
            checkpoint(RenameCheckpoint::AfterPostflush)?;
            cancelled(cancel)?;
            self.owned.check_parent(parent)?;
            let renamed_expected = ExpectedSource {
                parent_identity: parent.identity.clone(),
                name: name.clone(),
                ..source.expected.clone()
            };
            verify_expected_source(&published.file, parent, &renamed_expected)?;
            cancelled(cancel)?;
            Ok(())
        })();
        if let Err(error) = after_dispatch {
            // Prefer a fresh native observation, then actual error evidence,
            // then the observation already obtained after the rename. Never
            // synthesize a returned reference from the requested leaf name.
            let observed = observe(&published.file, &published.parent)
                .ok()
                .or_else(|| error.observed.as_deref().cloned())
                .unwrap_or_else(|| published.observation.clone());
            return MutationOutcome::PendingUnknown(
                err(
                    MutationErrorKind::UnknownOutcome,
                    &format!("rename dispatched; reconcile without replay: {error:?}"),
                )
                .with_observation(observed),
            );
        }
        MutationOutcome::Applied(published)
    }
}

impl MutationStorage for WindowsRenameStorage {
    type Parent = WindowsParent;
    type SourceLease = WindowsSourceLease;
    type OwnedTemp = WindowsOwnedTemp;
    type Published = WindowsPublished;

    fn capabilities(&self) -> MutationCapabilities {
        MutationCapabilities {
            seek: Capability::Supported,
            exclusive_create: Capability::Unsupported,
            no_replace_publish: Capability::Unsupported,
            same_volume_no_replace_rename: Capability::Supported,
            content_verification: Capability::Supported,
            durable_flush: Capability::Unknown,
            leased_source_retirement: Capability::Unsupported,
            staged_replacement: Capability::Unsupported,
        }
    }

    fn acquire_source(
        &mut self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
    ) -> Result<WindowsSourceLease, MutationError> {
        self.owned
            .acquire_source_with_mode(parent, expected, SourceLeaseMode::WriteThroughRename)
    }

    fn create_temp(
        &mut self,
        _parent: &WindowsParent,
        _job_id: &str,
        _cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsOwnedTemp> {
        MutationOutcome::NoEffect(err(
            MutationErrorKind::Unsupported,
            "rename guard cannot create temporary files",
        ))
    }

    fn publish_no_replace(
        &mut self,
        _temp: &mut WindowsOwnedTemp,
        _parent: &WindowsParent,
        _name: &MutationName,
        _cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        MutationOutcome::NoEffect(err(
            MutationErrorKind::Unsupported,
            "rename guard cannot publish copied files",
        ))
    }

    fn rename_no_replace(
        &mut self,
        source: &mut WindowsSourceLease,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsPublished> {
        self.rename_no_replace_with(source, parent, name, cancel, |_| Ok(()))
    }

    fn verify(
        &mut self,
        published: &WindowsPublished,
        expected: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        self.owned.verify(published, expected)
    }

    fn retire_source(
        &mut self,
        _source: &mut WindowsSourceLease,
        _cancel: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        MutationOutcome::NoEffect(err(
            MutationErrorKind::Unsupported,
            "rename guard cannot retire a source separately",
        ))
    }

    fn inspect(
        &mut self,
        published: &WindowsPublished,
    ) -> Result<MutationObservation, MutationError> {
        self.owned.inspect(published)
    }
}

#[cfg(test)]
mod journal_flush_tests {
    use super::*;

    #[test]
    fn os_rejected_flush_poison_is_permanent_and_preserves_bytes() {
        let storage =
            WindowsMutationStorage::for_owned_synthetic_tree(tempfile::tempdir().unwrap()).unwrap();
        let parent = storage.parent(Path::new("")).unwrap();
        let name = MutationName::Windows("journal".encode_utf16().collect());
        let mut writer = storage.create_journal(&parent, &name).unwrap();
        writer.append(b"owned fixture").unwrap();
        let _ack = writer.flush_file_buffers().unwrap();
        let location = writer.location();
        drop(writer);
        // Test-only construction with reduced access provokes a real Windows
        // FlushFileBuffers error. The public API cannot create this bad handle.
        let file = open_relative_file(
            parent.handle(),
            units(&name).unwrap(),
            false,
            FILE_GENERIC_READ,
        )
        .unwrap();
        let mut rejected = WindowsJournalFile {
            file,
            location: location.clone(),
            poisoned: false,
        };
        assert!(
            matches!(rejected.flush_file_buffers(), Err(e) if e.kind == MutationErrorKind::PermissionDenied)
        );
        assert_eq!(rejected.read_all().unwrap(), b"owned fixture");
        assert!(
            matches!(rejected.append(b"must not write"), Err(e) if e.kind == MutationErrorKind::UnknownOutcome)
        );
        assert!(
            matches!(rejected.flush_file_buffers(), Err(e) if e.kind == MutationErrorKind::UnknownOutcome)
        );
        drop(rejected);
        assert_eq!(storage.read_journal(&location).unwrap(), b"owned fixture");
    }
}

#[cfg(test)]
mod published_sync_tests {
    use super::*;
    use std::path::PathBuf;

    fn applied<T>(outcome: MutationOutcome<T>) -> T {
        match outcome {
            MutationOutcome::Applied(value) => value,
            other => panic!(
                "expected Applied, got {}",
                match other {
                    MutationOutcome::NoEffect(_) => "NoEffect",
                    _ => "PendingUnknown",
                }
            ),
        }
    }
    fn name(value: &str) -> MutationName {
        MutationName::Windows(value.encode_utf16().collect())
    }
    fn fixture(
        leaf: MutationName,
    ) -> (
        WindowsMutationStorage,
        WindowsPublished,
        ContentDigest,
        PathBuf,
    ) {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_owned();
        std::fs::write(path.join("source"), b"retained original").unwrap();
        std::fs::create_dir(path.join("target")).unwrap();
        let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let parent = storage.parent(Path::new("target")).unwrap();
        let cancel = AtomicBool::new(false);
        let mut temp = applied(storage.create_temp(&parent, "sync", &cancel));
        let bytes = vec![0x51; 131_091];
        temp.write_all(&bytes).unwrap();
        temp.sync_data().unwrap();
        let expected = ContentDigest {
            sha256: Sha256::digest(&bytes).into(),
            length: bytes.len() as u64,
        };
        let published = applied(storage.publish_no_replace(&mut temp, &parent, &leaf, &cancel));
        (storage, published, expected, path)
    }
    #[test]
    fn verified_unicode_publication_sync_retains_identity_source_and_guards() {
        let leaf = name("שלום-🎵.mp3");
        let (mut storage, mut published, expected, path) = fixture(leaf.clone());
        let before = storage.inspect(&published).unwrap();
        let ack = storage
            .sync_published_file(&mut published, &expected, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(ack.observation(), &before);
        assert_eq!(ack.observation().actual_name, leaf);
        assert!(std::fs::remove_file(path.join("target/שלום-🎵.mp3")).is_err());
        assert!(std::fs::rename(path.join("target"), path.join("renamed-parent")).is_err());
        assert_eq!(
            std::fs::read(path.join("source")).unwrap(),
            b"retained original"
        );
        assert_eq!(storage.capabilities().durable_flush, Capability::Unknown);
        drop(published);
        let parent = storage.parent(Path::new("target")).unwrap();
        let reopened = storage.observe_source(&parent, &leaf).unwrap();
        assert_eq!(reopened.identity, before.identity);
        assert_eq!(reopened.digest, expected);
    }
    #[test]
    fn cancellation_before_dispatch_and_after_sync_is_not_false_rollback() {
        let (mut storage, mut published, expected, _) = fixture(name("final"));
        let cancel = AtomicBool::new(true);
        assert!(
            matches!(storage.sync_published_file_with(&mut published, &expected, &cancel, |_| panic!("cancelled dispatch")), Err(e) if e.kind == MutationErrorKind::Cancelled)
        );
        assert!(!published.sync_poisoned);
        cancel.store(false, Ordering::Release);
        let ack = storage
            .sync_published_file_with(&mut published, &expected, &cancel, |file| {
                unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
                cancel.store(true, Ordering::Release);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            ack.observation().identity,
            storage.inspect(&published).unwrap().identity
        );
        assert!(cancel.load(Ordering::Acquire));
        assert!(!published.sync_poisoned);
    }
    #[test]
    fn foreign_owner_and_wrong_content_reject_before_flush() {
        let (mut storage, mut published, expected, _) = fixture(name("final"));
        let mut foreign =
            WindowsMutationStorage::for_owned_synthetic_tree(tempfile::tempdir().unwrap()).unwrap();
        let cancel = AtomicBool::new(false);
        assert!(
            matches!(foreign.sync_published_file_with(&mut published, &expected, &cancel, |_| panic!("foreign dispatch")), Err(e) if e.kind == MutationErrorKind::StalePlan)
        );
        let mut wrong = expected.clone();
        wrong.sha256[0] ^= 1;
        assert!(
            matches!(storage.sync_published_file_with(&mut published, &wrong, &cancel, |_| panic!("wrong digest dispatch")), Err(e) if e.kind == MutationErrorKind::VerificationFailed)
        );
        assert!(!published.sync_poisoned);
        let _ack = storage
            .sync_published_file(&mut published, &expected, &cancel)
            .unwrap();
    }
    #[test]
    fn real_os_rejected_read_only_rename_sync_poison_is_permanent() {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_owned();
        std::fs::write(path.join("source"), b"known object").unwrap();
        let mut storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let parent = storage.parent(Path::new("")).unwrap();
        let expected = storage.observe_source(&parent, &name("source")).unwrap();
        let mut source = storage.acquire_source(&parent, &expected).unwrap();
        let cancel = AtomicBool::new(false);
        let mut published =
            applied(storage.rename_no_replace(&mut source, &parent, &name("renamed"), &cancel));
        let error = storage
            .sync_published_file(&mut published, &expected.digest, &cancel)
            .unwrap_err();
        assert_eq!(error.kind, MutationErrorKind::UnknownOutcome);
        assert!(error.detail.contains("PermissionDenied"), "{error:?}");
        assert!(published.sync_poisoned);
        assert!(
            matches!(storage.sync_published_file_with(&mut published, &expected.digest, &cancel, |_| panic!("must not retry rejected sync")), Err(e) if e.kind == MutationErrorKind::UnknownOutcome)
        );
        assert_eq!(
            storage
                .verify(&published, &expected.digest)
                .unwrap()
                .identity,
            expected.identity
        );
        drop(published);
        assert_eq!(
            std::fs::read(path.join("renamed")).unwrap(),
            b"known object"
        );
        assert!(!path.join("source").exists());
    }
    #[test]
    fn lost_flush_responses_preserve_published_evidence_and_forbid_retry() {
        for after in [false, true] {
            let (mut storage, mut published, expected, path) = fixture(name("final"));
            let cancel = AtomicBool::new(false);
            let result =
                storage.sync_published_file_with(&mut published, &expected, &cancel, |file| {
                    if after {
                        unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }
                            .map_err(win_error)?;
                    }
                    Err(err(MutationErrorKind::Io, "injected response loss"))
                });
            assert!(matches!(result, Err(e) if e.kind == MutationErrorKind::UnknownOutcome));
            assert!(published.sync_poisoned);
            assert!(
                matches!(storage.sync_published_file_with(&mut published, &expected, &cancel, |_| panic!("must not replay sync")), Err(e) if e.kind == MutationErrorKind::UnknownOutcome)
            );
            let observed = storage.verify(&published, &expected).unwrap();
            drop(published);
            let parent = storage.parent(Path::new("target")).unwrap();
            let reopened = storage.observe_source(&parent, &name("final")).unwrap();
            assert_eq!(reopened.identity, observed.identity);
            assert_eq!(reopened.digest, expected);
            assert_eq!(
                std::fs::read(path.join("source")).unwrap(),
                b"retained original"
            );
        }
    }
    #[test]
    fn native_unpaired_utf16_publication_is_not_lossily_reinterpreted() {
        let leaf = MutationName::Windows(vec![0x5e9, 0xd800, 0x2e, 0x6d, 0x70, 0x33]);
        let (mut storage, mut published, expected, _) = fixture(leaf.clone());
        let ack = storage
            .sync_published_file(&mut published, &expected, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(ack.observation().actual_name, leaf);
        // This primitive handles raw UTF-16. The string-based Copy planner still
        // rejects this filename; the test does not change that separate boundary.
        assert_eq!(storage.capabilities().durable_flush, Capability::Unknown);
    }
}

#[cfg(test)]
mod rename_guard_tests {
    use super::*;
    use std::{ffi::OsString, path::PathBuf};

    struct Fixture {
        storage: WindowsRenameStorage,
        source_parent: WindowsParent,
        target: WindowsParent,
        expected: ExpectedSource,
        path: PathBuf,
        bytes: Vec<u8>,
    }
    fn name(value: &str) -> MutationName {
        MutationName::Windows(value.encode_utf16().collect())
    }
    fn fixture() -> Fixture {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_owned();
        std::fs::create_dir(path.join("מקור")).unwrap();
        std::fs::create_dir(path.join("יעד")).unwrap();
        let bytes: Vec<_> = (0..131_091).map(|index| (index % 251) as u8).collect();
        let source_path = path.join("מקור/שיר-🎵.mp3");
        std::fs::write(&source_path, &bytes).unwrap();
        let modified = format!(
            "{:?}",
            std::fs::metadata(&source_path).unwrap().modified().unwrap()
        );
        let mut owned = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let source_parent = owned.parent(Path::new("מקור")).unwrap();
        let target = owned.parent(Path::new("יעד")).unwrap();
        let mut expected = owned
            .observe_source(&source_parent, &name("שיר-🎵.mp3"))
            .unwrap();
        expected.modified = Some(modified);
        Fixture {
            storage: WindowsRenameStorage::new(owned),
            source_parent,
            target,
            expected,
            path,
            bytes,
        }
    }
    fn applied<T>(outcome: MutationOutcome<T>) -> T {
        match outcome {
            MutationOutcome::Applied(value) => value,
            MutationOutcome::NoEffect(error) => panic!("unexpected NoEffect: {error:?}"),
            MutationOutcome::PendingUnknown(error) => {
                panic!("unexpected PendingUnknown: {error:?}")
            }
        }
    }
    fn no_effect<T>(outcome: MutationOutcome<T>, kind: MutationErrorKind) {
        match outcome {
            MutationOutcome::NoEffect(error) => assert_eq!(error.kind, kind, "{error:?}"),
            _ => panic!("expected NoEffect"),
        }
    }
    fn pending<T>(outcome: MutationOutcome<T>) -> MutationError {
        match outcome {
            MutationOutcome::PendingUnknown(error) => {
                assert_eq!(error.kind, MutationErrorKind::UnknownOutcome);
                error
            }
            _ => panic!("expected PendingUnknown after actual rename"),
        }
    }
    fn assert_source_untouched(fixture: &Fixture) {
        assert_eq!(
            std::fs::read(fixture.path.join("מקור/שיר-🎵.mp3")).unwrap(),
            fixture.bytes
        );
        assert!(!fixture.path.join("יעד/renamed.mp3").exists());
    }
    fn assert_renamed(fixture: &Fixture, error: &MutationError) {
        let observed = error
            .observed
            .as_ref()
            .expect("actual destination evidence");
        assert_eq!(observed.identity, fixture.expected.identity);
        assert_eq!(observed.parent_identity, fixture.target.identity);
        assert_eq!(observed.actual_name, name("renamed.mp3"));
        let reopened = open_file(
            fixture.target.handle(),
            units(&name("renamed.mp3")).unwrap(),
            false,
        )
        .unwrap();
        assert_eq!(observe(&reopened, &fixture.target).unwrap(), **observed);
        assert_eq!(digest(&reopened).unwrap(), fixture.expected.digest);
        drop(reopened);
        assert!(!fixture.path.join("מקור/שיר-🎵.mp3").exists());
        assert_eq!(
            std::fs::read(fixture.path.join("יעד/renamed.mp3")).unwrap(),
            fixture.bytes
        );
    }

    #[test]
    fn cross_directory_hebrew_rename_preserves_object_content_and_pinned_guards() {
        let mut f = fixture();
        let mut source = f
            .storage
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        let leaf = name("שלום-🎵.mp3");
        let published = applied(f.storage.rename_no_replace(
            &mut source,
            &f.target,
            &leaf,
            &AtomicBool::new(false),
        ));
        assert!(source.file.is_none());
        let observed = f.storage.verify(&published, &f.expected.digest).unwrap();
        assert_eq!(observed.identity, f.expected.identity);
        assert_eq!(observed.parent_identity, f.target.identity);
        assert_eq!(observed.actual_name, leaf);
        assert_eq!(observed.size, f.expected.size);
        assert!(!f.path.join("מקור/שיר-🎵.mp3").exists());
        assert!(std::fs::remove_file(f.path.join("יעד/שלום-🎵.mp3")).is_err());
        assert!(std::fs::rename(f.path.join("יעד"), f.path.join("other-target")).is_err());
        assert!(std::fs::rename(f.path.join("מקור"), f.path.join("other-source")).is_err());
        drop(published);
        let reopened = f.storage.owned.observe_source(&f.target, &leaf).unwrap();
        assert_eq!(reopened.identity, f.expected.identity);
        assert_eq!(reopened.digest, f.expected.digest);
        assert_eq!(
            std::fs::read(f.path.join("יעד/שלום-🎵.mp3")).unwrap(),
            f.bytes
        );
    }

    #[test]
    fn write_through_is_a_real_native_open_mode_and_lease_denies_sharing() {
        use windows::Wdk::Storage::FileSystem::{
            FILE_MODE_INFORMATION, FileModeInformation, NtQueryInformationFile,
        };
        let mut f = fixture();
        let source = f
            .storage
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        let file = source.file.as_ref().unwrap();
        let mut mode = FILE_MODE_INFORMATION::default();
        let mut ios = IO_STATUS_BLOCK::default();
        let status = unsafe {
            NtQueryInformationFile(
                HANDLE(file.as_raw_handle()),
                &mut ios,
                (&mut mode as *mut FILE_MODE_INFORMATION).cast(),
                size_of::<FILE_MODE_INFORMATION>() as u32,
                FileModeInformation,
            )
        };
        assert!(status.0 >= 0, "native mode query failed: {status:?}");
        assert_ne!(mode.Mode & FILE_WRITE_THROUGH.0, 0);
        assert_ne!(mode.Mode & FILE_SYNCHRONOUS_IO_NONALERT.0, 0);
        // A real flush establishes that this held handle has write access.
        unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.unwrap();
        let path = f.path.join("מקור/שיר-🎵.mp3");
        assert!(File::open(&path).is_err());
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
        assert!(std::fs::remove_file(&path).is_err());
        drop(source);
        assert_source_untouched(&f);
    }

    #[test]
    fn raw_utf16_name_is_preserved_without_lossy_substitution() {
        let mut f = fixture();
        let mut source = f
            .storage
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        let leaf = MutationName::Windows(vec![0x5e9, 0xd800, 0x2e, 0x6d, 0x70, 0x33]);
        let published = applied(f.storage.rename_no_replace(
            &mut source,
            &f.target,
            &leaf,
            &AtomicBool::new(false),
        ));
        assert_eq!(f.storage.inspect(&published).unwrap().actual_name, leaf);
        drop(published);
        let reopened = f.storage.owned.observe_source(&f.target, &leaf).unwrap();
        assert_eq!(reopened.identity, f.expected.identity);
        assert_eq!(reopened.digest, f.expected.digest);
        assert_eq!(
            std::fs::read(
                f.path
                    .join("יעד")
                    .join(OsString::from_wide(units(&leaf).unwrap()))
            )
            .unwrap(),
            f.bytes
        );
    }

    #[test]
    fn exact_and_case_collision_never_replace_or_consume_source() {
        for requested in ["Collision.mp3", "collision.mp3"] {
            let mut f = fixture();
            std::fs::write(f.path.join("יעד/Collision.mp3"), b"must remain").unwrap();
            let mut source = f
                .storage
                .acquire_source(&f.source_parent, &f.expected)
                .unwrap();
            no_effect(
                f.storage.rename_no_replace(
                    &mut source,
                    &f.target,
                    &name(requested),
                    &AtomicBool::new(false),
                ),
                MutationErrorKind::StalePlan,
            );
            assert!(source.file.is_some());
            drop(source);
            assert_source_untouched(&f);
            assert_eq!(
                std::fs::read(f.path.join("יעד/Collision.mp3")).unwrap(),
                b"must remain"
            );
        }
    }

    #[test]
    fn foreign_parent_foreign_lease_and_ordinary_lease_fail_before_flush() {
        let mut f = fixture();
        let foreign = fixture();
        let cancel = AtomicBool::new(false);
        let mut ordinary = f
            .storage
            .owned
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        no_effect(
            f.storage.rename_no_replace_with(
                &mut ordinary,
                &f.target,
                &name("renamed.mp3"),
                &cancel,
                |_| panic!("ordinary lease must not reach a flush"),
            ),
            MutationErrorKind::Unsupported,
        );
        drop(ordinary);
        let mut source = f
            .storage
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        no_effect(
            f.storage.rename_no_replace_with(
                &mut source,
                &foreign.target,
                &name("renamed.mp3"),
                &cancel,
                |_| panic!("foreign parent must not reach a flush"),
            ),
            MutationErrorKind::StalePlan,
        );
        let mut other_storage = foreign.storage;
        no_effect(
            other_storage.rename_no_replace_with(
                &mut source,
                &foreign.target,
                &name("renamed.mp3"),
                &cancel,
                |_| panic!("foreign lease must not reach a flush"),
            ),
            MutationErrorKind::StalePlan,
        );
        drop(source);
        assert_source_untouched(&f);
    }

    #[test]
    fn every_supplied_precondition_is_checked_on_write_through_acquisition() {
        let mut f = fixture();
        for field in 0..6 {
            let mut wrong = f.expected.clone();
            match field {
                0 => wrong.identity = f.target.identity.clone(),
                1 => wrong.parent_identity = f.target.identity.clone(),
                2 => wrong.name = name("שיר-🎵.MP3"),
                3 => wrong.size = Some(f.expected.size.unwrap() + 1),
                4 => wrong.digest.sha256[0] ^= 1,
                _ => wrong.modified = Some("not the actual modified time".into()),
            }
            assert!(matches!(
                f.storage.acquire_source(&f.source_parent, &wrong),
                Err(error) if error.kind == MutationErrorKind::StalePlan
            ));
        }
        assert_source_untouched(&f);
    }

    #[test]
    fn cancellation_before_rename_preserves_lease_and_namespace() {
        for stage in [
            None,
            Some(RenameCheckpoint::BeforePreflush),
            Some(RenameCheckpoint::AfterPreflush),
            Some(RenameCheckpoint::BeforeRename),
        ] {
            let mut f = fixture();
            let mut source = f
                .storage
                .acquire_source(&f.source_parent, &f.expected)
                .unwrap();
            let cancel = AtomicBool::new(stage.is_none());
            no_effect(
                f.storage.rename_no_replace_with(
                    &mut source,
                    &f.target,
                    &name("renamed.mp3"),
                    &cancel,
                    |current| {
                        if Some(current) == stage {
                            cancel.store(true, Ordering::Release);
                        }
                        Ok(())
                    },
                ),
                MutationErrorKind::Cancelled,
            );
            assert!(source.file.is_some());
            drop(source);
            assert_source_untouched(&f);
        }
    }

    #[test]
    fn cancellation_after_rename_or_sync_is_pending_with_actual_evidence_no_replay() {
        for stage in [
            RenameCheckpoint::AfterRename,
            RenameCheckpoint::BeforePostflush,
            RenameCheckpoint::AfterPostflush,
        ] {
            let mut f = fixture();
            let mut source = f
                .storage
                .acquire_source(&f.source_parent, &f.expected)
                .unwrap();
            let cancel = AtomicBool::new(false);
            let error = pending(f.storage.rename_no_replace_with(
                &mut source,
                &f.target,
                &name("renamed.mp3"),
                &cancel,
                |current| {
                    if current == stage {
                        cancel.store(true, Ordering::Release);
                    }
                    Ok(())
                },
            ));
            assert!(error.detail.contains("Cancelled"));
            assert!(source.file.is_none());
            assert_renamed(&f, &error);
            cancel.store(false, Ordering::Release);
            no_effect(
                f.storage.rename_no_replace_with(
                    &mut source,
                    &f.target,
                    &name("retry.mp3"),
                    &cancel,
                    |_| panic!("consumed lease cannot replay"),
                ),
                MutationErrorKind::StalePlan,
            );
            assert!(!f.path.join("יעד/retry.mp3").exists());
        }
    }

    #[test]
    fn failures_around_real_flush_and_rename_boundaries_preserve_effect_classification() {
        for stage in [
            RenameCheckpoint::BeforePreflush,
            RenameCheckpoint::AfterPreflush,
            RenameCheckpoint::BeforeRename,
            RenameCheckpoint::AfterRename,
            RenameCheckpoint::BeforePostflush,
            RenameCheckpoint::AfterPostflush,
        ] {
            let mut f = fixture();
            let mut source = f
                .storage
                .acquire_source(&f.source_parent, &f.expected)
                .unwrap();
            let outcome = f.storage.rename_no_replace_with(
                &mut source,
                &f.target,
                &name("renamed.mp3"),
                &AtomicBool::new(false),
                |current| {
                    if current == stage {
                        Err(err(
                            MutationErrorKind::Io,
                            "injected boundary response loss",
                        ))
                    } else {
                        Ok(())
                    }
                },
            );
            if matches!(
                stage,
                RenameCheckpoint::BeforePreflush
                    | RenameCheckpoint::AfterPreflush
                    | RenameCheckpoint::BeforeRename
            ) {
                no_effect(outcome, MutationErrorKind::Io);
                assert!(source.file.is_some());
                drop(source);
                assert_source_untouched(&f);
            } else {
                let error = pending(outcome);
                assert!(source.file.is_none());
                assert_renamed(&f, &error);
                no_effect(
                    f.storage.rename_no_replace_with(
                        &mut source,
                        &f.target,
                        &name("retry.mp3"),
                        &AtomicBool::new(false),
                        |_| panic!("must not replay a lost rename/sync response"),
                    ),
                    MutationErrorKind::StalePlan,
                );
            }
        }
    }

    #[test]
    fn actual_native_preflush_rejection_is_no_effect_with_source_retained() {
        let mut f = fixture();
        let mut source = f
            .storage
            .owned
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        // Test-only marker override preserves a real read-only native handle,
        // provoking the OS failure. Public code cannot forge this lease mode.
        source.mode = SourceLeaseMode::WriteThroughRename;
        no_effect(
            f.storage.rename_no_replace(
                &mut source,
                &f.target,
                &name("renamed.mp3"),
                &AtomicBool::new(false),
            ),
            MutationErrorKind::PermissionDenied,
        );
        assert!(source.file.is_some());
        drop(source);
        assert_source_untouched(&f);
    }

    #[test]
    fn rename_only_capabilities_do_not_enable_copy_retirement_or_global_durability() {
        let mut f = fixture();
        let caps = f.storage.capabilities();
        assert_eq!(caps.seek, Capability::Supported);
        assert_eq!(caps.same_volume_no_replace_rename, Capability::Supported);
        assert_eq!(caps.content_verification, Capability::Supported);
        assert_eq!(caps.exclusive_create, Capability::Unsupported);
        assert_eq!(caps.no_replace_publish, Capability::Unsupported);
        assert_eq!(caps.leased_source_retirement, Capability::Unsupported);
        assert_eq!(caps.staged_replacement, Capability::Unsupported);
        assert_eq!(caps.durable_flush, Capability::Unknown);
        assert_eq!(
            f.storage.owned().capabilities().durable_flush,
            Capability::Unknown
        );
        let cancel = AtomicBool::new(false);
        no_effect(
            f.storage.create_temp(&f.target, "blocked", &cancel),
            MutationErrorKind::Unsupported,
        );
        let mut temp = applied(f.storage.owned.create_temp(&f.target, "fixture", &cancel));
        temp.write_all(b"retained temp").unwrap();
        no_effect(
            f.storage
                .publish_no_replace(&mut temp, &f.target, &name("final.mp3"), &cancel),
            MutationErrorKind::Unsupported,
        );
        assert!(temp.file.is_some());
        let mut source = f
            .storage
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        no_effect(
            f.storage.retire_source(&mut source, &cancel),
            MutationErrorKind::Unsupported,
        );
        assert!(source.file.is_some());
        drop(source);
        drop(temp);
        assert_source_untouched(&f);
        assert_eq!(
            std::fs::read(f.path.join("יעד/.sorter-fixture.tmp")).unwrap(),
            b"retained temp"
        );
        assert!(!f.path.join("יעד/final.mp3").exists());
        assert!(!f.path.join("יעד/.sorter-blocked.tmp").exists());
    }

    #[test]
    fn differing_volume_ids_are_positive_rejection_without_copy_fallback() {
        let mut f = fixture();
        let source = f
            .storage
            .acquire_source(&f.source_parent, &f.expected)
            .unwrap();
        let mut other_volume = f.target.clone();
        let ObjectIdentity::Windows { volume, .. } = &mut other_volume.identity else {
            panic!("fixture must expose Windows volume identity");
        };
        *volume ^= 1;
        // Models a second volume ID, not a real cross-volume runtime test. The
        // native helper must reject the positive mismatch before dispatch.
        assert!(matches!(
            rename(source.file.as_ref().unwrap(), &other_volume, &name("renamed.mp3")),
            Err(error) if error.kind == MutationErrorKind::Unsupported
        ));
        drop(source);
        assert_source_untouched(&f);
    }
}
