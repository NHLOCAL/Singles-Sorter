//! Explicit ordinary/quiescent NTFS album cleanup. No recursive deletion.
use super::*;
use super::album_rename::{album_observe, open_album_directory, require_album_ntfs};

fn check_empty_directory_representation(file: &File) -> Result<(), MutationError> {
    ensure_directory(file)?;
    let attributes = repair_basic(file)?.FileAttributes;
    let ordinary = FILE_ATTRIBUTE_DIRECTORY.0
        | FILE_ATTRIBUTE_ARCHIVE.0
        | FILE_ATTRIBUTE_HIDDEN.0
        | FILE_ATTRIBUTE_SYSTEM.0
        | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED.0;
    if attributes & FILE_ATTRIBUTE_DIRECTORY.0 == 0 || attributes & !ordinary != 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "directory cleanup rejects readonly or unsupported attributes",
        ));
    }
    check_empty_directory_streams(file)?;
    check_repair_extended_attributes(file)
}

fn check_empty_directory_streams(file: &File) -> Result<(), MutationError> {
    // Directories have no unnamed $DATA stream. Win32 explicitly documents
    // ERROR_HANDLE_EOF for FileStreamInfo with no returned streams. A successful
    // record, buffer overflow, unsupported call or access error is never emptiness.
    // u64 storage provides FILE_STREAM_INFO's required 8-byte alignment.
    let mut buffer = [0u64; 128];
    match unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileStreamInfo,
            buffer.as_mut_ptr().cast(),
            std::mem::size_of_val(&buffer) as u32,
        )
    } {
        Err(error) if error.code().0 as u32 & 0xffff == 38 => Ok(()), // ERROR_HANDLE_EOF
        Ok(()) => Err(err(
            MutationErrorKind::Unsupported,
            "directory has data streams",
        )),
        Err(error) => Err(err(
            MutationErrorKind::Unsupported,
            &format!("cannot exclude directory data streams: {error}"),
        )),
    }
}

fn close_directory_once(file: File) -> Result<(), MutationError> {
    use std::os::windows::io::IntoRawHandle;
    // Do not reconstruct File or retry after an uncertain CloseHandle response:
    // the numerical handle may already have been recycled for an unrelated object.
    let handle = HANDLE(file.into_raw_handle());
    unsafe { windows::Win32::Foundation::CloseHandle(handle) }.map_err(win_error)
}

impl WindowsMutationStorage {
    /// Explicit album Move authority. Planning read handles must be released
    /// first. The exact source-root DELETE handle is acquired before effects.
    pub fn for_selected_album_move_roots(
        source: WindowsSelectedRootGrant,
        target: WindowsSelectedRootGrant,
        journal: WindowsSelectedRootGrant,
        cancel: &AtomicBool,
    ) -> Result<Self, MutationError> {
        cancelled(cancel)?;
        validate_root_path(&source.path)?;
        let ancestors = chain(source.path.parent().ok_or_else(|| err(MutationErrorKind::Unsupported, "album source needs parent"))?).map_err(read_error)?;
        for ancestor in &ancestors { ensure_directory(ancestor)?; }
        let parent = ancestors.last().ok_or_else(|| err(MutationErrorKind::Unsupported, "album source needs held parent"))?;
        let name = MutationName::Windows(source.path.file_name().ok_or_else(|| err(MutationErrorKind::InvalidName, "album source has no leaf"))?.encode_wide().collect());
        let directory = open_album_directory(parent, units(&name)?, true)?;
        if identity(&directory).map_err(read_error)? != source.expected_identity {
            return Err(err(MutationErrorKind::StalePlan, "album source root differs from selected grant"));
        }
        require_album_ntfs(&directory)?;
        move_streams::require_move_volume(&directory)?;
        check_empty_directory_representation(&directory)?;
        let original = album_observe(&directory, &identity(parent).map_err(read_error)?)?;
        let source = Arc::new(Root {
            path: source.path, role: RootRole::Source, _ancestors: ancestors,
            directory, _ownership: RootOwnership::Selected, album_cleanup: true, album_cleanup_original: Some(original),
        });
        let target = pin_root(target.path, Some(&target.expected_identity), RootRole::Target, RootOwnership::Selected)?;
        let journal = pin_root(journal.path, Some(&journal.expected_identity), RootRole::Journal, RootOwnership::Selected)?;
        ensure_disjoint_roots(&source, &target)?;
        ensure_disjoint_roots(&source, &journal)?;
        ensure_disjoint_roots(&target, &journal)?;
        cancelled(cancel)?;
        Ok(Self { roots: vec![source, target, journal], domain: WindowsMutationDomain::SelectedRoots })
    }

