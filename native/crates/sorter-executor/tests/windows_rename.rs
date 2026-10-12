//! Owned generated NTFS fixtures only. Driver faults wrap real native I/O.
//! Cross-compilation/Unix skips do not establish Windows runtime acceptance.
#![cfg(all(windows, feature = "experimental-windows-rename"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, binding::*, digest, physical_journal::*, recovery::*, results::*,
    transfer::*, windows_copy::*, windows_journal::*, windows_rename::*,
};
use sorter_io::desktop::{DesktopStorage, windows_mutation::*};
use sorter_storage_contracts::*;
use std::{
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

struct Metadata;
impl MetadataProvider for Metadata {
    fn open(
        &mut self,
        _: &EntryObservation,
    ) -> std::result::Result<Box<dyn MetadataFields>, MetadataError> {
        Err(MetadataError::LegacyOpen {
            kind: "authored-untagged".into(),
            detail: "fixture".into(),
        })
    }
}
struct Ai;
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        AiProfile::Disabled
    }
    fn classify(
        &mut self,
        _: &str,
    ) -> std::result::Result<sorter_core::ClassifierFact, RecognitionError> {
        panic!("disabled")
    }
    fn ner(&mut self, _: &str) -> std::result::Result<Vec<String>, RecognitionError> {
        panic!("disabled")
    }
}
fn name(s: &str) -> MutationName {
    MutationName::Windows(s.encode_utf16().collect())
}
struct Fixture {
    owned: WindowsRenameStorage,
    path: PathBuf,
    source: PathBuf,
    destination: PathBuf,
    bytes: Vec<u8>,
    plan: JobPlanV2,
    scope: ExecutionScope,
    binding: ExecutionBinding,
    sink: WindowsJournalStore,
}
impl Fixture {
    fn new() -> Self {
        Self::with_mode(false)
    }
    fn with_mode(copy_mode: bool) -> Self {
        let tree = tempfile::tempdir().unwrap();
        let path = tree.path().to_owned();
        std::fs::create_dir(path.join("source")).unwrap();
        std::fs::create_dir_all(path.join("target/אמן")).unwrap();
        std::fs::create_dir(path.join("journal")).unwrap();
        let source = path.join("source/song-שלום-🎵.mp3");
        let destination = path.join("target/אמן/song-שלום-🎵.mp3");
        let bytes = (0..TRANSFER_BUFFER_BYTES * 2 + 19)
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>();
        std::fs::write(&source, &bytes).unwrap();
        let owned = WindowsMutationStorage::for_owned_synthetic_tree(tree).unwrap();
        let mut raw = DesktopStorage::new(&[path.join("source"), path.join("target")]).unwrap();
        let source_root = raw.reference(&path.join("source")).unwrap();
        let target_root = raw.reference(&path.join("target")).unwrap();
        let mut reads = OwnedCopyReadStorage::new(&owned, &source_root, &target_root).unwrap();
        let request = JobRequestV2 {
            mode: JobMode::Sort,
            source: reads.reference(&path.join("source")).unwrap(),
            target: Some(reads.reference(&path.join("target")).unwrap()),
            settings: Settings {
                copy_mode,
                main_folder_only: true,
                singles_folder: false,
                ..Settings::default()
            },
            alias_rows: vec![vec!["song".into(), "אמן".into()]],
            resource_ids: vec!["authored-alias".into()],
            adapter_versions: vec!["authored-untagged".into()],
            ai_profile: AiProfile::Disabled,
        };
        let plan = plan_job(&request, &mut reads, &mut Metadata, &mut Ai).unwrap();
        assert_eq!(plan.operations.len(), 1, "{plan:?}");
        assert!(!plan.blockers.iter().any(|b| b == "UNKNOWN_CAPABILITY"));
        let resources = Resources {
            resource_ids: request.resource_ids,
            adapter_versions: request.adapter_versions,
            ai_profile: AiProfile::Disabled,
            resource_hashes: vec![digest(b"aliases")],
            adapter_hashes: vec![digest(b"metadata")],
            model_hash: None,
        };
        let OperationKind::SingleTransfer {
            source: planned_source,
            ..
        } = &plan.operations[0].kind
        else {
            panic!("fixture requires one single transfer: {plan:?}");
        };
        // Preserve the exact native-enumeration reference inspected in the plan.
        // A separately spelled path can name the same file without being the same
        // immutable StorageRef (e.g. Windows slash versus backslash separators).
        let inspected_source = reads.stat(&planned_source.reference).unwrap();
        assert_eq!(
            &inspected_source, planned_source,
            "native source observation changed before fixture binding"
        );
        let scope = ExecutionScope {
            inspected_plan: plan.binding_sha256.clone(),
            source_root: reads.stat(&request.source).unwrap().identity.unwrap(),
            target_root: reads
                .stat(request.target.as_ref().unwrap())
                .unwrap()
                .identity
                .unwrap(),
            inspected_sources: vec![SourceEvidence {
                observation: inspected_source,
                sha256: digest(&bytes),
            }],
            resources,
            allowed_modes: vec![if copy_mode {
                TransferMode::Copy
            } else {
                TransferMode::Move
            }],
        };
        drop(reads);
        drop(raw);
        let owned = WindowsRenameStorage::new(owned);
        let binding = if copy_mode {
            validate_owned_copy(
                owned.owned(),
                &plan,
                &mut Metadata,
                &mut Ai,
                &scope.resources,
                &scope,
            )
        } else {
            validate_owned_rename(
                &owned,
                &plan,
                &mut Metadata,
                &mut Ai,
                &scope.resources,
                &scope,
            )
        }
        .unwrap_or_else(|error| panic!("owned fixture binding failed: {error:?}; plan={plan:?}"));
        let log_parent = owned.owned().parent(Path::new("journal")).unwrap();
        let sink = WindowsJournalStore::new(
            owned
                .owned()
                .create_journal(&log_parent, &name("job.jsonl"))
                .unwrap(),
        );
        Self {
            owned,
            path,
            source,
            destination,
            bytes,
            plan,
            scope,
            binding,
            sink,
        }
    }
    fn recovery_scope(&self) -> RecoveryScope {
        RecoveryScope {
            original_scope: self.scope.clone(),
            inspected_binding_sha256: self.binding.sha256().unwrap(),
        }
    }
}

