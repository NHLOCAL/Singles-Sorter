//! Preview-bound NTFS data-stream evidence for the separately scoped copied Move.
//! Known streams are leased; new external stream creation still requires the
//! documented ordinary-use assumption that no other writer changes the file.
use super::*;
use serde::{Deserialize, Serialize};

const MAX_STREAM_INVENTORY: usize = 1024 * 1024;
const MAX_STREAMS: usize = 256;

/// Data only. Deserialization cannot construct a source lease or authorize paths.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsMoveStreamEvidence {
    /// Native stream suffix, including leading ':' and trailing ':$DATA'.
    pub name: Vec<u16>,
    pub length: u64,
    pub sha256: [u8; 32],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowsMoveSourceEvidence {
    pub identity: ObjectIdentity,
    pub attributes: u32,
    /// Includes the unnamed stream first, then named streams in UTF-16 order.
    pub streams: Vec<WindowsMoveStreamEvidence>,
}

pub struct WindowsCopiedMoveSource {
    main: WindowsSourceLease,
    named: Vec<File>,
    evidence: WindowsMoveSourceEvidence,
}
impl WindowsCopiedMoveSource {
    pub fn evidence(&self) -> &WindowsMoveSourceEvidence {
        &self.evidence
    }
    pub fn observation(&self) -> &MutationObservation {
        &self.main.observation
    }
}
impl Read for WindowsCopiedMoveSource {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.main.read(buffer)
    }
}
impl Seek for WindowsCopiedMoveSource {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.main.seek(position)
    }
}

impl WindowsMutationStorage {
    /// Inspect under existing source-root authority. This never creates a file.
    pub fn inspect_copied_move_source(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        cancel: &AtomicBool,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        Ok(self.acquire_move_streams(parent, expected, None, cancel)?.evidence)
    }

    /// The exact preview inventory must survive revalidation before any effects.
    pub fn acquire_copied_move_source(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        preview: &WindowsMoveSourceEvidence,
        cancel: &AtomicBool,
    ) -> Result<WindowsCopiedMoveSource, MutationError> {
        self.acquire_move_streams(parent, expected, Some(preview), cancel)
    }

    fn acquire_move_streams(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        preview: Option<&WindowsMoveSourceEvidence>,
        cancel: &AtomicBool,
    ) -> Result<WindowsCopiedMoveSource, MutationError> {
        cancelled(cancel)?;
        if !matches!(self.domain, WindowsMutationDomain::SelectedRoots | WindowsMutationDomain::OwnedSyntheticTree) {
            return Err(err(MutationErrorKind::Unsupported, "copied Move source domain required"));
        }
        self.check_role(parent, RootRole::Source)?;
        if expected.parent_identity != parent.identity {
            return Err(err(MutationErrorKind::StalePlan, "source parent mismatch"));
        }
        let file = open_file(parent.handle(), units(&expected.name)?, false)?;
        let observation = verify_move_main(&file, parent, expected, cancel)?;
        let main = WindowsSourceLease {
            file: Some(file), parent: parent.clone(), expected: expected.clone(), observation,
            mode: SourceLeaseMode::Ordinary, repair: None,
        };
        let file = main.file.as_ref().expect("new source lease");
        let attributes = move_representation(file)?;
        let inventory = stream_inventory(file)?;
        let mut streams = Vec::with_capacity(inventory.len());
        let mut named = Vec::new();
        for (suffix, length) in &inventory {
            cancelled(cancel)?;
            let digest = if suffix == &unnamed() {
                let actual = stream_digest(file, expected.digest.length, cancel)?;
                if actual != expected.digest || actual.length != *length {
                    return Err(err(MutationErrorKind::StalePlan, "unnamed source stream changed"));
                }
                actual
            } else {
                let stream = open_named_stream(parent.handle(), units(&expected.name)?, suffix, false)?;
                if identity(&stream).map_err(read_error)? != expected.identity {
                    return Err(err(MutationErrorKind::StalePlan, "named stream file identity differs"));
                }
                move_representation(&stream)?;
                let actual = stream_digest(&stream, *length, cancel)?;
                named.push(stream);
                actual
            };
            streams.push(WindowsMoveStreamEvidence { name: suffix.clone(), length: *length, sha256: digest.sha256 });
        }
        if stream_inventory(file)? != inventory || move_representation(file)? != attributes {
            return Err(err(MutationErrorKind::StalePlan, "source stream representation changed during inspection"));
        }
        let evidence = WindowsMoveSourceEvidence { identity: expected.identity.clone(), attributes, streams };
        if preview.is_some_and(|preview| preview != &evidence) {
            return Err(err(MutationErrorKind::StalePlan, "source streams differ from preview"));
        }
        Ok(WindowsCopiedMoveSource { main, named, evidence })
    }