    /// Preview/pre-effect representation check on an identity-pinned directory.
    /// This is evidence under quiescent writers, not an exclusion of late ADS/EA.
    pub fn inspect_album_cleanup_directory(
        &self, directory: &WindowsParent, cancel: &AtomicBool,
    ) -> Result<MutationObservation, MutationError> {
        cancelled(cancel)?;
        self.check_role(directory, RootRole::Source)?;
        require_album_ntfs(directory.handle())?;
        move_streams::require_move_volume(directory.handle())?;
        check_empty_directory_representation(directory.handle())?;
        let parent_identity = if let Some(parent) = directory.ancestors.iter().rev().nth(1) {
            identity(parent).map_err(read_error)?
        } else if !directory.ancestors.is_empty() {
            identity(&directory.root.directory).map_err(read_error)?
        } else {
            identity(directory.root._ancestors.last().ok_or_else(|| err(MutationErrorKind::Unsupported, "album directory has no held parent"))?).map_err(read_error)?
        };
        let observed = album_observe(directory.handle(), &parent_identity)?;
        cancelled(cancel)?;
        Ok(observed)
    }

    pub fn remove_empty_album_directory(
        &self, parent: &WindowsParent, name: &MutationName,
        expected: &ObjectIdentity, cancel: &AtomicBool,
    ) -> MutationOutcome<MutationObservation> {
        let prepared = (|| {
            cancelled(cancel)?;
            self.check_role(parent, RootRole::Source)?;
            if !parent.root.album_cleanup { return Err(err(MutationErrorKind::Unsupported, "explicit album cleanup authority required")); }
            let file = open_album_directory(parent.handle(), units(name)?, true)?;
            let observed = album_observe(&file, parent.identity())?;
            if observed.identity != *expected || observed.actual_name != *name {
                return Err(err(MutationErrorKind::StalePlan, "cleanup directory identity/name changed").with_observation(observed));
            }
            check_empty(&file, cancel)?;
            Ok((file, observed))
        })();
        match prepared {
            Err(error) => MutationOutcome::NoEffect(error),
            Ok((file, observed)) => dispose_directory(file, parent.handle(), observed, cancel),
        }
    }

    /// Call only after releasing every source Parent/file/watch guard. A live
    /// alias prevents consuming the root; no handle is closed/reopened to cope.
    /// Target and application-journal roots remain held after source removal.
    pub fn take_empty_album_source_root(
        &mut self, expected: &ObjectIdentity, cancel: &AtomicBool,
    ) -> Result<WindowsAlbumRootCleanup, MutationError> {
        let prepared = (|| {
            cancelled(cancel)?;
            let index = self.roots.iter().position(|root| root.role == RootRole::Source)
                .ok_or_else(|| err(MutationErrorKind::Unsupported, "source root already consumed"))?;
            let root = &self.roots[index];
            if !root.album_cleanup || Arc::strong_count(root) != 1 {
                return Err(err(MutationErrorKind::Locked, "album source root still has active grants"));
            }
            for ancestor in &root._ancestors { ensure_directory(ancestor)?; }
            let parent = root._ancestors.last().ok_or_else(|| err(MutationErrorKind::Unsupported, "source root parent absent"))?;
            let observed = album_observe(&root.directory, &identity(parent).map_err(read_error)?)?;
            if observed.identity != *expected || root.album_cleanup_original.as_ref() != Some(&observed) {
                return Err(err(MutationErrorKind::StalePlan, "source root changed before cleanup").with_observation(observed));
            }
            check_empty(&root.directory, cancel)?;
            Ok((index, observed))
        })();
        let (index, observed) = prepared?;
        let root = self.roots.remove(index);
        let root = match Arc::try_unwrap(root) {
            Ok(root) => root,
            Err(root) => { self.roots.insert(index, root); return Err(err(MutationErrorKind::Locked, "source root grant still held")); }
        };
        let Root { directory, _ancestors: ancestors, .. } = root;
        // The parent was validated above and remains privately held through the
        // one-shot close and original-leaf probe. It is never exposed as a grant.
        Ok(WindowsAlbumRootCleanup { file: Some(directory), ancestors, observed })
    }
}

