mod support;
use sha2::{Digest, Sha256};
use sorter_core::discovery::{ObjectIdentity, StorageRef};
use sorter_executor::{ExecutionError, binding::*, physical_journal::*, results::*, transfer::*};
use sorter_storage_contracts::*;
use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use support::*;
fn id(s: &str) -> ObjectIdentity {
    ObjectIdentity::Test { id: s.into() }
}
fn obs(s: &str, parent: &str, name: MutationName, len: usize) -> MutationObservation {
    MutationObservation {
        reference: StorageRef::TestOpaque { id: s.into() },
        identity: id(s),
        parent_identity: id(parent),
        actual_name: name,
        size: Some(len as u64),
    }
}
fn err() -> MutationError {
    MutationError::new(MutationErrorKind::Io, "injected")
}
struct Lease {
    cursor: Cursor<Vec<u8>>,
    expected: ExpectedSource,
}
#[derive(Clone)]
struct Temp {
    data: Vec<u8>,
    observation: MutationObservation,
}
struct Fake {
    source: Option<Vec<u8>>,
    temps: usize,
    published: HashMap<String, Temp>,
    retirements: usize,
    renames: usize,
    trace: Vec<&'static str>,
    fault: Option<(&'static str, usize)>,
    seen: HashMap<&'static str, usize>,
    cancel_at: Option<&'static str>,
    flag: Arc<AtomicBool>,
    short: usize,
    atomic: bool,
    pending: Option<&'static str>,
    deferred: bool,
    corrupt_temp: bool,
    corrupt_published: bool,
    wrong_name: bool,
    source_replaced: bool,
    replace_at: Option<&'static str>,
    unknown_flush: bool,
    zero_write: bool,
    disk_full: bool,
    altered_stream: Option<Vec<u8>>,
    interrupt_read: bool,
    interrupt_write: bool,
    max_buffer: usize,
}
impl Fake {
    fn new(data: &[u8], flag: Arc<AtomicBool>) -> Self {
        Self {
            source: Some(data.to_vec()),
            temps: 0,
            published: HashMap::new(),
            retirements: 0,
            renames: 0,
            trace: vec![],
            fault: None,
            seen: HashMap::new(),
            cancel_at: None,
            flag,
            short: 2,
            atomic: false,
            pending: None,
            deferred: false,
            corrupt_temp: false,
            corrupt_published: false,
            wrong_name: false,
            source_replaced: false,
            replace_at: None,
            unknown_flush: false,
            zero_write: false,
            disk_full: false,
            altered_stream: None,
            interrupt_read: false,
            interrupt_write: false,
            max_buffer: 0,
        }
    }
    fn hit(&mut self, at: &'static str) -> std::result::Result<(), MutationError> {
        self.trace.push(at);
        if self.replace_at == Some(at) {
            self.source_replaced = true;
        }
        let n = self.seen.entry(at).or_default();
        *n += 1;
        if self.cancel_at == Some(at) {
            self.flag.store(true, Ordering::Release)
        }
        if self.fault == Some((at, *n)) {
            Err(err())
        } else {
            Ok(())
        }
    }
    fn pending<T>(&self, at: &str) -> Option<MutationOutcome<T>> {
        (self.pending == Some(at)).then(|| {
            MutationOutcome::PendingUnknown(MutationError::new(
                MutationErrorKind::UnknownOutcome,
                "response lost",
            ))
        })
    }
}
impl MutationStorage for Fake {
    type Parent = String;
    type SourceLease = Lease;
    type OwnedTemp = Temp;
    type Published = String;
    fn capabilities(&self) -> MutationCapabilities {
        MutationCapabilities {
            seek: Capability::Supported,
            exclusive_create: Capability::Supported,
            no_replace_publish: Capability::Supported,
            same_volume_no_replace_rename: Capability::Supported,
            content_verification: Capability::Supported,
            durable_flush: if self.unknown_flush {
                Capability::Unknown
            } else {
                Capability::Supported
            },
            leased_source_retirement: Capability::Supported,
            staged_replacement: Capability::Unsupported,
        }
    }
    fn acquire_source(
        &mut self,
        _: &String,
        e: &ExpectedSource,
    ) -> std::result::Result<Lease, MutationError> {
        self.hit("acquire")?;
        let data = self.source.as_ref().ok_or_else(err)?;
        let digest: [u8; 32] = Sha256::digest(data).into();
        if self.source_replaced || digest != e.digest.sha256 || data.len() as u64 != e.digest.length
        {
            return Err(MutationError::new(
                MutationErrorKind::StalePlan,
                "replacement",
            ));
        }
        Ok(Lease {
            cursor: Cursor::new(data.clone()),
            expected: e.clone(),
        })
    }
    fn create_temp(&mut self, p: &String, _: &str, c: &AtomicBool) -> MutationOutcome<Temp> {
        if c.load(Ordering::Acquire) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Cancelled,
                "cancel",
            ));
        }
        if let Err(e) = self.hit("allocate") {
            return MutationOutcome::NoEffect(e);
        }
        if let Some(p) = self.pending("allocate") {
            return p;
        }
        self.temps += 1;
        MutationOutcome::Applied(Temp {
            data: vec![],
            observation: obs(
                &format!("temp{}", self.temps),
                p,
                MutationName::Provider(format!("tmp{}", self.temps)),
                0,
            ),
        })
    }
    fn publish_no_replace(
        &mut self,
        t: &mut Temp,
        p: &String,
        n: &MutationName,
        c: &AtomicBool,
    ) -> MutationOutcome<String> {
        if c.load(Ordering::Acquire) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Cancelled,
                "cancel",
            ));
        }
        if let Err(e) = self.hit("publish") {
            return MutationOutcome::NoEffect(e);
        }
        let key = format!("{p}/{n:?}");
        if self.published.contains_key(&key) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::StalePlan,
                "collision",
            ));
        }
        let mut t = t.clone();
        t.observation.actual_name = if self.wrong_name {
            MutationName::Provider("wrong.mp3".into())
        } else {
            n.clone()
        };
        t.observation.size = Some(t.data.len() as u64);
        self.published.insert(key.clone(), t);
        if let Some(p) = self.pending("publish") {
            return p;
        }
        MutationOutcome::Applied(key)
    }
    fn rename_no_replace(
        &mut self,
        s: &mut Lease,
        p: &String,
        n: &MutationName,
        c: &AtomicBool,
    ) -> MutationOutcome<String> {
        if c.load(Ordering::Acquire) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Cancelled,
                "cancel",
            ));
        }
        if let Err(e) = self.hit("rename") {
            return MutationOutcome::NoEffect(e);
        }
        let key = format!("{p}/{n:?}");
        if self.published.contains_key(&key) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::StalePlan,
                "collision",
            ));
        }
        let data = self.source.take().unwrap();
        let mut o = obs("song", p, n.clone(), data.len());
        o.identity = s.expected.identity.clone();
        self.published.insert(
            key.clone(),
            Temp {
                data,
                observation: o,
            },
        );
        self.renames += 1;
        if let Some(p) = self.pending("rename") {
            return p;
        }
        MutationOutcome::Applied(key)
    }
    fn verify(
        &mut self,
        p: &String,
        d: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.hit("verify")?;
        let t = &self.published[p];
        let hash: [u8; 32] = Sha256::digest(&t.data).into();
        if self.corrupt_published || hash != d.sha256 || t.data.len() as u64 != d.length {
            return Err(MutationError::new(
                MutationErrorKind::VerificationFailed,
                "readback",
            ));
        }
        Ok(t.observation.clone())
    }
    fn retire_source(
        &mut self,
        s: &mut Lease,
        c: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        if c.load(Ordering::Acquire) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Cancelled,
                "cancel",
            ));
        }
        if let Err(e) = self.hit("retire") {
            return MutationOutcome::NoEffect(e);
        }
        if self.source_replaced {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::StalePlan,
                "replacement",
            ));
        }
        let o = obs(
            "song",
            "source",
            s.expected.name.clone(),
            s.expected.digest.length as usize,
        );
        if self.deferred {
            return MutationOutcome::Applied(RetirementOutcome::DispositionRequested(o));
        }
        self.source = None;
        self.retirements += 1;
        if let Some(p) = self.pending("retire") {
            return p;
        }
        MutationOutcome::Applied(RetirementOutcome::Retired(o))
    }
    fn inspect(&mut self, p: &String) -> std::result::Result<MutationObservation, MutationError> {
        self.hit("inspect")?;
        Ok(self.published[p].observation.clone())
    }
}
impl TransferStorage for Fake {
    fn source_parent(
        &mut self,
        _: &SourceEvidence,
        r: &ObjectIdentity,
    ) -> std::result::Result<String, MutationError> {
        assert_eq!(r, &id("source"));
        self.hit("source_parent")?;
        Ok("source".into())
    }
    fn destination_parent(
        &mut self,
        d: &DestinationEvidence,
        r: &ObjectIdentity,
    ) -> std::result::Result<String, MutationError> {
        assert_eq!(r, &id("target"));
        self.hit("destination_parent")?;
        Ok(d.components[..d.components.len() - 1].join("/"))
    }
    fn parent_identity(&self, p: &String) -> ObjectIdentity {
        id(p)
    }
    fn temp_observation(&self, t: &Temp) -> MutationObservation {
        t.observation.clone()
    }
    fn rewind_source(&mut self, s: &mut Lease) -> std::result::Result<(), MutationError> {
        self.hit("rewind")?;
        if let Some(bytes) = self.altered_stream.take() {
            s.cursor = Cursor::new(bytes);
        }
        s.cursor.set_position(0);
        Ok(())
    }
    fn read_source(&mut self, s: &mut Lease, b: &mut [u8]) -> io::Result<usize> {
        self.max_buffer = self.max_buffer.max(b.len());
        self.hit("read").map_err(io::Error::other)?;
        if self.interrupt_read {
            self.interrupt_read = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        s.cursor.read(b)
    }
    fn write_temp(&mut self, t: &mut Temp, b: &[u8]) -> io::Result<usize> {
        self.hit("write").map_err(io::Error::other)?;
        if self.disk_full {
            return Err(io::ErrorKind::StorageFull.into());
        }
        if self.interrupt_write {
            self.interrupt_write = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.zero_write {
            return Ok(0);
        }
        let n = b.len().min(self.short);
        t.data.extend(&b[..n]);
        Ok(n)
    }
    fn sync_temp(&mut self, _: &mut Temp) -> std::result::Result<(), MutationError> {
        self.hit("sync")
    }
    fn verify_temp(
        &mut self,
        t: &Temp,
        d: &ContentDigest,
    ) -> std::result::Result<MutationObservation, MutationError> {
        self.hit("temp_verify")?;
        let hash: [u8; 32] = Sha256::digest(&t.data).into();
        if self.corrupt_temp || hash != d.sha256 || t.data.len() as u64 != d.length {
            return Err(MutationError::new(
                MutationErrorKind::VerificationFailed,
                "temp mismatch",
            ));
        }
        let mut o = t.observation.clone();
        o.size = Some(t.data.len() as u64);
        Ok(o)
    }
    fn rename_eligible(&self, _: &Lease, _: &String) -> bool {
        self.atomic
    }
}
fn execute(
    binding: &ExecutionBinding,
    fake: &mut Fake,
    sink: &mut Sink,
) -> (JobResult, PhysicalJournalV2) {
    let mut journal = PhysicalJournalV2::create(binding, "test-job", sink).unwrap();
    let flag = fake.flag.clone();
    let result = run_job(binding, fake, &mut journal, sink, &flag, &mut |_| {}).unwrap();
    (result, journal)
}
#[test]
fn copies_short_writes_and_verifies_before_retirement() {
    let b = binding(false);
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.interrupt_read = true;
    f.interrupt_write = true;
    let (r, _) = execute(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Complete);
    assert_eq!(r.committed_destinations, 1);
    assert_eq!(r.committed_operations, vec![0]);
    assert_eq!(
        r.effect_receipts
            .iter()
            .map(|receipt| (receipt.operation, receipt.kind.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "file_published"), (0, "source_retired")]
    );
    assert_eq!(r.moved_sources, 1);
    assert_eq!(f.retirements, 1);
    assert!(f.source.is_none());
    assert_eq!(f.seen["verify"], 2);
}
#[test]
fn duplicate_artists_require_two_distinct_suffixed_destinations() {
    let b = binding(true);
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    let (r, _) = execute(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Complete);
    assert_eq!(r.committed_destinations, 2);
    assert_eq!(r.committed_operations, vec![0, 1]);
    assert_eq!(
        r.effect_receipts
            .iter()
            .map(|receipt| (receipt.operation, receipt.kind.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (0, "file_published"),
            (1, "file_published"),
            (1, "source_retired")
        ]
    );
    assert_eq!(r.moved_sources, 1);
    assert_eq!(f.retirements, 1);
    assert_eq!(f.published.len(), 2);
    assert_eq!(f.seen["acquire"], 1);
}
#[test]
fn copy_mode_never_retires_or_renames() {
    let b = binding_with(false, true, b"audio");
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.atomic = true;
    let (r, _) = execute(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Complete);
    assert_eq!(r.moved_sources, 0);
    assert!(f.source.is_some());
    assert_eq!(f.renames, 0);
}
#[test]
fn every_copy_primitive_failure_preserves_source() {
    for point in [
        "source_parent",
        "destination_parent",
        "acquire",
        "allocate",
        "rewind",
        "read",
        "write",
        "sync",
        "temp_verify",
        "publish",
        "inspect",
        "verify",
        "retire",
    ] {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        f.fault = Some((point, 1));
        let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
        assert!(f.source.is_some(), "{point}");
        assert_eq!(r.moved_sources, 0, "{point}");
        assert_ne!(r.status, JobStatus::Complete, "{point}");
    }
}
#[test]
fn duet_failure_and_reverification_failure_preserve_source() {
    for (point, n) in [("publish", 2), ("verify", 2), ("verify", 3), ("verify", 4)] {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        f.fault = Some((point, n));
        let (r, _) = execute(&binding(true), &mut f, &mut Sink::default());
        assert!(f.source.is_some());
        assert_eq!(r.moved_sources, 0);
        assert_ne!(r.status, JobStatus::Complete);
    }
}
#[test]
fn corruption_and_stale_identity_never_retire() {
    for mode in 0..5 {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        match mode {
            0 => f.corrupt_temp = true,
            1 => f.corrupt_published = true,
            2 => f.wrong_name = true,
            3 => f.source_replaced = true,
            _ => f.zero_write = true,
        };
        let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
        assert!(f.source.is_some());
        assert_eq!(r.moved_sources, 0);
        assert_ne!(r.status, JobStatus::Complete);
    }
}
#[test]
fn cancellation_at_every_copy_phase_keeps_source_except_known_retirement() {
    for point in [
        "source_parent",
        "acquire",
        "destination_parent",
        "allocate",
        "rewind",
        "read",
        "write",
        "sync",
        "temp_verify",
        "publish",
        "inspect",
        "verify",
        "retire",
    ] {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        f.cancel_at = Some(point);
        let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
        assert_eq!(r.status, JobStatus::Cancelled, "{point}");
        if point == "retire" {
            assert_eq!(r.moved_sources, 1);
            assert!(f.source.is_none())
        } else {
            assert!(f.source.is_some(), "{point}");
            assert_eq!(r.moved_sources, 0, "{point}")
        }
    }
}
#[test]
fn pending_and_deferred_outcomes_are_never_false_cancelled_or_completed() {
    for point in ["allocate", "publish", "retire"] {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        f.pending = Some(point);
        f.cancel_at = Some(point);
        let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
        assert_eq!(r.status, JobStatus::NeedsRecovery, "{point}");
        assert_eq!(r.moved_sources, 0);
    }
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.deferred = true;
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::NeedsRecovery);
    assert_eq!(r.committed_destinations, 1);
    assert_eq!(r.moved_sources, 0);
    assert!(f.source.is_some());
}
#[test]
fn atomic_rename_is_single_unmodified_destination_only() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.atomic = true;
    let (r, j) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Complete);
    assert_eq!(r.moved_sources, 1);
    assert_eq!(f.renames, 1);
    assert_eq!(f.temps, 0);
    assert_eq!(f.retirements, 0);
    assert!(
        !j.events()
            .iter()
            .any(|e| e.phase == Phase::SourceRetirementPrepared)
    );
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.atomic = true;
    let (r, _) = execute(&binding(true), &mut f, &mut Sink::default());
    assert_eq!(r.committed_destinations, 2);
    assert_eq!(f.renames, 0);
}
#[test]
fn cancellation_during_known_rename_records_its_real_effect() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.atomic = true;
    f.cancel_at = Some("rename");
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.moved_sources, 1);
    assert_eq!(r.committed_destinations, 1);
    assert_eq!(r.status, JobStatus::Cancelled);
}
#[test]
fn lost_rename_response_requires_recovery_and_no_retry() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.atomic = true;
    f.pending = Some("rename");
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::NeedsRecovery);
    assert_eq!(r.moved_sources, 0);
    assert_eq!(f.renames, 1);
    assert!(f.source.is_none());
}
#[test]
fn bounded_stream_handles_multiple_buffers() {
    let bytes = vec![17; TRANSFER_BUFFER_BYTES * 3 + 17];
    let b = binding_with(false, false, &bytes);
    let mut f = Fake::new(&bytes, Arc::new(AtomicBool::new(false)));
    f.short = 13001;
    let (r, _) = execute(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Complete);
    assert!(f.max_buffer <= TRANSFER_BUFFER_BYTES);
    assert_eq!(f.published.values().next().unwrap().data, bytes);
}
#[test]
fn move_rejects_unknown_durability_before_allocating() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.unknown_flush = true;
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Partial);
    assert_eq!(f.temps, 0);
    assert!(f.source.is_some());
}
#[test]
fn every_journal_failure_stops_without_false_move_success() {
    for atomic in [false, true] {
        let binding = binding(false);
        let flag = Arc::new(AtomicBool::new(false));
        let mut probe = Fake::new(b"audio", flag.clone());
        probe.atomic = atomic;
        let mut baseline = Sink::default();
        let (_, j) = execute(&binding, &mut probe, &mut baseline);
        let boundaries = baseline.boundary;
        assert!(!j.events().is_empty());
        for boundary in 4..boundaries {
            let mut sink = Sink::default();
            let mut journal = PhysicalJournalV2::create(&binding, "test-job", &mut sink).unwrap();
            sink.fail = Some(boundary);
            let mut f = Fake::new(b"audio", flag.clone());
            f.atomic = atomic;
            let r = run_job(
                &binding,
                &mut f,
                &mut journal,
                &mut sink,
                &flag,
                &mut |_| {},
            )
            .unwrap();
            assert_eq!(r.status, JobStatus::NeedsRecovery, "{atomic}/{boundary}");
            assert_eq!(r.moved_sources, 0, "{atomic}/{boundary}");
            if !atomic && f.source.is_none() {
                assert_eq!(r.committed_destinations, 1);
                assert!(
                    journal
                        .events()
                        .iter()
                        .any(|e| e.phase == Phase::SourceRetirementPrepared)
                );
            }
            if !f.published.is_empty() {
                assert!(journal.events().iter().any(|e| matches!(
                    e.phase,
                    Phase::DestinationPublishPrepared | Phase::AtomicRenamePrepared
                )));
            }
        }
    }
}
#[test]
fn an_existing_journal_is_never_implicitly_resumed() {
    let b = binding(false);
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    let mut sink = Sink::default();
    let (_, mut journal) = execute(&b, &mut f, &mut sink);
    let flag = f.flag.clone();
    assert_eq!(
        run_job(&b, &mut f, &mut journal, &mut sink, &flag, &mut |_| {}),
        Err(ExecutionError::NeedsRecovery)
    );
}

