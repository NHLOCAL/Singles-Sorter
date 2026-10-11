//! Experimental primitives with explicit synthetic or selected-root authority.
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
    path::{Component, Path, PathBuf},
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
            FILETIME, HANDLE, HLOCAL, LocalFree, OBJ_CASE_INSENSITIVE, OBJ_DONT_REPARSE, RtlNtStatusToDosError,
            STATUS_REPARSE_POINT_ENCOUNTERED, UNICODE_STRING,
        },
        Security::{
            ACL, ATTRIBUTE_SECURITY_INFORMATION, DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION,
            LABEL_SECURITY_INFORMATION, SCOPE_SECURITY_INFORMATION,
            GetLengthSid, GetSecurityDescriptorControl, GetSecurityDescriptorSacl, IsValidAcl, IsValidSid,
            OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
            Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
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
/// Authority origin is separate from serialized planning evidence. Executors must
/// bind this domain as well as the exact selected roots before dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsMutationDomain {
    OwnedSyntheticTree,
    SelectedRoots,
    SelectedRepair,
}

/// A trusted native host's explicit grant for one selected directory.
///
/// This type intentionally implements neither Deserialize, Serialize nor Clone.
/// The host must obtain the path and expected full Windows identity from its
/// selection/consent flow, never turn a serialized plan into a grant. Constructing
/// a grant does not open or mutate the path; consumption pins and revalidates it.
pub struct WindowsSelectedRootGrant {
    path: PathBuf,
    expected_identity: ObjectIdentity,
}
impl WindowsSelectedRootGrant {
    pub fn new(path: PathBuf, expected_identity: ObjectIdentity) -> Result<Self, MutationError> {
        validate_root_path(&path)?;
        if !matches!(expected_identity, ObjectIdentity::Windows { .. }) {
            return Err(err(
                MutationErrorKind::Unsupported,
                "full Windows root identity required",
            ));
        }
        Ok(Self {
            path,
            expected_identity,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RootRole {
    Synthetic,
    Source,
    Target,
    Journal,
    Repair,
}
// Only synthetic ownership can run TempDir cleanup. Selected roots hold no
// deletion/cleanup object, including when construction fails partway through.
enum RootOwnership {
    Synthetic { _tree: tempfile::TempDir },
    Selected,
}
struct Root {
    path: PathBuf,
    role: RootRole,
    _ancestors: Vec<File>,
    directory: File,
    _ownership: RootOwnership,
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
fn validate_root_path(path: &Path) -> Result<(), MutationError> {
    use std::path::Prefix;
    super::validate_path(path).map_err(read_error)?;
    if !matches!(path.components().next(),Some(Component::Prefix(p)) if matches!(p.kind(),Prefix::Disk(_)|Prefix::VerbatimDisk(_)))
    {
        return Err(err(
            MutationErrorKind::Unsupported,
            "only local drive-backed roots",
        ));
    }
    for component in path.components() {
        if let Component::Normal(name) = component {
            units(&MutationName::Windows(name.encode_wide().collect()))?;
        }
    }
    if path.file_name().is_none() {
        return Err(err(
            MutationErrorKind::Unsupported,
            "drive anchors cannot be mutation roots",
        ));
    }
    Ok(())
}
fn pin_root(
    path: PathBuf,
    expected: Option<&ObjectIdentity>,
    role: RootRole,
    ownership: RootOwnership,
) -> Result<Arc<Root>, MutationError> {
    validate_root_path(&path)?;
    let mut ancestors = chain(&path).map_err(read_error)?;
    let observed = ancestors
        .pop()
        .ok_or_else(|| err(MutationErrorKind::Io, "no root"))?;
    for ancestor in &ancestors {
        ensure_directory(ancestor)?;
    }
    let root_name = path
        .file_name()
        .ok_or_else(|| err(MutationErrorKind::InvalidName, "root leaf missing"))?
        .encode_wide()
        .collect::<Vec<_>>();
    let directory = open_directory(
        ancestors.last().ok_or_else(|| {
            err(
                MutationErrorKind::Unsupported,
                "root anchor cannot be a mutation root",
            )
        })?,
        &root_name,
    )?;
    let observed_identity = identity(&directory).map_err(read_error)?;
    if observed_identity != identity(&observed).map_err(read_error)?
        || expected.is_some_and(|expected| expected != &observed_identity)
    {
        return Err(err(
            MutationErrorKind::StalePlan,
            "selected root identity changed",
        ));
    }
    ensure_directory(&directory)?;
    // A handle-derived volume GUID path is unavailable for SMB; a mapped drive
    // letter alone is never locality evidence.
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
            "only local NTFS primitives are implemented",
        ));
    }
    Ok(Arc::new(Root {
        path,
        role,
        _ancestors: ancestors,
        directory,
        _ownership: ownership,
    }))
}
fn ensure_disjoint_roots(a: &Root, b: &Root) -> Result<(), MutationError> {
    let a_id = identity(&a.directory).map_err(read_error)?;
    let b_id = identity(&b.directory).map_err(read_error)?;
    for (root, other_id) in [(a, &b_id), (b, &a_id)] {
        // Compare each root against the other's full held ancestry. Shared
        // ancestors are expected and never become an authorization scope.
        for ancestor in root
            ._ancestors
            .iter()
            .chain(std::iter::once(&root.directory))
        {
            if &identity(ancestor).map_err(read_error)? == other_id {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "selected roots must be distinct and non-overlapping",
                ));
            }
        }
    }
    Ok(())
}
pub struct WindowsMutationStorage {
    roots: Vec<Arc<Root>>,
    domain: WindowsMutationDomain,
}
impl WindowsMutationStorage {
    /// Consume a fresh caller-owned synthetic TempDir, retaining fixture cleanup.
    pub fn for_owned_synthetic_tree(tree: tempfile::TempDir) -> Result<Self, MutationError> {
        let path = tree.path().to_owned();
        let root = pin_root(
            path,
            None,
            RootRole::Synthetic,
            RootOwnership::Synthetic { _tree: tree },
        )?;
        Ok(Self {
            roots: vec![root],
            domain: WindowsMutationDomain::OwnedSyntheticTree,
        })
    }
    /// Independently pin exactly the source, target and application-journal
    /// roots. No common ancestor is authorized. Dropping this capability never
    /// deletes any selected directory or file. This is not restart/replay consent.
    pub fn for_selected_roots(
        source: WindowsSelectedRootGrant,
        target: WindowsSelectedRootGrant,
        journal: WindowsSelectedRootGrant,
    ) -> Result<Self, MutationError> {
        let mut roots: Vec<Arc<Root>> = Vec::new();
        for (grant, role) in [
            (source, RootRole::Source),
            (target, RootRole::Target),
            (journal, RootRole::Journal),
        ] {
            let root = pin_root(
                grant.path,
                Some(&grant.expected_identity),
                role,
                RootOwnership::Selected,
            )?;
            for previous in &roots {
                ensure_disjoint_roots(previous, &root)?;
            }
            roots.push(root);
        }
        Ok(Self {
            roots,
            domain: WindowsMutationDomain::SelectedRoots,
        })
    }
    /// Explicit in-place repair authority: the selected source may receive only
    /// staged files and retained originals; application journal remains disjoint.
    pub fn for_selected_repair_roots(source: WindowsSelectedRootGrant, journal: WindowsSelectedRootGrant) -> Result<Self, MutationError> {
        let source=pin_root(source.path,Some(&source.expected_identity),RootRole::Repair,RootOwnership::Selected)?;
        let journal=pin_root(journal.path,Some(&journal.expected_identity),RootRole::Journal,RootOwnership::Selected)?;
        ensure_disjoint_roots(&source,&journal)?;
        Ok(Self{roots:vec![source,journal],domain:WindowsMutationDomain::SelectedRepair})
    }
    pub fn selected_repair_reference(&self)->Result<StorageRef,MutationError>{self.selected_reference(RootRole::Repair)}
    pub fn selected_repair_root(&self)->Result<WindowsParent,MutationError>{self.parent_under(self.selected_root(RootRole::Repair)?,Path::new(""))}
    pub fn domain(&self) -> WindowsMutationDomain {
        self.domain
    }
    fn selected_root(&self, role: RootRole) -> Result<&Arc<Root>, MutationError> {
        self.roots
            .iter()
            .find(|root| root.role == role)
            .ok_or_else(|| {
                err(
                    MutationErrorKind::Unsupported,
                    "selected-root capability required",
                )
            })
    }
    fn selected_reference(&self, role: RootRole) -> Result<StorageRef, MutationError> {
        Ok(StorageRef::WindowsPath {
            utf16: self
                .selected_root(role)?
                .path
                .as_os_str()
                .encode_wide()
                .collect(),
        })
    }
    pub fn selected_source_reference(&self) -> Result<StorageRef, MutationError> {
        self.selected_reference(RootRole::Source)
    }
    pub fn selected_target_reference(&self) -> Result<StorageRef, MutationError> {
        self.selected_reference(RootRole::Target)
    }
    pub fn selected_journal_reference(&self) -> Result<StorageRef, MutationError> {
        self.selected_reference(RootRole::Journal)
    }
    pub fn selected_source_root(&self) -> Result<WindowsParent, MutationError> {
        self.parent_under(self.selected_root(RootRole::Source)?, Path::new(""))
    }
    pub fn selected_target_root(&self) -> Result<WindowsParent, MutationError> {
        self.parent_under(self.selected_root(RootRole::Target)?, Path::new(""))
    }
    pub fn selected_journal_root(&self) -> Result<WindowsParent, MutationError> {
        self.parent_under(self.selected_root(RootRole::Journal)?, Path::new(""))
    }
    /// Fixture-only convenience. Selected scopes require a role or full reference.
    pub fn parent(&self, relative: &Path) -> Result<WindowsParent, MutationError> {
        if self.domain != WindowsMutationDomain::OwnedSyntheticTree {
            return Err(err(
                MutationErrorKind::Unsupported,
                "selected roots require an explicit root role or reference",
            ));
        }
        self.parent_under(&self.roots[0], relative)
    }
    fn parent_under(
        &self,
        root: &Arc<Root>,
        relative: &Path,
    ) -> Result<WindowsParent, MutationError> {
        let raw = relative.as_os_str().encode_wide().collect::<Vec<_>>();
        if raw
            .split(|u| matches!(*u, 47 | 92))
            .any(|part| part == [46] || part == [46, 46])
        {
            return Err(err(
                MutationErrorKind::InvalidName,
                "dot components are not accepted",
            ));
        }
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
            let parent = ancestors.last().unwrap_or(&root.directory);
            let child = open_directory(parent, name)?;
            ensure_directory(&child)?;
            ancestors.push(child);
        }
        let id = identity(ancestors.last().unwrap_or(&root.directory)).map_err(read_error)?;
        let parent = WindowsParent {
            root: root.clone(),
            ancestors: Arc::new(ancestors),
            identity: id,
        };
        self.check_parent(&parent)?;
        Ok(parent)
    }
    /// Resolve a directory beneath one explicitly authorized root, using held
    /// ancestry. A serialized Windows path is evidence, never an ownership grant.
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
        let path = PathBuf::from(std::ffi::OsString::from_wide(utf16));
        super::validate_path(&path).map_err(read_error)?;
        for root in &self.roots {
            if let Ok(relative) = path.strip_prefix(&root.path) {
                return self.parent_under(root, relative);
            }
        }
        Err(err(
            MutationErrorKind::StalePlan,
            "reference outside authorized roots",
        ))
    }
    fn check_parent(&self, parent: &WindowsParent) -> Result<(), MutationError> {
        if !self
            .roots
            .iter()
            .any(|root| Arc::ptr_eq(root, &parent.root))
        {
            return Err(err(
                MutationErrorKind::StalePlan,
                "parent belongs to another scope",
            ));
        }
        ensure_directory(&parent.root.directory)?;
        for f in parent.root._ancestors.iter().chain(parent.ancestors.iter()) {
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
    fn check_role(&self, parent: &WindowsParent, role: RootRole) -> Result<(), MutationError> {
        self.check_parent(parent)?;
        let repair_role = self.domain == WindowsMutationDomain::SelectedRepair && parent.root.role == RootRole::Repair && matches!(role, RootRole::Source | RootRole::Target);
        if parent.root.role != RootRole::Synthetic && parent.root.role != role && !repair_role {
            return Err(err(
                MutationErrorKind::StalePlan,
                "parent has a different selected-root role",
            ));
        }
        Ok(())
    }
    /// Read-only source/target evidence, never consent. Target observations are
    /// permitted for reconciliation; leasing for mutation remains source-only.
    pub fn observe_source(
        &mut self,
        parent: &WindowsParent,
        name: &MutationName,
    ) -> Result<ExpectedSource, MutationError> {
        self.check_parent(parent)?;
        if parent.root.role == RootRole::Journal {
            return Err(err(
                MutationErrorKind::StalePlan,
                "journal root is not data authority",
            ));
        }
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
impl WindowsMutationStorage {
    /// Exclusively create one destination directory under an authorized target
    /// parent. The executor must persist its intent before calling this primitive.
    /// FILE_CREATE never adopts an existing directory. No directory-flush or
    /// namespace durability claim is made. A post-dispatch error requires
    /// reconciliation; neither retry nor automatic deletion is authorized.
    pub fn create_destination_directory(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsParent> {
        self.create_destination_directory_with(parent, name, cancel, || Ok(()))
    }
    fn create_destination_directory_with(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
        after_create: impl FnOnce() -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsParent> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.check_role(parent, RootRole::Target)?;
            units(name)?;
            // Retain the exact already-pinned ancestry. All potentially failing
            // handle duplication happens before namespace dispatch.
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
            Ok(ancestors) => ancestors,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        let file = match open_directory_handle(
            parent.handle(),
            units(name).expect("validated child name"),
            true,
        ) {
            Ok(file) => file,
            Err(error) if error.kind == MutationErrorKind::UnknownOutcome => {
                return MutationOutcome::PendingUnknown(error);
            }
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        let mut observation = None;
        let verified = (|| {
            after_create()?;
            ensure_directory(&file)?;
            let observed = observe(&file, parent)?;
            let matches = observed.actual_name == *name;
            observation = Some(observed);
            if !matches {
                return Err(err(
                    MutationErrorKind::VerificationFailed,
                    "created directory name changed",
                ));
            }
            self.check_role(parent, RootRole::Target)?;
            cancelled(cancel)?;
            Ok(())
        })();
        if let Err(error) = verified {
            let mut unknown = err(
                MutationErrorKind::UnknownOutcome,
                &format!("directory creation dispatched; reconcile without replay: {error:?}"),
            );
            if let Some(observed) = observation.or_else(|| observe(&file, parent).ok()) {
                unknown = unknown.with_observation(observed);
            }
            return MutationOutcome::PendingUnknown(unknown);
        }
        let id = observation
            .expect("verified directory observation")
            .identity;
        ancestors.push(file);
        MutationOutcome::Applied(WindowsParent {
            ancestors: Arc::new(ancestors),
            identity: id,
            root: parent.root.clone(),
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

impl WindowsJournalLocation {
    pub fn identity(&self) -> &ObjectIdentity {
        &self.identity
    }
    pub fn name(&self) -> &MutationName {
        &self.name
    }
}

/// Evidence only that the OS accepted a file-buffer flush request.
/// This is not a namespace barrier or a mutation durability capability.
#[derive(Debug)]
#[must_use]
pub struct FileBufferFlushAcknowledgment {
    _private: (),
}

/// Private append-only journal bytes on a separately authorized NTFS journal root.
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
        self.check_role(parent, RootRole::Journal)?;
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

    /// Read an existing journal after the host explicitly regrants all selected
    /// roots. The recorded identity is evidence, not authority: only this
    /// capability's application-journal root is used. No writable reopen or
    /// automatic recovery/replay is exposed.
    pub fn read_selected_journal(
        &self,
        name: &MutationName,
        expected_identity: &ObjectIdentity,
    ) -> Result<Vec<u8>, MutationError> {
        if !matches!(expected_identity, ObjectIdentity::Windows { .. }) {
            return Err(err(
                MutationErrorKind::Unsupported,
                "full Windows journal identity required",
            ));
        }
        let parent = self.selected_journal_root()?;
        self.read_journal(&WindowsJournalLocation {
            parent,
            name: name.clone(),
            identity: expected_identity.clone(),
        })
    }

    /// Read-only close/reopen inspection, using previously captured identity and
    /// held ancestry. A supplied filename is never sufficient recovery authority.
    /// This is not a production restart-root authorization API.
    pub fn read_journal(
        &self,
        location: &WindowsJournalLocation,
    ) -> Result<Vec<u8>, MutationError> {
        self.check_role(&location.parent, RootRole::Journal)?;
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
        read_journal_bytes(&mut file, location.parent.root.role != RootRole::Synthetic)
    }
}

fn read_journal_bytes(file: &mut File, selected: bool) -> Result<Vec<u8>, MutationError> {
    const LIMIT: usize = 64 * 1024 * 1024;
    let length = file.metadata().map_err(io_error)?.len();
    if selected && length > LIMIT as u64 {
        return Err(err(MutationErrorKind::Unsupported, "selected journal exceeds 64 MiB read limit"));
    }
    let mut bytes = Vec::new();
    if selected {
        bytes.try_reserve_exact(length as usize).map_err(|error| err(MutationErrorKind::Io, &format!("bounded journal allocation: {error}")))?;
        // Bound allocation as well as reads, including if size changes after the
        // native length observation. A one-byte stack probe detects excess data.
        let mut chunk = [0u8; 64 * 1024];
        loop {
            let remaining = LIMIT - bytes.len();
            if remaining == 0 {
                let mut probe = [0u8; 1];
                if file.read(&mut probe).map_err(io_error)? != 0 {
                    return Err(err(MutationErrorKind::Unsupported, "selected journal exceeds 64 MiB read limit"));
                }
                break;
            }
            let amount = remaining.min(chunk.len());
            let n = file.read(&mut chunk[..amount]).map_err(io_error)?;
            if n == 0 { break; }
            bytes.try_reserve_exact(n).map_err(|error| err(MutationErrorKind::Io, &format!("bounded journal allocation: {error}")))?;
            bytes.extend_from_slice(&chunk[..n]);
        }
    } else {
        file.read_to_end(&mut bytes).map_err(io_error)?;
    }
    Ok(bytes)
}

impl WindowsJournalFile {
    pub fn location(&self) -> WindowsJournalLocation {
        self.location.clone()
    }

    pub fn read_all(&mut self) -> Result<Vec<u8>, MutationError> {
        let result = (|| {
            self.file.seek(SeekFrom::Start(0)).map_err(io_error)?;
            read_journal_bytes(&mut self.file, self.location.parent.root.role != RootRole::Synthetic)
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
    let file = open_directory_handle(parent, name, false)?;
    ensure_directory(&file)?;
    Ok(file)
}
fn open_directory_handle(parent: &File, name: &[u16], create: bool) -> Result<File, MutationError> {
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
            if create { FILE_CREATE } else { FILE_OPEN },
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
            MutationErrorKind::UnknownOutcome,
            "directory open returned no handle",
        ));
    }
    Ok(unsafe { File::from_raw_handle(handle.0) })
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
    repair: Option<RepairPreservation>,
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
    repair: Option<RepairTempState>,
}
pub struct WindowsPublished {
    file: File,
    parent: WindowsParent,
    observation: MutationObservation,
    sync_poisoned: bool,
    repair: Option<RepairPublication>,
}
impl WindowsPublished {
    pub fn observation(&self) -> &MutationObservation { &self.observation }
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
        self.check_role(&published.parent, RootRole::Target)?;
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
impl Seek for WindowsSourceLease {
    fn seek(&mut self, position:SeekFrom)->io::Result<u64>{
        self.file.as_mut().ok_or_else(||io::Error::other("lease consumed"))?.seek(position)
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
        let file = self.file.as_mut().ok_or_else(|| io::Error::other("temp consumed"))?;
        if let Some(state) = &mut self.repair {
            if state.finalized_write_time.is_some() || state.poisoned {
                return Err(io::Error::other("repair temp finalized or poisoned"));
            }
            // A main-stream share-denying handle is NOT an ACL/ADS lock. Check
            // each write as well as the create/finalize/publication boundaries.
            let checked = state.original.check_original(&state.held_original)
                .and_then(|_| check_repair_representation(file))
                .and_then(|_| state.original.check_security(file));
            if let Err(error) = checked {
                state.poisoned = true;
                return Err(io::Error::other(format!("repair preservation: {error:?}")));
            }
        }
        file.write(buf)
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
            self.check_role(parent, RootRole::Target)?;
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
            repair: None,
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
            self.check_role(&temp.parent, RootRole::Target)?;
            self.check_role(parent, RootRole::Target)?;
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
            self.check_role(parent, RootRole::Target)?;
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
        if self.domain==WindowsMutationDomain::SelectedRepair{return MutationOutcome::NoEffect(err(MutationErrorKind::Unsupported,"repair domain never unlinks originals"));}
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
        self.check_role(&published.parent, RootRole::Target)?;
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
        self.check_role(parent, RootRole::Source)?;
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
            repair: None,
        })
    }
    fn validate_source(&self, source: &WindowsSourceLease) -> Result<(), MutationError> {
        self.check_role(&source.parent, RootRole::Source)?;
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
        repair: None,
    })
}

/// Rename-only guard for explicitly authorized NTFS mutation roots.
///
/// Its narrower contract is a native write-through source open, checked preflush,
/// same-volume no-replace rename, and checked flush/full verification on the very
/// same held object. It does not assert generic namespace or power-loss durability.
/// In particular, `durable_flush` remains Unknown and this type cannot publish
/// copies, retire sources, or create host consent from serialized planning facts.
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
            self.owned.check_role(&source.parent, RootRole::Source)?;
            self.owned.check_role(parent, RootRole::Target)?;
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
            self.owned.check_role(parent, RootRole::Target)?;
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

#[cfg(test)]
mod selected_directory_uncertainty_tests {
    use super::*;

    #[test]
    fn directory_post_dispatch_failure_is_unknown_and_never_retried_or_removed() {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_owned();
        let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let parent = storage.parent(Path::new("")).unwrap();
        let name = MutationName::Windows("created".encode_utf16().collect());
        let result = storage.create_destination_directory_with(
            &parent,
            &name,
            &AtomicBool::new(false),
            || {
                Err(err(
                    MutationErrorKind::Cancelled,
                    "injected cancellation after dispatch",
                ))
            },
        );
        assert!(
            matches!(result, MutationOutcome::PendingUnknown(error) if error.kind == MutationErrorKind::UnknownOutcome && error.observed.is_some())
        );
        assert!(path.join("created").is_dir());
        assert!(
            matches!(storage.create_destination_directory(&parent, &name, &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::StalePlan)
        );
    }
}

// Repair metadata is capability-local evidence, never deserialized authority.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairSecurity {
    owner: Vec<u8>,
    group: Vec<u8>,
    dacl: Vec<u8>,
    // Mandatory integrity label, resource claims and Central Access Policy ACEs
    // are access controls too; READ_CONTROL suffices to query these portions.
    access_sacl: Option<Vec<u8>>,
    control: u16,
}

struct SecurityAllocation(PSECURITY_DESCRIPTOR);
impl Drop for SecurityAllocation {
    fn drop(&mut self) {
        if !self.0.0.is_null() {
            unsafe { let _ = LocalFree(Some(HLOCAL(self.0.0))); }
        }
    }
}

fn repair_security(file: &File) -> Result<RepairSecurity, MutationError> {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let mut owner = PSID::default();
    let mut group = PSID::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut access_sacl: *mut ACL = std::ptr::null_mut();
    let result = unsafe {
        GetSecurityInfo(
            HANDLE(file.as_raw_handle()), SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION
                | LABEL_SECURITY_INFORMATION | ATTRIBUTE_SECURITY_INFORMATION | SCOPE_SECURITY_INFORMATION,
            Some(&mut owner), Some(&mut group), Some(&mut dacl), None,
            Some(&mut descriptor),
        )
    };
    let allocation = SecurityAllocation(descriptor);
    if result.0 != 0 {
        return Err(io_error(io::Error::from_raw_os_error(result.0 as i32)));
    }
    if allocation.0.0.is_null() || owner.0.is_null() || group.0.is_null() || dacl.is_null() {
        return Err(err(MutationErrorKind::Unsupported, "repair requires owner, group and a non-null DACL"));
    }
    let mut control = 0u16;
    let mut revision = 0u32;
    unsafe { GetSecurityDescriptorControl(allocation.0, &mut control, &mut revision) }.map_err(win_error)?;
    let mut access_present = Default::default();
    let mut access_defaulted = Default::default();
    unsafe { GetSecurityDescriptorSacl(allocation.0, &mut access_present, &mut access_sacl, &mut access_defaulted) }.map_err(win_error)?;
    // Compare exact SID and ACL bytes, including ACE order and inheritance flags.
    // This is conservative equivalence (false negatives are safe), not an access
    // check or a claim that ACL/ADS changes are excluded by a main-stream lease.
    if !unsafe { IsValidSid(owner) }.as_bool()
        || !unsafe { IsValidSid(group) }.as_bool()
        || !unsafe { IsValidAcl(dacl) }.as_bool()
        || (!access_sacl.is_null() && !unsafe { IsValidAcl(access_sacl) }.as_bool())
        || control & 0x0004 == 0
    {
        return Err(err(MutationErrorKind::Unsupported, "unrecognized repair security descriptor"));
    }
    Ok(RepairSecurity {
        owner: unsafe { std::slice::from_raw_parts(owner.0.cast::<u8>(), GetLengthSid(owner) as usize) }.to_vec(),
        group: unsafe { std::slice::from_raw_parts(group.0.cast::<u8>(), GetLengthSid(group) as usize) }.to_vec(),
        dacl: unsafe { std::slice::from_raw_parts(dacl.cast::<u8>(), (*dacl).AclSize as usize) }.to_vec(),
        access_sacl: if access_sacl.is_null() { None } else {
            Some(unsafe { std::slice::from_raw_parts(access_sacl.cast::<u8>(), (*access_sacl).AclSize as usize) }.to_vec())
        },
        // OWNER/GROUP_DEFAULTED; DACL and access-SACL PRESENT/DEFAULTED,
        // AUTO_INHERIT_REQ, AUTO_INHERITED and PROTECTED. Audit ACEs are not read.
        control: control & 0x3f3f,
    })
}

fn repair_basic(file: &File) -> Result<FILE_BASIC_INFO, MutationError> {
    let mut info = FILE_BASIC_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(HANDLE(file.as_raw_handle()), FileBasicInfo,
            (&mut info as *mut FILE_BASIC_INFO).cast(), size_of::<FILE_BASIC_INFO>() as u32)
    }.map_err(win_error)?;
    Ok(info)
}

fn repair_attributes(attributes: u32) -> Result<u32, MutationError> {
    // The initial ordinary-file domain deliberately rejects readonly, EFS,
    // sparse, compressed, offline/recall, integrity and every unrecognized bit.
    let ordinary = FILE_ATTRIBUTE_NORMAL.0 | FILE_ATTRIBUTE_ARCHIVE.0
        | FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0 | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0;
    if attributes & !ordinary != 0 {
        return Err(err(MutationErrorKind::Unsupported, "repair cannot preserve this file representation or attribute"));
    }
    Ok(attributes & !FILE_ATTRIBUTE_NORMAL.0)
}

fn only_unnamed_stream(bytes: &[u8]) -> Result<(), MutationError> {
    let invalid = || err(MutationErrorKind::Unsupported, "repair stream inventory is unsupported or malformed");
    let header = std::mem::offset_of!(FILE_STREAM_INFO, StreamName);
    let mut offset = 0usize;
    let mut count = 0usize;
    loop {
        if offset.checked_add(header).is_none_or(|end| end > bytes.len()) { return Err(invalid()); }
        let next = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + header;
        let end = start.checked_add(length).filter(|end| *end <= bytes.len()).ok_or_else(invalid)?;
        if length % 2 != 0 { return Err(invalid()); }
        let name: Vec<u16> = bytes[start..end].chunks_exact(2).map(|u| u16::from_le_bytes([u[0], u[1]])).collect();
        if name != "::$DATA".encode_utf16().collect::<Vec<_>>() || count != 0 {
            return Err(err(MutationErrorKind::Unsupported, "repair with named or multiple data streams is unsupported"));
        }
        count += 1;
        if next == 0 { return Ok(()); }
        if next % 8 != 0 || next < header + length { return Err(invalid()); }
        offset = offset.checked_add(next).ok_or_else(invalid)?;
    }
}

fn check_repair_streams(file: &File) -> Result<(), MutationError> {
    // u64 storage satisfies FILE_STREAM_INFO's required 8-byte alignment.
    let mut size = 4096usize;
    loop {
        let mut allocation = vec![0u64; size / 8];
        let result = unsafe {
            GetFileInformationByHandleEx(HANDLE(file.as_raw_handle()), FileStreamInfo,
                allocation.as_mut_ptr().cast(), size as u32)
        };
        match result {
            Ok(()) => {
                let bytes = unsafe { std::slice::from_raw_parts(allocation.as_ptr().cast::<u8>(), size) };
                return only_unnamed_stream(bytes);
            }
            Err(error) if matches!(error.code().0 as u32 & 0xffff, 122 | 234) && size < 1024 * 1024 => { size *= 2; }
            Err(error) => return Err(err(MutationErrorKind::Unsupported,
                &format!("cannot establish a complete repair stream inventory: {error}"))),
        }
    }
}

fn check_repair_representation(file: &File) -> Result<FILE_BASIC_INFO, MutationError> {
    ensure_regular(file)?;
    let basic = repair_basic(file)?;
    repair_attributes(basic.FileAttributes)?;
    check_repair_streams(file)?;
    Ok(basic)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RepairPreservation {
    identity: ObjectIdentity,
    security: RepairSecurity,
    attributes: u32,
    creation_time: i64,
    access_time: i64,
    original_write_time: i64,
}
impl RepairPreservation {
    fn capture(file: &File) -> Result<Self, MutationError> {
        let basic = check_repair_representation(file)?;
        // Zero and negative FILETIME values have setter sentinel meanings.
        if basic.CreationTime <= 0 || basic.LastAccessTime <= 0 {
            return Err(err(MutationErrorKind::Unsupported, "repair timestamp cannot be preserved exactly"));
        }
        Ok(Self {
            identity: identity(file).map_err(read_error)?, security: repair_security(file)?,
            attributes: repair_attributes(basic.FileAttributes)?, creation_time: basic.CreationTime,
            access_time: basic.LastAccessTime, original_write_time: basic.LastWriteTime,
        })
    }
    fn check_security(&self, file: &File) -> Result<(), MutationError> {
        if repair_security(file)? != self.security {
            return Err(err(MutationErrorKind::Unsupported, "repair owner/group/DACL/integrity/resource-policy protection differs; no audio bytes may be staged"));
        }
        Ok(())
    }
    fn check_original(&self, file: &File) -> Result<(), MutationError> {
        let basic = check_repair_representation(file)?;
        self.check_security(file)?;
        if identity(file).map_err(read_error)? != self.identity
            || repair_attributes(basic.FileAttributes)? != self.attributes
            || basic.CreationTime != self.creation_time || basic.LastWriteTime != self.original_write_time
        {
            return Err(err(MutationErrorKind::StalePlan, "repair original metadata changed"));
        }
        // LastAccessTime may change due to our own source reads; ChangeTime
        // changes on the intended filename/backup rename. Neither is a lease.
        Ok(())
    }
}
#[derive(Clone)]
struct RepairTempState {
    original: RepairPreservation,
    // Duplicate of the same source file object, never a path reopen. Retained
    // through publication for source/backup checks even during streaming writes.
    held_original: Arc<File>,
    finalized_write_time: Option<i64>,
    poisoned: bool,
}
#[derive(Clone)]
enum RepairPublication {
    Original(RepairPreservation),
    Replacement(RepairTempState),
}

fn check_finalized_repair(file: &File, state: &RepairTempState) -> Result<(), MutationError> {
    let write_time = state.finalized_write_time.ok_or_else(|| err(MutationErrorKind::StalePlan, "repair staging not finalized"))?;
    if state.poisoned { return Err(err(MutationErrorKind::UnknownOutcome, "repair staging metadata poisoned")); }
    state.original.check_original(&state.held_original)?;
    let basic = check_repair_representation(file)?;
    state.original.check_security(file)?;
    if repair_attributes(basic.FileAttributes)? != state.original.attributes
        || basic.CreationTime != state.original.creation_time
        || basic.LastAccessTime != state.original.access_time || basic.LastWriteTime != write_time
    {
        return Err(err(MutationErrorKind::VerificationFailed, "staged repair metadata differs"));
    }
    Ok(())
}

fn repair_pending(error: MutationError, observation: &MutationObservation) -> MutationError {
    err(MutationErrorKind::UnknownOutcome, &format!("repair has retained objects; reconcile without replay: {error:?}"))
        .with_observation(error.observed.as_deref().cloned().unwrap_or_else(|| observation.clone()))
}

/// Narrow in-place staging/backup adapter. There is deliberately no source unlink
/// or backup cleanup method. The exact retained original is required to publish.
/// See docs/windows-fix-preservation.md for the supported security domain and races.
pub struct WindowsRepairStorage { rename: WindowsRenameStorage }
impl WindowsRepairStorage {
    pub fn new(roots: WindowsMutationStorage) -> Result<Self, MutationError> {
        if !matches!(roots.domain(), WindowsMutationDomain::SelectedRepair | WindowsMutationDomain::OwnedSyntheticTree) {
            return Err(err(MutationErrorKind::Unsupported, "repair-specific root grant required"));
        }
        Ok(Self { rename: WindowsRenameStorage::new(roots) })
    }
    pub fn roots(&self) -> &WindowsMutationStorage { self.rename.owned() }
    pub fn acquire_source(&mut self, parent: &WindowsParent, expected: &ExpectedSource) -> Result<WindowsSourceLease, MutationError> {
        self.rename.acquire_source(parent, expected)
    }
    /// Invoke after the optional filename-first rename, and only for tag changes.
    /// A filename-only rename retains the original representation without staging.
    pub fn prepare_metadata_preservation(&self, source: &mut WindowsSourceLease) -> Result<(), MutationError> {
        self.rename.owned.validate_source(source)?;
        let file = source.file.as_ref().expect("validated source");
        if let Some(snapshot) = &source.repair { return snapshot.check_original(file); }
        source.repair = Some(RepairPreservation::capture(file)?);
        Ok(())
    }
    pub fn create_staged(&mut self, source: &WindowsSourceLease, correlation: &str, cancel: &AtomicBool) -> MutationOutcome<WindowsOwnedTemp> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.rename.owned.validate_source(source)?;
            let snapshot = source.repair.as_ref().ok_or_else(|| err(MutationErrorKind::Unsupported, "repair preservation was not prepared"))?;
            snapshot.check_original(source.file.as_ref().expect("validated source"))?;
            let held_original = Arc::new(source.file.as_ref().expect("validated source").try_clone().map_err(io_error)?);
            Ok((snapshot.clone(), held_original))
        })();
        let (snapshot, held_original) = match preflight { Ok(value) => value, Err(error) => return MutationOutcome::NoEffect(error) };
        let mut temp = match self.rename.owned.create_temp(&source.parent, correlation, cancel) {
            MutationOutcome::Applied(value) => value,
            MutationOutcome::NoEffect(error) => return MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => return MutationOutcome::PendingUnknown(error),
        };
        let checked = (|| {
            cancelled(cancel)?;
            let file = temp.file.as_ref().expect("created temp");
            check_repair_representation(file)?;
            snapshot.check_security(file)?;
            snapshot.check_original(source.file.as_ref().expect("validated source"))?;
            Ok(())
        })();
        if let Err(error) = checked {
            // An empty object exists. Do not delete it or return a writable guard,
            // and never describe the failed post-create checks as NoEffect.
            return MutationOutcome::PendingUnknown(repair_pending(error, &temp.observation));
        }
        temp.repair = Some(RepairTempState { original: snapshot, held_original, finalized_write_time: None, poisoned: false });
        MutationOutcome::Applied(temp)
    }
    /// Flush content, set ordinary attributes and original creation/access times,
    /// then verify on the same handle. LastWriteTime is the completion time of the
    /// changed content, not the original mtime. No writes are accepted afterward.
    pub fn finalize_staged(&self, source: &WindowsSourceLease, temp: &mut WindowsOwnedTemp) -> Result<(), MutationError> {
        self.rename.owned.validate_source(source)?;
        self.rename.owned.check_role(&temp.parent, RootRole::Target)?;
        let snapshot = source.repair.as_ref().ok_or_else(|| err(MutationErrorKind::Unsupported, "repair preservation was not prepared"))?;
        let state = temp.repair.as_mut().ok_or_else(|| err(MutationErrorKind::Unsupported, "ordinary temp is not repair staging"))?;
        if state.original != *snapshot || source.parent.identity != temp.parent.identity || state.poisoned {
            return Err(err(MutationErrorKind::StalePlan, "repair staging/source mismatch or poisoned state"));
        }
        let file = temp.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
        snapshot.check_original(source.file.as_ref().expect("validated source"))?;
        if identity(file).map_err(read_error)? != temp.identity { return Err(err(MutationErrorKind::StalePlan, "temp identity changed")); }
        if state.finalized_write_time.is_some() { return check_finalized_repair(file, state); }
        check_repair_representation(file)?;
        snapshot.check_security(file)?;
        let ticks = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| err(MutationErrorKind::Unsupported, "system clock predates Unix epoch"))?.as_nanos() / 100;
        let write_time = i64::try_from(ticks + 116_444_736_000_000_000u128)
            .map_err(|_| err(MutationErrorKind::Unsupported, "repair timestamp overflow"))?;
        let basic = FILE_BASIC_INFO {
            CreationTime: snapshot.creation_time, LastAccessTime: snapshot.access_time,
            LastWriteTime: write_time, ChangeTime: 0,
            FileAttributes: if snapshot.attributes == 0 { FILE_ATTRIBUTE_NORMAL.0 } else { snapshot.attributes },
        };
        // No retry through this guard after a dispatched metadata/flush failure.
        state.poisoned = true;
        let dispatched = (|| {
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
            unsafe { SetFileInformationByHandle(HANDLE(file.as_raw_handle()), FileBasicInfo,
                (&basic as *const FILE_BASIC_INFO).cast(), size_of::<FILE_BASIC_INFO>() as u32) }.map_err(win_error)?;
            let freeze = FILETIME { dwLowDateTime: u32::MAX, dwHighDateTime: u32::MAX };
            unsafe { SetFileTime(HANDLE(file.as_raw_handle()), None, Some(&freeze), Some(&freeze)) }.map_err(win_error)?;
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
            snapshot.check_original(source.file.as_ref().expect("validated source"))?;
            Ok(())
        })();
        if let Err(error) = dispatched { return Err(repair_pending(error, &temp.observation)); }
        state.finalized_write_time = Some(write_time);
        state.poisoned = false;
        if let Err(error) = check_finalized_repair(file, state) {
            state.poisoned = true;
            return Err(repair_pending(error, &temp.observation));
        }
        Ok(())
    }
    /// Preserve filename-first behavior while rebinding the same held original.
    pub fn rename_original(&mut self, source: &mut WindowsSourceLease, name: &MutationName, cancel: &AtomicBool) -> MutationOutcome<WindowsSourceLease> {
        if let Some(snapshot) = &source.repair {
            if let Err(error) = source.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "source consumed")).and_then(|file| snapshot.check_original(file)) {
                return MutationOutcome::NoEffect(error);
            }
        }
        let parent = source.parent.clone();
        let mut expected = source.expected.clone();
        let snapshot = source.repair.clone();
        match self.rename.rename_no_replace(source, &parent, name, cancel) {
            MutationOutcome::Applied(published) => {
                if let Some(snapshot) = &snapshot {
                    if let Err(error) = snapshot.check_original(&published.file) {
                        return MutationOutcome::PendingUnknown(repair_pending(error, &published.observation));
                    }
                }
                expected.name = name.clone(); expected.parent_identity = parent.identity.clone();
                MutationOutcome::Applied(WindowsSourceLease { file: Some(published.file), parent, expected,
                    observation: published.observation, mode: SourceLeaseMode::WriteThroughRename, repair: snapshot })
            }
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }
    pub fn preserve_original(&mut self, source: &mut WindowsSourceLease, backup_name: &MutationName, cancel: &AtomicBool) -> MutationOutcome<WindowsPublished> {
        let snapshot = match source.repair.as_ref() {
            Some(snapshot) => snapshot.clone(),
            None => return MutationOutcome::NoEffect(err(MutationErrorKind::Unsupported, "repair preservation was not prepared")),
        };
        let parent = source.parent.clone();
        let held = match source.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "source consumed"))
            .and_then(|file| file.try_clone().map_err(io_error)) {
            Ok(file) => file,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        // This checkpoint runs after digest/preflush work, immediately before the
        // native rename. The main-stream lease cannot stop concurrent ADS/ACL edits.
        match self.rename.rename_no_replace_with(source, &parent, backup_name, cancel, |point| {
            if point == RenameCheckpoint::BeforeRename {
                snapshot.check_original(&held)?;
            }
            Ok(())
        }) {
            MutationOutcome::Applied(mut published) => {
                if let Err(error) = snapshot.check_original(&published.file) {
                    return MutationOutcome::PendingUnknown(repair_pending(error, &published.observation));
                }
                published.repair = Some(RepairPublication::Original(snapshot));
                MutationOutcome::Applied(published)
            }
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }
    pub fn publish_staged(&mut self, temp: &mut WindowsOwnedTemp, backup: &WindowsPublished, parent: &WindowsParent, name: &MutationName, cancel: &AtomicBool) -> MutationOutcome<WindowsPublished> {
        let preflight = (|| {
            cancelled(cancel)?;
            self.rename.owned.check_role(parent, RootRole::Target)?;
            self.rename.owned.inspect(backup)?;
            if temp.parent.identity != parent.identity || backup.parent.identity != parent.identity {
                return Err(err(MutationErrorKind::StalePlan, "repair staging and backup require the exact original parent"));
            }
            let Some(RepairPublication::Original(snapshot)) = &backup.repair else { return Err(err(MutationErrorKind::Unsupported, "matching retained original required")); };
            let state = temp.repair.as_ref().ok_or_else(|| err(MutationErrorKind::Unsupported, "ordinary temp is not repair staging"))?;
            if state.original != *snapshot { return Err(err(MutationErrorKind::StalePlan, "repair staging/backup identity mismatch")); }
            snapshot.check_original(&backup.file)?;
            check_finalized_repair(temp.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?, state)?;
            Ok(state.clone())
        })();
        let state = match preflight {
            Ok(state) => state,
            Err(error) => return MutationOutcome::PendingUnknown(repair_pending(error, &backup.observation)),
        };
        match self.rename.owned.publish_no_replace(temp, parent, name, cancel) {
            MutationOutcome::Applied(mut published) => {
                // The publication now owns the held original evidence; do not
                // leave an extra share-denying guard in the consumed temp.
                temp.repair = None;
                published.repair = Some(RepairPublication::Replacement(state.clone()));
                if let Err(error) = state.original.check_original(&backup.file).and_then(|_| check_finalized_repair(&published.file, &state)) {
                    return MutationOutcome::PendingUnknown(repair_pending(error, &published.observation));
                }
                MutationOutcome::Applied(published)
            }
            MutationOutcome::NoEffect(error) | MutationOutcome::PendingUnknown(error) =>
                MutationOutcome::PendingUnknown(repair_pending(error, &backup.observation)),
        }
    }
    /// Fresh held-handle evidence, including bytes written before interruption.
    pub fn observe_staged(&self, temp: &WindowsOwnedTemp) -> Result<MutationObservation, MutationError> {
        self.rename.owned.check_role(&temp.parent, RootRole::Target)?;
        let file = temp.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
        let observed = observe(file, &temp.parent)?;
        if observed.identity != temp.identity || observed.actual_name != temp.observation.actual_name {
            return Err(err(MutationErrorKind::StalePlan, "staged repair object changed").with_observation(observed));
        }
        Ok(observed)
    }
    pub fn verify_staged_fields(&self,temp:&WindowsOwnedTemp,filename:&str,repairs:&[sorter_core::fix_plan::FieldRepair])->Result<(),MutationError>{
        let file=temp.file.as_ref().ok_or_else(||err(MutationErrorKind::StalePlan,"temp consumed"))?;
        let mut read=file.try_clone().map_err(io_error)?;read.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let length=read.metadata().map_err(io_error)?.len();
        let mut fields=crate::metadata::MetadataReader.open(crate::storage::ReadInput::seekable(read,length),filename)
            .map_err(|_|err(MutationErrorKind::VerificationFailed,"staged native metadata open failed"))?;
        for repair in repairs {
            let item=fields.get(repair.field).map_err(|_|err(MutationErrorKind::VerificationFailed,"staged native metadata field failed"))?;
            if item.value!=Ok(sorter_core::metadata::LegacyScalar::Text(repair.replacement.clone())){return Err(err(MutationErrorKind::VerificationFailed,"staged native metadata differs"));}
        }
        Ok(())
    }
    pub fn sync_published(&mut self, published: &mut WindowsPublished, expected: &ContentDigest, cancel: &AtomicBool) -> Result<PublishedFileSyncAcknowledgment, MutationError> {
        self.verify_published(published, expected)?;
        let acknowledgement = self.rename.owned.sync_published_file(published, expected, cancel)?;
        self.verify_published(published, expected)?;
        Ok(acknowledgement)
    }
    pub fn verify_published(&mut self, published: &mut WindowsPublished, expected: &ContentDigest) -> Result<MutationObservation, MutationError> {
        let Some(RepairPublication::Replacement(state)) = &published.repair else {
            return Err(err(MutationErrorKind::Unsupported, "verified repair publication required"));
        };
        check_finalized_repair(&published.file, state).map_err(|error| repair_pending(error, &published.observation))?;
        self.rename.owned.verify(published, expected)
    }
}

#[cfg(test)]
mod repair_backup_tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Win32::Security::{
        GetSecurityDescriptorDacl, PROTECTED_DACL_SECURITY_INFORMATION,
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SetSecurityInfo},
    };

    fn name(s: &str) -> MutationName { MutationName::Windows(s.encode_utf16().collect()) }
    fn applied<T>(outcome: MutationOutcome<T>) -> T {
        match outcome { MutationOutcome::Applied(value) => value, _ => panic!("expected applied") }
    }
    fn grant(path: &Path) -> WindowsSelectedRootGrant {
        let handles = chain(path).unwrap();
        WindowsSelectedRootGrant::new(path.to_path_buf(), identity(handles.last().unwrap()).unwrap()).unwrap()
    }
    fn fixture() -> (WindowsRepairStorage, WindowsParent, ExpectedSource, PathBuf) {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_path_buf();
        std::fs::write(path.join("song.mp3"), b"original").unwrap();
        let mut roots = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let parent = roots.parent(Path::new("")).unwrap();
        let expected = roots.observe_source(&parent, &name("song.mp3")).unwrap();
        (WindowsRepairStorage::new(roots).unwrap(), parent, expected, path)
    }
    fn prepared(storage: &mut WindowsRepairStorage, parent: &WindowsParent, expected: &ExpectedSource) -> WindowsSourceLease {
        let mut source = storage.acquire_source(parent, expected).unwrap();
        storage.prepare_metadata_preservation(&mut source).unwrap();
        source
    }
    fn staged(storage: &mut WindowsRepairStorage, source: &WindowsSourceLease, correlation: &str) -> WindowsOwnedTemp {
        let mut temp = applied(storage.create_staged(source, correlation, &AtomicBool::new(false)));
        temp.write_all(b"repaired").unwrap();
        storage.finalize_staged(source, &mut temp).unwrap();
        temp
    }
    fn restrict_dacl(path: &Path) {
        // Generated fixture only: owner-rights full access, protected from wider
        // parent inheritance. No token privileges or real user paths are changed.
        let file = std::fs::OpenOptions::new().access_mode(READ_CONTROL.0 | WRITE_DAC.0)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0).open(path).unwrap();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(
            windows::core::w!("D:P(A;;FA;;;OW)"), 1, &mut descriptor, None,
        ) }.unwrap();
        let allocation = SecurityAllocation(descriptor);
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut present = Default::default();
        let mut defaulted = Default::default();
        unsafe { GetSecurityDescriptorDacl(allocation.0, &mut present, &mut dacl, &mut defaulted) }.unwrap();
        let result = unsafe { SetSecurityInfo(HANDLE(file.as_raw_handle()), SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            None, None, Some(dacl), None) };
        assert_eq!(result.0, 0);
    }

    #[test]
    fn selected_repair_roots_are_disjoint_nonowning_and_never_unlink() {
        let tree = tempfile::tempdir().unwrap();
        let source = tree.path().join("music");
        let journal = tree.path().join("journals");
        std::fs::create_dir(&source).unwrap(); std::fs::create_dir(&journal).unwrap();
        std::fs::write(source.join("song.mp3"), b"original").unwrap();
        let mut roots = WindowsMutationStorage::for_selected_repair_roots(grant(&source), grant(&journal)).unwrap();
        assert_eq!(roots.domain(), WindowsMutationDomain::SelectedRepair);
        assert!(roots.selected_source_root().is_err()); assert!(roots.selected_target_root().is_err());
        let parent = roots.selected_repair_root().unwrap();
        let expected = roots.observe_source(&parent, &name("song.mp3")).unwrap();
        let mut lease = roots.acquire_source(&parent, &expected).unwrap();
        assert!(matches!(roots.retire_source(&mut lease, &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::Unsupported));
        drop(lease); drop(parent); drop(roots);
        assert_eq!(std::fs::read(source.join("song.mp3")).unwrap(), b"original");
        assert!(journal.is_dir());
        assert!(WindowsMutationStorage::for_selected_repair_roots(grant(&source), grant(&source)).is_err());
    }

    #[test]
    fn planned_filename_rename_rebinds_original_before_tag_staging() {
        let (mut storage, parent, expected, path) = fixture();
        let mut original = prepared(&mut storage, &parent, &expected);
        let snapshot = original.repair.clone();
        let mut rebound = applied(storage.rename_original(&mut original, &name("clean.mp3"), &AtomicBool::new(false)));
        assert!(original.file.is_none());
        assert_eq!(rebound.expected.identity, expected.identity);
        assert_eq!(rebound.expected.name, name("clean.mp3"));
        assert_eq!(rebound.repair, snapshot);
        let mut bytes = Vec::new(); rebound.rewind().unwrap(); rebound.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"original");
        let backup = applied(storage.preserve_original(&mut rebound, &name("original.sorter-backup"), &AtomicBool::new(false)));
        drop(backup);
        assert!(!path.join("song.mp3").exists()); assert!(!path.join("clean.mp3").exists());
        assert_eq!(std::fs::read(path.join("original.sorter-backup")).unwrap(), b"original");
    }

    #[test]
    fn ads_rejects_tag_staging_but_filename_only_cleanup_retains_whole_object() {
        let (mut storage, parent, _, path) = fixture();
        std::fs::write(path.join("song.mp3:notes"), b"original notes").unwrap();
        let expected = storage.rename.owned.observe_source(&parent, &name("song.mp3")).unwrap();
        let mut source = storage.acquire_source(&parent, &expected).unwrap();
        let mut renamed = applied(storage.rename_original(&mut source, &name("clean.mp3"), &AtomicBool::new(false)));
        let error = storage.prepare_metadata_preservation(&mut renamed).unwrap_err();
        assert_eq!(error.kind, MutationErrorKind::Unsupported);
        assert!(matches!(storage.create_staged(&renamed, "ads", &AtomicBool::new(false)), MutationOutcome::NoEffect(_)));
        drop(renamed); drop(source);
        assert!(!path.join(".sorter-ads.tmp").exists());
        assert_eq!(std::fs::read(path.join("clean.mp3")).unwrap(), b"original");
        assert_eq!(std::fs::read(path.join("clean.mp3:notes")).unwrap(), b"original notes");
    }

    #[test]
    fn inherited_security_metadata_and_retained_original_survive_publication() {
        let (mut storage, parent, _, path) = fixture();
        {
            let file = std::fs::OpenOptions::new().access_mode(FILE_READ_ATTRIBUTES.0 | FILE_WRITE_ATTRIBUTES.0)
                .open(path.join("song.mp3")).unwrap();
            let basic = FILE_BASIC_INFO { CreationTime: 125_000_000_000_000_000,
                LastAccessTime: 126_000_000_000_000_000, LastWriteTime: 127_000_000_000_000_000,
                ChangeTime: 0, FileAttributes: FILE_ATTRIBUTE_ARCHIVE.0 | FILE_ATTRIBUTE_HIDDEN.0
                    | FILE_ATTRIBUTE_SYSTEM.0 | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0 };
            unsafe { SetFileInformationByHandle(HANDLE(file.as_raw_handle()), FileBasicInfo,
                (&basic as *const FILE_BASIC_INFO).cast(), size_of::<FILE_BASIC_INFO>() as u32) }.unwrap();
        }
        let expected = storage.rename.owned.observe_source(&parent, &name("song.mp3")).unwrap();
        let mut source = prepared(&mut storage, &parent, &expected);
        let snapshot = source.repair.clone().unwrap();
        let mut temp = staged(&mut storage, &source, "repair");
        assert!(temp.write_all(b"cannot write after finalize").is_err());
        let backup = applied(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)));
        assert_eq!(backup.observation.identity, snapshot.identity);
        let mut published = applied(storage.publish_staged(&mut temp, &backup, &parent, &name("song.mp3"), &AtomicBool::new(false)));
        let expected = ContentDigest { sha256: Sha256::digest(b"repaired").into(), length: 8 };
        storage.sync_published(&mut published, &expected, &AtomicBool::new(false)).unwrap();
        storage.verify_published(&mut published, &expected).unwrap();
        assert_eq!(repair_security(&published.file).unwrap(), snapshot.security);
        let basic = repair_basic(&published.file).unwrap();
        assert_eq!(repair_attributes(basic.FileAttributes).unwrap(), snapshot.attributes);
        assert_eq!(basic.CreationTime, snapshot.creation_time);
        assert_eq!(basic.LastAccessTime, snapshot.access_time);
        assert_ne!(basic.LastWriteTime, snapshot.original_write_time);
        snapshot.check_original(&backup.file).unwrap();
        drop(published); drop(backup);
        // Reopen after handle closure to catch deferred native timestamp updates.
        let file = File::open(path.join("song.mp3")).unwrap();
        let reopened = repair_basic(&file).unwrap();
        assert_eq!(reopened.CreationTime, snapshot.creation_time);
        assert_eq!(reopened.LastAccessTime, snapshot.access_time);
        assert_eq!(reopened.LastWriteTime, basic.LastWriteTime);
        drop(file);
        assert_eq!(std::fs::read(path.join("song.mp3")).unwrap(), b"repaired");
        assert_eq!(std::fs::read(path.join("original.sorter-backup")).unwrap(), b"original");
    }

    #[test]
    fn restrictive_protected_acl_never_writes_under_wider_inherited_acl() {
        let (mut storage, parent, _, path) = fixture();
        restrict_dacl(&path.join("song.mp3"));
        let expected = storage.rename.owned.observe_source(&parent, &name("song.mp3")).unwrap();
        let source = prepared(&mut storage, &parent, &expected);
        assert_ne!(source.repair.as_ref().unwrap().security.control & 0x1000, 0);
        let error = match storage.create_staged(&source, "acl", &AtomicBool::new(false)) {
            MutationOutcome::PendingUnknown(error) => error,
            _ => panic!("wider inherited DACL must reject the created empty temp"),
        };
        assert_eq!(error.kind, MutationErrorKind::UnknownOutcome);
        assert_eq!(error.observed.as_ref().unwrap().actual_name, name(".sorter-acl.tmp"));
        assert_eq!(error.observed.as_ref().unwrap().size, Some(0));
        assert_eq!(std::fs::metadata(path.join(".sorter-acl.tmp")).unwrap().len(), 0);
        drop(source);
        assert_eq!(std::fs::read(path.join("song.mp3")).unwrap(), b"original");
    }

    #[test]
    fn mandatory_integrity_policy_mismatch_never_writes_staging() {
        let (mut storage, parent, _, path) = fixture();
        {
            // Add medium-integrity NO_READ_UP to the generated original. This
            // does not elevate its integrity above the test token or change any
            // token privilege, but is stricter than ordinary default NO_WRITE_UP.
            let file = std::fs::OpenOptions::new().access_mode(READ_CONTROL.0 | WRITE_OWNER.0)
                .open(path.join("song.mp3")).unwrap();
            let mut descriptor = PSECURITY_DESCRIPTOR::default();
            unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(
                windows::core::w!("S:(ML;;NWNR;;;ME)"), 1, &mut descriptor, None,
            ) }.unwrap();
            let allocation = SecurityAllocation(descriptor);
            let mut sacl: *mut ACL = std::ptr::null_mut();
            let mut present = Default::default();
            let mut defaulted = Default::default();
            unsafe { GetSecurityDescriptorSacl(allocation.0, &mut present, &mut sacl, &mut defaulted) }.unwrap();
            let result = unsafe { SetSecurityInfo(HANDLE(file.as_raw_handle()), SE_FILE_OBJECT,
                LABEL_SECURITY_INFORMATION, None, None, None, Some(sacl)) };
            assert_eq!(result.0, 0);
        }
        let expected = storage.rename.owned.observe_source(&parent, &name("song.mp3")).unwrap();
        let source = prepared(&mut storage, &parent, &expected);
        assert!(source.repair.as_ref().unwrap().security.access_sacl.is_some());
        assert!(matches!(storage.create_staged(&source, "integrity", &AtomicBool::new(false)),
            MutationOutcome::PendingUnknown(error) if error.observed.as_ref().is_some_and(|observed| observed.size == Some(0))));
        assert_eq!(std::fs::metadata(path.join(".sorter-integrity.tmp")).unwrap().len(), 0);
        drop(source);
        assert_eq!(std::fs::read(path.join("song.mp3")).unwrap(), b"original");
    }

    #[test]
    fn ordinary_attribute_policy_rejects_efs_sparse_compressed_and_unknown_flags() {
        for flag in [FILE_ATTRIBUTE_ENCRYPTED.0, FILE_ATTRIBUTE_SPARSE_FILE.0,
            FILE_ATTRIBUTE_COMPRESSED.0, FILE_ATTRIBUTE_READONLY.0, FILE_ATTRIBUTE_OFFLINE.0, 0x8000_0000] {
            assert_eq!(repair_attributes(FILE_ATTRIBUTE_ARCHIVE.0 | flag).unwrap_err().kind, MutationErrorKind::Unsupported);
        }
        assert_eq!(repair_attributes(FILE_ATTRIBUTE_NORMAL.0).unwrap(), 0);
        assert_eq!(repair_attributes(FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0).unwrap(),
            FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0);
    }

    #[test]
    fn stream_inventory_parser_is_bounded_and_rejects_named_or_malformed_entries() {
        let header = std::mem::offset_of!(FILE_STREAM_INFO, StreamName);
        let record = |stream: &str| {
            let encoded: Vec<u16> = stream.encode_utf16().collect();
            let mut bytes = vec![0u8; header + encoded.len() * 2];
            bytes[4..8].copy_from_slice(&((encoded.len() * 2) as u32).to_le_bytes());
            for (index, unit) in encoded.iter().enumerate() {
                bytes[header + index * 2..header + index * 2 + 2].copy_from_slice(&unit.to_le_bytes());
            }
            bytes
        };
        let plain = record("::$DATA");
        only_unnamed_stream(&plain).unwrap();
        assert!(only_unnamed_stream(&record(":notes:$DATA")).is_err());
        assert!(only_unnamed_stream(&plain[..header - 1]).is_err());
        let mut malformed = plain.clone();
        malformed[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(only_unnamed_stream(&malformed).is_err());
        malformed = plain;
        malformed[..4].copy_from_slice(&8u32.to_le_bytes());
        assert!(only_unnamed_stream(&malformed).is_err());
    }

    #[test]
    fn repair_equivalence_checks_owner_group_dacl_integrity_policy_and_protection_independently() {
        let (mut storage, parent, expected, _) = fixture();
        let source = prepared(&mut storage, &parent, &expected);
        let file = source.file.as_ref().unwrap();
        for part in 0..5 {
            let mut snapshot = source.repair.clone().unwrap();
            match part {
                0 => snapshot.security.owner.push(0),
                1 => snapshot.security.group.push(0),
                2 => snapshot.security.dacl.push(0),
                3 => snapshot.security.access_sacl = Some(vec![0]),
                _ => snapshot.security.control ^= 0x1000,
            }
            assert!(snapshot.check_security(file).is_err());
        }
    }

    #[test]
    fn source_acl_change_after_prepare_blocks_empty_stage_creation() {
        let (mut storage, parent, expected, path) = fixture();
        let source = prepared(&mut storage, &parent, &expected);
        restrict_dacl(&path.join("song.mp3"));
        assert!(matches!(storage.create_staged(&source, "changed", &AtomicBool::new(false)), MutationOutcome::NoEffect(_)));
        assert!(!path.join(".sorter-changed.tmp").exists());
    }

    #[test]
    fn staged_acl_change_is_rechecked_before_each_audio_write() {
        let (mut storage, parent, expected, path) = fixture();
        let source = prepared(&mut storage, &parent, &expected);
        let mut temp = applied(storage.create_staged(&source, "acl-write", &AtomicBool::new(false)));
        restrict_dacl(&path.join(".sorter-acl-write.tmp"));
        assert!(temp.write_all(b"audio must not be exposed").is_err());
        assert_eq!(storage.observe_staged(&temp).unwrap().size, Some(0));
    }

    #[test]
    fn retained_backup_security_change_blocks_publication_with_recovery_evidence() {
        let (mut storage, parent, expected, path) = fixture();
        let mut source = prepared(&mut storage, &parent, &expected);
        let mut temp = staged(&mut storage, &source, "security-change");
        let backup = applied(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)));
        restrict_dacl(&path.join("original.sorter-backup"));
        assert!(matches!(storage.publish_staged(&mut temp, &backup, &parent, &name("song.mp3"), &AtomicBool::new(false)), MutationOutcome::PendingUnknown(error) if error.observed.is_some()));
        assert!(temp.file.is_some());
        assert!(!path.join("song.mp3").exists());
        drop(backup); drop(temp);
        assert_eq!(std::fs::read(path.join("original.sorter-backup")).unwrap(), b"original");
    }

    #[test]
    fn changed_stream_inventory_is_rechecked_at_prepare_backup_and_publication() {
        let (mut storage, parent, expected, path) = fixture();
        let mut source = prepared(&mut storage, &parent, &expected);
        let mut temp = staged(&mut storage, &source, "stream-change");
        // Some native share combinations also prevent this ADS open. Such a
        // rejection is not evidence that all possible ADS/security edits lock.
        match std::fs::write(path.join("song.mp3:late"), b"late source stream") {
            Ok(()) => {
                assert!(matches!(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::Unsupported));
                return;
            }
            Err(error) => assert!(matches!(error.raw_os_error(), Some(32 | 33))),
        }
        let backup = applied(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)));
        match std::fs::write(path.join("original.sorter-backup:late"), b"late backup stream") {
            Ok(()) => assert!(matches!(storage.publish_staged(&mut temp, &backup, &parent, &name("song.mp3"), &AtomicBool::new(false)), MutationOutcome::PendingUnknown(_))),
            Err(error) => assert!(matches!(error.raw_os_error(), Some(32 | 33))),
        }
        assert!(temp.file.is_some());
    }

    #[test]
    fn staging_requires_prepared_same_identity_and_matching_backup() {
        let (mut storage, parent, expected, path) = fixture();
        let mut source = storage.acquire_source(&parent, &expected).unwrap();
        assert!(matches!(storage.create_staged(&source, "unprepared", &AtomicBool::new(false)), MutationOutcome::NoEffect(_)));
        storage.prepare_metadata_preservation(&mut source).unwrap();
        let mut temp = staged(&mut storage, &source, "identity");
        std::fs::write(path.join("other.mp3"), b"other").unwrap();
        let other_expected = storage.rename.owned.observe_source(&parent, &name("other.mp3")).unwrap();
        let mut other = prepared(&mut storage, &parent, &other_expected);
        assert!(storage.finalize_staged(&other, &mut temp).is_err());
        let backup = applied(storage.preserve_original(&mut other, &name("other.backup"), &AtomicBool::new(false)));
        assert!(matches!(storage.publish_staged(&mut temp, &backup, &parent, &name("wrong.mp3"), &AtomicBool::new(false)), MutationOutcome::PendingUnknown(_)));
        assert!(!path.join("wrong.mp3").exists());
    }

    #[test]
    fn backup_collision_never_overwrites_or_consumes_original() {
        let (mut storage, parent, expected, path) = fixture();
        std::fs::write(path.join("original.sorter-backup"), b"foreign").unwrap();
        let mut source = prepared(&mut storage, &parent, &expected);
        assert!(matches!(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)), MutationOutcome::NoEffect(_)));
        assert!(source.file.is_some()); drop(source);
        assert_eq!(std::fs::read(path.join("song.mp3")).unwrap(), b"original");
        assert_eq!(std::fs::read(path.join("original.sorter-backup")).unwrap(), b"foreign");
    }

    #[test]
    fn cancellation_before_create_and_backup_leaves_original_untouched() {
        let (mut storage, parent, expected, path) = fixture();
        let mut source = prepared(&mut storage, &parent, &expected);
        assert!(matches!(storage.create_staged(&source, "cancelled", &AtomicBool::new(true)), MutationOutcome::NoEffect(_)));
        assert!(matches!(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(true)), MutationOutcome::NoEffect(_)));
        drop(source);
        assert_eq!(std::fs::read(path.join("song.mp3")).unwrap(), b"original");
        assert!(!path.join(".sorter-cancelled.tmp").exists()); assert!(!path.join("original.sorter-backup").exists());
    }

    #[test]
    fn cancellation_after_backup_retains_original_and_reports_pending_recovery() {
        let (mut storage, parent, expected, path) = fixture();
        let mut source = prepared(&mut storage, &parent, &expected);
        let mut temp = staged(&mut storage, &source, "repair");
        let backup = applied(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)));
        assert!(matches!(storage.publish_staged(&mut temp, &backup, &parent, &name("song.mp3"), &AtomicBool::new(true)), MutationOutcome::PendingUnknown(_)));
        assert_eq!(storage.observe_staged(&temp).unwrap().size, Some(8));
        drop(backup); drop(temp);
        assert_eq!(std::fs::read(path.join("original.sorter-backup")).unwrap(), b"original");
        assert!(!path.join("song.mp3").exists());
    }

    #[test]
    fn foreign_original_name_after_backup_is_never_replaced() {
        let (mut storage, parent, expected, path) = fixture();
        let mut source = prepared(&mut storage, &parent, &expected);
        let mut temp = staged(&mut storage, &source, "repair");
        let backup = applied(storage.preserve_original(&mut source, &name("original.sorter-backup"), &AtomicBool::new(false)));
        std::fs::write(path.join("song.mp3"), b"foreign").unwrap();
        assert!(matches!(storage.publish_staged(&mut temp, &backup, &parent, &name("song.mp3"), &AtomicBool::new(false)), MutationOutcome::PendingUnknown(_)));
        drop(backup); drop(temp);
        assert_eq!(std::fs::read(path.join("song.mp3")).unwrap(), b"foreign");
        assert_eq!(std::fs::read(path.join("original.sorter-backup")).unwrap(), b"original");
    }

    #[test]
    fn selected_repair_journal_reads_are_identity_checked_and_bounded() {
        let tree = tempfile::tempdir().unwrap();
        let source = tree.path().join("music"); let journal = tree.path().join("journals");
        std::fs::create_dir(&source).unwrap(); std::fs::create_dir(&journal).unwrap();
        let roots = WindowsMutationStorage::for_selected_repair_roots(grant(&source), grant(&journal)).unwrap();
        let parent = roots.selected_journal_root().unwrap();
        let mut writer = roots.create_journal(&parent, &name("large.journal")).unwrap();
        writer.file.set_len(64 * 1024 * 1024 + 1).unwrap();
        assert!(writer.read_all().is_err());
        let location = writer.location(); drop(writer);
        assert!(roots.read_selected_journal(&name("large.journal"), location.identity()).is_err());
        let mut small = roots.create_journal(&parent, &name("small.journal")).unwrap();
        small.append(b"evidence").unwrap();
        let small_location = small.location(); drop(small);
        assert_eq!(roots.read_selected_journal(&name("small.journal"), small_location.identity()).unwrap(), b"evidence");
        assert!(roots.read_selected_journal(&name("small.journal"), location.identity()).is_err());
    }
}
