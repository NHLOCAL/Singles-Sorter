//! Preview-bound NTFS data-stream evidence for the separately scoped copied Move.
//! Known streams are leased; new external stream creation still requires the
//! documented ordinary-use assumption that no other writer changes the file.
use super::*;

const MAX_STREAM_INVENTORY: usize = 1024 * 1024;
const MAX_STREAMS: usize = 256;

/// Native data only; the executor separately serializes its bound journal evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowsMoveStreamEvidence {
    /// Native stream suffix, including leading ':' and trailing ':$DATA'.
    pub name: Vec<u16>,
    pub length: u64,
    pub sha256: [u8; 32],
}
#[derive(Clone, Debug, PartialEq, Eq)]
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
    required_destinations: usize,
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
    /// Read-only full-stream observation of one exact file under a retained root.
    /// No deletion access or mutation-capable source lease is acquired. The caller
    /// must bind the returned identity and complete inventory to its own record.
    pub fn inspect_copied_move_streams(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        expected_identity: &ObjectIdentity,
        cancel: &AtomicBool,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        self.inspect_copied_move_streams_observed(parent, name, expected_identity, cancel, &mut |_| Ok(()))
    }

    /// Same exact read-only evidence, with cooperative probes and actual bytes.
    pub fn inspect_copied_move_streams_observed(
        &self,
        parent: &WindowsParent,
        name: &MutationName,
        expected_identity: &ObjectIdentity,
        cancel: &AtomicBool,
        observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        inspection_probe(cancel, observe, || self.check_parent(parent))?;
        if !matches!(parent.root.role, RootRole::Source | RootRole::Target | RootRole::Synthetic) {
            return Err(err(MutationErrorKind::Unsupported, "stream inspection requires media root"));
        }
        let file = inspection_probe(cancel, observe, || {
            open_relative_file(parent.handle(), units(name)?, false, FILE_GENERIC_READ)
        })?;
        if inspection_probe(cancel, observe, || identity(&file).map_err(read_error))? != *expected_identity {
            return Err(err(MutationErrorKind::StalePlan, "stream inspection identity changed"));
        }
        let attributes = inspection_probe(cancel, observe, || move_representation(&file))?;
        let inventory = inspection_probe(cancel, observe, || stream_inventory(&file))?;
        let mut streams = Vec::with_capacity(inventory.len());
        let mut held = Vec::new();
        for (suffix, length) in &inventory {
            cancelled(cancel)?;
            let digest = if suffix == &unnamed() {
                stream_digest_observed(&file, *length, cancel, observe)?
            } else {
                let stream = inspection_probe(cancel, observe, || {
                    open_named_stream(parent.handle(), units(name)?, suffix, false)
                })?;
                if inspection_probe(cancel, observe, || identity(&stream).map_err(read_error))? != *expected_identity {
                    return Err(err(MutationErrorKind::StalePlan, "inspected named stream identity changed"));
                }
                inspection_probe(cancel, observe, || move_representation(&stream))?;
                let digest = stream_digest_observed(&stream, *length, cancel, observe)?;
                held.push(stream);
                digest
            };
            streams.push(WindowsMoveStreamEvidence { name: suffix.clone(), length: *length, sha256: digest.sha256 });
        }
        if inspection_probe(cancel, observe, || stream_inventory(&file))? != inventory
            || inspection_probe(cancel, observe, || move_representation(&file))? != attributes {
            return Err(err(MutationErrorKind::StalePlan, "inspected stream inventory changed"));
        }
        inspection_probe(cancel, observe, || self.check_parent(parent))?;
        Ok(WindowsMoveSourceEvidence { identity: expected_identity.clone(), attributes, streams })
    }

    /// Inspect under existing source-root authority. This never creates a file.
    pub fn inspect_copied_move_source(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        cancel: &AtomicBool,
    ) -> Result<WindowsMoveSourceEvidence, MutationError> {
        Ok(self
            .acquire_move_streams(parent, expected, None, 1, cancel)?
            .evidence)
    }

    /// The exact preview inventory must survive revalidation before any effects.
    pub fn acquire_copied_move_source(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        preview: &WindowsMoveSourceEvidence,
        required_destinations: usize,
        cancel: &AtomicBool,
    ) -> Result<WindowsCopiedMoveSource, MutationError> {
        self.acquire_move_streams(
            parent,
            expected,
            Some(preview),
            required_destinations,
            cancel,
        )
    }

    fn acquire_move_streams(
        &self,
        parent: &WindowsParent,
        expected: &ExpectedSource,
        preview: Option<&WindowsMoveSourceEvidence>,
        required_destinations: usize,
        cancel: &AtomicBool,
    ) -> Result<WindowsCopiedMoveSource, MutationError> {
        cancelled(cancel)?;
        if required_destinations == 0 || required_destinations > 1024 {
            return Err(err(
                MutationErrorKind::Unsupported,
                "invalid copied Move destination cohort size",
            ));
        }
        if !matches!(
            self.domain,
            WindowsMutationDomain::SelectedRoots | WindowsMutationDomain::OwnedSyntheticTree
        ) {
            return Err(err(
                MutationErrorKind::Unsupported,
                "copied Move source domain required",
            ));
        }
        self.check_role(parent, RootRole::Source)?;
        if expected.parent_identity != parent.identity {
            return Err(err(MutationErrorKind::StalePlan, "source parent mismatch"));
        }
        let file = open_file(parent.handle(), units(&expected.name)?, false)?;
        let observation = verify_move_main(&file, parent, expected, cancel)?;
        let main = WindowsSourceLease {
            file: Some(file),
            parent: parent.clone(),
            expected: expected.clone(),
            observation,
            mode: SourceLeaseMode::Ordinary,
            repair: None,
        };
        let file = main.file.as_ref().expect("new source lease");
        require_move_volume(file)?;
        let attributes = move_representation(file)?;
        let inventory = stream_inventory(file)?;
        let mut streams = Vec::with_capacity(inventory.len());
        let mut named = Vec::new();
        for (suffix, length) in &inventory {
            cancelled(cancel)?;
            let digest = if suffix == &unnamed() {
                let actual = stream_digest(file, expected.digest.length, cancel)?;
                if actual != expected.digest || actual.length != *length {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "unnamed source stream changed",
                    ));
                }
                actual
            } else {
                let stream =
                    open_named_stream(parent.handle(), units(&expected.name)?, suffix, false)?;
                if identity(&stream).map_err(read_error)? != expected.identity {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "named stream file identity differs",
                    ));
                }
                move_representation(&stream)?;
                let actual = stream_digest(&stream, *length, cancel)?;
                named.push(stream);
                actual
            };
            streams.push(WindowsMoveStreamEvidence {
                name: suffix.clone(),
                length: *length,
                sha256: digest.sha256,
            });
        }
        if stream_inventory(file)? != inventory || move_representation(file)? != attributes {
            return Err(err(
                MutationErrorKind::StalePlan,
                "source stream representation changed during inspection",
            ));
        }
        let evidence = WindowsMoveSourceEvidence {
            identity: expected.identity.clone(),
            attributes,
            streams,
        };
        if preview.is_some_and(|preview| preview != &evidence) {
            return Err(err(
                MutationErrorKind::StalePlan,
                "source streams differ from preview",
            ));
        }
        Ok(WindowsCopiedMoveSource {
            main,
            named,
            evidence,
            required_destinations,
        })
    }

    pub fn verify_copied_move_source(
        &self,
        source: &WindowsCopiedMoveSource,
        cancel: &AtomicBool,
    ) -> Result<(), MutationError> {
        cancelled(cancel)?;
        self.check_role(&source.main.parent, RootRole::Source)?;
        let file = source
            .main
            .file
            .as_ref()
            .ok_or_else(|| err(MutationErrorKind::StalePlan, "source consumed"))?;
        verify_move_main(file, &source.main.parent, &source.main.expected, cancel)?;
        if move_representation(file)? != source.evidence.attributes {
            return Err(err(
                MutationErrorKind::StalePlan,
                "source representation changed",
            ));
        }
        let expected = source
            .evidence
            .streams
            .iter()
            .map(|stream| (stream.name.clone(), stream.length))
            .collect::<Vec<_>>();
        if stream_inventory(file)? != expected {
            return Err(err(
                MutationErrorKind::StalePlan,
                "source stream membership changed",
            ));
        }
        for (stream, expected) in source
            .named
            .iter()
            .zip(source.evidence.streams.iter().skip(1))
        {
            move_representation(stream)?;
            if stream_digest(stream, expected.length, cancel)?.sha256 != expected.sha256 {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "leased named stream content changed",
                ));
            }
        }
        if source.named.len() + 1 != source.evidence.streams.len() {
            return Err(err(
                MutationErrorKind::StalePlan,
                "source stream lease set differs",
            ));
        }
        cancelled(cancel)
    }
}

