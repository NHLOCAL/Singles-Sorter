//! Generated fixtures only. Real local NTFS operations; response faults are injected.
#![cfg(all(windows, feature = "experimental-windows-copy"))]
use sorter_core::{Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, binding::*, digest, physical_journal::*, recovery::*, results::*,
    transfer::*, windows_copy::*, windows_journal::*,
};
use sorter_io::desktop::{DesktopStorage, windows_mutation::*};
use sorter_storage_contracts::*;
use std::{
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
    owned: WindowsMutationStorage,
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
                copy_mode: true,
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
        let source_reference = reads.reference(&source).unwrap();
        let scope = ExecutionScope {
            inspected_plan: plan.binding_sha256.clone(),
            source_root: reads.stat(&request.source).unwrap().identity.unwrap(),
            target_root: reads
                .stat(request.target.as_ref().unwrap())
                .unwrap()
                .identity
                .unwrap(),
            inspected_sources: vec![SourceEvidence {
                observation: reads.stat(&source_reference).unwrap(),
                sha256: digest(&bytes),
            }],
            resources,
            allowed_modes: vec![TransferMode::Copy],
        };
        drop(reads);
        drop(raw);
        let binding = validate_owned_copy(
            &owned,
            &plan,
            &mut Metadata,
            &mut Ai,
            &scope.resources,
            &scope,
        )
        .unwrap();
        let log_parent = owned.parent(Path::new("journal")).unwrap();
        let sink = WindowsJournalStore::new(
            owned
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
    fn unchanged_source(&self) {
        assert_eq!(std::fs::read(&self.source).unwrap(), self.bytes);
    }
}
#[test]
fn exact_unicode_copy_preserves_source_and_reopens_read_only() {
    let mut f = Fixture::new();
    assert_eq!(f.owned.capabilities().durable_flush, Capability::Unknown);
    let mut journal = PhysicalJournalV2::create(&f.binding, "copy", &mut f.sink).unwrap();
    let flag = AtomicBool::new(false);
    let source = f.source.clone();
    let source_parent = f.path.join("source");
    let mut calls = 0;
    let result = run_job(
        &f.binding,
        &mut f.owned,
        &mut journal,
        &mut f.sink,
        &flag,
        &mut |_| {
            calls += 1;
            assert!(std::fs::write(&source, b"changed").is_err());
            assert!(
                std::fs::rename(&source_parent, source_parent.with_file_name("foreign")).is_err()
            );
        },
    )
    .unwrap();
    assert!(calls > 1);
    assert_eq!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 1);
    assert_eq!(result.moved_sources, 0);
    f.unchanged_source();
    assert_eq!(std::fs::read(&f.destination).unwrap(), f.bytes);
    assert!(
        !f.destination
            .parent()
            .unwrap()
            .join(".sorter-copy.tmp")
            .exists()
    );
    let recovery_scope = f.recovery_scope();
    let location = f.sink.location();
    drop(f.sink);
    let bytes = f.owned.read_journal(&location).unwrap();
    assert!(matches!(
        PhysicalJournalV2::open_for_recovery(&recovery_scope, "copy", &bytes),
        Err(E::Scope)
    ));
    let recovered = open_owned_copy_recovery(&f.owned, &recovery_scope, "copy", &bytes).unwrap();
    let mut observations = WindowsCopyRecovery::new(&f.owned, recovered.binding()).unwrap();
    let report = recover_job(recovered.binding(), &recovered, &mut observations).unwrap();
    assert_eq!(report.operations[0].status, RecoveryStatus::Committed);
    let before = f.owned.read_journal(&location).unwrap();
    let mut forbidden = MemorySink::default();
    assert!(matches!(
        PhysicalJournalV2::create(recovered.binding(), "again", &mut forbidden),
        Err(E::Scope)
    ));
    assert!(forbidden.bytes.is_empty());
    assert_eq!(f.owned.read_journal(&location).unwrap(), before);
}
#[test]
fn collision_and_foreign_temp_are_never_overwritten_or_adopted() {
    for collide_temp in [false, true] {
        let mut f = Fixture::new();
        let foreign = if collide_temp {
            f.destination.parent().unwrap().join(".sorter-copy.tmp")
        } else {
            f.destination.clone()
        };
        std::fs::write(&foreign, b"FOREIGN").unwrap();
        let mut journal = PhysicalJournalV2::create(&f.binding, "copy", &mut f.sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut f.owned,
            &mut journal,
            &mut f.sink,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert_ne!(result.status, JobStatus::Complete);
        assert_eq!(result.committed_destinations, 0);
        assert_eq!(std::fs::read(&foreign).unwrap(), b"FOREIGN");
        f.unchanged_source();
        if collide_temp {
            assert!(!f.destination.exists());
        }
    }
}
#[test]
fn changed_source_and_wrong_owned_tree_fail_before_any_temp() {
    let mut f = Fixture::new();
    std::fs::write(&f.source, b"changed").unwrap();
    assert!(
        validate_owned_copy(
            &f.owned,
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    let mut journal = PhysicalJournalV2::create(&f.binding, "copy", &mut f.sink).unwrap();
    let result = run_job(
        &f.binding,
        &mut f.owned,
        &mut journal,
        &mut f.sink,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert!(!f.destination.exists());
    assert!(
        !f.destination
            .parent()
            .unwrap()
            .join(".sorter-copy.tmp")
            .exists()
    );
    assert_eq!(std::fs::read(&f.source).unwrap(), b"changed");
    let other = Fixture::new();
    assert!(
        validate_owned_copy(
            &other.owned,
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
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
#[test]
fn cancellation_retains_source_and_orphan_then_recovery_cannot_resume() {
    for during in [false, true] {
        let mut f = Fixture::new();
        let flag = AtomicBool::new(!during);
        let mut journal = PhysicalJournalV2::create(&f.binding, "copy", &mut f.sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut f.owned,
            &mut journal,
            &mut f.sink,
            &flag,
            &mut |_| {
                flag.store(true, Ordering::Release);
            },
        )
        .unwrap();
        assert_eq!(result.status, JobStatus::Cancelled);
        assert_eq!(result.committed_destinations, 0);
        f.unchanged_source();
        assert!(!f.destination.exists());
        let temp = f.destination.parent().unwrap().join(".sorter-copy.tmp");
        assert_eq!(temp.exists(), during);
        let temp_before = during.then(|| std::fs::read(&temp).unwrap());
        let scope = f.recovery_scope();
        let location = f.sink.location();
        drop(f.sink);
        let bytes = f.owned.read_journal(&location).unwrap();
        let mut recovered = open_owned_copy_recovery(&f.owned, &scope, "copy", &bytes).unwrap();
        let mut observations = WindowsCopyRecovery::new(&f.owned, recovered.binding()).unwrap();
        let _ = recover_job(recovered.binding(), &recovered, &mut observations).unwrap();
        let binding = recovered.binding().clone();
        let mut forbidden = MemorySink::default();
        assert!(
            run_job(
                &binding,
                &mut f.owned,
                &mut recovered,
                &mut forbidden,
                &AtomicBool::new(false),
                &mut |_| {}
            )
            .is_err()
        );
        assert!(forbidden.bytes.is_empty());
        assert_eq!(f.owned.read_journal(&location).unwrap(), bytes);
        if let Some(before) = temp_before {
            assert_eq!(std::fs::read(&temp).unwrap(), before);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Fault {
    Short,
    PartialWriteLost,
    SyncLost,
    PublishLost,
}
struct FaultStorage {
    inner: WindowsMutationStorage,
    fault: Fault,
}
fn lost() -> MutationError {
    MutationError::new(
        MutationErrorKind::UnknownOutcome,
        "injected lost response after real I/O",
    )
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
        p: &Self::Parent,
        job: &str,
        c: &AtomicBool,
    ) -> MutationOutcome<Self::OwnedTemp> {
        self.inner.create_temp(p, job, c)
    }
    fn publish_no_replace(
        &mut self,
        t: &mut Self::OwnedTemp,
        p: &Self::Parent,
        n: &MutationName,
        c: &AtomicBool,
    ) -> MutationOutcome<Self::Published> {
        let result = self.inner.publish_no_replace(t, p, n, c);
        if self.fault == Fault::PublishLost && matches!(result, MutationOutcome::Applied(_)) {
            MutationOutcome::PendingUnknown(lost())
        } else {
            result
        }
    }
    fn rename_no_replace(
        &mut self,
        _: &mut Self::SourceLease,
        _: &Self::Parent,
        _: &MutationName,
        _: &AtomicBool,
    ) -> MutationOutcome<Self::Published> {
        panic!("Copy cannot rename source")
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
        panic!("Copy cannot retire source")
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
    fn temp_observation(&self, t: &Self::OwnedTemp) -> MutationObservation {
        self.inner.temp_observation(t)
    }
    fn rewind_source(
        &mut self,
        s: &mut Self::SourceLease,
    ) -> std::result::Result<(), MutationError> {
        self.inner.rewind_source(s)
    }
    fn read_source(&mut self, s: &mut Self::SourceLease, b: &mut [u8]) -> io::Result<usize> {
        self.inner.read_source(s, b)
    }
    fn write_temp(&mut self, t: &mut Self::OwnedTemp, b: &[u8]) -> io::Result<usize> {
        let bound = b.len().min(997);
        let result = self.inner.write_temp(t, &b[..bound]);
        if self.fault == Fault::PartialWriteLost && result.is_ok() {
            Err(io::Error::other(
                "injected response loss after partial write",
            ))
        } else {
            result
        }
    }
    fn sync_temp(&mut self, t: &mut Self::OwnedTemp) -> std::result::Result<(), MutationError> {
        self.inner.sync_temp(t)?;
        if self.fault == Fault::SyncLost {
            Err(lost())
        } else {
            Ok(())
        }
    }
    fn verify_temp(
        &mut self,
        t: &Self::OwnedTemp,
        d: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.inner.verify_temp(t, d)
    }
    fn rename_eligible(&self, _: &Self::SourceLease, _: &Self::Parent) -> bool {
        false
    }
}
#[test]
fn short_writes_complete_but_write_sync_and_publish_response_loss_never_claim_success() {
    for fault in [
        Fault::Short,
        Fault::PartialWriteLost,
        Fault::SyncLost,
        Fault::PublishLost,
    ] {
        let f = Fixture::new();
        let scope = f.recovery_scope();
        let mut sink = f.sink;
        let mut storage = FaultStorage {
            inner: f.owned,
            fault,
        };
        let mut journal = PhysicalJournalV2::create(&f.binding, "copy", &mut sink).unwrap();
        let result = run_job(
            &f.binding,
            &mut storage,
            &mut journal,
            &mut sink,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(result.status == JobStatus::Complete, fault == Fault::Short);
        assert_eq!(
            result.committed_destinations,
            usize::from(fault == Fault::Short)
        );
        assert_eq!(std::fs::read(&f.source).unwrap(), f.bytes);
        if matches!(fault, Fault::Short | Fault::PublishLost) {
            assert_eq!(std::fs::read(&f.destination).unwrap(), f.bytes);
        } else {
            assert!(!f.destination.exists());
        }
        if fault == Fault::SyncLost || fault == Fault::PublishLost {
            assert_eq!(result.status, JobStatus::NeedsRecovery);
        }
        let location = sink.location();
        drop(sink);
        let bytes = storage.inner.read_journal(&location).unwrap();
        let recovered = open_owned_copy_recovery(&storage.inner, &scope, "copy", &bytes).unwrap();
        let mut observations =
            WindowsCopyRecovery::new(&storage.inner, recovered.binding()).unwrap();
        let report = recover_job(recovered.binding(), &recovered, &mut observations).unwrap();
        assert_eq!(
            report.operations[0].status == RecoveryStatus::Committed,
            fault == Fault::Short
        );
        assert_eq!(storage.inner.read_journal(&location).unwrap(), bytes);
    }
}
struct FaultSink {
    inner: WindowsJournalStore,
    phase: Phase,
    partial: bool,
    fail_sync: bool,
    armed: bool,
}
impl JournalStore for FaultSink {
    fn read_all(&mut self) -> sorter_executor::Result<Vec<u8>> {
        self.inner.read_all()
    }
    fn append(&mut self, bytes: &[u8]) -> sorter_executor::Result<()> {
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        self.armed = value["body"]["event"]["phase"] == serde_json::to_value(self.phase).unwrap();
        if self.armed && self.partial {
            self.inner.append(&bytes[..bytes.len() / 2])?;
            return Err(E::Io);
        }
        self.inner.append(bytes)?;
        if self.armed && !self.fail_sync {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
    fn sync(&mut self) -> sorter_executor::Result<()> {
        self.inner.sync()?;
        if self.armed && self.fail_sync {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
}
#[test]
fn journal_partial_write_and_lost_flush_at_every_copy_phase_stop_and_reopen_without_mutation() {
    for phase in [
        Phase::Prepared,
        Phase::TempAllocationPrepared,
        Phase::TempAllocated,
        Phase::BytesWritePrepared,
        Phase::BytesSynced,
        Phase::DestinationPublishPrepared,
        Phase::DestinationPublished,
        Phase::DestinationVerified,
        Phase::Committed,
    ] {
        for (partial, fail_sync) in [(true, false), (false, false), (false, true)] {
            let f = Fixture::new();
            let scope = f.recovery_scope();
            let mut owned = f.owned;
            let mut sink = FaultSink {
                inner: f.sink,
                phase,
                partial,
                fail_sync,
                armed: false,
            };
            let mut journal = PhysicalJournalV2::create(&f.binding, "copy", &mut sink).unwrap();
            let result = run_job(
                &f.binding,
                &mut owned,
                &mut journal,
                &mut sink,
                &AtomicBool::new(false),
                &mut |_| {},
            )
            .unwrap();
            assert_eq!(result.status, JobStatus::NeedsRecovery, "{phase:?}");
            assert_eq!(result.committed_destinations, 0);
            assert_eq!(std::fs::read(&f.source).unwrap(), f.bytes);
            let destination_before = f
                .destination
                .exists()
                .then(|| std::fs::read(&f.destination).unwrap());
            if let Some(bytes) = &destination_before {
                assert_eq!(*bytes, f.bytes);
            }
            let location = sink.inner.location();
            drop(sink);
            let bytes = owned.read_journal(&location).unwrap();
            let recovered = open_owned_copy_recovery(&owned, &scope, "copy", &bytes).unwrap();
            let mut observations = WindowsCopyRecovery::new(&owned, recovered.binding()).unwrap();
            let _ = recover_job(recovered.binding(), &recovered, &mut observations).unwrap();
            assert_eq!(owned.read_journal(&location).unwrap(), bytes);
            assert_eq!(
                f.destination
                    .exists()
                    .then(|| std::fs::read(&f.destination).unwrap()),
                destination_before
            );
        }
    }
}
#[test]
fn mixed_domains_scope_forgery_and_disabled_modes_are_rejected() {
    let f = Fixture::new();
    let mut mixed = f.plan.clone();
    mixed.observations[0].reference = StorageRef::TestOpaque { id: "fake".into() };
    assert!(
        validate_owned_copy(
            &f.owned,
            &mixed,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    let mut changed = f.plan.clone();
    changed.request.settings.copy_mode = false;
    assert!(
        validate_owned_copy(
            &f.owned,
            &changed,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    changed = f.plan.clone();
    changed.request.settings.main_folder_only = false;
    assert!(
        validate_owned_copy(
            &f.owned,
            &changed,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        )
        .is_err()
    );
    let mut wrong_scope = f.scope.clone();
    wrong_scope.target_root = ObjectIdentity::Test { id: "fake".into() };
    assert!(
        validate_owned_copy(
            &f.owned,
            &f.plan,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &wrong_scope
        )
        .is_err()
    );
    let mut read = DesktopStorage::new(&[f.path.join("source"), f.path.join("target")]).unwrap();
    assert!(matches!(
        validate_execution(
            &f.plan,
            &mut read,
            &mut Metadata,
            &mut Ai,
            &f.scope.resources,
            &f.scope
        ),
        Err(E::Unsupported)
    ));
    let raw_plan = plan_job(&f.plan.request, &mut read, &mut Metadata, &mut Ai).unwrap();
    assert!(raw_plan.blockers.iter().any(|b| b == "UNKNOWN_CAPABILITY"));
    // A failed native probe cannot become a supported capability observation.
    let mut probed = OwnedCopyReadStorage::new(
        &f.owned,
        &f.plan.request.source,
        f.plan.request.target.as_ref().unwrap(),
    )
    .unwrap();
    use std::os::windows::fs::OpenOptionsExt;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(&f.source)
        .unwrap();
    assert!(
        probed
            .stat(&f.scope.inspected_sources[0].observation.reference)
            .is_err()
    );
    drop(lock);
    f.unchanged_source();
    assert!(!f.destination.exists());
}

#[test]
fn serialized_domain_flags_and_foreign_capabilities_cannot_mint_copy_authority() {
    let f = Fixture::new();
    assert!(WindowsCopyRecovery::new(&f.owned, &f.binding).is_err());
    let mut sink = MemorySink::default();
    PhysicalJournalV2::create(&f.binding, "copy", &mut sink).unwrap();
    let header = String::from_utf8(sink.bytes.clone()).unwrap();
    let begin = header.find("\"binding\":").unwrap() + "\"binding\":".len();
    let end = header.find(",\"binding_sha256\":").unwrap();
    let original = &header[begin..end];
    for variant in 0..5 {
        let forged = match variant {
            0 => original
                .replace("\"synthetic_only\":false", "\"synthetic_only\":true")
                .replace(",\"owned_windows_copy\":true", ""),
            1 => original.replace("\"synthetic_only\":false", "\"synthetic_only\":true"),
            2 => original.replacen("\"mode\":\"Copy\"", "\"mode\":\"Move\"", 1),
            3 => {
                let start = original.find("\"operations\":[").unwrap() + "\"operations\":[".len();
                let end = original.find("],\"scope_sha256\":").unwrap();
                format!(
                    "{},{}{}",
                    &original[..end],
                    &original[start..end],
                    &original[end..]
                )
            }
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
        // Preserve struct field order while recomputing both public hashes.
        let binding_hash = digest(forged.as_bytes());
        let body = format!(
            "{{\"schema\":2,\"job_id\":\"copy\",\"binding\":{forged},\"binding_sha256\":\"{binding_hash}\"}}"
        );
        let bytes = format!(
            "{{\"body\":{body},\"sha256\":\"{}\"}}\n",
            digest(body.as_bytes())
        )
        .into_bytes();
        let scope = RecoveryScope {
            original_scope: f.scope.clone(),
            inspected_binding_sha256: binding_hash,
        };
        assert!(open_owned_copy_recovery(&f.owned, &scope, "copy", &bytes).is_err());
    }
    let mut other = Fixture::new();
    assert!(
        other
            .owned
            .parent_reference(&f.plan.request.source)
            .is_err()
    );
    assert!(other.owned.parent(Path::new("../source")).is_err());
    // Lexical case-sensitive prefix checks alone miss NTFS ancestor aliases.
    std::fs::create_dir_all(other.path.join("source/a/b/target")).unwrap();
    use std::os::windows::ffi::OsStrExt;
    let nested = StorageRef::WindowsPath {
        utf16: other
            .path
            .join("SOURCE/a/b/target")
            .as_os_str()
            .encode_wide()
            .collect(),
    };
    assert!(matches!(
        OwnedCopyReadStorage::new(&other.owned, &other.plan.request.source, &nested),
        Err(E::Scope)
    ));
    assert!(matches!(
        OwnedCopyReadStorage::new(&other.owned, &nested, &other.plan.request.source),
        Err(E::Scope)
    ));

    let mut journal = PhysicalJournalV2::create(&f.binding, "foreign", &mut other.sink).unwrap();
    let result = run_job(
        &f.binding,
        &mut other.owned,
        &mut journal,
        &mut other.sink,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    assert_ne!(result.status, JobStatus::Complete);
    assert_eq!(result.committed_destinations, 0);
    f.unchanged_source();
    other.unchanged_source();
    assert!(!f.destination.exists());
    assert!(!other.destination.exists());
}