/// Keeps only the private original-parent evidence after the root handle is
/// consumed. It cannot authorize mutation of any sibling or parent entry.
pub struct WindowsAlbumRootCleanup {
    file: Option<File>,
    ancestors: Vec<File>,
    observed: MutationObservation,
}
impl WindowsAlbumRootCleanup {
    pub fn observation(&self) -> &MutationObservation { &self.observed }
    pub fn remove_empty(&mut self, cancel: &AtomicBool) -> MutationOutcome<MutationObservation> {
        let Some(parent) = self.ancestors.last() else { return MutationOutcome::NoEffect(err(MutationErrorKind::Unsupported, "root parent missing")); };
        let Some(file) = self.file.take() else { return MutationOutcome::NoEffect(err(MutationErrorKind::Unsupported, "root cleanup is one-shot")); };
        dispose_directory(file, parent, self.observed.clone(), cancel)
    }
    pub fn inspect_original(&self, cancel: &AtomicBool) -> Result<Option<MutationObservation>, MutationError> {
        cancelled(cancel)?;
        for ancestor in &self.ancestors { ensure_directory(ancestor)?; }
        let parent = self.ancestors.last().ok_or_else(|| err(MutationErrorKind::Unsupported, "root parent missing"))?;
        if identity(parent).map_err(read_error)? != self.observed.parent_identity {
            return Err(err(MutationErrorKind::StalePlan, "original root parent changed"));
        }
        let probe = super::super::open_child(parent, units(&self.observed.actual_name)?, true);
        cancelled(cancel)?;
        match probe {
            Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(None),
            Err(error) => Err(read_error(error)),
            Ok(file) => { let observed = album_observe(&file, &self.observed.parent_identity)?; cancelled(cancel)?; Ok(Some(observed)) }
        }
    }
}

