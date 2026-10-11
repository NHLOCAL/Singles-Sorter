use super::*;
use std::{
    ffi::OsString,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle},
    },
};
use windows::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_OPEN, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT, NtCreateFile,
        },
    },
    Win32::{
        Foundation::{
            HANDLE, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, OBJECT_ATTRIBUTE_FLAGS,
            RtlNtStatusToDosError, STATUS_REPARSE_POINT_ENCOUNTERED, UNICODE_STRING,
        },
        Storage::FileSystem::*,
        System::IO::IO_STATUS_BLOCK,
    },
    core::PWSTR,
};

fn win_error(e: windows::core::Error) -> StorageError {
    StorageError::from(std::io::Error::from_raw_os_error(
        (e.code().0 as u32 & 0xffff) as i32,
    ))
}
fn case_attributes(parent: &File, exact: bool) -> Result<OBJECT_ATTRIBUTE_FLAGS, StorageError> {
    if exact {
        return Ok(OBJ_DONT_REPARSE);
    }
    let mut info = FILE_CASE_SENSITIVE_INFO::default();
    let result = unsafe {
        GetFileInformationByHandleEx(
            HANDLE(parent.as_raw_handle()),
            FileCaseSensitiveInfo,
            (&mut info as *mut FILE_CASE_SENSITIVE_INFO).cast(),
            size_of::<FILE_CASE_SENSITIVE_INFO>() as u32,
        )
    };
    match result {
        Ok(()) if info.Flags & 1 != 0 => Ok(OBJ_DONT_REPARSE),
        Ok(()) => Ok(OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE),
        Err(e) if matches!(e.code().0 as u32 & 0xffff, 1 | 50 | 87) => {
            Ok(OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE)
        }
        Err(e) => Err(win_error(e)),
    }
}
/// Exactly one child name, resolved relative to the already-open parent object.
/// OBJ_DONT_REPARSE is never removed on retry/failure. OPEN_REPARSE_POINT also
/// prevents normal final-component reparse processing. No absolute path reopen.
pub(super) fn open_child(parent: &File, name: &[u16], read: bool) -> Result<File, StorageError> {
    open_child_impl(parent, name, read, false)
}
fn open_child_impl(
    parent: &File,
    name: &[u16],
    read: bool,
    exact: bool,
) -> Result<File, StorageError> {
    validate_child_units(name)?;
    let mut units = name.to_vec();
    let length = u16::try_from(units.len() * 2).map_err(|_| {
        error(
            StorageErrorKind::InvalidReference,
            "child name exceeds UNICODE_STRING",
        )
    })?;
    let name = UNICODE_STRING {
        Length: length,
        MaximumLength: length,
        Buffer: PWSTR(units.as_mut_ptr()),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()),
        ObjectName: &name,
        Attributes: case_attributes(parent, exact)?,
        ..Default::default()
    };
    let mut handle = HANDLE::default();
    let mut status_block = IO_STATUS_BLOCK::default();
    // All referenced buffers/parent remain live during this synchronous call.
    // FILE_OPEN cannot create/truncate; no write/delete access is requested.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_READ_ATTRIBUTES
                | SYNCHRONIZE
                | if read {
                    FILE_READ_DATA
                } else {
                    FILE_ACCESS_RIGHTS(0)
                },
            &attributes,
            &mut status_block,
            None,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            None,
            0,
        )
    };
    if status == STATUS_REPARSE_POINT_ENCOUNTERED {
        return Err(error(
            StorageErrorKind::UnsafeLink,
            "native relative open encountered a reparse point",
        ));
    }
    if status.0 < 0 {
        return Err(StorageError::from(std::io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(error(
            StorageErrorKind::Io,
            "native relative open returned no handle",
        ));
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}
/// Acquire read/list rights through the same parent object, then verify the full
/// ID against the metadata handle. Only drive/share anchors use ReOpenFile;
/// their original handle came from CreateFileW, as that API requires.
pub(super) fn reopen_bound(handles: &[File], path: &Path) -> Result<File, StorageError> {
    let observed = handles.last().expect("absolute root");
    let file = if handles.len() > 1 {
        let name = path
            .file_name()
            .ok_or_else(|| {
                error(
                    StorageErrorKind::InvalidReference,
                    "missing child component",
                )
            })?
            .encode_wide()
            .collect::<Vec<_>>();
        open_child(&handles[handles.len() - 2], &name, true)?
    } else {
        let handle = unsafe {
            ReOpenFile(
                HANDLE(observed.as_raw_handle()),
                (FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS,
            )
        }
        .map_err(win_error)?;
        unsafe { File::from_raw_handle(handle.0) }
    };
    if identity(&file)? != identity(observed)? {
        return Err(error(
            StorageErrorKind::Changed,
            "handle-bound reopen identity changed",
        ));
    }
    Ok(file)
}
pub(super) fn directory_records(directory: &File) -> Result<Vec<DirectoryRecord>, StorageError> {
    const _: () = assert!(std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileName) == 88);
    const _: () = assert!(std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileId) == 72);
    const _: () = assert!(std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileNameLength) == 60);
    const _: () = assert!(std::mem::offset_of!(FILE_ID_EXTD_DIR_INFO, FileAttributes) == 56);
    // u64 storage satisfies the documented 8-byte record alignment.
    let mut buffer = vec![0u64; 8192];
    let mut entries = vec![];
    let mut first = true;
    loop {
        buffer.fill(0);
        let result = unsafe {
            GetFileInformationByHandleEx(
                HANDLE(directory.as_raw_handle()),
                if first {
                    FileIdExtdDirectoryRestartInfo
                } else {
                    FileIdExtdDirectoryInfo
                },
                buffer.as_mut_ptr().cast(),
                (buffer.len() * 8) as u32,
            )
        };
        first = false;
        match result {
            Err(e) if e.code().0 as u32 & 0xffff == 18 => break, // ERROR_NO_MORE_FILES
            Err(e) => return Err(win_error(e)), // no partial success or permissive fallback
            Ok(()) => {}
        }
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), buffer.len() * 8) };
        entries.extend(parse_directory_records(bytes)?);
    }
    Ok(entries)
}
pub(super) fn windows_children(
    backend: &Backend,
    path: &Path,
    handles: &[File],
) -> Result<Vec<EntryObservation>, StorageError> {
    let directory = reopen_bound(handles, path)?;
    if kind(&directory.metadata()?) != EntryKind::Directory {
        return Err(error(
            StorageErrorKind::UnsafeLink,
            "directory changed to reparse/non-directory",
        ));
    }
    let parent_identity = identity(&directory)?;
    let ObjectIdentity::Windows { volume, .. } = parent_identity else {
        unreachable!("Windows identity")
    };
    let mut result = vec![];
    for record in directory_records(&directory)? {
        let child_path = path.join(OsString::from_wide(&record.name)); // display/reference only, never opened
        let reference = StorageRef::WindowsPath {
            utf16: child_path.as_os_str().encode_wide().collect(),
        };
        let observed_id = ObjectIdentity::Windows {
            volume,
            file_id: record.file_id,
        };
        if record.attributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            result.push(EntryObservation {
                reference,
                identity: Some(observed_id),
                parent_identity: Some(parent_identity.clone()),
                name: String::from_utf16_lossy(&record.name),
                kind: EntryKind::LinkOrReparse,
                size: None,
                modified: None,
                capabilities: Capabilities::default(),
            });
            continue;
        }
        let child = open_child_impl(&directory, &record.name, false, true)?;
        if identity(&child)? != observed_id {
            return Err(error(
                StorageErrorKind::Changed,
                "enumerated child identity changed",
            ));
        }
        let observation = backend.observe(&reference, &[directory.try_clone()?, child])?;
        if observation.parent_identity != Some(parent_identity.clone()) {
            return Err(error(
                StorageErrorKind::Changed,
                "enumerated parent identity changed",
            ));
        }
        result.push(observation);
    }
    Ok(result)
}
