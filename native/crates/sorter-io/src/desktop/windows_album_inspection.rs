//! Fresh, read-only evidence for an Album source whose selected root may be gone.
//! The containing parent is private and can probe only one recorded native leaf.
use super::*;

/// A fresh host selection for inspecting one recorded original Album location.
/// Unlike mutation roots, the containing parent may be a local drive anchor.
/// No saved path/identity may be used as the host's selection evidence.
pub struct WindowsAlbumInspectionParentGrant {
    path: PathBuf,
    expected_identity: ObjectIdentity,
}

impl WindowsAlbumInspectionParentGrant {
    pub fn new(path: PathBuf, expected_identity: ObjectIdentity) -> Result<Self, MutationError> {
        validate_parent_path(&path)?;
        if !matches!(expected_identity, ObjectIdentity::Windows { .. }) {
            return Err(err(MutationErrorKind::Unsupported, "full Windows parent identity required"));
        }
        Ok(Self { path, expected_identity })
    }
}

/// Present objects are never converted into source-tree authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowsAlbumOriginalLocation {
    Absent,
    OriginalPresent(MutationObservation),
    ForeignPresent(MutationObservation),
}

/// Holds only read/attribute handles. There is no parent/root getter, arbitrary
/// child lookup, mutation method, cleanup conversion or deserialization route.
pub struct WindowsAlbumOriginalLocationInspection {
    ancestors: Vec<File>,
    original: MutationObservation,
}