fn check_empty(file: &File, cancel: &AtomicBool) -> Result<(), MutationError> {
    cancelled(cancel)?;
    move_streams::require_move_volume(file)?;
    check_empty_directory_representation(file)?;
    if !directory_members_with_limits(file, cancel, 100_000, 1024)?.is_empty() {
        return Err(err(MutationErrorKind::StalePlan, "album cleanup directory is not empty"));
    }
    cancelled(cancel)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CleanupCheckpoint { BeforeDisposition, AfterDisposition, AfterClose }
fn dispose_directory(
    file: File, parent: &File, observed: MutationObservation, cancel: &AtomicBool,
) -> MutationOutcome<MutationObservation> {
    dispose_directory_with(file, parent, observed, cancel, |_| Ok(()), close_directory_once)
}
fn dispose_directory_with(
    file: File, parent: &File, observed: MutationObservation, cancel: &AtomicBool,
    mut checkpoint: impl FnMut(CleanupCheckpoint) -> Result<(), MutationError>,
    close: impl FnOnce(File) -> Result<(), MutationError>,
) -> MutationOutcome<MutationObservation> {
    // Final representation/membership checks are mandatory. The kernel also
    // refuses a nonempty directory; no recursive fallback exists.
    let final_checks = (|| {
        checkpoint(CleanupCheckpoint::BeforeDisposition)?;
        check_empty(&file, cancel)?;
        ensure_directory(parent)?;
        if identity(parent).map_err(read_error)? != observed.parent_identity
            || album_observe(&file, &observed.parent_identity)? != observed
        {
            return Err(err(MutationErrorKind::StalePlan, "album cleanup object moved or changed before dispatch"));
        }
        cancelled(cancel)
    })();
    if let Err(error) = final_checks { return MutationOutcome::NoEffect(error); }
    let disposition = FILE_DISPOSITION_INFO_EX { Flags: FILE_DISPOSITION_INFO_EX_FLAGS(
        FILE_DISPOSITION_FLAG_DELETE.0 | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS.0,
    ) };
    if let Err(error) = unsafe { SetFileInformationByHandle(
        HANDLE(file.as_raw_handle()), FileDispositionInfoEx,
        (&disposition as *const FILE_DISPOSITION_INFO_EX).cast(), size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
    ) } {
        let _closed = close(file);
        return MutationOutcome::PendingUnknown(err(MutationErrorKind::UnknownOutcome,
            &format!("directory disposition returned an error; inspect without replay: {error}")).with_observation(observed));
    }
    let response = checkpoint(CleanupCheckpoint::AfterDisposition);
    let closed = close(file);
    let verified = (|| {
        response?;
        closed?;
        checkpoint(CleanupCheckpoint::AfterClose)?;
        cancelled(cancel)?;
        ensure_directory(parent)?;
        let probe = super::super::open_child(parent, units(&observed.actual_name)?, true);
        cancelled(cancel)?;
        let result = match probe {
            Err(error) if error.kind == crate::storage::StorageErrorKind::NotFound => Ok(()),
            Ok(other) if identity(&other).map_err(read_error)? != observed.identity => Ok(()),
            Ok(_) => Err(err(MutationErrorKind::UnknownOutcome, "original directory remains after disposition")),
            Err(error) => Err(read_error(error)),
        };
        cancelled(cancel)?;
        result
    })();
    match verified {
        Ok(()) => MutationOutcome::Applied(observed),
        Err(error) => MutationOutcome::PendingUnknown(err(MutationErrorKind::UnknownOutcome,
            &format!("album cleanup dispatched; inspect without replay: {error:?}")).with_observation(observed)),
    }
}

#[cfg(test)]
mod album_cleanup_tests {
    use super::*;
    fn grant(path: &Path) -> WindowsSelectedRootGrant {
        let chain = chain(path).unwrap();
        WindowsSelectedRootGrant::new(path.to_path_buf(), identity(chain.last().unwrap()).unwrap()).unwrap()
    }
    struct Fixture { _tree: tempfile::TempDir, source: PathBuf, target: PathBuf, journal: PathBuf }
    impl Fixture {
        fn new() -> Self {
            let tree = tempfile::tempdir().unwrap();
            let root = tree.path().canonicalize().unwrap();
            let source = root.join("source"); let target = root.join("target"); let journal = root.join("journal");
            for path in [&source, &target, &journal] { std::fs::create_dir(path).unwrap(); }
            Self { _tree: tree, source, target, journal }
        }
        fn storage(&self) -> WindowsMutationStorage {
            WindowsMutationStorage::for_selected_album_move_roots(grant(&self.source), grant(&self.target), grant(&self.journal), &AtomicBool::new(false)).unwrap()
        }
    }
    fn applied<T>(outcome: MutationOutcome<T>) -> T {
        match outcome { MutationOutcome::Applied(value) => value, _ => panic!("expected acknowledged cleanup") }
    }
    #[test]
    fn exact_root_handle_cleanup_keeps_readonly_original_leaf_probe() {
        let fixture = Fixture::new(); let mut storage = fixture.storage();
        let id = storage.selected_source_root().unwrap().identity().clone();
        let mut cleanup = storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).unwrap();
        let observed = applied(cleanup.remove_empty(&AtomicBool::new(false)));
        assert_eq!(observed.identity, id); assert_eq!(observed.size, None);
        assert!(!fixture.source.exists());
        assert!(cleanup.inspect_original(&AtomicBool::new(false)).unwrap().is_none());
        std::fs::create_dir(&fixture.source).unwrap();
        assert_ne!(cleanup.inspect_original(&AtomicBool::new(false)).unwrap().unwrap().identity, id);
        assert!(matches!(cleanup.remove_empty(&AtomicBool::new(false)), MutationOutcome::NoEffect(_)));
        assert!(fixture.source.is_dir()); assert!(fixture.target.is_dir()); assert!(fixture.journal.is_dir());
    }
    #[test]
    fn live_parent_grant_prevents_root_consumption() {
        let fixture = Fixture::new(); let mut storage = fixture.storage();
        let root = storage.selected_source_root().unwrap(); let id = root.identity().clone();
        assert!(storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).is_err());
        assert!(fixture.source.is_dir()); drop(root);
        assert!(storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).is_ok());
        assert!(fixture.source.is_dir());
    }
    #[test]
    fn nonempty_root_and_cancellation_retain_source() {
        let fixture = Fixture::new(); std::fs::write(fixture.source.join("late.txt"), b"retained").unwrap();
        let mut storage = fixture.storage(); let id = storage.selected_source_root().unwrap().identity().clone();
        assert!(storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).is_err());
        assert!(storage.take_empty_album_source_root(&id, &AtomicBool::new(true)).is_err());
        assert_eq!(std::fs::read(fixture.source.join("late.txt")).unwrap(), b"retained");
    }
    #[test]
    fn ordinary_selected_roots_do_not_gain_cleanup_authority() {
        let fixture = Fixture::new();
        let mut storage = WindowsMutationStorage::for_selected_roots(grant(&fixture.source), grant(&fixture.target), grant(&fixture.journal)).unwrap();
        let id = storage.selected_source_root().unwrap().identity().clone();
        assert!(storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).is_err());
        assert!(fixture.source.is_dir());
    }
    #[test]
    fn child_cleanup_requires_exact_identity_and_empty_membership() {
        let fixture = Fixture::new(); std::fs::create_dir(fixture.source.join("album")).unwrap();
        let child_id = grant(&fixture.source.join("album")).expected_identity;
        let storage = fixture.storage(); let root = storage.selected_source_root().unwrap();
        let name = MutationName::Windows("album".encode_utf16().collect());
        assert!(matches!(storage.remove_empty_album_directory(&root, &name, root.identity(), &AtomicBool::new(false)), MutationOutcome::NoEffect(_)));
        assert!(fixture.source.join("album").is_dir());
        applied(storage.remove_empty_album_directory(&root, &name, &child_id, &AtomicBool::new(false)));
        assert!(!fixture.source.join("album").exists()); assert!(fixture.source.is_dir());
    }
    #[test]
    fn named_directory_stream_refuses_before_root_effects() {
        let fixture = Fixture::new();
        let stream = PathBuf::from(format!("{}:notes", fixture.source.display()));
        std::fs::write(&stream, b"directory-auxiliary").unwrap();
        assert!(WindowsMutationStorage::for_selected_album_move_roots(grant(&fixture.source), grant(&fixture.target), grant(&fixture.journal), &AtomicBool::new(false)).is_err());
        assert_eq!(std::fs::read(stream).unwrap(), b"directory-auxiliary"); assert!(fixture.source.is_dir());
    }
    #[test]
    fn dispatch_response_loss_and_cancellation_are_not_no_effect() {
        for stage in [CleanupCheckpoint::BeforeDisposition, CleanupCheckpoint::AfterDisposition, CleanupCheckpoint::AfterClose] {
            for cancellation in [false, true] {
                let fixture = Fixture::new(); let mut storage = fixture.storage();
                let id = storage.selected_source_root().unwrap().identity().clone();
                let mut cleanup = storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).unwrap();
                let cancel = AtomicBool::new(false);
                let outcome = dispose_directory_with(cleanup.file.take().unwrap(), cleanup.ancestors.last().unwrap(), cleanup.observed.clone(), &cancel,
                    |point| { if point == stage { if cancellation { cancel.store(true, Ordering::Release); } else { return Err(err(MutationErrorKind::Io, "lost response")); } } Ok(()) }, close_directory_once);
                if stage == CleanupCheckpoint::BeforeDisposition {
                    assert!(matches!(outcome, MutationOutcome::NoEffect(_))); assert!(fixture.source.is_dir());
                } else { assert!(matches!(outcome, MutationOutcome::PendingUnknown(_))); }
            }
        }
    }
    #[test]
    fn uncertain_close_consumes_once_and_foreign_replacement_survives() {
        let fixture = Fixture::new(); let mut storage = fixture.storage();
        let id = storage.selected_source_root().unwrap().identity().clone();
        let mut cleanup = storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).unwrap();
        let closed = std::cell::Cell::new(0);
        let outcome = dispose_directory_with(cleanup.file.take().unwrap(), cleanup.ancestors.last().unwrap(), cleanup.observed.clone(), &AtomicBool::new(false), |_| Ok(()),
            |file| { closed.set(closed.get() + 1); close_directory_once(file)?; std::fs::create_dir(&fixture.source).map_err(io_error)?; Err(err(MutationErrorKind::Io, "close response lost")) });
        assert_eq!(closed.get(), 1); assert!(matches!(outcome, MutationOutcome::PendingUnknown(_)));
        assert_ne!(cleanup.inspect_original(&AtomicBool::new(false)).unwrap().unwrap().identity, id);
        assert!(fixture.source.is_dir());
    }

    #[test]
    fn late_directory_stream_is_refused_after_final_checkpoint() {
        use std::os::windows::fs::OpenOptionsExt;
        let fixture = Fixture::new(); let mut storage = fixture.storage();
        let id = storage.selected_source_root().unwrap().identity().clone();
        let mut cleanup = storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).unwrap();
        let stream = PathBuf::from(format!("{}:late", fixture.source.display()));
        let outcome = dispose_directory_with(cleanup.file.take().unwrap(), cleanup.ancestors.last().unwrap(), cleanup.observed.clone(), &AtomicBool::new(false),
            |point| { if point == CleanupCheckpoint::BeforeDisposition {
                let mut file = std::fs::OpenOptions::new().write(true).create_new(true)
                    .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0).open(&stream).map_err(io_error)?;
                file.write_all(b"late directory data").map_err(io_error)?;
            } Ok(()) }, close_directory_once);
        assert!(matches!(outcome, MutationOutcome::NoEffect(_)));
        assert_eq!(std::fs::read(stream).unwrap(), b"late directory data"); assert!(fixture.source.is_dir());
    }
    #[test]
    fn moved_root_or_native_share_refusal_preserves_the_original() {
        let fixture = Fixture::new(); let mut storage = fixture.storage();
        let id = storage.selected_source_root().unwrap().identity().clone();
        let mut cleanup = storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).unwrap();
        let moved = fixture.source.with_file_name("moved"); let did_move = std::cell::Cell::new(false);
        let outcome = dispose_directory_with(cleanup.file.take().unwrap(), cleanup.ancestors.last().unwrap(), cleanup.observed.clone(), &AtomicBool::new(false),
            |point| { if point == CleanupCheckpoint::BeforeDisposition {
                match std::fs::rename(&fixture.source, &moved) {
                    Ok(()) => { did_move.set(true); std::fs::create_dir(&fixture.source).map_err(io_error)?; }
                    Err(error) => { assert_eq!(error.raw_os_error(), Some(32)); return Err(io_error(error)); }
                }
            } Ok(()) }, close_directory_once);
        assert!(matches!(outcome, MutationOutcome::NoEffect(_))); assert!(fixture.source.is_dir());
        if did_move.get() { assert!(moved.is_dir()); }
    }

