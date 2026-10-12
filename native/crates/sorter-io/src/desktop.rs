//! Identity-aware desktop reads. Windows is the product backend. Linux is an
//! explicitly named test backend, not a claim of portable product acceptance.
use crate::storage::*;
#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(any(target_os = "linux", test))]
use std::fs;
use std::fs::{File, Metadata};
use std::path::{Component, Path, PathBuf};

#[cfg(any(windows, test))]
#[path = "desktop/records.rs"]
mod records;
#[cfg(any(windows, test))]
use records::*;
#[cfg(windows)]
#[path = "desktop/windows_read.rs"]
mod windows_read;
#[cfg(windows)]
use windows_read::*;

/// Per-enumeration limits for the explicit Windows album planning route.
/// The whole-tree collector still owns its separate aggregate budget.
#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowsDirectoryReadLimits {
    pub max_entries: usize,
    /// Sum of complete child reference lengths, measured in UTF-16 code units.
    pub max_path_units: usize,
    /// Includes the native call which establishes end-of-directory.
    pub max_batches: usize,
}
#[cfg(windows)]
impl WindowsDirectoryReadLimits {
    pub const ALBUM_DEFAULT: Self = Self { max_entries: 100_000, max_path_units: 16 * 1024 * 1024, max_batches: 1024 };
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.max_entries > Self::ALBUM_DEFAULT.max_entries
            || self.max_path_units > Self::ALBUM_DEFAULT.max_path_units
            || self.max_batches == 0 || self.max_batches > Self::ALBUM_DEFAULT.max_batches {
            return Err(error(StorageErrorKind::BudgetExceeded, "album directory limits exceed the supported bounded domain"));
        }
        Ok(())
    }
}
#[cfg(windows)]
impl Default for WindowsDirectoryReadLimits {
    fn default() -> Self { Self::ALBUM_DEFAULT }
}

