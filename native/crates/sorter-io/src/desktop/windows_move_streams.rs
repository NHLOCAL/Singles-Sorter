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
        let main = self.acquire_source_with_mode(parent, expected, SourceLeaseMode::Ordinary)?;
        let file = main.file.as_ref().expect("new source lease");
        let attributes = move_representation(file)?;
        let inventory = stream_inventory(file)?;
        let mut streams = Vec::with_capacity(inventory.len());
        let mut named = Vec::new();
        for (suffix, length) in &inventory {
            cancelled(cancel)?;
            let digest = if suffix == &unnamed() {
                let actual = digest(file)?;
                if actual != expected.digest || actual.length != *length {
                    return Err(err(MutationErrorKind::StalePlan, "unnamed source stream changed"));
                }
                actual
            } else {
                let stream = open_named_stream(parent.handle(), units(&expected.name)?, suffix, false)?;
                if identity(&stream).map_err(read_error)? != expected.identity {
                    return Err(err(MutationErrorKind::StalePlan, "named stream file identity differs"));
                }
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
        self.validate_source(&source.main)?;
        let file = source.main.file.as_ref().expect("validated source");
        if move_representation(file)? != source.evidence.attributes {
            return Err(err(MutationErrorKind::StalePlan, "source representation changed"));
        }
        let expected = source.evidence.streams.iter().map(|stream| (stream.name.clone(), stream.length)).collect::<Vec<_>>();
        if stream_inventory(file)? != expected {
            return Err(err(MutationErrorKind::StalePlan, "source stream membership changed"));
        }
        for (stream, expected) in source.named.iter().zip(source.evidence.streams.iter().skip(1)) {
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
        let name = if name.is_empty() { unnamed() } else { name };
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
    fn malformed_inventory_rejects_bad_lengths_names_duplicate_and_negative_size() {
        fn record(name: &str) -> Vec<u8> {
            let name = name.encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>();
            let mut bytes = vec![0; 24 + name.len()];
            bytes[4..8].copy_from_slice(&(name.len() as u32).to_le_bytes());
            bytes[24..].copy_from_slice(&name);
            bytes
        }
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
