//! Read-only adapters; shared contracts are owned by core.
pub use sorter_core::discovery::*;
use std::collections::{HashMap, HashSet};
use std::io;
#[cfg(any(target_os = "linux", test))]
use std::io::Read;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relationship {
    Same,
    Ancestor,
    Descendant,
    Unknown,
}
/// Deliberately has no `Disjoint` result: incomplete ancestry cannot prove disjointness.
/// Parent links are evidence, never string prefix comparisons or Android ID parsing.
pub fn relationship(
    a: &EntryObservation,
    b: &EntryObservation,
    observations: &[EntryObservation],
) -> Relationship {
    let (Some(a_id), Some(b_id)) = (&a.identity, &b.identity) else {
        return Relationship::Unknown;
    };
    if a_id == b_id {
        return Relationship::Same;
    }
    fn reaches(
        child: &EntryObservation,
        ancestor: &ObjectIdentity,
        observations: &[EntryObservation],
    ) -> bool {
        let mut next = child.parent_identity.as_ref();
        let mut seen = HashSet::new();
        while let Some(id) = next {
            if id == ancestor {
                return true;
            }
            if !seen.insert(id) {
                return false;
            }
            let candidates: Vec<_> = observations
                .iter()
                .filter(|e| e.identity.as_ref() == Some(id))
                .collect();
            let Some(first) = candidates.first() else {
                return false;
            };
            if candidates
                .iter()
                .any(|e| e.parent_identity != first.parent_identity)
            {
                return false;
            }
            next = first.parent_identity.as_ref();
        }
        false
    }
    if reaches(b, a_id, observations) {
        Relationship::Ancestor
    } else if reaches(a, b_id, observations) {
        Relationship::Descendant
    } else {
        Relationship::Unknown
    }
}
/// Deterministic synthetic adapter, including fake SAF observations. Not an Android provider.
#[derive(Default)]
pub struct MemoryStorage {
    entries: HashMap<StorageRef, (EntryObservation, Vec<u8>)>,
    listings: HashMap<StorageRef, Vec<StorageRef>>,
    errors: HashMap<StorageRef, StorageErrorKind>,
}
impl MemoryStorage {
    pub fn insert(&mut self, entry: EntryObservation, bytes: Vec<u8>) {
        self.entries.insert(entry.reference.clone(), (entry, bytes));
    }
    pub fn remove(&mut self, reference: &StorageRef) {
        self.entries.remove(reference);
    }
    pub fn set_children(&mut self, reference: &StorageRef, children: Vec<StorageRef>) {
        self.listings.insert(reference.clone(), children);
    }
    pub fn fail(&mut self, reference: &StorageRef, kind: StorageErrorKind) {
        self.errors.insert(reference.clone(), kind);
    }
    fn get(&self, reference: &StorageRef) -> Result<&(EntryObservation, Vec<u8>), StorageError> {
        if let Some(kind) = self.errors.get(reference) {
            return Err(StorageError::new(*kind, "synthetic provider failure"));
        }
        self.entries
            .get(reference)
            .ok_or_else(|| StorageError::new(StorageErrorKind::NotFound, "synthetic entry absent"))
    }
}
impl ReadStorage for MemoryStorage {
    fn stat(&mut self, reference: &StorageRef) -> Result<EntryObservation, StorageError> {
        Ok(self.get(reference)?.0.clone())
    }
    fn children(&mut self, reference: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        let entry = &self.get(reference)?.0;
        if entry.kind == EntryKind::LinkOrReparse {
            return Err(StorageError::new(
                StorageErrorKind::UnsafeLink,
                "do not traverse links",
            ));
        }
        if entry.kind != EntryKind::Directory {
            return Err(StorageError::new(
                StorageErrorKind::NotDirectory,
                "not a directory",
            ));
        }
        if entry.capabilities.list == Some(false) {
            return Err(StorageError::new(
                StorageErrorKind::PermissionDenied,
                "listing denied by provider",
            ));
        }
        self.listings
            .get(reference)
            .into_iter()
            .flatten()
            .map(|r| self.get(r).map(|e| e.0.clone()))
            .collect()
    }
    fn open_read(&mut self, reference: &StorageRef) -> Result<ReadInput, StorageError> {
        let (entry, bytes) = self.get(reference)?;
        if entry.kind == EntryKind::LinkOrReparse {
            return Err(StorageError::new(
                StorageErrorKind::UnsafeLink,
                "do not follow links",
            ));
        }
        if entry.kind != EntryKind::File {
            return Err(StorageError::new(
                StorageErrorKind::NotFile,
                "not a regular file",
            ));
        }
        if entry.capabilities.read == Some(false) {
            return Err(StorageError::new(
                StorageErrorKind::PermissionDenied,
                "read capability absent",
            ));
        }
        let input = if entry.capabilities.seek == Some(true) {
            ReadInput::seekable(io::Cursor::new(bytes.clone()), bytes.len() as u64)
        } else {
            ReadInput::streaming(io::Cursor::new(bytes.clone()), bytes.len() as u64)
        };
        // Both the evidence and cloned bytes come from the current immutable
        // entry borrow. Declared size is not evidence for the actual byte count.
        Ok(match (&entry.identity, &entry.modified) {
            (Some(identity), Some(modified)) => input.with_binding(ReadBinding {
                identity: identity.clone(),
                size: bytes.len() as u64,
                modified: modified.clone(),
            }),
            _ => input,
        })
    }
}