fn set_ea(path: &Path) -> Result<(), MutationError> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows::Wdk::Storage::FileSystem::{FILE_FULL_EA_INFORMATION, NtSetEaFile};
    let file = std::fs::OpenOptions::new()
        .access_mode(FILE_WRITE_EA.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
        .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
        .open(path)
        .map_err(io_error)?;
    let name = b"SORTER_ALBUM_TEST";
    let offset = std::mem::offset_of!(FILE_FULL_EA_INFORMATION, EaName);
    let length = offset + name.len() + 2;
    let mut allocation = vec![0u32; length.div_ceil(size_of::<u32>())];
    let pointer = allocation.as_mut_ptr().cast::<FILE_FULL_EA_INFORMATION>();
    unsafe {
        (*pointer).NextEntryOffset = 0;
        (*pointer).Flags = 0;
        (*pointer).EaNameLength = name.len() as u8;
        (*pointer).EaValueLength = 1;
        std::ptr::copy_nonoverlapping(name.as_ptr(), pointer.cast::<u8>().add(offset), name.len());
        *pointer.cast::<u8>().add(offset + name.len() + 1) = b'x';
    }
    let mut status_block = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtSetEaFile(
            HANDLE(file.as_raw_handle()),
            &mut status_block,
            pointer.cast(),
            length as u32,
        )
    };
    if status.0 != 0 || unsafe { status_block.Anonymous.Status }.0 != 0 {
        return Err(err(
            MutationErrorKind::Io,
            &format!("fixture EA failed: {status:?}"),
        ));
    }
    Ok(())
}

    #[test]
    fn directory_ea_refuses_preview_and_late_dispatch() {
        let initial = Fixture::new(); set_ea(&initial.source).unwrap();
        assert!(WindowsMutationStorage::for_selected_album_move_roots(grant(&initial.source), grant(&initial.target), grant(&initial.journal), &AtomicBool::new(false)).is_err());
        assert!(initial.source.is_dir());
        let fixture = Fixture::new(); let mut storage = fixture.storage();
        let id = storage.selected_source_root().unwrap().identity().clone();
        let mut cleanup = storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).unwrap();
        let inserted = std::cell::Cell::new(false);
        let outcome = dispose_directory_with(cleanup.file.take().unwrap(), cleanup.ancestors.last().unwrap(), cleanup.observed.clone(), &AtomicBool::new(false),
            |point| { if point == CleanupCheckpoint::BeforeDisposition {
                match set_ea(&fixture.source) {
                    Ok(()) => inserted.set(true),
                    Err(error) => { assert_eq!(error.kind, MutationErrorKind::Locked); return Err(error); }
                }
            } Ok(()) }, close_directory_once);
        assert!(matches!(outcome, MutationOutcome::NoEffect(_))); assert!(fixture.source.is_dir());
        if inserted.get() {
            assert!(WindowsMutationStorage::for_selected_album_move_roots(grant(&fixture.source), grant(&fixture.target), grant(&fixture.journal), &AtomicBool::new(false)).is_err());
        }
    }

    #[test]
    fn captured_child_directory_metadata_survives_source_retirement_changes() {
        use std::os::windows::fs::OpenOptionsExt;
        let fixture = Fixture::new(); std::fs::create_dir(fixture.source.join("disc")).unwrap();
        let storage = fixture.storage();
        let source = storage.parent_reference(&StorageRef::WindowsPath { utf16: fixture.source.join("disc").as_os_str().encode_wide().collect() }).unwrap();
        let before = repair_basic(source.handle()).unwrap();
        let captured = storage.capture_album_move_directory_metadata(&source, &AtomicBool::new(false)).unwrap();
        let target = storage.selected_target_root().unwrap();
        let mut created = applied(storage.create_selected_copy_directory(&target, &MutationName::Windows("disc".encode_utf16().collect()), &AtomicBool::new(false)));
        let write = std::fs::OpenOptions::new().access_mode(FILE_WRITE_ATTRIBUTES.0)
            .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
            .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
            .open(fixture.source.join("disc")).unwrap();
        let changed = (before.LastWriteTime + 20_000_000) as u64;
        let changed = FILETIME { dwLowDateTime: changed as u32, dwHighDateTime: (changed >> 32) as u32 };
        unsafe { SetFileTime(HANDLE(write.as_raw_handle()), None, None, Some(&changed)) }.unwrap();
        drop(write);
        assert_ne!(repair_basic(source.handle()).unwrap().LastWriteTime, before.LastWriteTime);
        let evidence = applied(storage.finalize_album_move_directory_metadata(&source, &captured, &mut created, &AtomicBool::new(false)));
        assert_eq!(evidence.write_time, before.LastWriteTime);
        assert_eq!(evidence.access_time, before.LastAccessTime);
    }

    #[test]
    fn root_relocation_before_take_cannot_become_a_new_baseline() {
        let fixture = Fixture::new();
        let other_parent = fixture.source.with_file_name("other-parent"); std::fs::create_dir(&other_parent).unwrap();
        let moved = other_parent.join("source");
        let mut storage = fixture.storage(); let id = storage.selected_source_root().unwrap().identity().clone();
        match std::fs::rename(&fixture.source, &moved) {
            Ok(()) => {
                std::fs::create_dir(&fixture.source).unwrap();
                assert!(storage.take_empty_album_source_root(&id, &AtomicBool::new(false)).is_err());
                drop(storage);
                assert_eq!(grant(&moved).expected_identity, id);
                assert_ne!(grant(&fixture.source).expected_identity, id);
                assert!(moved.is_dir()); assert!(fixture.source.is_dir());
            }
            Err(error) => {
                assert_eq!(error.raw_os_error(), Some(32));
                drop(storage); assert_eq!(grant(&fixture.source).expected_identity, id);
                assert!(!moved.exists());
            }
        }
    }

}