const JOB: &str = "rename";
const ATOMIC_PHASES: [Phase; 5] = [
    Phase::Prepared,
    Phase::AtomicRenamePrepared,
    Phase::AtomicRenameObserved,
    Phase::DestinationVerified,
    Phase::Committed,
];
fn assert_layout(source: &Path, destination: &Path, bytes: &[u8], moved: bool) {
    assert_eq!(source.exists(), !moved);
    assert_eq!(destination.exists(), moved);
    assert_eq!(
        std::fs::read(if moved { destination } else { source }).unwrap(),
        bytes
    );
    assert_eq!(
        std::fs::read_dir(source.parent().unwrap()).unwrap().count(),
        usize::from(!moved)
    );
    assert_eq!(
        std::fs::read_dir(destination.parent().unwrap())
            .unwrap()
            .count(),
        usize::from(moved)
    );
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for child in std::fs::read_dir(path).unwrap() {
            let path = child.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}
#[derive(Default)]
struct MemorySink {
    bytes: Vec<u8>,
}
impl JournalStore for MemorySink {
    fn read_all(&mut self) -> sorter_executor::Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }
    fn append(&mut self, bytes: &[u8]) -> sorter_executor::Result<()> {
        self.bytes.extend(bytes);
        Ok(())
    }
    fn sync(&mut self) -> sorter_executor::Result<()> {
        Ok(())
    }
}
fn recovery_without_replay(
    storage: &mut WindowsRenameStorage,
    scope: &RecoveryScope,
    bytes: &[u8],
) -> RecoveryReport {
    assert!(matches!(
        PhysicalJournalV2::open_for_recovery(scope, JOB, bytes),
        Err(E::Scope)
    ));
    assert!(open_owned_copy_recovery(storage.owned(), scope, JOB, bytes).is_err());
    let mut journal = open_owned_rename_recovery(storage, scope, JOB, bytes).unwrap();
    let binding = journal.binding().clone();
    let mut observations = WindowsRenameRecovery::new(storage, &binding).unwrap();
    assert!(WindowsCopyRecovery::new(storage.owned(), &binding).is_err());
    let report = recover_job(&binding, &journal, &mut observations).unwrap();
    let mut forbidden = MemorySink::default();
    assert!(matches!(
        PhysicalJournalV2::create(&binding, "replay", &mut forbidden),
        Err(E::Scope)
    ));
    let op = &binding.operations()[0];
    let event = Event {
        copy_metadata:None,
        operation: op.id,
        destination_id: op.destination.id.clone(),
        expected_source: op.source.observation.identity.clone().unwrap(),
        observed_identity: None,
        sha256: op.source.sha256.clone(),
        phase: Phase::Prepared,
    };
    assert!(matches!(
        journal.append(&mut forbidden, event),
        Err(E::Scope)
    ));
    assert!(
        run_job(
            &binding,
            storage,
            &mut journal,
            &mut forbidden,
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    assert!(forbidden.bytes.is_empty());
    report
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    None,
    ResponseBefore,
    ResponseAfter,
    CancelAfter,
    Ineligible,
    MissingContract,
}
struct FaultStorage {
    inner: WindowsRenameStorage,
    fault: Fault,
    rename_calls: usize,
}
impl FaultStorage {
    fn new(inner: WindowsRenameStorage, fault: Fault) -> Self {
        Self {
            inner,
            fault,
            rename_calls: 0,
        }
    }
}
fn lost(observed: MutationObservation) -> MutationError {
    MutationError::new(
        MutationErrorKind::UnknownOutcome,
        "injected rename response loss",
    )
    .with_observation(observed)
}
impl MutationStorage for FaultStorage {
    type Parent = WindowsParent;
    type SourceLease = WindowsSourceLease;
    type OwnedTemp = WindowsOwnedTemp;
    type Published = WindowsPublished;
    fn capabilities(&self) -> MutationCapabilities {
        self.inner.capabilities()
    }
    fn acquire_source(
        &mut self,
        p: &Self::Parent,
        e: &ExpectedSource,
    ) -> std::result::Result<Self::SourceLease, MutationError> {
        self.inner.acquire_source(p, e)
    }
    fn create_temp(
        &mut self,
        _: &Self::Parent,
        _: &str,
        _: &AtomicBool,
    ) -> MutationOutcome<Self::OwnedTemp> {
        panic!("rename-only driver must never allocate a copy temp")
    }
    fn publish_no_replace(
        &mut self,
        _: &mut Self::OwnedTemp,
        _: &Self::Parent,
        _: &MutationName,
        _: &AtomicBool,
    ) -> MutationOutcome<Self::Published> {
        panic!("rename-only driver must never publish a copy")
    }
    fn rename_no_replace(
        &mut self,
        s: &mut Self::SourceLease,
        p: &Self::Parent,
        n: &MutationName,
        c: &AtomicBool,
    ) -> MutationOutcome<Self::Published> {
        self.rename_calls += 1;
        if self.fault == Fault::ResponseBefore {
            return MutationOutcome::PendingUnknown(lost(s.observation().clone()));
        }
        match self.inner.rename_no_replace(s, p, n, c) {
            MutationOutcome::Applied(published) => {
                if self.fault == Fault::ResponseAfter {
                    let observed = self.inner.inspect(&published).unwrap();
                    MutationOutcome::PendingUnknown(lost(observed))
                } else {
                    if self.fault == Fault::CancelAfter {
                        c.store(true, Ordering::Release);
                    }
                    MutationOutcome::Applied(published)
                }
            }
            other => other,
        }
    }
    fn verify(
        &mut self,
        p: &Self::Published,
        d: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.inner.verify(p, d)
    }
    fn retire_source(
        &mut self,
        _: &mut Self::SourceLease,
        _: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        panic!("atomic rename must never retire a source separately")
    }
    fn inspect(
        &mut self,
        p: &Self::Published,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.inner.inspect(p)
    }
}
impl TransferStorage for FaultStorage {
    fn source_parent(
        &mut self,
        s: &SourceEvidence,
        r: &ObjectIdentity,
    ) -> std::result::Result<Self::Parent, MutationError> {
        self.inner.source_parent(s, r)
    }
    fn destination_parent(
        &mut self,
        d: &DestinationEvidence,
        r: &ObjectIdentity,
    ) -> std::result::Result<Self::Parent, MutationError> {
        self.inner.destination_parent(d, r)
    }
    fn parent_identity(&self, p: &Self::Parent) -> ObjectIdentity {
        self.inner.parent_identity(p)
    }
    fn temp_observation(&self, _: &Self::OwnedTemp) -> MutationObservation {
        panic!("no rename temp")
    }
    fn rewind_source(
        &mut self,
        _: &mut Self::SourceLease,
    ) -> std::result::Result<(), MutationError> {
        panic!("rename cannot stream-copy")
    }
    fn read_source(&mut self, _: &mut Self::SourceLease, _: &mut [u8]) -> io::Result<usize> {
        panic!("rename cannot stream-copy")
    }
    fn write_temp(&mut self, _: &mut Self::OwnedTemp, _: &[u8]) -> io::Result<usize> {
        panic!("no rename temp")
    }
    fn sync_temp(&mut self, _: &mut Self::OwnedTemp) -> std::result::Result<(), MutationError> {
        panic!("no rename temp")
    }
    fn verify_temp(
        &mut self,
        _: &Self::OwnedTemp,
        _: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        panic!("no rename temp")
    }
    fn rename_eligible(&self, s: &Self::SourceLease, p: &Self::Parent) -> bool {
        self.fault != Fault::Ineligible && self.inner.rename_eligible(s, p)
    }
    fn owned_windows_rename_contract(&self) -> bool {
        self.fault != Fault::MissingContract && self.inner.owned_windows_rename_contract()
    }
}

#[test]
fn same_native_identity_unicode_bytes_and_exact_atomic_phases_survive_rename() {
    let f = Fixture::new();
    let scope = f.recovery_scope();
    let expected = f.scope.inspected_sources[0]
        .observation
        .identity
        .clone()
        .unwrap();
    let mut storage = FaultStorage::new(f.owned, Fault::None);
    let mut sink = f.sink;
    assert_eq!(storage.capabilities().durable_flush, Capability::Unknown);
    assert!(storage.owned_windows_rename_contract());
    assert!(WindowsRenameRecovery::new(&storage.inner, &f.binding).is_err());
    let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
    let result = run_job(
        &f.binding,
        &mut storage,
        &mut journal,
        &mut sink,
        &AtomicBool::new(false),
        &mut |_| panic!("rename must not copy chunks"),
    )
    .unwrap();
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 1);
    assert_eq!(result.moved_sources, 1);
    assert_eq!(storage.rename_calls, 1);
    assert_eq!(
        journal.events().iter().map(|e| e.phase).collect::<Vec<_>>(),
        ATOMIC_PHASES
    );
    for event in &journal.events()[1..] {
        assert_eq!(event.observed_identity.as_ref(), Some(&expected));
    }
    assert_eq!(result.items[0].observations[0].identity, expected);
    assert_eq!(
        result.items[0].observations[0].actual_name,
        name("song-שלום-🎵.mp3")
    );
    assert_layout(&f.source, &f.destination, &f.bytes, true);
    let location = sink.location();
    drop(sink);
    let bytes = storage.inner.owned().read_journal(&location).unwrap();
    let before = snapshot(&f.path);
    let report = recovery_without_replay(&mut storage.inner, &scope, &bytes);
    assert_eq!(report.operations[0].status, RecoveryStatus::Committed);
    assert_eq!(snapshot(&f.path), before);
}

#[test]
fn collisions_stale_sources_and_foreign_owned_scopes_are_fail_closed() {
    for collision in [true, false] {
        let mut f = Fixture::new();
        if collision {
            std::fs::write(&f.destination, b"FOREIGN").unwrap();
        } else {
            std::fs::write(&f.source, b"changed").unwrap();
        }
        assert!(
            validate_owned_rename(
                &f.owned,
                &f.plan,
                &mut Metadata,
                &mut Ai,
                &f.scope.resources,
                &f.scope
            )
            .is_err()
        );
        let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut f.sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut f.owned,
            &mut journal,
            &mut f.sink,
            &AtomicBool::new(false),
            &mut |_| panic!("no copy fallback"),
        )
        .unwrap();
        assert_ne!(result.status, JobStatus::Complete);
        assert_eq!(result.committed_destinations, 0);
        assert_eq!(result.moved_sources, 0);
        assert_eq!(
            std::fs::read(&f.source).unwrap(),
            if collision {
                f.bytes.as_slice()
            } else {
                b"changed"
            }
        );
        if collision {
            assert_eq!(std::fs::read(&f.destination).unwrap(), b"FOREIGN");
        } else {
            assert!(!f.destination.exists());
        }
        assert!(
            !f.destination
                .parent()
                .unwrap()
                .join(".sorter-copy.tmp")
                .exists()
        );
        assert!(
            !journal
                .events()
                .iter()
                .any(|e| e.phase == Phase::AtomicRenameObserved)
        );
    }
    let f = Fixture::new();
    let mut other = Fixture::new();
    assert!(
        validate_owned_rename(
            &other.owned,
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut other.sink).unwrap();
    let result = run_job(
        &f.binding,
        &mut other.owned,
        &mut journal,
        &mut other.sink,
        &AtomicBool::new(false),
        &mut |_| panic!("no copy fallback"),
    )
    .unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 0);
    assert_layout(&f.source, &f.destination, &f.bytes, false);
    assert_layout(&other.source, &other.destination, &other.bytes, false);
}