/// No product platform currently has a validated writable streaming fallback.
/// Native Windows file reads are already seekable and remain supported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamingSpoolCapability {
    LinuxTestOnly,
    Unsupported,
}
pub const fn streaming_spool_capability() -> StreamingSpoolCapability {
    if cfg!(target_os = "linux") {
        StreamingSpoolCapability::LinuxTestOnly
    } else {
        StreamingSpoolCapability::Unsupported
    }
}

/// One cumulative budget per job, shared across all recordings and failed attempts.
/// Dropping a spool does not replenish the budget.
pub struct SpoolBudget {
    _limit: u64,
    used: u64,
}
impl SpoolBudget {
    pub fn new(limit: u64) -> Self {
        Self {
            _limit: limit,
            used: 0,
        }
    }
    pub fn used(&self) -> u64 {
        self.used
    }
}
#[cfg(any(target_os = "linux", test))]
pub(crate) fn copy_to_scratch(
    input: &mut ReadInput,
    output: &mut impl io::Write,
    budget: &mut SpoolBudget,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<u64, StorageError> {
    use std::sync::atomic::Ordering;
    let mut buffer = [0; 8192];
    let mut written = 0;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err(StorageError::new(
                StorageErrorKind::Cancelled,
                "spool cancelled",
            ));
        }
        let remaining = budget._limit - budget.used;
        // One byte at an exhausted budget distinguishes exact EOF from overflow;
        // it is never written. The fixed buffer is independent of recording size.
        let count = remaining.clamp(1, buffer.len() as u64) as usize;
        let n = match input.read(&mut buffer[..count]) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result.map_err(StorageError::from)?,
        };
        if n == 0 {
            break;
        }
        if n as u64 > remaining {
            return Err(StorageError::new(
                StorageErrorKind::BudgetExceeded,
                "job spool budget exhausted",
            ));
        }
        budget.used += n as u64;
        if cancelled.load(Ordering::Acquire) {
            return Err(StorageError::new(
                StorageErrorKind::Cancelled,
                "spool cancelled",
            ));
        }
        output.write_all(&buffer[..n])?;
        written += n as u64;
    }
    output.flush()?;
    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
        return Err(StorageError::new(
            StorageErrorKind::Cancelled,
            "spool cancelled",
        ));
    }
    Ok(written)
}

/// Share one read-only reference registry between discovery and metadata adapters.
/// In particular, Linux test opaque references must not be resolved by a second,
/// independently allocated registry. No mutation capability is introduced.
pub struct SharedReadStorage<S>(std::rc::Rc<std::cell::RefCell<S>>);
impl<S> Clone for SharedReadStorage<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<S: ReadStorage> SharedReadStorage<S> {
    pub fn new(storage: S) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(storage)))
    }
}
impl<S: ReadStorage> ReadStorage for SharedReadStorage<S> {
    fn stat(&mut self, reference: &StorageRef) -> Result<EntryObservation, StorageError> {
        self.0.borrow_mut().stat(reference)
    }
    fn children(&mut self, reference: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        self.0.borrow_mut().children(reference)
    }
    fn open_read(&mut self, reference: &StorageRef) -> Result<ReadInput, StorageError> {
        self.0.borrow_mut().open_read(reference)
    }
}

#[cfg(test)]
mod spool_tests {
    use super::*;
    use std::{io::Write, sync::atomic::AtomicBool};
    struct FullDisk;
    impl Write for FullDisk {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::StorageFull.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn disk_full_is_typed_and_charged_to_job_budget() {
        let mut input = ReadInput::streaming(io::repeat(1).take(20_000), 20_000);
        let mut budget = SpoolBudget::new(20_000);
        let e = copy_to_scratch(
            &mut input,
            &mut FullDisk,
            &mut budget,
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(e.kind, StorageErrorKind::DiskFull);
        assert_eq!(budget.used(), 8192);
    }
    struct PartialSink {
        bytes: usize,
    }
    impl Write for PartialSink {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let n = bytes.len().min(7);
            self.bytes += n;
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[test]
    fn partial_writes_are_completed_without_overrunning_budget() {
        let mut input = ReadInput::streaming(io::repeat(1).take(20_000), 20_000);
        let mut budget = SpoolBudget::new(20_000);
        let mut sink = PartialSink { bytes: 0 };
        assert_eq!(
            copy_to_scratch(&mut input, &mut sink, &mut budget, &AtomicBool::new(false)).unwrap(),
            20_000
        );
        assert_eq!(sink.bytes, 20_000);
        assert_eq!(budget.used(), 20_000);
    }
}