pub(super) fn verify_move_main(
    file: &File,
    parent: &WindowsParent,
    expected: &ExpectedSource,
    cancel: &AtomicBool,
) -> Result<MutationObservation, MutationError> {
    cancelled(cancel)?;
    ensure_regular(file)?;
    let observed = observe(file, parent)?;
    if expected.parent_identity != parent.identity
        || observed.identity != expected.identity
        || observed.actual_name != expected.name
        || expected
            .size
            .is_some_and(|size| observed.size != Some(size))
        || stream_digest(file, expected.digest.length, cancel)? != expected.digest
        || expected.modified.as_ref().is_some_and(|expected| {
            file.metadata()
                .and_then(|metadata| metadata.modified())
                .map(|time| format!("{time:?}"))
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
fn unnamed() -> Vec<u16> {
    "::$DATA".encode_utf16().collect()
}
fn move_representation(file: &File) -> Result<u32, MutationError> {
    ensure_regular(file)?;
    check_repair_extended_attributes(file)?;
    let basic = repair_basic(file)?;
    // No silent readonly override or dropping special source representations.
    if basic.FileAttributes & !(FILE_ATTRIBUTE_NORMAL.0 | FILE_ATTRIBUTE_ARCHIVE.0) != 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "copied Move requires ordinary non-readonly representation",
        ));
    }
    Ok(basic.FileAttributes & !FILE_ATTRIBUTE_NORMAL.0)
}
fn stream_inventory(file: &File) -> Result<Vec<(Vec<u16>, u64)>, MutationError> {
    let mut size = 4096;
    loop {
        let mut allocation = vec![0u64; size / 8];
        let result = unsafe {
            GetFileInformationByHandleEx(
                HANDLE(file.as_raw_handle()),
                FileStreamInfo,
                allocation.as_mut_ptr().cast(),
                size as u32,
            )
        };
        match result {
            Ok(()) => {
                return parse_inventory(unsafe {
                    std::slice::from_raw_parts(allocation.as_ptr().cast::<u8>(), size)
                });
            }
            Err(error)
                if matches!(error.code().0 as u32 & 0xffff, 122 | 234)
                    && size < MAX_STREAM_INVENTORY =>
            {
                size *= 2
            }
            Err(error) => return Err(win_error(error)),
        }
    }
}
fn parse_inventory(bytes: &[u8]) -> Result<Vec<(Vec<u16>, u64)>, MutationError> {
    let invalid = || {
        err(
            MutationErrorKind::Unsupported,
            "unbounded or malformed NTFS stream inventory",
        )
    };
    let header = std::mem::offset_of!(FILE_STREAM_INFO, StreamName);
    let mut offset = 0usize;
    let mut streams = Vec::new();
    loop {
        if streams.len() >= MAX_STREAMS
            || offset
                .checked_add(header)
                .is_none_or(|end| end > bytes.len())
        {
            return Err(invalid());
        }
        let next = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let size = i64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
        let start = offset + header;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(invalid)?;
        if !length.is_multiple_of(2) || size < 0 {
            return Err(invalid());
        }
        let name = bytes[start..end]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|unit| u16::from_le_bytes(*unit))
            .collect::<Vec<_>>();
        if name.is_empty() {
            return Err(invalid());
        }
        let suffix = ":$DATA".encode_utf16().collect::<Vec<_>>();
        if name.first() != Some(&58)
            || !name.ends_with(&suffix)
            || name.len() < suffix.len() + 1
            || name[1..name.len() - suffix.len()]
                .iter()
                .any(|u| matches!(*u, 0 | 47 | 58 | 92))
        {
            return Err(invalid());
        }
        if streams.iter().any(|(existing, _)| existing == &name) {
            return Err(invalid());
        }
        streams.push((name, size as u64));
        if next == 0 {
            break;
        }
        if !next.is_multiple_of(8) || next < header + length {
            return Err(invalid());
        }
        offset = offset.checked_add(next).ok_or_else(invalid)?;
    }
    if !streams.iter().any(|(name, _)| name == &unnamed()) {
        return Err(invalid());
    }
    streams.sort_by(|(a, _), (b, _)| (a != &unnamed()).cmp(&(b != &unnamed())).then(a.cmp(b)));
    Ok(streams)
}
pub(super) fn stream_digest(
    file: &File,
    length: u64,
    cancel: &AtomicBool,
) -> Result<ContentDigest, MutationError> {
    stream_digest_with(file, length, cancel, |_| {})
}
fn stream_digest_with(
    file: &File,
    length: u64,
    cancel: &AtomicBool,
    mut checkpoint: impl FnMut(u64),
) -> Result<ContentDigest, MutationError> {
    let mut total = 0u64;
    stream_digest_observed(file, length, cancel, &mut |event| {
        if let WindowsInspectionEvent::BytesRead(count) = event {
            total = total.checked_add(count as u64).ok_or_else(|| err(MutationErrorKind::Unsupported, "inspection counter overflow"))?;
            if count != 0 { checkpoint(total); }
        }
        Ok(())
    })
}
fn stream_digest_observed(
    file: &File,
    length: u64,
    cancel: &AtomicBool,
    observe: &mut dyn FnMut(WindowsInspectionEvent) -> Result<(), MutationError>,
) -> Result<ContentDigest, MutationError> {
    let mut reader = file;
    let original_position = inspection_probe(cancel, observe, || reader.stream_position().map_err(io_error))?;
    let result = (|| {
        inspection_probe(cancel, observe, || reader.seek(SeekFrom::Start(0)).map_err(io_error))?;
        let mut hash = Sha256::new();
        let mut total = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            cancelled(cancel)?;
            // Verify exact length and one-byte EOF, including growing streams.
            let amount = length.saturating_sub(total).min(buffer.len() as u64).max(1) as usize;
            let read = reader.read(&mut buffer[..amount]).map_err(io_error);
            let count = match read { Ok(count) => count, Err(error) => { cancelled(cancel)?; return Err(error); } };
            observe(WindowsInspectionEvent::BytesRead(count))?;
            cancelled(cancel)?;
            if count == 0 { break; }
            total = total.checked_add(count as u64).filter(|total| *total <= length)
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "stream exceeds preview length"))?;
            hash.update(&buffer[..count]);
        }
        if total != length { return Err(err(MutationErrorKind::StalePlan, "stream length changed")); }
        Ok(ContentDigest { length, sha256: hash.finalize().into() })
    })();
    // Restore even when stopped; this is read-handle cleanup, never a file write.
    let restored = reader.seek(SeekFrom::Start(original_position)).map_err(io_error);
    cancelled(cancel)?;
    match (result, restored) {
        (Ok(digest), Ok(_)) => Ok(digest),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

fn open_named_stream(
    parent: &File,
    base: &[u16],
    suffix: &[u16],
    create: bool,
) -> Result<File, MutationError> {
    let mut name = base.iter().chain(suffix).copied().collect::<Vec<_>>();
    let byte_length = name
        .len()
        .checked_mul(2)
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| {
            err(
                MutationErrorKind::InvalidName,
                "stream path exceeds native bound",
            )
        })?;
    let string = UNICODE_STRING {
        Length: byte_length,
        MaximumLength: byte_length,
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
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_GENERIC_READ
                | if create {
                    FILE_GENERIC_WRITE
                } else {
                    FILE_ACCESS_RIGHTS(0)
                },
            &attributes,
            &mut ios,
            None,
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_DELETE,
            if create { FILE_CREATE } else { FILE_OPEN },
            FILE_NON_DIRECTORY_FILE
                | FILE_OPEN_REPARSE_POINT
                | FILE_SYNCHRONOUS_IO_NONALERT
                | if create {
                    FILE_WRITE_THROUGH
                } else {
                    NTCREATEFILE_CREATE_OPTIONS(0)
                },
            None,
            0,
        )
    };
    if status.0 != 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(err(
            MutationErrorKind::UnknownOutcome,
            "stream open returned no handle",
        ));
    }
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
                size: Some(5),
                modified: None,
                digest: ContentDigest {
                    length: 5,
                    sha256: Sha256::digest(b"audio").into(),
                },
            };
            Self {
                storage,
                parent,
                expected,
                path,
            }
        }
        fn ads(&self, name: &str) -> PathBuf {
            let mut path = self.path.as_os_str().to_os_string();
            path.push(format!(":{name}"));
            path.into()
        }
        fn inspect(&self) -> WindowsMoveSourceEvidence {
            self.storage
                .inspect_copied_move_source(&self.parent, &self.expected, &AtomicBool::new(false))
                .unwrap()
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
        let notes = preview
            .streams
            .iter()
            .find(|stream| stream.name == ":notes:$DATA".encode_utf16().collect::<Vec<_>>())
            .unwrap();
        assert_eq!(notes.length, 14);
        assert_eq!(notes.sha256, Sha256::digest(b"retained notes").as_slice());
        let source = f
            .storage
            .acquire_copied_move_source(
                &f.parent,
                &f.expected,
                &preview,
                1,
                &AtomicBool::new(false),
            )
            .unwrap();
        f.storage
            .verify_copied_move_source(&source, &AtomicBool::new(false))
            .unwrap();
        assert!(std::fs::write(f.ads("notes"), b"changed").is_err());
        assert!(std::fs::rename(&f.path, f.path.with_extension("renamed")).is_err());
    }

    #[test]
    fn readonly_full_stream_probe_binds_identity_and_does_not_change_bytes() {
        let f = Fixture::new();
        std::fs::write(f.ads("notes"), b"retained notes").unwrap();
        let preview = f.inspect();
        let observed = f
            .storage
            .inspect_copied_move_streams(
                &f.parent,
                &f.expected.name,
                &f.expected.identity,
                &AtomicBool::new(false),
            )
            .unwrap();
        assert_eq!(observed, preview);
        let foreign = ObjectIdentity::Windows {
            volume: 0,
            file_id: [0; 16],
        };
        assert!(
            f.storage
                .inspect_copied_move_streams(
                    &f.parent,
                    &f.expected.name,
                    &foreign,
                    &AtomicBool::new(false)
                )
                .is_err()
        );
        assert!(
            f.storage
                .inspect_copied_move_streams(
                    &f.parent,
                    &f.expected.name,
                    &f.expected.identity,
                    &AtomicBool::new(true)
                )
                .is_err()
        );
        assert_eq!(std::fs::read(f.ads("notes")).unwrap(), b"retained notes");
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
            assert!(
                f.storage
                    .acquire_copied_move_source(
                        &f.parent,
                        &f.expected,
                        &preview,
                        1,
                        &AtomicBool::new(false)
                    )
                    .is_err()
            );
            assert_eq!(std::fs::read(&f.path).unwrap(), b"audio");
        }
    }

    #[test]
    fn late_stream_creation_is_observed_without_claiming_universal_exclusion() {
        use std::os::windows::fs::OpenOptionsExt;
        let f = Fixture::new();
        let preview = f.inspect();
        let source = f
            .storage
            .acquire_copied_move_source(
                &f.parent,
                &f.expected,
                &preview,
                1,
                &AtomicBool::new(false),
            )
            .unwrap();
        let opened = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_DELETE.0)
            .open(f.ads("late"));
        match opened {
            Ok(mut file) => {
                file.write_all(b"late bytes").unwrap();
                file.sync_all().unwrap();
                drop(file);
                assert!(
                    f.storage
                        .verify_copied_move_source(&source, &AtomicBool::new(false))
                        .is_err()
                );
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
        assert!(
            f.storage
                .inspect_copied_move_source(&f.parent, &f.expected, &AtomicBool::new(false))
                .is_err()
        );
        std::fs::set_permissions(&f.path, original).unwrap();
        assert!(
            f.storage
                .inspect_copied_move_source(&f.parent, &f.expected, &AtomicBool::new(true))
                .is_err()
        );
        assert_eq!(std::fs::read(&f.path).unwrap(), b"audio");
    }

    #[test]
    fn named_sparse_representation_refuses_without_flattening() {
        let f = Fixture::new();
        let path = f.ads("sparse");
        let stream = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut returned = 0;
        unsafe {
            windows::Win32::System::IO::DeviceIoControl(
                HANDLE(stream.as_raw_handle()),
                0x000900c4, // FSCTL_SET_SPARSE
                None,
                0,
                None,
                0,
                Some(&mut returned),
                None,
            )
        }
        .unwrap();
        stream.set_len(1024 * 1024).unwrap();
        assert!(move_representation(&stream).is_err());
        drop(stream);
        assert!(
            f.storage
                .inspect_copied_move_source(&f.parent, &f.expected, &AtomicBool::new(false))
                .is_err()
        );
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
        })
        .unwrap_err();
        assert_eq!(error.kind, MutationErrorKind::Cancelled);
        assert_eq!(observed, 64 * 1024);
        assert_eq!(std::fs::read(&f.path).unwrap(), bytes);
    }

    #[test]
    fn malformed_inventory_rejects_bad_lengths_names_duplicate_and_negative_size() {
        fn record(name: &str) -> Vec<u8> {
            let name = name
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>();
            let mut bytes = vec![0; 24 + name.len()];
            bytes[4..8].copy_from_slice(&(name.len() as u32).to_le_bytes());
            bytes[24..].copy_from_slice(&name);
            bytes
        }
        assert!(parse_inventory(&[0u8; 24]).is_err());
        let valid = record("::$DATA");
        assert_eq!(parse_inventory(&valid).unwrap(), vec![(unnamed(), 0)]);
        for name in [
            ":bad/name:$DATA",
            ":bad:name:$DATA",
            ":x:$INDEX_ALLOCATION",
            ":only:$DATA",
        ] {
            assert!(parse_inventory(&record(name)).is_err());
        }
        let mut negative = valid.clone();
        negative[8..16].copy_from_slice(&(-1i64).to_le_bytes());
        assert!(parse_inventory(&negative).is_err());
        let mut odd = valid.clone();
        odd[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert!(parse_inventory(&odd).is_err());
        for length in 0..valid.len() {
            assert!(parse_inventory(&valid[..length]).is_err());
        }
        let aligned = valid.len().next_multiple_of(8);
        let mut duplicate = valid.clone();
        duplicate.resize(aligned, 0);
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
    synced: bool,
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
    pub fn observation(&self) -> &MutationObservation {
        self.main.observation()
    }
    pub fn sync_data(&self) -> Result<(), MutationError> {
        self.main.sync_data()
    }
    pub fn verify_content_controlled(
        &self,
        digest: &ContentDigest,
        cancel: &AtomicBool,
    ) -> Result<MutationObservation, MutationError> {
        if self.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "copied Move temp poisoned",
            ));
        }
        self.main.verify_content_controlled(digest, cancel)
    }
    pub fn verify_content(
        &self,
        digest: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        if self.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "copied Move temp poisoned",
            ));
        }
        self.main.verify_content(digest)
    }
}
impl WindowsCopiedMovePublished {
    pub fn observation(&self) -> &MutationObservation {
        self.main.observation()
    }
    pub fn main_digest(&self) -> ContentDigest {
        let stream = &self.evidence.streams[0];
        ContentDigest {
            length: stream.length,
            sha256: stream.sha256,
        }
    }
}