    pub fn verify_copied_move_source(
        &self,
        source: &WindowsCopiedMoveSource,
        cancel: &AtomicBool,
    ) -> Result<(), MutationError> {
        cancelled(cancel)?;
        self.check_role(&source.main.parent, RootRole::Source)?;
        let file = source.main.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "source consumed"))?;
        verify_move_main(file, &source.main.parent, &source.main.expected, cancel)?;
        if move_representation(file)? != source.evidence.attributes {
            return Err(err(MutationErrorKind::StalePlan, "source representation changed"));
        }
        let expected = source.evidence.streams.iter().map(|stream| (stream.name.clone(), stream.length)).collect::<Vec<_>>();
        if stream_inventory(file)? != expected {
            return Err(err(MutationErrorKind::StalePlan, "source stream membership changed"));
        }
        for (stream, expected) in source.named.iter().zip(source.evidence.streams.iter().skip(1)) {
            move_representation(stream)?;
            if stream_digest(stream, expected.length, cancel)?.sha256 != expected.sha256 {
                return Err(err(MutationErrorKind::StalePlan, "leased named stream content changed"));
            }
        }
        if source.named.len() + 1 != source.evidence.streams.len() {
            return Err(err(MutationErrorKind::StalePlan, "source stream lease set differs"));
        }
        cancelled(cancel)
    }
}

