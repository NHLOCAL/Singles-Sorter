//! Real NTFS file I/O with protocol fault injection; not power-cut evidence.
#![cfg(all(windows, feature = "experimental-windows-journal"))]
mod support;
use sorter_executor::{
    ExecutionError as E, Result, binding::*, physical_journal::*, recovery::*,
    windows_journal::WindowsJournalStore,
};
use sorter_io::{desktop::windows_mutation::WindowsMutationStorage, mutation::*};
use std::path::Path;

fn name(s: &str) -> MutationName {
    MutationName::Windows(s.encode_utf16().collect())
}
fn event(binding: &ExecutionBinding, phase: Phase) -> Event {
    let op = &binding.operations()[0];
    Event {
        copy_metadata:None,
        operation: 0,
        destination_id: op.destination.id.clone(),
        expected_source: op.source.observation.identity.clone().unwrap(),
        observed_identity: None,
        sha256: op.source.sha256.clone(),
        phase,
    }
}
fn fixture() -> (
    WindowsMutationStorage,
    WindowsJournalStore,
    std::path::PathBuf,
) {
    let tree = tempfile::tempdir().unwrap();
    let path = tree.path().to_owned();
    let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("")).unwrap();
    let file = storage
        .create_journal(&parent, &name("journal.jsonl"))
        .unwrap();
    (storage, WindowsJournalStore::new(file), path)
}
#[test]
fn physical_journal_close_reopen_is_identity_bound_and_recovery_only() {
    let (storage, mut sink, path) = fixture();
    let b = support::binding(false);
    let location = sink.location();
    let mut journal = PhysicalJournalV2::create(&b, "native", &mut sink).unwrap();
    journal
        .append(&mut sink, event(&b, Phase::Prepared))
        .unwrap();
    let expected = sink.read_all().unwrap();
    assert!(std::fs::read(path.join("journal.jsonl")).is_err());
    assert!(storage.read_journal(&location).is_err());
    drop(sink);
    let bytes = storage.read_journal(&location).unwrap();
    assert_eq!(bytes, expected);
    let mut recovered =
        PhysicalJournalV2::open_for_recovery(&support::recovery_scope(false), "native", &bytes)
            .unwrap();
    assert_eq!(recovered.events().len(), 1);
    struct ReadOnlyObservation {
        source: ObservedObject,
    }
    impl RecoveryStorage for ReadOnlyObservation {
        fn source(
            &mut self,
            _: &sorter_core::discovery::StorageRef,
        ) -> Result<Option<ObservedObject>> {
            Ok(Some(self.source.clone()))
        }
        fn destination(&mut self, _: &DestinationEvidence) -> Result<Option<ObservedObject>> {
            Ok(None)
        }
    }
    let op = &b.operations()[0];
    let mut observations = ReadOnlyObservation {
        source: ObservedObject {
            identity: op.source.observation.identity.clone().unwrap(),
            sha256: op.source.sha256.clone(),
        },
    };
    let report = recover_job(recovered.binding(), &recovered, &mut observations).unwrap();
    assert_eq!(
        report.operations[0].status,
        RecoveryStatus::ReadyForExplicitResume
    );

    let mut untouched = support::Sink::default();
    assert_eq!(
        recovered.append(&mut untouched, event(&b, Phase::Cancelled)),
        Err(E::Scope)
    );
    assert!(matches!(
        PhysicalJournalV2::create(recovered.binding(), "new", &mut untouched),
        Err(E::Scope)
    ));
    assert!(untouched.bytes.is_empty());
    assert_eq!(storage.read_journal(&location).unwrap(), expected);
}
#[test]
fn journal_creation_never_overwrites_and_wrong_scope_cannot_reopen() {
    let (storage, sink, path) = fixture();
    let parent = storage.parent(Path::new("")).unwrap();
    let location = sink.location();
    assert!(
        storage
            .create_journal(&parent, &name("journal.jsonl"))
            .is_err()
    );
    assert!(storage.create_journal(&parent, &name("../escape")).is_err());
    drop(sink);
    assert!(
        storage
            .create_journal(&parent, &name("journal.jsonl"))
            .is_err()
    );
    let foreign =
        WindowsMutationStorage::for_owned_synthetic_tree(tempfile::tempdir().unwrap()).unwrap();
    assert!(foreign.read_journal(&location).is_err());
    assert!(
        foreign
            .create_journal(&parent, &name("foreign.jsonl"))
            .is_err()
    );
    assert_eq!(std::fs::read(path.join("journal.jsonl")).unwrap(), b"");
}
#[test]
fn replacement_identity_is_rejected_and_location_pins_ancestors() {
    let tree = tempfile::tempdir().unwrap();
    let path = tree.path().to_owned();
    std::fs::create_dir(path.join("nested")).unwrap();
    let storage = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
    let parent = storage.parent(Path::new("nested")).unwrap();
    let file = storage.create_journal(&parent, &name("j")).unwrap();
    let location = file.location();
    drop(file);
    drop(parent);
    assert!(std::fs::rename(path.join("nested"), path.join("moved")).is_err());
    std::fs::rename(path.join("nested/j"), path.join("nested/old")).unwrap();
    std::fs::write(path.join("nested/j"), b"replacement").unwrap();
    assert!(
        matches!(storage.read_journal(&location), Err(e) if e.kind == MutationErrorKind::StalePlan)
    );
}
#[derive(Clone, Copy)]
enum Fault {
    BeforeWrite,
    PartialWrite,
    LostWriteResponse,
    BeforeSync,
    LostSyncResponse,
}
struct FaultSink {
    inner: WindowsJournalStore,
    fault: Option<Fault>,
}
impl JournalStore for FaultSink {
    fn read_all(&mut self) -> Result<Vec<u8>> {
        self.inner.read_all()
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        match self.fault {
            Some(Fault::BeforeWrite) => Err(E::Io),
            Some(Fault::PartialWrite) => {
                self.inner.append(&bytes[..bytes.len() / 2])?;
                Err(E::Io)
            }
            Some(Fault::LostWriteResponse) => {
                self.inner.append(bytes)?;
                Err(E::Io)
            }
            _ => self.inner.append(bytes),
        }
    }
    fn sync(&mut self) -> Result<()> {
        match self.fault {
            Some(Fault::BeforeSync) => Err(E::Io),
            Some(Fault::LostSyncResponse) => {
                self.inner.sync()?;
                Err(E::Io)
            }
            _ => self.inner.sync(),
        }
    }
}
#[test]
fn native_write_and_flush_response_failures_poison_protocol_without_retry() {
    for fault in [
        Fault::BeforeWrite,
        Fault::PartialWrite,
        Fault::LostWriteResponse,
        Fault::BeforeSync,
        Fault::LostSyncResponse,
    ] {
        let (storage, inner, _) = fixture();
        let location = inner.location();
        let mut sink = FaultSink { inner, fault: None };
        let b = support::binding(false);
        let mut journal = PhysicalJournalV2::create(&b, "fault", &mut sink).unwrap();
        sink.fault = Some(fault);
        assert_eq!(
            journal.append(&mut sink, event(&b, Phase::Prepared)),
            Err(E::Io)
        );
        assert!(journal.needs_recovery());
        let before_retry = sink.read_all().unwrap();
        sink.fault = None;
        assert_eq!(
            journal.append(&mut sink, event(&b, Phase::Prepared)),
            Err(E::NeedsRecovery)
        );
        assert_eq!(sink.read_all().unwrap(), before_retry);
        drop(sink);
        let bytes = storage.read_journal(&location).unwrap();
        assert_eq!(bytes, before_retry);
        let recovered =
            PhysicalJournalV2::open_for_recovery(&support::recovery_scope(false), "fault", &bytes)
                .unwrap();
        match fault {
            Fault::BeforeWrite => assert!(recovered.events().is_empty()),
            Fault::PartialWrite => assert!(!recovered.incomplete_tail().is_empty()),
            _ => assert_eq!(recovered.events().len(), 1),
        }
        // Readback after failed sync is evidence of bytes, never a durability claim.
        assert_eq!(storage.read_journal(&location).unwrap(), bytes);
    }
}
#[test]
fn incomplete_native_tail_is_preserved_and_blocks_even_authorized_append() {
    let (storage, mut sink, _) = fixture();
    let location = sink.location();
    let b = support::binding(false);
    PhysicalJournalV2::create(&b, "tail", &mut sink).unwrap();
    sink.append(b"{incomplete").unwrap();
    sink.sync().unwrap();
    let bytes = sink.read_all().unwrap();
    let mut reopened = PhysicalJournalV2::open(&b, "tail", &bytes).unwrap();
    assert_eq!(
        reopened.append(&mut sink, event(&b, Phase::Prepared)),
        Err(E::IncompleteTail)
    );
    assert_eq!(sink.read_all().unwrap(), bytes);
    drop(sink);
    assert_eq!(storage.read_journal(&location).unwrap(), bytes);
}
#[test]
fn real_journal_does_not_promote_native_mutation_flush_capability() {
    let (storage, mut sink, _) = fixture();
    sink.append(b"fixture").unwrap();
    sink.sync().unwrap();
    assert_eq!(storage.capabilities().durable_flush, Capability::Unknown);
}