impl WindowsMutationStorage {
    pub fn create_copied_move_temp(
        &mut self,
        parent: &WindowsParent,
        job_id: &str,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopiedMoveTemp> {
        match self.create_temp(parent, job_id, cancel) {
            MutationOutcome::Applied(main) => MutationOutcome::Applied(WindowsCopiedMoveTemp {
                main,
                named: Vec::new(),
                evidence: None,
                poisoned: false,
            }),
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }

    /// Must follow a synced auxiliary-stream write intent binding the complete
    /// preview and this owned temp identity. Any dispatched failure is uncertain.
    pub fn copy_bound_move_streams(
        &self,
        source: &WindowsCopiedMoveSource,
        temp: &mut WindowsCopiedMoveTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsMoveSourceEvidence> {
        self.copy_bound_move_streams_with(source, temp, cancel, |_| Ok(()))
    }

    fn copy_bound_move_streams_with(
        &self,
        source: &WindowsCopiedMoveSource,
        temp: &mut WindowsCopiedMoveTemp,
        cancel: &AtomicBool,
        mut checkpoint: impl FnMut(&str) -> Result<(), MutationError>,
    ) -> MutationOutcome<WindowsMoveSourceEvidence> {
        let before = (|| {
            checkpoint("before")?;
            if temp.poisoned || temp.evidence.is_some() {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "copied Move stream stage already attempted",
                ));
            }
            self.verify_copied_move_source(source, cancel)?;
            self.check_role(&temp.main.parent, RootRole::Target)?;
            temp.main
                .verify_content_controlled(&source.main.expected.digest, cancel)?;
            let file = temp
                .main
                .file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
            if stream_inventory(file)? != vec![(unnamed(), source.main.expected.digest.length)] {
                return Err(err(
                    MutationErrorKind::StalePlan,
                    "fresh temp has unexpected streams",
                ));
            }
            Ok(())
        })();
        if let Err(error) = before {
            return MutationOutcome::NoEffect(error);
        }
        temp.poisoned = true;
        let staged = (|| {
            for (source_file, expected) in source
                .named
                .iter()
                .zip(source.evidence.streams.iter().skip(1))
            {
                cancelled(cancel)?;
                let mut destination = open_named_stream(
                    temp.main.parent.handle(),
                    units(&temp.main.observation.actual_name)?,
                    &expected.name,
                    true,
                )?;
                checkpoint("after_create")?;
                if identity(&destination).map_err(read_error)? != temp.main.identity {
                    return Err(err(
                        MutationErrorKind::VerificationFailed,
                        "created stream belongs to another temp",
                    ));
                }
                let mut reader = source_file;
                reader.seek(SeekFrom::Start(0)).map_err(io_error)?;
                let mut bytes = [0u8; 64 * 1024];
                let mut total = 0u64;
                let mut hash = Sha256::new();
                loop {
                    cancelled(cancel)?;
                    let count = reader.read(&mut bytes).map_err(io_error)?;
                    if count == 0 {
                        break;
                    }
                    total = total
                        .checked_add(count as u64)
                        .filter(|n| *n <= expected.length)
                        .ok_or_else(|| {
                            err(MutationErrorKind::StalePlan, "source named stream grew")
                        })?;
                    destination.write_all(&bytes[..count]).map_err(io_error)?;
                    hash.update(&bytes[..count]);
                }
                checkpoint("after_write")?;
                let actual: [u8; 32] = hash.finalize().into();
                if total != expected.length || actual != expected.sha256 {
                    return Err(err(
                        MutationErrorKind::StalePlan,
                        "source named stream changed during copy",
                    ));
                }
                unsafe { FlushFileBuffers(HANDLE(destination.as_raw_handle())) }
                    .map_err(win_error)?;
                // Freeze close-time access/write updates on every held stream,
                // before the shared file metadata finalizer sets copied times.
                let freeze = FILETIME {
                    dwLowDateTime: u32::MAX,
                    dwHighDateTime: u32::MAX,
                };
                unsafe {
                    SetFileTime(
                        HANDLE(destination.as_raw_handle()),
                        None,
                        Some(&freeze),
                        Some(&freeze),
                    )
                }
                .map_err(win_error)?;
                if stream_digest(&destination, expected.length, cancel)?.sha256 != expected.sha256 {
                    return Err(err(
                        MutationErrorKind::VerificationFailed,
                        "named stream readback differs",
                    ));
                }
                unsafe { FlushFileBuffers(HANDLE(destination.as_raw_handle())) }
                    .map_err(win_error)?;
                temp.named.push(destination);
                checkpoint("after_flush")?;
            }
            checkpoint("before_verify")?;
            self.verify_copied_move_source(source, cancel)?;
            verify_bound_streams(
                temp.main.file.as_ref().expect("retained temp"),
                &temp.named,
                &source.evidence,
                cancel,
            )?;
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
        &self,
        source: &WindowsCopiedMoveSource,
        temp: &mut WindowsCopiedMoveTemp,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopyMetadataEvidence> {
        if temp.poisoned || temp.evidence.as_ref() != Some(&source.evidence) {
            return MutationOutcome::NoEffect(err(
                MutationErrorKind::StalePlan,
                "auxiliary streams not verified",
            ));
        }
        self.finalize_selected_copy_metadata_controlled(&source.main, &mut temp.main, cancel)
    }

    pub fn publish_copied_move_no_replace(
        &mut self,
        temp: &mut WindowsCopiedMoveTemp,
        parent: &WindowsParent,
        name: &MutationName,
        cancel: &AtomicBool,
    ) -> MutationOutcome<WindowsCopiedMovePublished> {
        if temp.main.copy_metadata.is_none() {
            return MutationOutcome::NoEffect(err(
                MutationErrorKind::StalePlan,
                "copied metadata must be acknowledged before publication",
            ));
        }
        let Some(evidence) = temp.evidence.clone().filter(|_| !temp.poisoned) else {
            return MutationOutcome::NoEffect(err(
                MutationErrorKind::StalePlan,
                "stream stage unavailable",
            ));
        };
        let checked = (|| {
            let file = temp
                .main
                .file
                .as_ref()
                .ok_or_else(|| err(MutationErrorKind::StalePlan, "temp consumed"))?;
            verify_bound_streams(file, &temp.named, &evidence, cancel)
        })();
        if let Err(error) = checked {
            return MutationOutcome::NoEffect(error);
        }
        temp.poisoned = true;
        match self.publish_finalized_copy_controlled(&mut temp.main, parent, name, cancel) {
            MutationOutcome::Applied(main) => {
                MutationOutcome::Applied(WindowsCopiedMovePublished {
                    main,
                    named: std::mem::take(&mut temp.named),
                    evidence,
                    poisoned: false,
                    synced: false,
                })
            }
            MutationOutcome::NoEffect(error) => MutationOutcome::NoEffect(error),
            MutationOutcome::PendingUnknown(error) => MutationOutcome::PendingUnknown(error),
        }
    }

    /// Namespace/metadata observation only; full hashes require cancellable verification.
    pub fn inspect_copied_move_publication(
        &mut self,
        published: &WindowsCopiedMovePublished,
    ) -> Result<MutationObservation, MutationError> {
        if published.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "copied Move publication poisoned",
            ));
        }
        self.inspect(&published.main)
    }

    pub fn verify_copied_move_publication(
        &mut self,
        published: &WindowsCopiedMovePublished,
        digest: &ContentDigest,
        cancel: &AtomicBool,
    ) -> Result<MutationObservation, MutationError> {
        self.verify_copied_move_publication_with(published, digest, cancel, |_| {})
    }
    fn verify_copied_move_publication_with(
        &mut self,
        published: &WindowsCopiedMovePublished,
        digest: &ContentDigest,
        cancel: &AtomicBool,
        checkpoint: impl FnMut(u64),
    ) -> Result<MutationObservation, MutationError> {
        if published.poisoned {
            return Err(err(
                MutationErrorKind::UnknownOutcome,
                "copied Move publication poisoned",
            ));
        }
        cancelled(cancel)?;
        let observation = self.inspect(&published.main)?;
        if published.main_digest() != *digest {
            return Err(err(
                MutationErrorKind::VerificationFailed,
                "bound published main digest differs",
            ));
        }
        verify_bound_streams_with(
            &published.main.file,
            &published.named,
            &published.evidence,
            cancel,
            checkpoint,
        )?;
        // Named-stream readback cannot silently change the copied file metadata.
        self.inspect(&published.main)?;
        cancelled(cancel)?;
        Ok(observation)
    }

    pub fn sync_copied_move_publication(
        &mut self,
        published: &mut WindowsCopiedMovePublished,
        digest: &ContentDigest,
        cancel: &AtomicBool,
    ) -> Result<MutationObservation, MutationError> {
        self.verify_copied_move_publication(published, digest, cancel)?;
        published.poisoned = true;
        for file in &published.named {
            cancelled(cancel)?;
            unsafe { FlushFileBuffers(HANDLE(file.as_raw_handle())) }.map_err(win_error)?;
        }
        cancelled(cancel)?;
        published.main.sync_poisoned = true;
        unsafe { FlushFileBuffers(HANDLE(published.main.file.as_raw_handle())) }
            .map_err(win_error)?;
        // Full post-flush verification remains chunk-cancellable. Once flush was
        // dispatched, every failure keeps both private publication guards poisoned.
        published.poisoned = false;
        let checked = self.verify_copied_move_publication(published, digest, cancel);
        match checked {
            Ok(observation) => {
                published.main.sync_poisoned = false;
                published.synced = true;
                Ok(observation)
            }
            Err(error) => {
                published.poisoned = true;
                Err(error)
            }
        }
    }
}
fn verify_bound_streams(
    main: &File,
    named: &[File],
    evidence: &WindowsMoveSourceEvidence,
    cancel: &AtomicBool,
) -> Result<(), MutationError> {
    verify_bound_streams_with(main, named, evidence, cancel, |_| {})
}
fn verify_bound_streams_with(
    main: &File,
    named: &[File],
    evidence: &WindowsMoveSourceEvidence,
    cancel: &AtomicBool,
    mut checkpoint: impl FnMut(u64),
) -> Result<(), MutationError> {
    let expected = evidence
        .streams
        .iter()
        .map(|stream| (stream.name.clone(), stream.length))
        .collect::<Vec<_>>();
    if stream_inventory(main)? != expected || named.len() + 1 != expected.len() {
        return Err(err(
            MutationErrorKind::VerificationFailed,
            "destination stream membership differs",
        ));
    }
    let expected_main = evidence.streams.first().ok_or_else(|| {
        err(
            MutationErrorKind::VerificationFailed,
            "missing bound main stream",
        )
    })?;
    if stream_digest_with(main, expected_main.length, cancel, &mut checkpoint)?.sha256
        != expected_main.sha256
    {
        return Err(err(
            MutationErrorKind::VerificationFailed,
            "destination main content differs",
        ));
    }
    let main_id = identity(main).map_err(read_error)?;
    for (file, expected) in named.iter().zip(evidence.streams.iter().skip(1)) {
        move_representation(file)?;
        if identity(file).map_err(read_error)? != main_id
            || stream_digest_with(file, expected.length, cancel, &mut checkpoint)?.sha256
                != expected.sha256
        {
            return Err(err(
                MutationErrorKind::VerificationFailed,
                "destination stream identity/content differs",
            ));
        }
    }
    cancelled(cancel)
}

#[cfg(test)]
mod move_stream_copy_tests {
    use super::*;
    fn applied<T>(outcome: MutationOutcome<T>) -> T {
        match outcome {
            MutationOutcome::Applied(value) => value,
            _ => panic!("expected applied owned effect"),
        }
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
                identity: identity(
                    chain(&path.join("source/song.mp3"))
                        .unwrap()
                        .last()
                        .unwrap(),
                )
                .unwrap(),
                parent_identity: parent.identity().clone(),
                name: MutationName::Windows("song.mp3".encode_utf16().collect()),
                size: Some(5),
                modified: None,
                digest: ContentDigest {
                    length: 5,
                    sha256: Sha256::digest(b"audio").into(),
                },
            };
            let preview = storage
                .inspect_copied_move_source(&parent, &expected, &AtomicBool::new(false))
                .unwrap();
            let source = storage
                .acquire_copied_move_source(
                    &parent,
                    &expected,
                    &preview,
                    1,
                    &AtomicBool::new(false),
                )
                .unwrap();
            Self {
                storage,
                source,
                target,
                path,
            }
        }
        fn temp(&mut self) -> WindowsCopiedMoveTemp {
            let mut temp = applied(self.storage.create_copied_move_temp(
                &self.target,
                "streams",
                &AtomicBool::new(false),
            ));
            self.source.seek(SeekFrom::Start(0)).unwrap();
            assert_eq!(std::io::copy(&mut self.source, &mut temp).unwrap(), 5);
            temp.sync_data().unwrap();
            temp
        }
    }

    #[test]
    fn publication_full_stream_verification_honors_mid_read_cancellation() {
        let mut f = Fixture::new();
        let mut temp = f.temp();
        applied(
            f.storage
                .copy_bound_move_streams(&f.source, &mut temp, &AtomicBool::new(false)),
        );
        applied(f.storage.finalize_copied_move_metadata(
            &f.source,
            &mut temp,
            &AtomicBool::new(false),
        ));
        let published = applied(f.storage.publish_copied_move_no_replace(
            &mut temp,
            &f.target,
            &MutationName::Windows("song.mp3".encode_utf16().collect()),
            &AtomicBool::new(false),
        ));
        let cancel = AtomicBool::new(false);
        let mut bytes = 0;
        let error = f
            .storage
            .verify_copied_move_publication_with(
                &published,
                &f.source.main.expected.digest,
                &cancel,
                |count| {
                    bytes += count;
                    cancel.store(true, Ordering::Release);
                },
            )
            .unwrap_err();
        assert_eq!(error.kind, MutationErrorKind::Cancelled);
        assert_eq!(bytes, 5);
        assert!(f.path.join("source/song.mp3").exists());
        assert!(matches!(
            f.storage
                .retire_copied_move_source(&mut f.source, &[&published], &cancel),
            MutationOutcome::NoEffect(_)
        ));
    }

    #[test]
    fn controlled_temp_metadata_and_publication_refuse_cancel_before_dispatch() {
        let mut f = Fixture::new();
        let mut temp = f.temp();
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            temp.verify_content_controlled(&f.source.main.expected.digest, &cancelled)
                .unwrap_err()
                .kind,
            MutationErrorKind::Cancelled
        );
        assert!(
            matches!(f.storage.copy_bound_move_streams(&f.source, &mut temp, &cancelled), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::Cancelled)
        );
        applied(
            f.storage
                .copy_bound_move_streams(&f.source, &mut temp, &AtomicBool::new(false)),
        );
        assert!(
            matches!(f.storage.finalize_copied_move_metadata(&f.source, &mut temp, &cancelled), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::Cancelled)
        );
        applied(f.storage.finalize_copied_move_metadata(
            &f.source,
            &mut temp,
            &AtomicBool::new(false),
        ));
        assert!(
            matches!(f.storage.publish_copied_move_no_replace(&mut temp, &f.target, &MutationName::Windows("song.mp3".encode_utf16().collect()), &cancelled), MutationOutcome::NoEffect(error) if error.kind == MutationErrorKind::Cancelled)
        );
        assert!(!f.path.join("target/song.mp3").exists());
        assert!(f.path.join("source/song.mp3").exists());
    }

    #[test]
    fn controlled_mixed_route_renames_entire_original_with_auxiliary_streams() {
        let mut f = Fixture::new();
        let expected = f.source.main.expected.clone();
        drop(f.source);
        let parent = f.storage.parent(Path::new("source")).unwrap();
        let mut source = f
            .storage
            .acquire_selected_rename_source_controlled(&parent, &expected, &AtomicBool::new(false))
            .unwrap();
        let published = applied(f.storage.rename_selected_source_controlled(
            &mut source,
            &f.target,
            &MutationName::Windows("renamed.mp3".encode_utf16().collect()),
            &AtomicBool::new(false),
        ));
        assert_eq!(published.observation.identity, expected.identity);
        f.storage
            .verify_selected_rename_controlled(
                &published,
                &expected.digest,
                &AtomicBool::new(false),
            )
            .unwrap();
        drop(published);
        assert!(!f.path.join("source/song.mp3").exists());
        assert_eq!(
            std::fs::read(f.path.join("target/renamed.mp3:notes")).unwrap(),
            b"notes"
        );
        assert_eq!(
            std::fs::read(f.path.join("target/renamed.mp3:Zone.Identifier")).unwrap(),
            b"zone"
        );
        assert_eq!(f.storage.capabilities().durable_flush, Capability::Unknown);
    }
    #[test]
    fn controlled_mixed_rename_faults_keep_dispatch_uncertainty_and_identity() {
        for boundary in [
            RenameCheckpoint::BeforePreflush,
            RenameCheckpoint::AfterPreflush,
            RenameCheckpoint::BeforeRename,
            RenameCheckpoint::AfterRename,
            RenameCheckpoint::BeforePostflush,
            RenameCheckpoint::AfterPostflush,
        ] {
            let mut f = Fixture::new();
            let expected = f.source.main.expected.clone();
            drop(f.source);
            let parent = f.storage.parent(Path::new("source")).unwrap();
            let mut source = f
                .storage
                .acquire_selected_rename_source_controlled(
                    &parent,
                    &expected,
                    &AtomicBool::new(false),
                )
                .unwrap();
            let outcome = f.storage.rename_write_through_inner(
                &mut source,
                &f.target,
                &MutationName::Windows("renamed.mp3".encode_utf16().collect()),
                &AtomicBool::new(false),
                true,
                |checkpoint| {
                    if checkpoint == boundary {
                        Err(err(
                            MutationErrorKind::UnknownOutcome,
                            "lost controlled rename response",
                        ))
                    } else {
                        Ok(())
                    }
                },
            );
            if matches!(
                boundary,
                RenameCheckpoint::BeforePreflush
                    | RenameCheckpoint::AfterPreflush
                    | RenameCheckpoint::BeforeRename
            ) {
                assert!(matches!(outcome, MutationOutcome::NoEffect(_)));
                assert!(f.path.join("source/song.mp3").exists());
            } else {
                assert!(matches!(outcome, MutationOutcome::PendingUnknown(_)));
                assert!(!f.path.join("source/song.mp3").exists());
                assert_eq!(
                    std::fs::read(f.path.join("target/renamed.mp3:notes")).unwrap(),
                    b"notes"
                );
                assert!(source.file.is_none());
            }
        }
    }

    #[test]
    fn acquisition_and_reverification_preserve_main_reader_position() {
        let mut f = Fixture::new();
        let mut first = [0u8; 2];
        f.source.read_exact(&mut first).unwrap();
        assert_eq!(&first, b"au");
        f.storage
            .verify_copied_move_source(&f.source, &AtomicBool::new(false))
            .unwrap();
        let mut rest = Vec::new();
        f.source.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, b"dio");
    }

    #[test]
    fn publication_refuses_missing_copied_metadata_without_namespace_effect() {
        let mut f = Fixture::new();
        let mut temp = f.temp();
        let cancel = AtomicBool::new(false);
        applied(
            f.storage
                .copy_bound_move_streams(&f.source, &mut temp, &cancel),
        );
        assert!(matches!(
            f.storage.publish_copied_move_no_replace(
                &mut temp,
                &f.target,
                &MutationName::Windows("copy.mp3".encode_utf16().collect()),
                &cancel,
            ),
            MutationOutcome::NoEffect(_)
        ));
        assert!(!f.path.join("target/copy.mp3").exists());
        assert!(f.path.join("source/song.mp3").exists());
    }

    #[test]
    fn named_streams_survive_verified_publication_metadata_and_handle_close() {
        let mut f = Fixture::new();
        let mut temp = f.temp();
        let cancel = AtomicBool::new(false);
        let evidence = applied(
            f.storage
                .copy_bound_move_streams(&f.source, &mut temp, &cancel),
        );
        assert_eq!(evidence, *f.source.evidence());
        applied(
            f.storage
                .finalize_copied_move_metadata(&f.source, &mut temp, &cancel),
        );
        let mut published = applied(f.storage.publish_copied_move_no_replace(
            &mut temp,
            &f.target,
            &MutationName::Windows("copy.mp3".encode_utf16().collect()),
            &cancel,
        ));
        f.storage
            .sync_copied_move_publication(&mut published, &f.source.main.expected.digest, &cancel)
            .unwrap();
        f.storage
            .verify_copied_move_publication(&published, &f.source.main.expected.digest, &cancel)
            .unwrap();
        drop(published);
        assert_eq!(
            std::fs::read(f.path.join("target/copy.mp3")).unwrap(),
            b"audio"
        );
        assert_eq!(
            std::fs::read(f.path.join("target/copy.mp3:notes")).unwrap(),
            b"notes"
        );
        assert_eq!(
            std::fs::read(f.path.join("target/copy.mp3:Zone.Identifier")).unwrap(),
            b"zone"
        );
        drop(f.source);
        assert_eq!(
            std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(),
            b"notes"
        );
    }

    #[test]
    fn stream_stage_faults_and_cancellation_keep_source_and_block_publication() {
        for point in [
            "before",
            "after_create",
            "after_write",
            "after_flush",
            "before_verify",
        ] {
            for cancellation in [false, true] {
                let mut f = Fixture::new();
                let mut temp = f.temp();
                let cancel = AtomicBool::new(false);
                let outcome = f.storage.copy_bound_move_streams_with(
                    &f.source,
                    &mut temp,
                    &cancel,
                    |stage| {
                        if stage == point {
                            if cancellation {
                                cancel.store(true, Ordering::Release);
                            }
                            return Err(err(MutationErrorKind::Io, "owned lost stream response"));
                        }
                        Ok(())
                    },
                );
                if point == "before" {
                    assert!(matches!(outcome, MutationOutcome::NoEffect(_)));
                } else {
                    assert!(matches!(outcome, MutationOutcome::PendingUnknown(_)));
                }
                assert!(matches!(
                    f.storage.publish_copied_move_no_replace(
                        &mut temp,
                        &f.target,
                        &MutationName::Windows("copy.mp3".encode_utf16().collect()),
                        &cancel,
                    ),
                    MutationOutcome::NoEffect(_)
                ));
                assert!(!f.path.join("target/copy.mp3").exists());
                drop(f.source);
                assert_eq!(
                    std::fs::read(f.path.join("source/song.mp3")).unwrap(),
                    b"audio"
                );
                assert_eq!(
                    std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(),
                    b"notes"
                );
            }
        }
    }

    fn publish(f: &mut Fixture, name: &str, sync: bool) -> WindowsCopiedMovePublished {
        let mut temp = f.temp();
        let cancel = AtomicBool::new(false);
        applied(
            f.storage
                .copy_bound_move_streams(&f.source, &mut temp, &cancel),
        );
        applied(
            f.storage
                .finalize_copied_move_metadata(&f.source, &mut temp, &cancel),
        );
        let mut published = applied(f.storage.publish_copied_move_no_replace(
            &mut temp,
            &f.target,
            &MutationName::Windows(name.encode_utf16().collect()),
            &cancel,
        ));
        if sync {
            f.storage
                .sync_copied_move_publication(
                    &mut published,
                    &f.source.main.expected.digest,
                    &cancel,
                )
                .unwrap();
        }
        published
    }

    #[test]
    fn full_distinct_synced_cohort_is_required_before_exact_source_retirement() {
        let mut f = Fixture::new();
        f.source.required_destinations = 2;
        let first = publish(&mut f, "first.mp3", true);
        let mut second = publish(&mut f, "second.mp3", false);
        let cancel = AtomicBool::new(false);
        for cohort in [vec![&first], vec![&first, &first], vec![&first, &second]] {
            assert!(matches!(
                f.storage
                    .retire_copied_move_source(&mut f.source, &cohort, &cancel),
                MutationOutcome::NoEffect(_)
            ));
            assert!(f.path.join("source/song.mp3").exists());
        }
        f.storage
            .sync_copied_move_publication(&mut second, &f.source.main.expected.digest, &cancel)
            .unwrap();
        assert!(matches!(
            applied(f.storage.retire_copied_move_source(
                &mut f.source,
                &[&first, &second],
                &cancel
            )),
            RetirementOutcome::Retired(_)
        ));
        assert!(!f.path.join("source/song.mp3").exists());
        drop(first);
        drop(second);
        for name in ["first.mp3", "second.mp3"] {
            assert_eq!(
                std::fs::read(f.path.join(format!("target/{name}"))).unwrap(),
                b"audio"
            );
            assert_eq!(
                std::fs::read(f.path.join(format!("target/{name}:notes"))).unwrap(),
                b"notes"
            );
            assert_eq!(
                std::fs::read(f.path.join(format!("target/{name}:Zone.Identifier"))).unwrap(),
                b"zone"
            );
        }
    }

    #[test]
    fn retirement_faults_after_dispatch_never_report_no_effect_or_allow_replay() {
        for point in [
            "before_disposition",
            "after_disposition",
            "before_close",
            "after_close",
            "before_namespace_check",
            "after_namespace_check",
        ] {
            for cancellation in [false, true] {
                let mut f = Fixture::new();
                let published = publish(&mut f, "copy.mp3", true);
                let cancel = AtomicBool::new(false);
                let mut closes = 0;
                let result = f.storage.retire_copied_move_source_with(
                    &mut f.source,
                    &[&published],
                    &cancel,
                    |stage| {
                        if stage == point {
                            if cancellation {
                                cancel.store(true, Ordering::Release);
                            }
                            return Err(err(
                                MutationErrorKind::Io,
                                "owned lost retirement response",
                            ));
                        }
                        Ok(())
                    },
                    |handle| {
                        closes += 1;
                        unsafe { windows::Win32::Foundation::CloseHandle(handle) }
                            .map_err(win_error)
                    },
                );
                if point == "before_disposition" {
                    assert!(matches!(result, MutationOutcome::NoEffect(_)));
                    assert_eq!(closes, 0);
                    assert!(f.path.join("source/song.mp3").exists());
                } else {
                    assert!(matches!(result, MutationOutcome::PendingUnknown(_)));
                    assert_eq!(closes, 1);
                    assert!(f.source.main.file.is_none());
                    assert!(matches!(
                        f.storage.retire_copied_move_source(
                            &mut f.source,
                            &[&published],
                            &AtomicBool::new(false)
                        ),
                        MutationOutcome::NoEffect(_)
                    ));
                }
                drop(published);
                assert_eq!(
                    std::fs::read(f.path.join("target/copy.mp3:notes")).unwrap(),
                    b"notes"
                );
            }
        }
    }

    #[test]
    fn lost_close_response_consumes_handle_once_and_foreign_old_link_survives() {
        let mut f = Fixture::new();
        let published = publish(&mut f, "copy.mp3", true);
        let source_path = f.path.join("source/song.mp3");
        let mut closes = 0;
        let result = f.storage.retire_copied_move_source_with(
            &mut f.source,
            &[&published],
            &AtomicBool::new(false),
            |stage| {
                if stage == "after_close" {
                    std::fs::write(&source_path, b"foreign").unwrap();
                }
                Ok(())
            },
            |handle| {
                closes += 1;
                unsafe { windows::Win32::Foundation::CloseHandle(handle) }.map_err(win_error)?;
                Err(err(
                    MutationErrorKind::UnknownOutcome,
                    "owned lost close response",
                ))
            },
        );
        assert!(matches!(result, MutationOutcome::PendingUnknown(_)));
        assert_eq!(closes, 1);
        assert_eq!(std::fs::read(source_path).unwrap(), b"foreign");
        drop(published);
        assert_eq!(
            std::fs::read(f.path.join("target/copy.mp3:notes")).unwrap(),
            b"notes"
        );
    }

    #[test]
    fn late_source_stream_detected_at_final_retirement_check_retains_all_bytes() {
        use std::os::windows::fs::OpenOptionsExt;
        let mut f = Fixture::new();
        let published = publish(&mut f, "copy.mp3", true);
        let extra = f.path.join("source/song.mp3:late");
        let result = f.storage.retire_copied_move_source_with(
            &mut f.source,
            &[&published],
            &AtomicBool::new(false),
            |stage| {
                if stage == "before_disposition" {
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .share_mode(FILE_SHARE_DELETE.0)
                        .open(&extra)
                        .unwrap();
                    file.write_all(b"late source bytes").unwrap();
                    file.sync_all().unwrap();
                }
                Ok(())
            },
            |_| panic!("changed source must not reach close"),
        );
        assert!(matches!(result, MutationOutcome::NoEffect(_)));
        drop(f.source);
        assert_eq!(std::fs::read(extra).unwrap(), b"late source bytes");
        assert_eq!(
            std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(),
            b"notes"
        );
    }

    #[test]
    fn collision_preserves_foreign_object_and_all_source_streams() {
        let mut f = Fixture::new();
        std::fs::write(f.path.join("target/copy.mp3"), b"foreign").unwrap();
        let mut temp = f.temp();
        let cancel = AtomicBool::new(false);
        applied(
            f.storage
                .copy_bound_move_streams(&f.source, &mut temp, &cancel),
        );
        applied(
            f.storage
                .finalize_copied_move_metadata(&f.source, &mut temp, &cancel),
        );
        assert!(!matches!(
            f.storage.publish_copied_move_no_replace(
                &mut temp,
                &f.target,
                &MutationName::Windows("copy.mp3".encode_utf16().collect()),
                &cancel,
            ),
            MutationOutcome::Applied(_)
        ));
        assert_eq!(
            std::fs::read(f.path.join("target/copy.mp3")).unwrap(),
            b"foreign"
        );
        drop(f.source);
        assert_eq!(
            std::fs::read(f.path.join("source/song.mp3:notes")).unwrap(),
            b"notes"
        );
    }
}