impl WindowsAlbumOriginalLocationInspection {
    pub fn open_observed(
        grant: WindowsAlbumInspectionParentGrant,
        original: MutationObservation,
        cancel: &AtomicBool,
        observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<Self, MutationError> {
        cancelled(cancel)?;
        validate_parent_path(&grant.path)?;
        units(&original.actual_name)?;
        if original.size.is_some()
            || original.parent_identity != grant.expected_identity
            || original.identity == original.parent_identity
            || !matches!((&original.identity, &original.parent_identity),
                (ObjectIdentity::Windows { volume: source, .. }, ObjectIdentity::Windows { volume: parent, .. }) if source == parent)
        {
            return Err(err(MutationErrorKind::StalePlan, "recorded original root does not match the selected parent"));
        }
        let ancestors = read_parent_chain(&grant.path, cancel, observe)?;
        let parent = ancestors.last().ok_or_else(|| err(MutationErrorKind::Io, "parent chain is empty"))?;
        let actual = inspection_probe(cancel, observe, || identity(parent).map_err(read_error))?;
        if actual != grant.expected_identity {
            return Err(err(MutationErrorKind::StalePlan, "selected original parent was replaced"));
        }
        require_local_ntfs(parent, cancel, observe)?;
        let parent_reference = observed_reference(parent, cancel, observe)?;
        if child_reference(&parent_reference, &original.actual_name)? != original.reference {
            return Err(err(MutationErrorKind::StalePlan, "recorded native leaf/reference does not match the held original parent"));
        }
        cancelled(cancel)?;
        Ok(Self { ancestors, original })
    }

    pub fn inspect_original_observed(
        &self,
        cancel: &AtomicBool,
        observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsAlbumOriginalLocation, MutationError> {
        cancelled(cancel)?;
        let parent = self.validate_parent_observed(cancel, observe)?;
        // Only this exact native leaf can be opened. A short selected alias is
        // never substituted for the recorded actual_name, even when absent.
        let result = inspection_probe(cancel, observe, || {
            match super::super::open_child(parent, units(&self.original.actual_name)?, false) {
                Ok(file) => Ok(Some(file)),
                Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(None),
                Err(error) => Err(read_error(error)),
            }
        })?;
        let actual = result.as_ref().map(|file| {
            read_observation(file, &self.original.parent_identity, cancel, observe)
        }).transpose()?;
        // An open directory handle is not a universal namespace-exclusion
        // guarantee. Recheck after the leaf probe, including its absent branch.
        self.validate_parent_observed(cancel, observe)?;
        cancelled(cancel)?;
        Ok(match actual {
            None => WindowsAlbumOriginalLocation::Absent,
            Some(actual) if actual.size.is_none() && actual == self.original => {
                WindowsAlbumOriginalLocation::OriginalPresent(actual)
            }
            Some(actual) => WindowsAlbumOriginalLocation::ForeignPresent(actual),
        })
    }

    fn validate_parent_observed(
        &self,
        cancel: &AtomicBool,
        observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<&File, MutationError> {
        for ancestor in &self.ancestors {
            inspection_probe(cancel, observe, || ensure_directory(ancestor))?;
        }
        let parent = self.ancestors.last().ok_or_else(|| err(MutationErrorKind::Io, "original parent is not held"))?;
        let parent_identity = inspection_probe(cancel, observe, || identity(parent).map_err(read_error))?;
        let parent_reference = observed_reference(parent, cancel, observe)?;
        if parent_identity != self.original.parent_identity
            || child_reference(&parent_reference, &self.original.actual_name)? != self.original.reference
        {
            return Err(err(MutationErrorKind::StalePlan, "held original parent identity or observed path changed"));
        }
        cancelled(cancel)?;
        Ok(parent)
    }
}

fn validate_parent_path(path: &Path) -> Result<(), MutationError> {
    super::super::validate_path(path).map_err(read_error)?;
    if !matches!(path.components().next(), Some(Component::Prefix(prefix))
        if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
    {
        return Err(err(MutationErrorKind::Unsupported, "only a local drive-backed original parent is supported"));
    }
    if path.as_os_str().encode_wide().count() > 32_767 {
        return Err(err(MutationErrorKind::Unsupported, "original parent path exceeds the supported bound"));
    }
    for component in path.components() {
        if let Component::Normal(name) = component {
            units(&MutationName::Windows(name.encode_wide().collect()))?;
        }
    }
    Ok(())
}

fn read_parent_chain(
    path: &Path,
    cancel: &AtomicBool,
    observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
) -> Result<Vec<File>, MutationError> {
    let paths = path.ancestors().collect::<Vec<_>>();
    let mut handles = Vec::<File>::new();
    for path in paths.iter().rev() {
        let file = inspection_probe(cancel, observe, || {
            if let Some(parent) = handles.last() {
                let leaf = path.file_name().ok_or_else(|| err(MutationErrorKind::InvalidName, "original parent component missing"))?;
                super::super::open_child(parent, &leaf.encode_wide().collect::<Vec<_>>(), false).map_err(read_error)
            } else {
                // The only absolute open is the existing drive anchor. Both
                // native read helpers refuse write/delete/create access.
                super::super::open_handle(path, false).map_err(read_error)
            }
        })?;
        inspection_probe(cancel, observe, || ensure_directory(&file))?;
        handles.push(file);
    }
    cancelled(cancel)?;
    Ok(handles)
}

fn require_local_ntfs(
    file: &File,
    cancel: &AtomicBool,
    observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
) -> Result<(), MutationError> {
    inspection_probe(cancel, observe, || {
        let mut local = [0u16; 32_768];
        let count = unsafe {
            GetFinalPathNameByHandleW(HANDLE(file.as_raw_handle()), &mut local, VOLUME_NAME_GUID)
        } as usize;
        let prefix = r"\\?\Volume{".encode_utf16().collect::<Vec<_>>();
        if count == 0 || count >= local.len() || !local[..count].starts_with(&prefix) {
            return Err(err(MutationErrorKind::Unsupported, "original parent has no local volume GUID"));
        }
        Ok(())
    })?;
    inspection_probe(cancel, observe, || {
        let mut filesystem = [0u16; 32];
        unsafe {
            GetVolumeInformationByHandleW(HANDLE(file.as_raw_handle()), None, None, None, None, Some(&mut filesystem))
        }.map_err(win_error)?;
        if filesystem.iter().copied().take_while(|unit| *unit != 0).collect::<Vec<_>>() != "NTFS".encode_utf16().collect::<Vec<_>>() {
            return Err(err(MutationErrorKind::Unsupported, "original parent requires NTFS"));
        }
        Ok(())
    })
}

fn observed_reference(
    file: &File,
    cancel: &AtomicBool,
    observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
) -> Result<StorageRef, MutationError> {
    inspection_probe(cancel, observe, || {
        let mut path = vec![0u16; 32_768];
        let count = unsafe {
            GetFinalPathNameByHandleW(HANDLE(file.as_raw_handle()), &mut path,
                GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0))
        } as usize;
        if count == 0 { return Err(io_error(io::Error::last_os_error())); }
        if count >= path.len() { return Err(err(MutationErrorKind::Unsupported, "observed original path exceeds bound")); }
        path.truncate(count);
        Ok(StorageRef::WindowsPath { utf16: path })
    })
}

fn child_reference(parent: &StorageRef, name: &MutationName) -> Result<StorageRef, MutationError> {
    let StorageRef::WindowsPath { utf16 } = parent else {
        return Err(err(MutationErrorKind::InvalidName, "original parent must be a Windows reference"));
    };
    let mut path = PathBuf::from(std::ffi::OsString::from_wide(utf16));
    path.push(std::ffi::OsString::from_wide(units(name)?));
    Ok(StorageRef::WindowsPath { utf16: path.as_os_str().encode_wide().collect() })
}

fn read_observation(
    file: &File,
    parent_identity: &ObjectIdentity,
    cancel: &AtomicBool,
    observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
) -> Result<MutationObservation, MutationError> {
    let reference = observed_reference(file, cancel, observe)?;
    let StorageRef::WindowsPath { utf16 } = &reference else { unreachable!() };
    let path = PathBuf::from(std::ffi::OsString::from_wide(utf16));
    let actual_name = MutationName::Windows(path.file_name().ok_or_else(|| err(MutationErrorKind::InvalidName, "observed original leaf missing"))?.encode_wide().collect());
    let metadata = inspection_probe(cancel, observe, || file.metadata().map_err(io_error))?;
    if kind(&metadata) == EntryKind::LinkOrReparse {
        return Err(err(MutationErrorKind::UnsafeReparse, "original leaf is a reparse point"));
    }
    let identity = inspection_probe(cancel, observe, || identity(file).map_err(read_error))?;
    Ok(MutationObservation { reference, identity, parent_identity: parent_identity.clone(), actual_name, size: (!metadata.is_dir()).then_some(metadata.len()) })
}

#[cfg(test)]
#[path = "windows_album_inspection_tests.rs"]
mod tests;

#[path = "windows_album_inspection_storage.rs"]
mod storage;
pub use storage::{WindowsAlbumInspectionDirectory, WindowsAlbumInspectionJournal,
    WindowsAlbumInspectionTarget, WindowsAlbumJournalInspection, WindowsAlbumJournalInspectionInputs,
    WindowsAlbumReadInspection, WindowsAlbumReadInspectionInputs, WindowsAlbumPresentJournalInspectionInputs};