#[test]
fn lost_responses_before_and_after_native_dispatch_preserve_actual_observations() {
    for fault in [Fault::ResponseBefore, Fault::ResponseAfter] {
        let f = Fixture::new();
        let scope = f.recovery_scope();
        let expected = f.scope.inspected_sources[0]
            .observation
            .identity
            .clone()
            .unwrap();
        let mut storage = FaultStorage::new(f.owned, fault);
        let mut sink = f.sink;
        let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut storage,
            &mut journal,
            &mut sink,
            &AtomicBool::new(false),
            &mut |_| panic!("no copy fallback"),
        )
        .unwrap();
        assert_eq!(result.status, JobStatus::NeedsRecovery, "{fault:?}");
        assert_eq!(result.committed_destinations, 0);
        assert_eq!(result.moved_sources, 0);
        assert_eq!(storage.rename_calls, 1);
        assert_eq!(
            result.items[0].error.as_ref().unwrap().kind,
            MutationErrorKind::UnknownOutcome
        );
        assert_eq!(result.items[0].observations.len(), 1);
        let observed = &result.items[0].observations[0];
        assert_eq!(observed.identity, expected);
        let moved = fault == Fault::ResponseAfter;
        let expected_parent = storage
            .inner
            .owned()
            .parent(if moved {
                Path::new("target/אמן")
            } else {
                Path::new("source")
            })
            .unwrap();
        assert_eq!(&observed.parent_identity, expected_parent.identity());
        let mut reads =
            DesktopStorage::new(&[f.path.join("source"), f.path.join("target")]).unwrap();
        let actual = reads.stat(&observed.reference).unwrap();
        assert_eq!(actual.identity, Some(expected));
        assert_eq!(
            actual.parent_identity.as_ref(),
            Some(expected_parent.identity())
        );
        assert_eq!(observed.actual_name, name("song-שלום-🎵.mp3"));
        assert_layout(&f.source, &f.destination, &f.bytes, moved);
        assert_eq!(
            journal.events().iter().map(|e| e.phase).collect::<Vec<_>>(),
            [
                Phase::Prepared,
                Phase::AtomicRenamePrepared,
                Phase::NeedsRecovery
            ]
        );
        let location = sink.location();
        drop(sink);
        let bytes = storage.inner.owned().read_journal(&location).unwrap();
        // A reused source name is never authority to retire the new object.
        if moved {
            std::fs::write(&f.source, b"FOREIGN replacement after rename").unwrap();
        }
        let before = snapshot(&f.path);
        let report = recovery_without_replay(&mut storage.inner, &scope, &bytes);
        assert_eq!(report.operations[0].status, RecoveryStatus::NeedsRecovery);
        assert_eq!(snapshot(&f.path), before);
    }
}

