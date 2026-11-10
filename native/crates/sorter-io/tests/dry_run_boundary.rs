#![cfg(any(windows, target_os = "linux"))]
mod support;
use sorter_io::{desktop::ScratchSpace, storage::*};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
#[cfg(target_os = "linux")]
use std::{
    io::{Seek, SeekFrom},
    sync::{Arc, atomic::Ordering},
};
use support::*;

#[derive(Debug, PartialEq, Eq)]
struct ManifestRow {
    name: PathBuf,
    bytes: Option<Vec<u8>>,
    length: u64,
    modified: std::time::SystemTime,
}
fn manifest(root: &Path) -> Vec<ManifestRow> {
    fn visit(root: &Path, at: &Path, result: &mut Vec<ManifestRow>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            result.push(ManifestRow {
                name: path.strip_prefix(root).unwrap().into(),
                bytes: if metadata.is_file() {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                },
                length: metadata.len(),
                modified: metadata.modified().unwrap(),
            });
            if metadata.is_dir() {
                visit(root, &path, result);
            }
        }
    }
    let metadata = fs::symlink_metadata(root).unwrap();
    let mut result = vec![ManifestRow {
        name: PathBuf::new(),
        bytes: None,
        length: metadata.len(),
        modified: metadata.modified().unwrap(),
    }];
    visit(root, root, &mut result);
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}
// Scan receives only ReadStorage. This wrapper deliberately has no write route;
// its write trap is tested separately before resetting the audit counter.
struct DenyWrite {
    inner: TestDesktop,
    write_attempts: usize,
    reads: usize,
}
impl DenyWrite {
    fn attempt_write(&mut self) -> io::Result<()> {
        self.write_attempts += 1;
        Err(io::ErrorKind::PermissionDenied.into())
    }
}
impl ReadStorage for DenyWrite {
    fn stat(&mut self, r: &StorageRef) -> Result<EntryObservation, StorageError> {
        self.inner.stat(r)
    }
    fn children(&mut self, r: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        self.inner.children(r)
    }
    fn open_read(&mut self, r: &StorageRef) -> Result<ReadInput, StorageError> {
        self.reads += 1;
        self.inner.open_read(r)
    }
}
fn scan(storage: &mut dyn ReadStorage, root: &StorageRef) {
    for entry in storage.children(root).unwrap() {
        match entry.kind {
            EntryKind::Directory => scan(storage, &entry.reference),
            EntryKind::File => {
                let mut input = storage.open_read(&entry.reference).unwrap();
                io::copy(&mut input, &mut io::sink()).unwrap();
            }
            _ => {}
        }
    }
}
#[test]
fn scan_preserves_source_target_and_preferences_manifest_with_zero_write_attempts() {
    let tree = synthetic_tree();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    fs::create_dir_all(source.join("אלבום𐐀")).unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(
        source.join("אלבום𐐀/song.mp3"),
        b"synthetic fixture, not encoded audio",
    )
    .unwrap();
    fs::write(target.join("existing.mp3"), b"synthetic target").unwrap();
    fs::write(tree.path().join("preferences.json"), b"{}").unwrap();
    let before = manifest(tree.path());
    let mut inner = TestDesktop::new(&[source.clone(), target.clone()]).unwrap();
    let s = reference(&mut inner, &source);
    let t = reference(&mut inner, &target);
    let mut storage = DenyWrite {
        inner,
        write_attempts: 0,
        reads: 0,
    };
    assert!(storage.attempt_write().is_err());
    storage.write_attempts = 0;
    scan(&mut storage, &s);
    scan(&mut storage, &t);
    assert_eq!(storage.reads, 2);
    assert_eq!(storage.write_attempts, 0);
    assert_eq!(manifest(tree.path()), before);
}
#[cfg(target_os = "linux")]
#[test]
fn spooling_is_bounded_seekable_outside_roots_and_removed_on_drop() {
    let tree = synthetic_tree();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    let scratch = tree.path().join("scratch");
    for path in [&source, &target, &scratch] {
        fs::create_dir(path).unwrap();
    }
    let before_source = manifest(&source);
    let before_target = manifest(&target);
    let space = ScratchSpace::new(&scratch, &[source.clone(), target.clone()]).unwrap();
    let mut budget = SpoolBudget::new(100_000);
    let cancel = AtomicBool::new(false);
    let input = ReadInput::streaming(io::repeat(42).take(50_000), 50_000);
    let mut result = space.spool(input, &mut budget, &cancel).unwrap();
    assert!(result.is_seekable());
    assert_eq!(budget.used(), 50_000);
    assert_eq!(result.seek(SeekFrom::End(-2)).unwrap(), 49_998);
    let mut bytes = vec![];
    result.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, vec![42, 42]);
    assert_eq!(manifest(&source), before_source);
    assert_eq!(manifest(&target), before_target);
    drop(result);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}