fn require_move_volume(file: &File) -> Result<(), MutationError> {
    let mut flags = 0;
    unsafe {
        GetVolumeInformationByHandleW(
            HANDLE(file.as_raw_handle()),
            None,
            None,
            None,
            Some(&mut flags),
            None,
        )
    }
    .map_err(win_error)?;
    // FILE_SUPPORTS_POSIX_UNLINK_RENAME, Windows SDK volume capability bit.
    if flags & 0x0000_0400 == 0 {
        return Err(err(
            MutationErrorKind::Unsupported,
            "source volume lacks POSIX unlink support",
        ));
    }
    Ok(())
}

impl WindowsMutationStorage {
    /// Invoke only after the complete required cohort and retirement intent are
    /// durably journaled. This checks every held destination again, then freshly
    /// checks the complete source inventory before requesting exact-object unlink.
    /// Live namespace removal is verified; crash recovery never infers automatic
    /// cleanup authority from this acknowledgment alone.
    pub fn retire_copied_move_source(
        &mut self,
        source: &mut WindowsCopiedMoveSource,
        destinations: &[&WindowsCopiedMovePublished],
        cancel: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        self.retire_copied_move_source_with(
            source,
            destinations,
            cancel,
            |_| Ok(()),
            |handle| unsafe { windows::Win32::Foundation::CloseHandle(handle) }.map_err(win_error),
        )
    }