fn error(kind: StorageErrorKind, detail: &str) -> StorageError {
    StorageError::new(kind, detail)
}
fn validate_path(path: &Path) -> Result<(), StorageError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(error(
            StorageErrorKind::InvalidReference,
            "an absolute path without dot components is required",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::path::Prefix;
        match path.components().next() {
            Some(Component::Prefix(p))
                if matches!(
                    p.kind(),
                    Prefix::Disk(_)
                        | Prefix::VerbatimDisk(_)
                        | Prefix::UNC(_, _)
                        | Prefix::VerbatimUNC(_, _)
                ) => {}
            _ => {
                return Err(error(
                    StorageErrorKind::InvalidReference,
                    "device namespaces are not storage roots",
                ));
            }
        }
        // Path::components normalizes interior `.`; reject raw dot segments
        // before constructing any component-relative native lookup.
        let raw: Vec<u16> = path.as_os_str().encode_wide().collect();
        if raw
            .split(|u| matches!(*u, 47 | 92))
            .any(|part| part == [46] || part == [46, 46])
        {
            return Err(error(
                StorageErrorKind::InvalidReference,
                "dot path components are not accepted",
            ));
        }
        for component in path.components() {
            if let Component::Normal(name) = component
                && name.encode_wide().any(|c| c == 0 || c == b':' as u16)
            {
                return Err(error(
                    StorageErrorKind::InvalidReference,
                    "NUL and alternate-stream references are not allowed",
                ));
            }
        }
    }
    Ok(())
}
#[cfg(windows)]
fn open_handle(path: &Path, read: bool) -> Result<File, StorageError> {
    use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};
    use windows::{Win32::Storage::FileSystem::*, core::PCWSTR};
    validate_path(path)?;
    if path.components().any(|c| matches!(c, Component::Normal(_))) {
        return Err(error(
            StorageErrorKind::InvalidReference,
            "absolute native opens are restricted to drive/share anchors",
        ));
    }
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(error(StorageErrorKind::InvalidReference, "embedded NUL"));
    }
    wide.push(0);
    // Used only for the drive/share anchor on Windows. Subsequent components
    // use NtCreateFile relative to held parent handles, with OBJ_DONT_REPARSE.
    // OPEN_EXISTING only; no write/delete access. RAII File owns the handle.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            if read {
                0x8000_0000
            } else {
                FILE_READ_ATTRIBUTES.0
            },
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }
    .map_err(|e| {
        StorageError::from(std::io::Error::from_raw_os_error(
            (e.code().0 as u32 & 0xffff) as i32,
        ))
    })?;
    Ok(unsafe { File::from_raw_handle(handle.0) })
}
#[cfg(target_os = "linux")]
fn open_handle(path: &Path, read: bool) -> Result<File, StorageError> {
    use std::os::unix::fs::OpenOptionsExt;
    // O_PATH lets stat observe the symlink object without reading its target.
    // Linux remains a test backend: this path-based ancestor walk is not a
    // production race-resistant openat2 sandbox.
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(if read {
            libc::O_NOFOLLOW | libc::O_NONBLOCK
        } else {
            libc::O_PATH | libc::O_NOFOLLOW
        })
        .open(path)
        .map_err(StorageError::from)
}
#[cfg(windows)]
fn identity(file: &File) -> Result<ObjectIdentity, StorageError> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::*};
    let mut info = FILE_ID_INFO::default();
    // Exact FILE_ID_INFO allocation/size; valid borrowed live file handle.
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .map_err(|e| {
        StorageError::from(std::io::Error::from_raw_os_error(
            (e.code().0 as u32 & 0xffff) as i32,
        ))
    })?;
    Ok(ObjectIdentity::Windows {
        volume: info.VolumeSerialNumber,
        file_id: info.FileId.Identifier,
    })
}
#[cfg(target_os = "linux")]
fn identity(file: &File) -> Result<ObjectIdentity, StorageError> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    Ok(ObjectIdentity::Test {
        id: format!("linux-test:{}:{}", m.dev(), m.ino()),
    })
}
fn kind(metadata: &Metadata) -> EntryKind {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return EntryKind::LinkOrReparse;
        }
    }
    if metadata.file_type().is_symlink() {
        EntryKind::LinkOrReparse
    } else if metadata.is_file() {
        EntryKind::File
    } else if metadata.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::Other
    }
}
/// Open and pin every component. Windows uses strict, single-component native
/// relative opens after the drive/share anchor; reparse statuses fail closed.
/// Final links are visible in directory observations and are never traversed.
fn chain(path: &Path) -> Result<Vec<File>, StorageError> {
    validate_path(path)?;
    let paths: Vec<_> = path.ancestors().collect();
    let mut handles = vec![];
    for p in paths.iter().rev() {
        #[cfg(target_os = "linux")]
        let f = open_handle(p, false)?;
        #[cfg(windows)]
        let f = if let Some(parent) = handles.last() {
            use std::os::windows::ffi::OsStrExt;
            let name = p
                .file_name()
                .ok_or_else(|| {
                    error(
                        StorageErrorKind::InvalidReference,
                        "missing child component",
                    )
                })?
                .encode_wide()
                .collect::<Vec<_>>();
            open_child(parent, &name, false)?
        } else {
            // Only the drive/share anchor is opened by absolute name.
            open_handle(p, false)?
        };
        if *p != path {
            match kind(&f.metadata()?) {
                EntryKind::LinkOrReparse => {
                    return Err(error(
                        StorageErrorKind::UnsafeLink,
                        "intermediate link/reparse point",
                    ));
                }
                EntryKind::Directory => {}
                _ => {
                    return Err(error(
                        StorageErrorKind::NotDirectory,
                        "intermediate component is not a directory",
                    ));
                }
            }
        }
        handles.push(f);
    }
    Ok(handles)
}
struct Backend {
    roots: Vec<ObjectIdentity>,
    // Pin selected roots on Windows to prevent their deletion/rename while used.
    _root_handles: Vec<File>,
    #[cfg(target_os = "linux")]
    paths: HashMap<StorageRef, PathBuf>,
    #[cfg(target_os = "linux")]
    next_reference: u64,
}
impl Backend {
    fn new(roots: &[PathBuf]) -> Result<Self, StorageError> {
        if roots.is_empty() {
            return Err(error(
                StorageErrorKind::OutsideRoots,
                "select at least one root",
            ));
        }
        let mut result = Self {
            roots: vec![],
            _root_handles: vec![],
            #[cfg(target_os = "linux")]
            paths: HashMap::new(),
            #[cfg(target_os = "linux")]
            next_reference: 0,
        };
        for path in roots {
            let mut handles = chain(path)?;
            let last = handles.last().expect("absolute path has root");
            match kind(&last.metadata()?) {
                EntryKind::Directory => {}
                EntryKind::LinkOrReparse => {
                    return Err(error(
                        StorageErrorKind::UnsafeLink,
                        "selected root is a link/reparse point",
                    ));
                }
                _ => {
                    return Err(error(
                        StorageErrorKind::NotDirectory,
                        "selected root must be a directory",
                    ));
                }
            }
            result.roots.push(identity(last)?);
            result._root_handles.append(&mut handles);
        }
        Ok(result)
    }
    fn checked(&self, path: &Path) -> Result<Vec<File>, StorageError> {
        let handles = chain(path)?;
        if !handles
            .iter()
            .map(identity)
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|id| self.roots.contains(id))
        {
            return Err(error(
                StorageErrorKind::OutsideRoots,
                "no selected-root identity in ancestry",
            ));
        }
        Ok(handles)
    }
    fn reference(&mut self, path: &Path) -> Result<StorageRef, StorageError> {
        self.checked(path)?;
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            Ok(StorageRef::WindowsPath {
                utf16: path.as_os_str().encode_wide().collect(),
            })
        }
        #[cfg(target_os = "linux")]
        {
            if let Some((r, _)) = self.paths.iter().find(|(_, p)| *p == path) {
                return Ok(r.clone());
            }
            let r = StorageRef::TestOpaque {
                id: format!("linux-test-reference:{}", self.next_reference),
            };
            self.next_reference += 1;
            self.paths.insert(r.clone(), path.into());
            Ok(r)
        }
    }
    fn path(&self, reference: &StorageRef) -> Result<PathBuf, StorageError> {
        #[cfg(windows)]
        if let StorageRef::WindowsPath { utf16 } = reference {
            use std::os::windows::ffi::OsStringExt;
            if utf16.contains(&0) {
                return Err(error(StorageErrorKind::InvalidReference, "embedded NUL"));
            }
            return Ok(std::ffi::OsString::from_wide(utf16).into());
        }
        #[cfg(target_os = "linux")]
        if let Some(path) = self.paths.get(reference) {
            return Ok(path.clone());
        }
        Err(error(
            StorageErrorKind::UnsupportedReference,
            "reference does not belong to this backend",
        ))
    }
    fn observe(
        &self,
        reference: &StorageRef,
        handles: &[File],
    ) -> Result<EntryObservation, StorageError> {
        let path = self.path(reference)?;
        let f = handles.last().expect("absolute path has root");
        let metadata = f.metadata()?;
        let kind = kind(&metadata);
        Ok(EntryObservation {
            reference: reference.clone(),
            identity: Some(identity(f)?),
            parent_identity: handles.iter().rev().nth(1).map(identity).transpose()?,
            name: path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned(),
            kind,
            size: if kind == EntryKind::File {
                Some(metadata.len())
            } else {
                None
            },
            modified: metadata.modified().ok().map(|t| format!("{t:?}")),
            // Read/list availability is only established by an attempted read.
            capabilities: Capabilities {
                read: None,
                list: None,
                seek: if kind == EntryKind::File {
                    Some(true)
                } else {
                    None
                },
            },
        })
    }
    fn stat(&mut self, reference: &StorageRef) -> Result<EntryObservation, StorageError> {
        let path = self.path(reference)?;
        let handles = self.checked(&path)?;
        self.observe(reference, &handles)
    }
    fn children(&mut self, reference: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        let path = self.path(reference)?;
        let handles = self.checked(&path)?;
        match kind(&handles.last().expect("root").metadata()?) {
            EntryKind::Directory => {}
            EntryKind::LinkOrReparse => {
                return Err(error(
                    StorageErrorKind::UnsafeLink,
                    "will not enumerate a link/reparse point",
                ));
            }
            _ => return Err(error(StorageErrorKind::NotDirectory, "not a directory")),
        }
        #[cfg(windows)]
        {
            windows_children(self, &path, &handles)
        }
        #[cfg(target_os = "linux")]
        {
            // Explicit Linux test backend; Windows never enumerates a path.
            let mut entries = vec![];
            for child in fs::read_dir(&path)? {
                let child = child?;
                let reference = self.reference(&child.path())?;
                let observation = self.stat(&reference)?;
                if observation.parent_identity != Some(identity(handles.last().expect("root"))?) {
                    return Err(error(
                        StorageErrorKind::Changed,
                        "enumerated child parent changed",
                    ));
                }
                entries.push(observation);
            }
            Ok(entries)
        }
    }

    fn open_read(&mut self, reference: &StorageRef) -> Result<ReadInput, StorageError> {
        let path = self.path(reference)?;
        let handles = self.checked(&path)?;
        let observed = handles.last().expect("root");
        match kind(&observed.metadata()?) {
            EntryKind::File => {}
            EntryKind::LinkOrReparse => {
                return Err(error(
                    StorageErrorKind::UnsafeLink,
                    "will not read a link/reparse point",
                ));
            }
            _ => return Err(error(StorageErrorKind::NotFile, "not a regular file")),
        }
        #[cfg(target_os = "linux")]
        let file = open_handle(&path, true)?;
        #[cfg(windows)]
        let file = reopen_bound(&handles, &path)?;
        let metadata = file.metadata()?;
        let read_identity = identity(&file)?;
        if kind(&metadata) != EntryKind::File || read_identity != identity(observed)? {
            return Err(error(
                StorageErrorKind::Changed,
                "object changed between observation and read open",
            ));
        }
        // Bind the exact read handle, not a fresh path stat. The metadata
        // provider also compares this with its caller's earlier observation.
        let binding = ReadBinding {
            identity: read_identity,
            size: metadata.len(),
            modified: format!("{:?}", metadata.modified()?),
        };
        Ok(ReadInput::seekable(file, binding.size).with_binding(binding))
    }
}
macro_rules! backend {
    ($name:ident) => {
        pub struct $name(Backend);
        impl $name {
            pub fn new(selected_roots: &[PathBuf]) -> Result<Self, StorageError> {
                Ok(Self(Backend::new(selected_roots)?))
            }
            pub fn reference(&mut self, path: &Path) -> Result<StorageRef, StorageError> {
                self.0.reference(path)
            }
        }
        impl ReadStorage for $name {
            fn stat(&mut self, reference: &StorageRef) -> Result<EntryObservation, StorageError> {
                self.0.stat(reference)
            }
            fn children(
                &mut self,
                reference: &StorageRef,
            ) -> Result<Vec<EntryObservation>, StorageError> {
                self.0.children(reference)
            }
            fn open_read(&mut self, reference: &StorageRef) -> Result<ReadInput, StorageError> {
                self.0.open_read(reference)
            }
        }
    };
}
#[cfg(windows)]
backend!(DesktopStorage);
#[cfg(target_os = "linux")]
backend!(LinuxTestStorage);