fn verify_move_main(
    file: &File, parent: &WindowsParent, expected: &ExpectedSource, cancel: &AtomicBool,
) -> Result<MutationObservation, MutationError> {
    cancelled(cancel)?;
    ensure_regular(file)?;
    let observed = observe(file, parent)?;
    if expected.parent_identity != parent.identity || observed.identity != expected.identity
        || observed.actual_name != expected.name
        || expected.size.is_some_and(|size| observed.size != Some(size))
        || stream_digest(file, expected.digest.length, cancel)? != expected.digest
        || expected.modified.as_ref().is_some_and(|expected| {
            file.metadata().and_then(|metadata| metadata.modified())
                .map(|time| format!("{time:?}")).as_ref().ok() != Some(expected)
        }) {
        return Err(err(MutationErrorKind::StalePlan, "source identity/name/content changed").with_observation(observed));
    }
    Ok(observed)
}
fn unnamed() -> Vec<u16> {
    "::$DATA".encode_utf16().collect()
}
fn move_representation(file: &File) -> Result<u32, MutationError> {
    ensure_regular(file)?;
    check_repair_extended_attributes(file)?;
    let basic = repair_basic(file)?;
    // No silent readonly override or dropping special source representations.
    if basic.FileAttributes & !(FILE_ATTRIBUTE_NORMAL.0 | FILE_ATTRIBUTE_ARCHIVE.0) != 0 {
        return Err(err(MutationErrorKind::Unsupported, "copied Move requires ordinary non-readonly representation"));
    }
    Ok(basic.FileAttributes & !FILE_ATTRIBUTE_NORMAL.0)
}
fn stream_inventory(file: &File) -> Result<Vec<(Vec<u16>, u64)>, MutationError> {
    let mut size = 4096;
    loop {
        let mut allocation = vec![0u64; size / 8];
        let result = unsafe { GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()), FileStreamInfo, allocation.as_mut_ptr().cast(), size as u32,
        ) };
        match result {
            Ok(()) => return parse_inventory(unsafe {
                std::slice::from_raw_parts(allocation.as_ptr().cast::<u8>(), size)
            }),
            Err(error) if matches!(error.code().0 as u32 & 0xffff, 122 | 234) && size < MAX_STREAM_INVENTORY => size *= 2,
            Err(error) => return Err(win_error(error)),
        }
    }
}
fn parse_inventory(bytes: &[u8]) -> Result<Vec<(Vec<u16>, u64)>, MutationError> {
    let invalid = || err(MutationErrorKind::Unsupported, "unbounded or malformed NTFS stream inventory");
    let header = std::mem::offset_of!(FILE_STREAM_INFO, StreamName);
    let mut offset = 0usize;
    let mut streams = Vec::new();
    loop {
        if streams.len() >= MAX_STREAMS || offset.checked_add(header).is_none_or(|end| end > bytes.len()) {
            return Err(invalid());
        }
        let next = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let size = i64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
        let start = offset + header;
        let end = start.checked_add(length).filter(|end| *end <= bytes.len()).ok_or_else(invalid)?;
        if !length.is_multiple_of(2) || size < 0 { return Err(invalid()); }
        let name = bytes[start..end].as_chunks::<2>().0.iter().map(|unit| u16::from_le_bytes(*unit)).collect::<Vec<_>>();
        if name.is_empty() { return Err(invalid()); }
        let suffix = ":$DATA".encode_utf16().collect::<Vec<_>>();
        if name.first() != Some(&58) || !name.ends_with(&suffix) || name.len() < suffix.len() + 1
            || name[1..name.len() - suffix.len()].iter().any(|u| matches!(*u, 0 | 47 | 58 | 92)) {
            return Err(invalid());
        }
        if streams.iter().any(|(existing, _)| existing == &name) { return Err(invalid()); }
        streams.push((name, size as u64));
        if next == 0 { break; }
        if !next.is_multiple_of(8) || next < header + length { return Err(invalid()); }
        offset = offset.checked_add(next).ok_or_else(invalid)?;
    }
    if !streams.iter().any(|(name, _)| name == &unnamed()) { return Err(invalid()); }
    streams.sort_by(|(a, _), (b, _)| (a != &unnamed()).cmp(&(b != &unnamed())).then(a.cmp(b)));
    Ok(streams)
}
fn stream_digest(file: &File, length: u64, cancel: &AtomicBool) -> Result<ContentDigest, MutationError> {
    stream_digest_with(file, length, cancel, |_| {})
}
fn stream_digest_with(
    file: &File, length: u64, cancel: &AtomicBool, mut checkpoint: impl FnMut(u64),
) -> Result<ContentDigest, MutationError> {
    let mut reader = file;
    reader.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        cancelled(cancel)?;
        let count = reader.read(&mut buffer).map_err(io_error)?;
        if count == 0 { break; }
        total = total.checked_add(count as u64).filter(|total| *total <= length)
            .ok_or_else(|| err(MutationErrorKind::StalePlan, "named stream exceeds preview length"))?;
        hash.update(&buffer[..count]);
        checkpoint(total);
    }
    if total != length { return Err(err(MutationErrorKind::StalePlan, "named stream length changed")); }
    Ok(ContentDigest { length, sha256: hash.finalize().into() })
}
fn open_named_stream(parent: &File, base: &[u16], suffix: &[u16], create: bool) -> Result<File, MutationError> {
    let mut name = base.iter().chain(suffix).copied().collect::<Vec<_>>();
    let byte_length = name.len().checked_mul(2).and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| err(MutationErrorKind::InvalidName, "stream path exceeds native bound"))?;
    let string = UNICODE_STRING { Length: byte_length, MaximumLength: byte_length, Buffer: PWSTR(name.as_mut_ptr()) };
    let attributes = OBJECT_ATTRIBUTES { Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.as_raw_handle()), ObjectName: &string,
        Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE, ..Default::default() };
    let mut handle = HANDLE::default();
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe { NtCreateFile(
        &mut handle, FILE_GENERIC_READ | if create { FILE_GENERIC_WRITE } else { FILE_ACCESS_RIGHTS(0) },
        &attributes, &mut ios, None, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE,
        if create { FILE_CREATE } else { FILE_OPEN },
        FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT | if create { FILE_WRITE_THROUGH } else { NTCREATEFILE_CREATE_OPTIONS(0) },
        None, 0,
    ) };
    if status.0 != 0 { return Err(io_error(io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32))); }
    if handle.is_invalid() { return Err(err(MutationErrorKind::UnknownOutcome, "stream open returned no handle")); }
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

#[cfg(test)]
mod move_stream_inventory_tests {
    use super::*;