    fn retire_copied_move_source_with(
        &mut self,
        source: &mut WindowsCopiedMoveSource,
        destinations: &[&WindowsCopiedMovePublished],
        cancel: &AtomicBool,
        mut checkpoint: impl FnMut(&str) -> Result<(), MutationError>,
        close: impl FnOnce(HANDLE) -> Result<(), MutationError>,
    ) -> MutationOutcome<RetirementOutcome> {
        use std::os::windows::io::IntoRawHandle;
        use windows::Wdk::Storage::FileSystem::{
            FILE_DISPOSITION_DELETE, FILE_DISPOSITION_INFORMATION_EX,
            FILE_DISPOSITION_INFORMATION_EX_FLAGS, FILE_DISPOSITION_POSIX_SEMANTICS,
            FileDispositionInformationEx,
        };
        let before = (|| {
            cancelled(cancel)?;
            if destinations.len() != source.required_destinations {
                return Err(err(
                    MutationErrorKind::VerificationFailed,
                    "required copied Move destinations missing",
                ));
            }
            let mut ids = std::collections::HashSet::new();
            for destination in destinations {
                if !destination.synced
                    || destination.evidence != source.evidence
                    || !ids.insert(destination.main.observation.identity.clone())
                {
                    return Err(err(
                        MutationErrorKind::VerificationFailed,
                        "destination cohort is unsynced, foreign or duplicated",
                    ));
                }
                self.verify_copied_move_publication(
                    destination,
                    &source.main.expected.digest,
                    cancel,
                )?;
            }
            checkpoint("before_disposition")?;
            self.verify_copied_move_source(source, cancel)?;
            let file = source.main.file.as_ref().expect("verified source");
            require_move_volume(file)?;
            cancelled(cancel)?;
            Ok(source.main.observation.clone())
        })();
        let observed = match before {
            Ok(observed) => observed,
            Err(error) => return MutationOutcome::NoEffect(error),
        };
        let disposition = FILE_DISPOSITION_INFORMATION_EX {
            Flags: FILE_DISPOSITION_INFORMATION_EX_FLAGS(
                FILE_DISPOSITION_DELETE.0 | FILE_DISPOSITION_POSIX_SEMANTICS.0,
            ),
        };
        let mut ios = IO_STATUS_BLOCK::default();
        let status = unsafe {
            NtSetInformationFile(
                HANDLE(
                    source
                        .main
                        .file
                        .as_ref()
                        .expect("verified source")
                        .as_raw_handle(),
                ),
                &mut ios,
                (&disposition as *const FILE_DISPOSITION_INFORMATION_EX).cast(),
                size_of::<FILE_DISPOSITION_INFORMATION_EX>() as u32,
                FileDispositionInformationEx,
            )
        };
        if status.0 < 0 {
            return MutationOutcome::NoEffect(io_error(io::Error::from_raw_os_error(unsafe {
                RtlNtStatusToDosError(status)
            }
                as i32)));
        }
        let mut uncertain = status.0 != 0 || unsafe { ios.Anonymous.Status }.0 != 0;
        for stage in ["after_disposition", "before_close"] {
            if checkpoint(stage).and_then(|()| cancelled(cancel)).is_err() {
                uncertain = true;
            }
        }
        // Consume ownership before the only close attempt. Never reconstitute or
        // retry a potentially recycled handle after a lost close response.
        let file = source
            .main
            .file
            .take()
            .expect("verified source consumed once");
        if close(HANDLE(file.into_raw_handle())).is_err() {
            uncertain = true;
        }
        source.named.clear();
        if checkpoint("after_close")
            .and_then(|()| cancelled(cancel))
            .is_err()
        {
            uncertain = true;
        }
        let checked = (|| {
            checkpoint("before_namespace_check")?;
            cancelled(cancel)?;
            self.check_role(&source.main.parent, RootRole::Source)?;
            if move_original_identity(&source.main.parent, &source.main.expected.name)?
                == Some(source.main.expected.identity.clone())
            {
                return Err(err(
                    MutationErrorKind::UnknownOutcome,
                    "original source link remains",
                ));
            }
            checkpoint("after_namespace_check")?;
            cancelled(cancel)
        })();
        if uncertain || checked.is_err() {
            return MutationOutcome::PendingUnknown(err(
                MutationErrorKind::UnknownOutcome, "copied Move retirement dispatched; inspect source and all destinations without replay",
            ).with_observation(observed));
        }
        MutationOutcome::Applied(RetirementOutcome::Retired(observed))
    }
}

