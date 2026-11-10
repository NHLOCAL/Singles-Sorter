//! Read-time identity binding, independent of native parser availability.
//! Linux desktop coverage is test-backend evidence, not Windows acceptance.
use sorter_io::{
    metadata::{MetadataError, MetadataProvider, MetadataReader, StorageMetadataProvider},
    storage::*,
};
use std::{
    io::{self, Cursor, Read, Seek, SeekFrom},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

fn bytes() -> Vec<u8> {
    // Known legacy header: reaching it proves binding validation ran too late.
    vec![0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11]
}
fn entry() -> EntryObservation {
    EntryObservation {
        reference: StorageRef::TestOpaque { id: "song".into() },
        identity: Some(ObjectIdentity::Test {
            id: "original".into(),
        }),
        parent_identity: None,
        name: "song.mp3".into(),
        kind: EntryKind::File,
        size: Some(bytes().len() as u64),
        modified: Some("before".into()),
        capabilities: Capabilities {
            read: Some(true),
            list: Some(false),
            seek: Some(true),
        },
    }
}
fn expect_storage_error(
    result: Result<sorter_io::metadata::MetadataHandle, MetadataError>,
    expected: StorageErrorKind,
) {
    match result {
        Err(MetadataError::Storage(error)) => assert_eq!(error.kind, expected),
        Err(error) => panic!("expected {expected:?} before parsing, got {error:?}"),
        Ok(_) => panic!("unverified metadata was accepted"),
    }
}

#[test]
fn changed_memory_object_size_or_modification_is_rejected() {
    let observed = entry();
    for change in ["identity", "size", "modified"] {
        let mut current = observed.clone();
        let mut data = bytes();
        match change {
            "identity" => {
                current.identity = Some(ObjectIdentity::Test {
                    id: "replacement".into(),
                })
            }
            "size" => {
                data.push(0);
                current.size = Some(data.len() as u64);
            }
            "modified" => current.modified = Some("after".into()),
            _ => unreachable!(),
        }
        let mut storage = MemoryStorage::default();
        storage.insert(current, data);
        let mut provider = StorageMetadataProvider {
            storage: &mut storage,
            reader: MetadataReader,
        };
        expect_storage_error(provider.open(&observed), StorageErrorKind::Changed);
    }
}

#[test]
fn missing_observed_identity_size_or_modification_fails_closed() {
    for missing in ["identity", "size", "modified"] {
        let current = entry();
        let mut observed = current.clone();
        match missing {
            "identity" => observed.identity = None,
            "size" => observed.size = None,
            "modified" => observed.modified = None,
            _ => unreachable!(),
        }
        let mut storage = MemoryStorage::default();
        storage.insert(current, bytes());
        let mut provider = StorageMetadataProvider {
            storage: &mut storage,
            reader: MetadataReader,
        };
        expect_storage_error(
            provider.open(&observed),
            StorageErrorKind::UnsupportedCapability,
        );
    }
}

#[test]
fn memory_binding_uses_actual_bytes_length_not_declared_size() {
    let observed = entry();
    let mut data = bytes();
    data.push(0);
    let mut storage = MemoryStorage::default();
    storage.insert(observed.clone(), data);
    let mut provider = StorageMetadataProvider {
        storage: &mut storage,
        reader: MetadataReader,
    };
    expect_storage_error(provider.open(&observed), StorageErrorKind::Changed);
}

#[test]
fn unchanged_memory_observation_reaches_metadata_reader() {
    let observed = entry();
    let mut storage = MemoryStorage::default();
    storage.insert(observed.clone(), bytes());
    let mut provider = StorageMetadataProvider {
        storage: &mut storage,
        reader: MetadataReader,
    };
    assert!(
        matches!(provider.open(&observed), Err(MetadataError::LegacyOpen { kind, .. }) if kind == "NotImplementedError")
    );
}

struct CountReads {
    reads: Arc<AtomicUsize>,
    cursor: Cursor<Vec<u8>>,
}
impl Read for CountReads {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.cursor.read(out)
    }
}
impl Seek for CountReads {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(from)
    }
}
struct ControlledStorage {
    reads: Arc<AtomicUsize>,
    binding: Option<ReadBinding>,
}
impl ReadStorage for ControlledStorage {
    fn stat(&mut self, _: &StorageRef) -> Result<EntryObservation, StorageError> {
        panic!("a separate stat cannot establish a read-handle binding")
    }
    fn children(&mut self, _: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        unreachable!()
    }
    fn open_read(&mut self, _: &StorageRef) -> Result<ReadInput, StorageError> {
        let input = ReadInput::seekable(
            CountReads {
                reads: self.reads.clone(),
                cursor: Cursor::new(bytes()),
            },
            bytes().len() as u64,
        );
        Ok(match &self.binding {
            Some(binding) => input.with_binding(binding.clone()),
            None => input,
        })
    }
}
#[test]
fn absent_handle_binding_fails_closed_without_reading_or_restat() {
    let reads = Arc::new(AtomicUsize::new(0));
    let mut storage = ControlledStorage {
        reads: reads.clone(),
        binding: None,
    };
    let mut provider = StorageMetadataProvider {
        storage: &mut storage,
        reader: MetadataReader,
    };
    expect_storage_error(
        provider.open(&entry()),
        StorageErrorKind::UnsupportedCapability,
    );
    assert_eq!(reads.load(Ordering::SeqCst), 0);
}