    struct Fixture {
        storage: WindowsMutationStorage,
        parent: WindowsParent,
        expected: ExpectedSource,
        path: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("song.mp3");
            std::fs::write(&path, b"audio").unwrap();
            let storage = WindowsMutationStorage::for_owned_synthetic_tree(root).unwrap();
            let parent = storage.parent(Path::new("")).unwrap();
            let expected = ExpectedSource {
                identity: identity(chain(&path).unwrap().last().unwrap()).unwrap(),
                parent_identity: parent.identity().clone(),
                name: MutationName::Windows("song.mp3".encode_utf16().collect()),
                size: Some(5), modified: None,
                digest: ContentDigest { length: 5, sha256: Sha256::digest(b"audio").into() },
            };
            Self { storage, parent, expected, path }
        }
        fn ads(&self, name: &str) -> PathBuf {
            let mut path = self.path.as_os_str().to_os_string();
            path.push(format!(":{name}"));
            path.into()
        }
        fn inspect(&self) -> WindowsMoveSourceEvidence {
            self.storage.inspect_copied_move_source(&self.parent, &self.expected, &AtomicBool::new(false)).unwrap()
        }
    }

    #[test]
    fn preview_binds_named_stream_names_lengths_hashes_and_source_identity() {
        let f = Fixture::new();
        std::fs::write(f.ads("notes"), b"retained notes").unwrap();
        std::fs::write(f.ads("Zone.Identifier"), b"zone").unwrap();
        let preview = f.inspect();
        assert_eq!(preview.identity, f.expected.identity);
        assert_eq!(preview.streams.len(), 3);
        assert_eq!(preview.streams[0].name, unnamed());
        assert_eq!(preview.streams[0].sha256, f.expected.digest.sha256);
        let notes = preview.streams.iter().find(|stream| stream.name == ":notes:$DATA".encode_utf16().collect::<Vec<_>>()).unwrap();
        assert_eq!(notes.length, 14);
        assert_eq!(notes.sha256, Sha256::digest(b"retained notes").as_slice());
        let source = f.storage.acquire_copied_move_source(&f.parent, &f.expected, &preview, &AtomicBool::new(false)).unwrap();
        f.storage.verify_copied_move_source(&source, &AtomicBool::new(false)).unwrap();
        assert!(std::fs::write(f.ads("notes"), b"changed").is_err());
        assert!(std::fs::rename(&f.path, f.path.with_extension("renamed")).is_err());
    }

    #[test]
    fn changed_extra_missing_and_same_length_changed_streams_refuse_preview() {
        for change in 0..3 {
            let f = Fixture::new();
            std::fs::write(f.ads("notes"), b"first").unwrap();
            let preview = f.inspect();
            match change {
                0 => std::fs::write(f.ads("extra"), b"late").unwrap(),
                1 => std::fs::remove_file(f.ads("notes")).unwrap(),
                _ => std::fs::write(f.ads("notes"), b"other").unwrap(),
            }
            assert!(f.storage.acquire_copied_move_source(&f.parent, &f.expected, &preview, &AtomicBool::new(false)).is_err());
            assert_eq!(std::fs::read(&f.path).unwrap(), b"audio");
        }
    }

    #[test]
    fn late_stream_creation_is_observed_without_claiming_universal_exclusion() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = Fixture::new();
        let preview = f.inspect();
        let source = f.storage.acquire_copied_move_source(&f.parent, &f.expected, &preview, &AtomicBool::new(false)).unwrap();
        let opened = std::fs::OpenOptions::new().write(true).create_new(true)
            .share_mode(FILE_SHARE_DELETE.0).open(f.ads("late"));
        match opened {
            Ok(mut file) => {
                file.write_all(b"late bytes").unwrap();
                file.sync_all().unwrap();
                drop(file);
                assert!(f.storage.verify_copied_move_source(&source, &AtomicBool::new(false)).is_err());
            }
            Err(error) => assert_eq!(error.raw_os_error(), Some(32)),
        }
        drop(source);
        assert_eq!(std::fs::read(&f.path).unwrap(), b"audio");
    }

    #[test]
    fn readonly_and_cancel_refuse_before_effects() {
        let f = Fixture::new();
        let original = std::fs::metadata(&f.path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        std::fs::set_permissions(&f.path, readonly).unwrap();
        assert!(f.storage.inspect_copied_move_source(&f.parent, &f.expected, &AtomicBool::new(false)).is_err());
        std::fs::set_permissions(&f.path, original).unwrap();
        assert!(f.storage.inspect_copied_move_source(&f.parent, &f.expected, &AtomicBool::new(true)).is_err());
        assert_eq!(std::fs::read(&f.path).unwrap(), b"audio");
    }

    #[test]
    fn named_sparse_representation_refuses_without_flattening() {
        let f = Fixture::new();
        let path = f.ads("sparse");
        let stream = std::fs::OpenOptions::new().read(true).write(true).create_new(true).open(&path).unwrap();
        let mut returned = 0;
        unsafe { windows::Win32::System::IO::DeviceIoControl(
            HANDLE(stream.as_raw_handle()), 0x000900c4, // FSCTL_SET_SPARSE
            None, 0, None, 0, Some(&mut returned), None,
        ) }.unwrap();
        stream.set_len(1024 * 1024).unwrap();
        assert!(move_representation(&stream).is_err());
        drop(stream);
        assert!(f.storage.inspect_copied_move_source(&f.parent, &f.expected, &AtomicBool::new(false)).is_err());
        assert_eq!(std::fs::metadata(path).unwrap().len(), 1024 * 1024);
        assert_eq!(std::fs::read(&f.path).unwrap(), b"audio");
    }

    #[test]
    fn stream_hash_cancellation_stops_at_a_bounded_chunk_for_main_or_named_data() {
        let f = Fixture::new();
        let bytes = vec![7u8; 256 * 1024];
        std::fs::write(&f.path, &bytes).unwrap();
        let file = File::open(&f.path).unwrap();
        let cancel = AtomicBool::new(false);
        let mut observed = 0;
        let error = stream_digest_with(&file, bytes.len() as u64, &cancel, |count| {
            observed = count;
            cancel.store(true, Ordering::Release);
        }).unwrap_err();
        assert_eq!(error.kind, MutationErrorKind::Cancelled);
        assert_eq!(observed, 64 * 1024);
        assert_eq!(std::fs::read(&f.path).unwrap(), bytes);
    }

    #[test]
    fn malformed_inventory_rejects_bad_lengths_names_duplicate_and_negative_size() {
        fn record(name: &str) -> Vec<u8> {
            let name = name.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
            let mut bytes = vec![0; 24 + name.len()];
            bytes[4..8].copy_from_slice(&(name.len() as u32).to_le_bytes());
            bytes[24..].copy_from_slice(&name);
            bytes
        }
        assert!(parse_inventory(&[0u8; 24]).is_err());
        let valid = record("::$DATA");
        assert_eq!(parse_inventory(&valid).unwrap(), vec![(unnamed(), 0)]);
        for name in [":bad/name:$DATA", ":bad:name:$DATA", ":x:$INDEX_ALLOCATION", ":only:$DATA"] {
            assert!(parse_inventory(&record(name)).is_err());
        }
        let mut negative = valid.clone(); negative[8..16].copy_from_slice(&(-1i64).to_le_bytes());
        assert!(parse_inventory(&negative).is_err());
        let mut odd = valid.clone(); odd[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert!(parse_inventory(&odd).is_err());
        for length in 0..valid.len() { assert!(parse_inventory(&valid[..length]).is_err()); }
        let aligned = valid.len().next_multiple_of(8);
        let mut duplicate = valid.clone(); duplicate.resize(aligned, 0);
        duplicate[..4].copy_from_slice(&(aligned as u32).to_le_bytes());
        duplicate.extend_from_slice(&valid);
        assert!(parse_inventory(&duplicate).is_err());
    }
}

/// A temp with private named-stream handles. Its ordinary main stream is the
/// only writer exposed to the transfer driver; named writes use the bound copy.
pub struct WindowsCopiedMoveTemp {
    main: WindowsOwnedTemp,
    named: Vec<File>,
    evidence: Option<WindowsMoveSourceEvidence>,
    poisoned: bool,
}
pub struct WindowsCopiedMovePublished {
    main: WindowsPublished,
    named: Vec<File>,
    evidence: WindowsMoveSourceEvidence,
    poisoned: bool,
}
impl Write for WindowsCopiedMoveTemp {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.poisoned || self.evidence.is_some() {
            return Err(io::Error::other("copied Move temp is frozen or poisoned"));
        }
        self.main.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.main.flush()
    }
}
impl WindowsCopiedMoveTemp {
    pub fn observation(&self) -> &MutationObservation { self.main.observation() }
    pub fn sync_data(&self) -> Result<(), MutationError> { self.main.sync_data() }
    pub fn verify_content(&self, digest: &ContentDigest) -> Result<MutationObservation, MutationError> {
        if self.poisoned { return Err(err(MutationErrorKind::UnknownOutcome, "copied Move temp poisoned")); }
        self.main.verify_content(digest)
    }
}
impl WindowsCopiedMovePublished {
    pub fn observation(&self) -> &MutationObservation { self.main.observation() }
}