#[test]
fn scratch_cannot_be_inside_either_selected_root_or_alias_to_it() {
    let tree = synthetic_tree();
    let source = tree.path().join("source");
    let target = tree.path().join("target");
    fs::create_dir(&source).unwrap();
    fs::create_dir_all(target.join("nested")).unwrap();
    let before = manifest(tree.path());
    for scratch in [&source, &target, &target.join("nested"), tree.path()] {
        assert_eq!(
            ScratchSpace::new(scratch, &[source.clone(), target.clone()])
                .err()
                .unwrap()
                .kind,
            StorageErrorKind::OutsideRoots
        );
    }
    assert_eq!(manifest(tree.path()), before);
    #[cfg(target_os = "linux")]
    {
        std::os::unix::fs::symlink(&source, tree.path().join("alias")).unwrap();
        assert_eq!(
            ScratchSpace::new(&tree.path().join("alias"), &[source, target])
                .err()
                .unwrap()
                .kind,
            StorageErrorKind::UnsafeLink
        );
    }
}
#[cfg(target_os = "linux")]
#[test]
fn spool_budget_is_cumulative_and_overflow_never_writes_past_limit() {
    let tree = synthetic_tree();
    let root = tree.path().join("root");
    let scratch = tree.path().join("scratch");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&scratch).unwrap();
    let space = ScratchSpace::new(&scratch, &[root]).unwrap();
    let mut budget = SpoolBudget::new(10);
    let cancel = AtomicBool::new(false);
    drop(
        space
            .spool(
                ReadInput::streaming(io::repeat(1).take(6), 6),
                &mut budget,
                &cancel,
            )
            .unwrap(),
    );
    assert_eq!(
        space
            .spool(
                ReadInput::streaming(io::repeat(2).take(5), 5),
                &mut budget,
                &cancel
            )
            .err()
            .unwrap()
            .kind,
        StorageErrorKind::BudgetExceeded
    );
    assert_eq!(budget.used(), 10);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}
#[cfg(target_os = "linux")]
struct CancelReader {
    cancel: Arc<AtomicBool>,
}
#[cfg(target_os = "linux")]
impl Read for CancelReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        assert!(buffer.len() <= 8192);
        buffer.fill(7);
        self.cancel.store(true, Ordering::Release);
        Ok(buffer.len())
    }
}
#[cfg(target_os = "linux")]
#[test]
fn cancellation_before_and_during_spool_cleans_up() {
    let tree = synthetic_tree();
    let root = tree.path().join("root");
    let scratch = tree.path().join("scratch");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&scratch).unwrap();
    let space = ScratchSpace::new(&scratch, &[root]).unwrap();
    let cancel = Arc::new(AtomicBool::new(true));
    let mut budget = SpoolBudget::new(1_000_000);
    assert_eq!(
        space
            .spool(
                ReadInput::streaming(io::repeat(0), 1_000_000),
                &mut budget,
                &cancel
            )
            .err()
            .unwrap()
            .kind,
        StorageErrorKind::Cancelled
    );
    assert_eq!(budget.used(), 0);
    cancel.store(false, Ordering::Release);
    assert_eq!(
        space
            .spool(
                ReadInput::streaming(
                    CancelReader {
                        cancel: cancel.clone()
                    },
                    1_000_000
                ),
                &mut budget,
                &cancel
            )
            .err()
            .unwrap()
            .kind,
        StorageErrorKind::Cancelled
    );
    assert!(budget.used() <= 8192);
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}
#[cfg(target_os = "linux")]
#[test]
fn stream_limit_errors_are_not_reported_as_successful_truncation() {
    let tree = synthetic_tree();
    let root = tree.path().join("root");
    let scratch = tree.path().join("scratch");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&scratch).unwrap();
    let space = ScratchSpace::new(&scratch, &[root]).unwrap();
    assert_eq!(
        space
            .spool(
                ReadInput::streaming(io::repeat(0).take(20), 10),
                &mut SpoolBudget::new(100),
                &AtomicBool::new(false)
            )
            .err()
            .unwrap()
            .kind,
        StorageErrorKind::BudgetExceeded
    );
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn windows_stream_spooling_is_unsupported_before_any_read_or_creation() {
    struct MustNotRead;
    impl Read for MustNotRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            panic!("unsupported fallback must not read input")
        }
    }
    let tree = synthetic_tree();
    let roots: Vec<_> = ["source", "target"]
        .iter()
        .map(|n| tree.path().join(n))
        .collect();
    let scratch = tree.path().join("scratch");
    for p in roots.iter().chain([&scratch]) {
        fs::create_dir(p).unwrap();
    }
    let space = ScratchSpace::new(&scratch, &roots).unwrap();
    let before = manifest(tree.path());
    let mut budget = SpoolBudget::new(100);
    assert_eq!(
        space
            .spool(
                ReadInput::streaming(MustNotRead, 100),
                &mut budget,
                &AtomicBool::new(false)
            )
            .err()
            .unwrap()
            .kind,
        StorageErrorKind::UnsupportedCapability
    );
    assert_eq!(budget.used(), 0);
    assert_eq!(manifest(tree.path()), before);
}

#[cfg(windows)]
#[test]
fn windows_in_place_scratch_junction_cannot_enable_stream_writes() {
    use support::windows_reparse::set_junction;
    for destination in ["source", "target"] {
        let tree = synthetic_tree();
        let source = tree.path().join("source");
        let target = tree.path().join("target");
        let scratch = tree.path().join("scratch");
        for path in [&source, &target, &scratch] {
            fs::create_dir(path).unwrap();
        }
        fs::write(source.join("synthetic.mp3"), b"synthetic").unwrap();
        let space = ScratchSpace::new(&scratch, &[source.clone(), target.clone()]).unwrap();
        let before_source = manifest(&source);
        let before_target = manifest(&target);
        set_junction(&scratch, &tree.path().join(destination));
        let mut budget = SpoolBudget::new(100);
        assert_eq!(
            space
                .spool(
                    ReadInput::streaming(io::repeat(7).take(20), 20),
                    &mut budget,
                    &AtomicBool::new(false)
                )
                .err()
                .unwrap()
                .kind,
            StorageErrorKind::UnsupportedCapability
        );
        assert_eq!(budget.used(), 0);
        assert_eq!(manifest(&source), before_source);
        assert_eq!(manifest(&target), before_target);
        drop(space);
        fs::remove_dir(&scratch).unwrap();
    }
}