#[test]
fn changed_read_binding_fails_before_reading_without_a_separate_stat() {
    for change in ["identity", "size", "modified", "input_bound"] {
        let mut observed = entry();
        let mut binding = ReadBinding {
            identity: observed.identity.clone().unwrap(),
            size: observed.size.unwrap(),
            modified: observed.modified.clone().unwrap(),
        };
        match change {
            "identity" => {
                binding.identity = ObjectIdentity::Test {
                    id: "replacement".into(),
                }
            }
            "size" => binding.size += 1,
            "modified" => binding.modified = "after".into(),
            "input_bound" => {
                binding.size += 1;
                observed.size = Some(binding.size);
            }
            _ => unreachable!(),
        }
        let reads = Arc::new(AtomicUsize::new(0));
        let mut storage = ControlledStorage {
            reads: reads.clone(),
            binding: Some(binding),
        };
        let mut provider = StorageMetadataProvider {
            storage: &mut storage,
            reader: MetadataReader,
        };
        expect_storage_error(provider.open(&observed), StorageErrorKind::Changed);
        assert_eq!(reads.load(Ordering::SeqCst), 0, "{change}");
    }
}

#[test]
fn missing_memory_source_evidence_cannot_manufacture_a_binding() {
    for missing in ["identity", "modified"] {
        let observed = entry();
        let mut current = observed.clone();
        match missing {
            "identity" => current.identity = None,
            "modified" => current.modified = None,
            _ => unreachable!(),
        }
        let mut storage = MemoryStorage::default();
        storage.insert(current, bytes());
        let mut provider = StorageMetadataProvider {
            storage: &mut storage,
            reader: MetadataReader,
        };
        expect_storage_error(
            provider.open(&observed),
            StorageErrorKind::UnsupportedCapability,
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn renamed_and_replaced_after_observation_is_rejected() {
    use sorter_io::desktop::LinuxTestStorage;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("song.mp3");
    std::fs::write(&path, bytes()).unwrap();
    let mut storage = LinuxTestStorage::new(&[temp.path().to_owned()]).unwrap();
    let reference = storage.reference(&path).unwrap();
    let observed = storage.stat(&reference).unwrap();
    let original_modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    std::fs::rename(&path, temp.path().join("original.mp3")).unwrap();
    std::fs::write(&path, bytes()).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(original_modified)
        .unwrap();
    let replacement = storage.stat(&reference).unwrap();
    assert_eq!(observed.size, replacement.size);
    assert_eq!(observed.modified, replacement.modified);
    assert_ne!(observed.identity, replacement.identity);
    let mut provider = StorageMetadataProvider {
        storage: &mut storage,
        reader: MetadataReader,
    };
    expect_storage_error(provider.open(&observed), StorageErrorKind::Changed);
}

#[cfg(target_os = "linux")]
#[test]
fn read_binding_and_bytes_stay_with_the_open_handle_after_path_replacement() {
    use sorter_io::desktop::LinuxTestStorage;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("song.mp3");
    std::fs::write(&path, bytes()).unwrap();
    let mut storage = LinuxTestStorage::new(&[temp.path().to_owned()]).unwrap();
    let reference = storage.reference(&path).unwrap();
    let observed = storage.stat(&reference).unwrap();
    let mut input = storage.open_read(&reference).unwrap();
    std::fs::rename(&path, temp.path().join("original.mp3")).unwrap();
    std::fs::write(&path, vec![0; bytes().len()]).unwrap();
    let binding = input.binding().unwrap();
    assert_eq!(Some(&binding.identity), observed.identity.as_ref());
    assert_eq!(Some(binding.size), observed.size);
    assert_eq!(Some(&binding.modified), observed.modified.as_ref());
    let mut read = Vec::new();
    input.read_to_end(&mut read).unwrap();
    assert_eq!(read, bytes());
    assert_ne!(
        observed.identity,
        storage.stat(&reference).unwrap().identity
    );
}