/// Disposable streaming fallback for the Linux test backend only.
/// Windows streaming spooling fails closed with UnsupportedCapability before
/// reading or creating anything. Windows ordinary seekable reads still work.
/// A directory's identity/no-share-delete pins do not prevent in-place reparse
/// mutation, so path-based scratch creation is not a safe Windows write boundary.
/// Caller must supply all selected roots. Construction never writes.
pub struct ScratchSpace {
    _base: PathBuf,
    _base_identity: ObjectIdentity,
    _pins: Vec<File>,
}
impl ScratchSpace {
    pub fn new(base: &Path, selected_roots: &[PathBuf]) -> Result<Self, StorageError> {
        if selected_roots.is_empty() {
            return Err(error(
                StorageErrorKind::OutsideRoots,
                "scratch exclusion requires selected roots",
            ));
        }
        let mut pins = chain(base)?;
        let base_file = pins.last().expect("absolute root");
        if kind(&base_file.metadata()?) == EntryKind::LinkOrReparse {
            return Err(error(
                StorageErrorKind::UnsafeLink,
                "scratch must not be a reparse point",
            ));
        }
        if kind(&base_file.metadata()?) != EntryKind::Directory {
            return Err(error(
                StorageErrorKind::NotDirectory,
                "scratch must be an existing directory",
            ));
        }
        let base_identity = identity(base_file)?;
        let base_ancestors = pins.iter().map(identity).collect::<Result<Vec<_>, _>>()?;
        for root in selected_roots {
            let mut handles = chain(root)?;
            let root_file = handles.last().expect("root");
            if kind(&root_file.metadata()?) == EntryKind::LinkOrReparse {
                return Err(error(
                    StorageErrorKind::UnsafeLink,
                    "selected root is a reparse point",
                ));
            }
            if kind(&root_file.metadata()?) != EntryKind::Directory {
                return Err(error(
                    StorageErrorKind::NotDirectory,
                    "selected root must be a directory",
                ));
            }
            let root_identity = identity(root_file)?;
            let root_ancestors = handles
                .iter()
                .map(identity)
                .collect::<Result<Vec<_>, _>>()?;
            if base_ancestors.contains(&root_identity) || root_ancestors.contains(&base_identity) {
                return Err(error(
                    StorageErrorKind::OutsideRoots,
                    "scratch and selected trees overlap by object identity",
                ));
            }
            pins.append(&mut handles);
        }
        Ok(Self {
            _base: base.into(),
            _base_identity: base_identity,
            _pins: pins,
        })
    }
    /// Fail closed: no directory lookup, input read, budget charge or creation.
    #[cfg(not(target_os = "linux"))]
    pub fn spool(
        &self,
        _input: ReadInput,
        _budget: &mut SpoolBudget,
        _cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ReadInput, StorageError> {
        Err(error(
            StorageErrorKind::UnsupportedCapability,
            "Windows streaming scratch fallback is unavailable until handle-relative no-reparse creation is validated; use native seekable file input",
        ))
    }
    #[cfg(target_os = "linux")]
    pub fn spool(
        &self,
        mut input: ReadInput,
        budget: &mut SpoolBudget,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ReadInput, StorageError> {
        use std::io::{Seek, SeekFrom};
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(error(StorageErrorKind::Cancelled, "spool cancelled"));
        }
        let guards = chain(&self._base)?;
        if identity(guards.last().expect("root"))? != self._base_identity {
            return Err(error(
                StorageErrorKind::Changed,
                "scratch directory changed",
            ));
        }
        // Linux test backend only: anonymous/unlinked scratch, never reopened
        // by name. This is not a production concurrent-mutation sandbox.
        // Failure, cancellation and success retain owned-handle cleanup.
        let mut file = tempfile::tempfile_in(&self._base)?;
        let length = crate::storage::copy_to_scratch(&mut input, &mut file, budget, cancelled)?;
        file.seek(SeekFrom::Start(0))?;
        Ok(ReadInput::seekable(file, length))
    }
}

#[cfg(test)]
#[path = "desktop/records_tests.rs"]
mod records_tests;
#[cfg(all(test, windows))]
#[path = "desktop/windows_tests.rs"]
mod windows_tests;

#[cfg(all(windows, feature = "experimental-windows-mutation"))]
#[path = "desktop/windows_mutation.rs"]
pub mod windows_mutation;