fn move_original_identity(
    parent: &WindowsParent,
    leaf: &MutationName,
) -> Result<Option<ObjectIdentity>, MutationError> {
    use windows::Win32::Foundation::STATUS_OBJECT_NAME_NOT_FOUND;
    ensure_directory(parent.handle())?;
    let mut name = units(leaf)?.to_vec();
    let string = UNICODE_STRING {
        Length: (name.len() * 2) as u16,
        MaximumLength: (name.len() * 2) as u16,
        Buffer: PWSTR(name.as_mut_ptr()),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: HANDLE(parent.handle().as_raw_handle()),
        ObjectName: &string,
        Attributes: OBJ_DONT_REPARSE | OBJ_CASE_INSENSITIVE,
        ..Default::default()
    };
    let mut handle = HANDLE::default();
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            &attributes,
            &mut ios,
            None,
            FILE_ATTRIBUTE_NORMAL,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            None,
            0,
        )
    };
    if status == STATUS_OBJECT_NAME_NOT_FOUND {
        return Ok(None);
    }
    if status.0 != 0 {
        return Err(io_error(io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        )));
    }
    if handle.is_invalid() {
        return Err(err(
            MutationErrorKind::UnknownOutcome,
            "source probe returned no handle",
        ));
    }
    let file = unsafe { File::from_raw_handle(handle.0) };
    if kind(&file.metadata().map_err(io_error)?) == EntryKind::LinkOrReparse {
        return Err(err(
            MutationErrorKind::UnsafeReparse,
            "source location became reparse",
        ));
    }
    Ok(Some(identity(&file).map_err(read_error)?))
}