// Cancellation happens only after the selected record was appended and its
// real native journal flush returned successfully. It is not a fake sync error.
struct CancelSink<'a> {
    inner: WindowsJournalStore,
    phase: Phase,
    flag: &'a AtomicBool,
    armed: bool,
    fired: bool,
}
impl JournalStore for CancelSink<'_> {
    fn read_all(&mut self) -> sorter_executor::Result<Vec<u8>> {
        self.inner.read_all()
    }
    fn append(&mut self, bytes: &[u8]) -> sorter_executor::Result<()> {
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        self.armed = value["body"]["event"]["phase"] == serde_json::to_value(self.phase).unwrap();
        self.inner.append(bytes)
    }
    fn sync(&mut self) -> sorter_executor::Result<()> {
        self.inner.sync()?;
        if self.armed {
            self.flag.store(true, Ordering::Release);
            self.fired = true;
        }
        Ok(())
    }
}

#[test]
fn cancellation_before_dispatch_and_after_applied_never_hide_a_completed_effect() {
    for after in [false, true] {
        let f = Fixture::new();
        let scope = f.recovery_scope();
        let mut storage = FaultStorage::new(
            f.owned,
            if after {
                Fault::CancelAfter
            } else {
                Fault::None
            },
        );
        let mut sink = f.sink;
        let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut storage,
            &mut journal,
            &mut sink,
            &AtomicBool::new(!after),
            &mut |_| panic!("no copy fallback"),
        )
        .unwrap();
        assert_eq!(
            result.status,
            if after {
                JobStatus::NeedsRecovery
            } else {
                JobStatus::Cancelled
            }
        );
        assert_eq!(result.committed_destinations, 0);
        assert_eq!(result.moved_sources, 0);
        assert_eq!(storage.rename_calls, usize::from(after));
        assert_eq!(
            result.items[0].error.as_ref().unwrap().kind,
            MutationErrorKind::Cancelled
        );
        assert_eq!(
            journal.events().last().unwrap().phase,
            if after {
                Phase::NeedsRecovery
            } else {
                Phase::Cancelled
            }
        );
        assert!(!journal.events().iter().any(|e| e.phase == Phase::Committed));
        if after {
            assert!(
                result.items[0]
                    .observations
                    .iter()
                    .all(|o| Some(&o.identity)
                        == f.scope.inspected_sources[0].observation.identity.as_ref())
            );
            assert!(!result.items[0].observations.is_empty());
        } else {
            assert!(result.items[0].observations.is_empty());
        }
        assert_layout(&f.source, &f.destination, &f.bytes, after);
        let location = sink.location();
        drop(sink);
        let bytes = storage.inner.owned().read_journal(&location).unwrap();
        let before = snapshot(&f.path);
        let report = recovery_without_replay(&mut storage.inner, &scope, &bytes);
        assert_eq!(report.operations[0].status, RecoveryStatus::NeedsRecovery);
        assert_eq!(snapshot(&f.path), before);
    }
    for phase in [
        Phase::AtomicRenamePrepared,
        Phase::AtomicRenameObserved,
        Phase::DestinationVerified,
    ] {
        let f = Fixture::new();
        let scope = f.recovery_scope();
        let flag = AtomicBool::new(false);
        let mut storage = FaultStorage::new(f.owned, Fault::None);
        let mut sink = CancelSink {
            inner: f.sink,
            phase,
            flag: &flag,
            armed: false,
            fired: false,
        };
        let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut storage,
            &mut journal,
            &mut sink,
            &flag,
            &mut |_| panic!("no copy fallback"),
        )
        .unwrap();
        assert!(
            sink.fired,
            "cancellation boundary was not reached: {phase:?}"
        );
        assert!(flag.load(Ordering::Acquire));
        let after = phase != Phase::AtomicRenamePrepared;
        assert_eq!(
            result.status,
            if after {
                JobStatus::NeedsRecovery
            } else {
                JobStatus::Cancelled
            }
        );
        assert_eq!(result.committed_destinations, 0);
        assert_eq!(result.moved_sources, 0);
        assert_eq!(storage.rename_calls, usize::from(after), "{phase:?}");
        assert_eq!(
            result.items[0].error.as_ref().unwrap().kind,
            MutationErrorKind::Cancelled
        );
        let final_phase = if after {
            Phase::NeedsRecovery
        } else {
            Phase::Cancelled
        };
        let boundary = ATOMIC_PHASES.iter().position(|p| *p == phase).unwrap();
        let mut expected_phases = ATOMIC_PHASES[..=boundary].to_vec();
        expected_phases.push(final_phase);
        assert_eq!(
            journal.events().iter().map(|e| e.phase).collect::<Vec<_>>(),
            expected_phases
        );
        assert!(!journal.events().iter().any(|e| e.phase == Phase::Committed));
        if after {
            assert!(!result.items[0].observations.is_empty());
            let expected = f.scope.inspected_sources[0]
                .observation
                .identity
                .as_ref()
                .unwrap();
            let parent = storage
                .inner
                .owned()
                .parent(Path::new("target/אמן"))
                .unwrap();
            let mut reads =
                DesktopStorage::new(&[f.path.join("source"), f.path.join("target")]).unwrap();
            for observed in &result.items[0].observations {
                assert_eq!(&observed.identity, expected);
                assert_eq!(&observed.parent_identity, parent.identity());
                assert_eq!(observed.actual_name, name("song-שלום-🎵.mp3"));
                let actual = reads.stat(&observed.reference).unwrap();
                assert_eq!(actual.identity.as_ref(), Some(expected));
                assert_eq!(actual.parent_identity.as_ref(), Some(parent.identity()));
            }
        } else {
            assert!(result.items[0].observations.is_empty());
        }
        assert_layout(&f.source, &f.destination, &f.bytes, after);
        let location = sink.inner.location();
        drop(sink);
        let bytes = storage.inner.owned().read_journal(&location).unwrap();
        let before = snapshot(&f.path);
        let report = recovery_without_replay(&mut storage.inner, &scope, &bytes);
        assert_eq!(report.operations[0].status, RecoveryStatus::NeedsRecovery);
        assert_eq!(snapshot(&f.path), before);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JournalFault {
    PartialAppend,
    AppendLost,
    SyncLost,
}
struct FaultSink {
    inner: WindowsJournalStore,
    phase: Phase,
    fault: JournalFault,
    armed: bool,
}
impl JournalStore for FaultSink {
    fn read_all(&mut self) -> sorter_executor::Result<Vec<u8>> {
        self.inner.read_all()
    }
    fn append(&mut self, bytes: &[u8]) -> sorter_executor::Result<()> {
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        self.armed = value["body"]["event"]["phase"] == serde_json::to_value(self.phase).unwrap();
        if self.armed && self.fault == JournalFault::PartialAppend {
            self.inner.append(&bytes[..bytes.len() / 2])?;
            return Err(E::Io);
        }
        self.inner.append(bytes)?;
        if self.armed && self.fault == JournalFault::AppendLost {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
    fn sync(&mut self) -> sorter_executor::Result<()> {
        self.inner.sync()?;
        if self.armed && self.fault == JournalFault::SyncLost {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
}
#[test]
fn partial_append_and_lost_append_or_sync_at_every_atomic_phase_stop_and_recover_read_only() {
    for phase in ATOMIC_PHASES {
        for fault in [
            JournalFault::PartialAppend,
            JournalFault::AppendLost,
            JournalFault::SyncLost,
        ] {
            let f = Fixture::new();
            let scope = f.recovery_scope();
            let mut storage = FaultStorage::new(f.owned, Fault::None);
            let mut sink = FaultSink {
                inner: f.sink,
                phase,
                fault,
                armed: false,
            };
            let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
            let result = run_job(
                &f.binding,
                &mut storage,
                &mut journal,
                &mut sink,
                &AtomicBool::new(false),
                &mut |_| panic!("no copy fallback"),
            )
            .unwrap();
            assert_eq!(
                result.status,
                JobStatus::NeedsRecovery,
                "{phase:?} {fault:?}"
            );
            assert_eq!(result.committed_destinations, 0);
            assert_eq!(result.moved_sources, 0);
            assert!(journal.needs_recovery());
            let moved = !matches!(phase, Phase::Prepared | Phase::AtomicRenamePrepared);
            assert_eq!(storage.rename_calls, usize::from(moved));
            assert_layout(&f.source, &f.destination, &f.bytes, moved);
            let location = sink.inner.location();
            drop(sink);
            let bytes = storage.inner.owned().read_journal(&location).unwrap();
            let before = snapshot(&f.path);
            let recovered =
                open_owned_rename_recovery(&storage.inner, &scope, JOB, &bytes).unwrap();
            assert_eq!(
                !recovered.incomplete_tail().is_empty(),
                fault == JournalFault::PartialAppend
            );
            let report = recovery_without_replay(&mut storage.inner, &scope, &bytes);
            let expected = if fault == JournalFault::PartialAppend {
                RecoveryStatus::NeedsRecovery
            } else if phase == Phase::Committed {
                RecoveryStatus::Committed
            } else if moved {
                RecoveryStatus::EffectProvenAwaitingJournal
            } else if phase == Phase::Prepared {
                RecoveryStatus::ReadyForExplicitResume
            } else {
                RecoveryStatus::NeedsRecovery
            };
            assert_eq!(report.operations[0].status, expected, "{phase:?} {fault:?}");
            assert_eq!(snapshot(&f.path), before);
        }
    }
}

fn forged_header(binding: &str) -> (Vec<u8>, String) {
    let binding_hash = digest(binding.as_bytes());
    let body = format!(
        "{{\"schema\":2,\"job_id\":\"{JOB}\",\"binding\":{binding},\"binding_sha256\":\"{binding_hash}\"}}"
    );
    (
        format!(
            "{{\"body\":{body},\"sha256\":\"{}\"}}\n",
            digest(body.as_bytes())
        )
        .into_bytes(),
        binding_hash,
    )
}
#[test]
fn copy_crossover_mixed_domains_and_rehashed_forgery_cannot_mint_rename_authority() {
    let f = Fixture::new();
    let mut copy = Fixture::with_mode(true);
    assert!(
        validate_owned_copy(
            f.owned.owned(),
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    assert!(
        validate_owned_rename(
            &copy.owned,
            &copy.plan,
            &mut Metadata,
            &mut Ai,
            &copy.scope.resources,
            &copy.scope
        )
        .is_err()
    );
    let copy_scope = copy.recovery_scope();
    let mut copy_header = MemorySink::default();
    PhysicalJournalV2::create(&copy.binding, JOB, &mut copy_header).unwrap();
    assert!(open_owned_rename_recovery(&copy.owned, &copy_scope, JOB, &copy_header.bytes).is_err());
    let recovered_copy =
        open_owned_copy_recovery(copy.owned.owned(), &copy_scope, JOB, &copy_header.bytes).unwrap();
    assert!(WindowsRenameRecovery::new(&copy.owned, recovered_copy.binding()).is_err());
    // A valid Copy binding cannot turn the rename-only native adapter into a copier.
    let mut copy_journal = PhysicalJournalV2::create(&copy.binding, JOB, &mut copy.sink).unwrap();
    let result = run_job(
        &copy.binding,
        &mut copy.owned,
        &mut copy_journal,
        &mut copy.sink,
        &AtomicBool::new(false),
        &mut |_| panic!("no copy fallback"),
    )
    .unwrap();
    assert_eq!(
        result.items[0].error.as_ref().unwrap().kind,
        MutationErrorKind::Unsupported
    );
    assert_layout(&copy.source, &copy.destination, &copy.bytes, false);
    let mut mixed = f.plan.clone();
    mixed.observations[0].reference = StorageRef::TestOpaque {
        id: "forged".into(),
    };
    assert!(
        validate_owned_rename(
            &f.owned,
            &mixed,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    let mut wrong_scope = f.scope.clone();
    wrong_scope.allowed_modes = vec![TransferMode::Copy];
    assert!(
        validate_owned_rename(
            &f.owned,
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &wrong_scope
        )
        .is_err()
    );
    let mut raw = DesktopStorage::new(&[f.path.join("source"), f.path.join("target")]).unwrap();
    assert!(matches!(
        validate_execution(
            &f.plan,
            &mut raw,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        ),
        Err(E::Unsupported)
    ));
    let mut sink = MemorySink::default();
    PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
    let header = String::from_utf8(sink.bytes).unwrap();
    let start = header.find("\"binding\":").unwrap() + "\"binding\":".len();
    let end = header.find(",\"binding_sha256\":").unwrap();
    let original = &header[start..end];
    for variant in 0..6 {
        let forged = match variant {
            0 => original.replace(
                "\"owned_windows_rename\":true",
                "\"owned_windows_copy\":true",
            ),
            1 => original.replace(
                "\"owned_windows_rename\":true",
                "\"owned_windows_copy\":true,\"owned_windows_rename\":true",
            ),
            2 => original.replace("\"synthetic_only\":false", "\"synthetic_only\":true"),
            3 => original.replacen("\"mode\":\"Move\"", "\"mode\":\"Copy\"", 1),
            4 => original.replacen("\"copy_effect\":false", "\"copy_effect\":true", 1),
            _ => {
                let real =
                    serde_json::to_string(&f.binding.operations()[0].source.observation.identity)
                        .unwrap();
                let fake = serde_json::to_string(&Some(ObjectIdentity::Test {
                    id: "forged".into(),
                }))
                .unwrap();
                original.replacen(
                    &format!("\"identity\":{real}"),
                    &format!("\"identity\":{fake}"),
                    1,
                )
            }
        };
        assert_ne!(
            forged, original,
            "forgery variant {variant} must change the input"
        );
        let (bytes, inspected_binding_sha256) = forged_header(&forged);
        let scope = RecoveryScope {
            original_scope: f.scope.clone(),
            inspected_binding_sha256,
        };
        assert!(
            open_owned_rename_recovery(&f.owned, &scope, JOB, &bytes).is_err(),
            "variant {variant}"
        );
    }
    assert!(
        open_owned_rename_recovery(&copy.owned, &f.recovery_scope(), JOB, header.as_bytes())
            .is_err()
    );
    // Hash-valid records are not permission to enter the Copy/retire protocol.
    // Use the real serializer field order and a successful atomic control to
    // distinguish semantic refusal from malformed JSON/checksum rejection.
    #[derive(serde::Serialize)]
    struct AuthoredRecord<'a> {
        sequence: u64,
        job_id: &'a str,
        plan_sha256: &'a str,
        previous: String,
        event: Event,
    }
    let op = &f.binding.operations()[0];
    let event = |phase| Event {
        copy_metadata:None,
        operation: op.id,
        destination_id: op.destination.id.clone(),
        expected_source: op.source.observation.identity.clone().unwrap(),
        observed_identity: if matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared) {
            None
        } else {
            op.source.observation.identity.clone()
        },
        sha256: op.source.sha256.clone(),
        phase,
    };
    let append_authored = |bytes: &mut Vec<u8>, phase| {
        let lines: Vec<_> = bytes
            .split(|b| *b == b'\n')
            .filter(|line| !line.is_empty())
            .collect();
        let last: serde_json::Value = serde_json::from_slice(lines.last().unwrap()).unwrap();
        let record = AuthoredRecord {
            sequence: lines.len() as u64,
            job_id: JOB,
            plan_sha256: f.binding.plan_sha256(),
            previous: last["sha256"].as_str().unwrap().to_owned(),
            event: event(phase),
        };
        let body = serde_json::to_string(&record).unwrap();
        bytes.extend_from_slice(
            format!(
                "{{\"body\":{body},\"sha256\":\"{}\"}}\n",
                digest(body.as_bytes())
            )
            .as_bytes(),
        );
    };
    let mut guarded_sink = MemorySink::default();
    let mut guarded = PhysicalJournalV2::create(&f.binding, JOB, &mut guarded_sink).unwrap();
    guarded
        .append(&mut guarded_sink, event(Phase::Prepared))
        .unwrap();
    let prepared_bytes = guarded_sink.bytes.clone();
    assert!(matches!(
        guarded.append(&mut guarded_sink, event(Phase::TempAllocationPrepared)),
        Err(E::InvalidTransition)
    ));
    assert_eq!(guarded_sink.bytes, prepared_bytes);
    let mut atomic_control = prepared_bytes.clone();
    for phase in ATOMIC_PHASES.into_iter().skip(1) {
        append_authored(&mut atomic_control, phase);
    }
    let accepted =
        open_owned_rename_recovery(&f.owned, &f.recovery_scope(), JOB, &atomic_control).unwrap();
    assert_eq!(
        accepted
            .events()
            .iter()
            .map(|e| e.phase)
            .collect::<Vec<_>>(),
        ATOMIC_PHASES
    );
    for phase in [
        Phase::TempAllocationPrepared,
        Phase::TempAllocated,
        Phase::BytesWritePrepared,
        Phase::BytesSynced,
        Phase::DestinationPublishPrepared,
        Phase::DestinationPublished,
        Phase::SourceRetirementPrepared,
        Phase::SourceRetired,
    ] {
        let mut forbidden = if matches!(
            phase,
            Phase::SourceRetirementPrepared | Phase::SourceRetired
        ) {
            atomic_control.clone()
        } else {
            prepared_bytes.clone()
        };
        append_authored(&mut forbidden, phase);
        assert!(
            matches!(
                open_owned_rename_recovery(&f.owned, &f.recovery_scope(), JOB, &forbidden),
                Err(E::CorruptJournal)
            ),
            "hash-valid forbidden phase {phase:?}"
        );
    }
    assert_layout(&f.source, &f.destination, &f.bytes, false);
}

#[test]
fn missing_rename_eligibility_or_narrow_contract_never_falls_back_to_copy_retire() {
    for fault in [Fault::Ineligible, Fault::MissingContract] {
        let f = Fixture::new();
        let mut storage = FaultStorage::new(f.owned, fault);
        let mut sink = f.sink;
        let mut journal = PhysicalJournalV2::create(&f.binding, JOB, &mut sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut storage,
            &mut journal,
            &mut sink,
            &AtomicBool::new(false),
            &mut |_| panic!("no copy fallback"),
        )
        .unwrap();
        assert_eq!(result.status, JobStatus::Partial);
        assert_eq!(
            result.items[0].error.as_ref().unwrap().kind,
            MutationErrorKind::Unsupported
        );
        assert_eq!(result.committed_destinations, 0);
        assert_eq!(result.moved_sources, 0);
        assert_eq!(storage.rename_calls, 0);
        assert_eq!(
            journal.events().iter().map(|e| e.phase).collect::<Vec<_>>(),
            [Phase::Prepared, Phase::Failed]
        );
        assert_layout(&f.source, &f.destination, &f.bytes, false);
    }
}