impl WindowsMutationStorage {
    pub fn create_copied_move_temp(
        &mut self, parent: &WindowsParent, job_id: &str, cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopiedMoveTemp> {
        match self.create_temp(parent, job_id, cancel) {
            MutationOutcome::Applied(main) => MutationOutcome::Applied(WindowsCopiedMoveTemp {
                main, named: Vec::new(), evidence: None, poisoned: false,
            }),
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }

    /// Must follow a synced auxiliary-stream write intent binding the complete
    /// preview and this owned temp identity. Any dispatched failure is uncertain.
    pub fn copy_bound_move_streams(
        &self, source: &WindowsCopiedMoveSource, temp: &mut WindowsCopiedMoveTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsMoveSourceEvidence> {
        self.copy_bound_move_streams_with(source, temp, cancel, |_| Ok(()))
    }

    fn copy_bound_move_streams_with(
        &self, source: &WindowsCopiedMoveSource, temp: &mut WindowsCopiedMoveTemp,
        cancel: &AtomicBool, mut checkpoint: impl FnMut(&str) -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsMoveSourceEvidence> {
        let before = (|| {
            checkpoint("before")?;
            if temp.poisoned || temp.evidence.is_some() {
                return Err(err(MutationErrorKind::StalePlan, "copied Move stream stage already attempted"));
            }
            self.verify_copied_move_source(source, cancel)?;
            self.check_role(&temp.main.parent, RootRole::Target)?;
            temp.main.verify_content(&source.main.expected.digest)?;
            let file = temp.main.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
            if stream_inventory(file)? != vec![(unnamed(), source.main.expected.digest.length)] {
                return Err(err(MutationErrorKind::StalePlan, "fresh temp has unexpected streams"));
            }
            Ok(())
        })();
        if let Err(error) = before { return MutationOutcome::NoEffect(error); }
        temp.poisoned = true;
        let staged = (|| {
            for (source_file, expected) in source.named.iter().zip(source.evidence.streams.iter().skip(1)) {
                cancelled(cancel)?;
                let mut destination = open_named_stream(
                    temp.main.parent.handle(), units(&temp.main.observation.actual_name)?, &expected.name, true,
                )?;
                checkpoint("after_create")?;
                if identity(&destination).map_err(read_error)? != temp.main.identity {
                    return Err(err(MutationErrorKind::VerificationFailed, "created stream belongs to another temp"));
                }
                let mut reader = source_file;
                reader.seek(SeekFrom::Start(0)).map_err(io_error)?;
                let mut bytes = [0u8; 64 * 1024];
                let mut total = 0u64;
                let mut hash = Sha256::new();
                loop {
                    cancelled(cancel)?;
                    let count = reader.read(&mut bytes).map_err(io_error)?;
                    if count == 0 { break; }
                    total = total.checked_add(count as u64).filter(|n| *n <= expected.length)
                        .ok_or_else(|| err(MutationErrorKind::StalePlan, "source named stream grew"))?;
                    destination.write_all(&bytes[..count]).map_err(io_error)?;
                    hash.update(&bytes[..count]);
                }
                checkpoint("after_write")?;
                let actual: [u8; 32] = hash.finalize().into();
                if total != expected.length || actual != expected.sha256 {
                    return Err(err(MutationErrorKind::StalePlan, "source named stream changed during copy"));
                }
                unsafe { FlushFileBuffers(HANDLE(destination.as_raw_handle())) }.map_err(win_error)?;
                // Freeze close-time access/write updates on every held stream,
                // before the shared file metadata finalizer sets copied times.
                let freeze = FILETIME { dwLowDateTime: u32::MAX, dwHighDateTime: u32::MAX };
                unsafe { SetFileTime(HANDLE(destination.as_raw_handle()), None, Some(&freeze), Some(&freeze)) }.map_err(win_error)?;
                if stream_digest(&destination, expected.length, cancel)?.sha256 != expected.sha256 {
                    return Err(err(MutationErrorKind::VerificationFailed, "named stream readback differs"));
                }
                unsafe { FlushFileBuffers(HANDLE(destination.as_raw_handle())) }.map_err(win_error)?;
                temp.named.push(destination);
                checkpoint("after_flush")?;
            }
            checkpoint("before_verify")?;
            self.verify_copied_move_source(source, cancel)?;
            verify_bound_streams(temp.main.file.as_ref().expect("retained temp"), &temp.named, &source.evidence, cancel)?;
            Ok(())
        })();
        if let Err(error) = staged {
            return MutationOutcome::PendingUnknown(err(
                MutationErrorKind::UnknownOutcome,
                &format!("auxiliary stream stage dispatched; reconcile retained source/temp: {error:?}"),
            ).with_observation(temp.main.observation.clone()));
        }
        temp.evidence = Some(source.evidence.clone());
        temp.poisoned = false;
        MutationOutcome::Applied(source.evidence.clone())
    }

    pub fn finalize_copied_move_metadata(
        &self, source: &WindowsCopiedMoveSource, temp: &mut WindowsCopiedMoveTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        if temp.poisoned || temp.evidence.as_ref() != Some(&source.evidence) {
            return MutationOutcome::NoEffect(err(MutationErrorKind::StalePlan, "auxiliary streams not verified"));
        }
        self.finalize_selected_copy_metadata(&source.main, &mut temp.main, cancel)
    }

    pub fn publish_copied_move_no_replace(
        &mut self, temp: &mut WindowsCopiedMoveTemp, parent: &WindowsParent,
        name: &MutationName, cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopiedMovePublished> {
        let Some(evidence) = temp.evidence.clone().filter(|_| !temp.poisoned) else {
            return MutationOutcome::NoEffect(err(MutationErrorKind::StalePlan, "stream stage unavailable"));
        };
        let checked = (|| {
            let file = temp.main.file.as_ref().ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
            verify_bound_streams(file, &temp.named, &evidence, cancel)
        })();
        if let Err(error) = checked { return MutationOutcome::NoEffect(error); }
        temp.poisoned = true;
        match self.publish_no_replace(&mut temp.main, parent, name, cancel) {
            MutationOutcome::Applied(main) => MutationOutcome::Applied(WindowsCopiedMovePublished {
                main, named: std::mem::take(&mut temp.named), evidence, poisoned: false,
            }),
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }

    pub fn verify_copied_move_publication(
        &mut self, published: &WindowsCopiedMovePublished, digest: &ContentDigest,
        cancel: &AtomicBool,
    ) -> Result<MutationObservation, MutationError> {
        if published.poisoned { return Err(err(MutationErrorKind::UnknownOutcome, "copied Move publication poisoned")); }
        let observation = self.verify(&published.main, digest)?;
        verify_bound_streams(&published.main.file, &published.named, &published.evidence, cancel)?;
        // Named-stream readback cannot silently change the copied file metadata.
        self.verify(&published.main, digest)?;
        Ok(observation)
    }

    pub fn sync_copied_move_publication(
        &mut self, published: &mut WindowsCopiedMovePublished, digest: &ContentDigest,
        cancel: &AtomicBool,
    ) -> Result<MutationObservation, MutationError> {
        self.verify_copied_move_publication(published, digest, cancel)?;
        published.poisoned = true;
        for file in &published.named {
            cancelled(cancel)?;
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
        }
        let acknowledgment = self.sync_published_file(&mut published.main, digest, cancel)?;
        let observation = acknowledgment.observation().clone();
        verify_bound_streams(&published.main.file, &published.named, &published.evidence, cancel)?;
        published.poisoned = false;
        Ok(observation)
    }
}
fn verify_bound_streams(
    main: &File, named: &[File], evidence: &WindowsMoveSourceEvidence, cancel: &AtomicBool,
) -> Result<(), MutationError> {
    let expected = evidence.streams.iter().map(|stream| (stream.name.clone(), stream.length)).collect::<Vec<_>>();
    if stream_inventory(main)? != expected || named.len() + 1 != expected.len() {
        return Err(err(MutationErrorKind::VerificationFailed, "destination stream membership differs"));
    }
    let main_id = identity(main).map_err(read_error)?;
    for (file, expected) in named.iter().zip(evidence.streams.iter().skip(1)) {
        move_representation(file)?;
        if identity(file).map_err(read_error)? != main_id || stream_digest(file, expected.length, cancel)?.sha256 != expected.sha256 {
            return Err(err(MutationErrorKind::VerificationFailed, "destination stream identity/content differs"));
        }
    }
    cancelled(cancel)
}

#[cfg(test)]
mod move_stream_copy_tests {
    use super::*;
    fn applied<T>(outcome: MutationOutcome<T>) -> T {
        match outcome { MutationOutcome::Applied(value) => value, _ => panic!("expected applied owned effect") }
    }
    struct Fixture {
        storage: WindowsMutationStorage,
        source: WindowsCopiedMoveSource,
        target: WindowsParent,
        path: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().to_path_buf();
            std::fs::create_dir(path.join("source")).unwrap();
            std::fs::create_dir(path.join("target")).unwrap();
            std::fs::write(path.join("source/song.mp3"), b"audio").unwrap();
            std::fs::write(path.join("source/song.mp3:notes"), b"notes").unwrap();
            std::fs::write(path.join("source/song.mp3:Zone.Identifier"), b"zone").unwrap();
            let storage = WindowsMutationStorage::for_owned_synthetic_tree(root).unwrap();
            let parent = storage.parent(Path::new("source")).unwrap();
            let target = storage.parent(Path::new("target")).unwrap();
            let expected = ExpectedSource {
                identity: identity(chain(&path.join("source/song.mp3")).unwrap().last().unwrap()).unwrap(),
                parent_identity: parent.identity().clone(), name: MutationName::Windows("song.mp3".encode_utf16().collect()),
                size: Some(5), modified: None, digest: ContentDigest { length: 5, sha256: Sha256::digest(b"audio").into() },
            };
            let preview = storage.inspect_copied_move_source(&parent, &expected, &AtomicBool::new(false)).unwrap();
            let source = storage.acquire_copied_move_source(&parent, &expected, &preview, &AtomicBool::new(false)).unwrap();
            Self { storage, source, target, path }
        }
        fn temp(&mut self) -> WindowsCopiedMoveTemp {
            let mut temp = applied(self.storage.create_copied_move_temp(&self.target, "streams", &AtomicBool::new(false)));
            temp.write_all(b"audio").unwrap();
            temp.sync_data().unwrap();
            temp
        }
    }

    #[test]
    fn named_streams_survive_verified_publication_metadata_and_handle_close() {
        let mut f = Fixture::new();
        let mut temp = f.temp();
        let cancel = AtomicBool::new(false);
        let evidence = applied(f.storage.copy_bound_move_streams(&f.source, &mut temp, &cancel));
        assert_eq!(evidence, *f.source.evidence());
        applied(f.storage.finalize_copied_move_metadata(&f.source, &mut temp, &cancel));
        let mut published = applied(f.storage.publish_copied_move_no_replace(
            &mut temp, &f.target, &MutationName::Windows("copy.mp3".encode_utf16().collect()), &cancel,
        ));
        f.storage.sync_copied_move_publication(&mut published, &f.source.main.expected.digest, &cancel).unwrap();
        f.storage.verify_copied_move_publication(&published, &f.source.main.expected.digest, &cancel).unwrap();
        drop(published);
        assert_eq!(std::fs::read(f.path.join("target/copy.mp3")).unwrap(), b"audio");
        assert_eq!(std::fs::read(f.path.join("target/copy.mp3:notes")).unwrap(), b"notes");
        assert_eq!(std::fs::read(f.path.join("target/copy.mp3:Zone.Identifier")).unwrap(), b"zone");
        drop(f.source);
        assert_eq!(std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(), b"notes");
    }

    #[test]
    fn stream_stage_faults_and_cancellation_keep_source_and_block_publication() {
        for point in ["before", "after_create", "after_write", "after_flush", "before_verify"] {
            for cancellation in [false, true] {
                let mut f = Fixture::new();
                let mut temp = f.temp();
                let cancel = AtomicBool::new(false);
                let outcome = f.storage.copy_bound_move_streams_with(&f.source, &mut temp, &cancel, |stage| {
                    if stage == point {
                        if cancellation { cancel.store(true, Ordering::Release); }
                        return Err(err(MutationErrorKind::Io, "owned lost stream response"));
                    }
                    Ok(())
                });
                if point == "before" { assert!(matches!(outcome, MutationOutcome::NoEffect(_))); }
                else { assert!(matches!(outcome, MutationOutcome::PendingUnknown(_))); }
                assert!(matches!(f.storage.publish_copied_move_no_replace(
                    &mut temp, &f.target, &MutationName::Windows("copy.mp3".encode_utf16().collect()), &cancel,
                ), MutationOutcome::NoEffect(_)));
                assert!(!f.path.join("target/copy.mp3").exists());
                drop(f.source);
                assert_eq!(std::fs::read(f.path.join("source/song.mp3")).unwrap(), b"audio");
                assert_eq!(std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(), b"notes");
            }
        }
    }

    #[test]
    fn collision_preserves_foreign_object_and_all_source_streams() {
        let mut f = Fixture::new();
        std::fs::write(f.path.join("target/copy.mp3"), b"foreign").unwrap();
        let mut temp = f.temp();
        let cancel = AtomicBool::new(false);
        applied(f.storage.copy_bound_move_streams(&f.source, &mut temp, &cancel));
        applied(f.storage.finalize_copied_move_metadata(&f.source, &mut temp, &cancel));
        assert!(!matches!(f.storage.publish_copied_move_no_replace(
            &mut temp, &f.target, &MutationName::Windows("copy.mp3".encode_utf16().collect()), &cancel,
        ), MutationOutcome::Applied(_)));
        assert_eq!(std::fs::read(f.path.join("target/copy.mp3")).unwrap(), b"foreign");
        drop(f.source);
        assert_eq!(std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(), b"notes");
    }
}