#[test]
fn replacement_before_retirement_is_rejected_by_the_same_lease() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.replace_at = Some("retire");
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.committed_destinations, 1);
    assert_eq!(r.moved_sources, 0);
    assert_eq!(r.status, JobStatus::Partial);
    assert!(f.source.is_some());
}
#[test]
fn newly_occupied_planned_destination_is_not_suffixed_or_overwritten() {
    let b = binding(false);
    let op = &b.operations()[0];
    let p = op.destination.components[..op.destination.components.len() - 1].join("/");
    let name = MutationName::Provider(op.destination.components.last().unwrap().clone());
    let key = format!("{p}/{name:?}");
    let existing = Temp {
        data: b"external".to_vec(),
        observation: obs("external", &p, name, 8),
    };
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.published.insert(key.clone(), existing);
    let (r, _) = execute(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Partial);
    assert_eq!(r.committed_destinations, 0);
    assert_eq!(f.published.len(), 1);
    assert_eq!(f.published[&key].data, b"external");
    assert_eq!(r.moved_sources, 0);
    assert!(f.source.is_some());
}
#[test]
fn cancelling_before_first_operation_allocates_nothing() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(true)));
    let (r, _) = execute(&binding(true), &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Cancelled);
    assert_eq!(r.committed_destinations, 0);
    assert_eq!(f.temps, 0);
    assert!(f.source.is_some());
    assert!(f.trace.is_empty());
}
#[test]
fn atomic_failure_after_the_effect_never_attempts_a_copy_or_deletion() {
    for point in ["inspect", "verify"] {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        f.atomic = true;
        f.fault = Some((point, 1));
        let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
        assert_eq!(r.status, JobStatus::NeedsRecovery);
        assert_eq!(r.moved_sources, 0);
        assert_eq!(f.renames, 1);
        assert_eq!(f.temps, 0);
        assert_eq!(f.retirements, 0);
        assert!(f.source.is_none());
        assert_eq!(f.published.len(), 1);
    }
}
#[test]
fn interrupted_writes_observe_cancellation_before_retry() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.interrupt_write = true;
    f.cancel_at = Some("write");
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Cancelled);
    assert_eq!(f.seen["write"], 1);
    assert_eq!(r.committed_destinations, 0);
    assert!(f.source.is_some());
}
impl sorter_executor::recovery::RecoveryStorage for Fake {
    fn source(
        &mut self,
        _: &StorageRef,
    ) -> sorter_executor::Result<Option<sorter_executor::recovery::ObservedObject>> {
        Ok(self
            .source
            .as_ref()
            .map(|b| sorter_executor::recovery::ObservedObject {
                identity: id("song"),
                sha256: sorter_executor::digest(b),
            }))
    }
    fn destination(
        &mut self,
        d: &DestinationEvidence,
    ) -> sorter_executor::Result<Option<sorter_executor::recovery::ObservedObject>> {
        let parent = d.components[..d.components.len() - 1].join("/");
        let name = MutationName::Provider(d.components.last().unwrap().clone());
        Ok(self.published.get(&format!("{parent}/{name:?}")).map(|t| {
            sorter_executor::recovery::ObservedObject {
                identity: t.observation.identity.clone(),
                sha256: sorter_executor::digest(&t.data),
            }
        }))
    }
}
#[test]
fn reopening_at_every_durable_boundary_never_authorizes_automatic_retry() {
    use sorter_executor::recovery::{RecoveryStatus, recover_job};
    for atomic in [false, true] {
        let binding = binding(false);
        let flag = Arc::new(AtomicBool::new(false));
        let mut p = Fake::new(b"audio", flag.clone());
        p.atomic = atomic;
        let mut baseline = Sink::default();
        execute(&binding, &mut p, &mut baseline);
        for boundary in 4..baseline.boundary {
            let mut sink = Sink::default();
            let mut j = PhysicalJournalV2::create(&binding, "test-job", &mut sink).unwrap();
            sink.fail = Some(boundary);
            let mut f = Fake::new(b"audio", flag.clone());
            f.atomic = atomic;
            run_job(&binding, &mut f, &mut j, &mut sink, &flag, &mut |_| {}).unwrap();
            let source_before = f.source.clone();
            let published_before = f.published.len();
            let renamed = f.renames;
            let retired = f.retirements;
            let reopened = PhysicalJournalV2::open(&binding, "test-job", &sink.bytes).unwrap();
            let report = recover_job(&binding, &reopened, &mut f).unwrap();
            if f.source.is_none()
                && !reopened
                    .events()
                    .iter()
                    .any(|e| matches!(e.phase, Phase::Committed | Phase::SourceRetired))
            {
                assert_ne!(report.operations[0].status, RecoveryStatus::Committed);
            }
            assert_eq!(f.source, source_before);
            assert_eq!(f.published.len(), published_before);
            assert_eq!(f.renames, renamed);
            assert_eq!(f.retirements, retired);
        }
    }
}
#[test]
fn measured_synthetic_dispatch_cost_does_not_claim_windows_performance() {
    let data = vec![7; TRANSFER_BUFFER_BYTES * 3 + 9];
    let b = binding_with(false, false, &data);
    for atomic in [false, true] {
        let mut f = Fake::new(&data, Arc::new(AtomicBool::new(false)));
        f.atomic = atomic;
        f.short = TRANSFER_BUFFER_BYTES;
        let start = std::time::Instant::now();
        let (r, _) = execute(&b, &mut f, &mut Sink::default());
        let elapsed = start.elapsed();
        assert_eq!(r.status, JobStatus::Complete);
        println!(
            "synthetic only: atomic={atomic}, elapsed_us={}, reads={}, writes={}, published_verifications={}",
            elapsed.as_micros(),
            f.seen.get("read").unwrap_or(&0),
            f.seen.get("write").unwrap_or(&0),
            f.seen.get("verify").unwrap_or(&0)
        );
        if atomic {
            assert!(!f.seen.contains_key("write"));
            assert_eq!(f.temps, 0)
        } else {
            assert_eq!(f.seen["write"], 4);
            assert_eq!(f.seen["read"], 5)
        }
    }
}
#[test]
fn changed_stream_length_or_digest_is_rejected_before_publication() {
    for bytes in [
        b"aud".as_slice(),
        b"audio-extra".as_slice(),
        b"other".as_slice(),
    ] {
        let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
        f.altered_stream = Some(bytes.to_vec());
        let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
        assert_eq!(r.committed_destinations, 0);
        assert_eq!(r.moved_sources, 0);
        assert!(f.source.is_some());
        assert!(f.published.is_empty());
    }
}
#[test]
fn disk_full_has_a_structured_error_and_keeps_the_source() {
    let mut f = Fake::new(b"audio", Arc::new(AtomicBool::new(false)));
    f.disk_full = true;
    let (r, _) = execute(&binding(false), &mut f, &mut Sink::default());
    assert_eq!(
        r.items[0].error.as_ref().unwrap().kind,
        MutationErrorKind::DiskFull
    );
    assert_eq!(r.committed_destinations, 0);
    assert_eq!(r.moved_sources, 0);
    assert!(f.source.is_some());
}
