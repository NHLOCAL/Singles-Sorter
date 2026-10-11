use sorter_core::{ClassifierFact, Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, Result, binding::*, digest, physical_journal::*, recovery::*,
};
use std::{collections::HashMap, io::Cursor};
fn reference(id: &str) -> StorageRef {
    StorageRef::TestOpaque { id: id.into() }
}
fn identity(id: &str) -> ObjectIdentity {
    ObjectIdentity::Test { id: id.into() }
}
fn entry(id: &str, name: &str, kind: EntryKind, parent: Option<&str>) -> EntryObservation {
    EntryObservation {
        reference: reference(id),
        identity: Some(identity(id)),
        parent_identity: parent.map(identity),
        name: name.into(),
        kind,
        size: Some(5),
        modified: Some("m1".into()),
        capabilities: Capabilities {
            read: Some(true),
            list: Some(true),
            seek: Some(true),
        },
    }
}
#[derive(Clone)]
struct PlanningStorage {
    entries: Vec<EntryObservation>,
    bytes: Vec<u8>,
}
impl ReadStorage for PlanningStorage {
    fn stat(&mut self, r: &StorageRef) -> std::result::Result<EntryObservation, StorageError> {
        self.entries
            .iter()
            .find(|e| &e.reference == r)
            .cloned()
            .ok_or(StorageError::new(StorageErrorKind::NotFound, "absent"))
    }
    fn children(
        &mut self,
        r: &StorageRef,
    ) -> std::result::Result<Vec<EntryObservation>, StorageError> {
        let id = self.stat(r)?.identity;
        Ok(self
            .entries
            .iter()
            .filter(|e| e.parent_identity == id)
            .cloned()
            .collect())
    }
    fn open_read(&mut self, r: &StorageRef) -> std::result::Result<ReadInput, StorageError> {
        let e = self.stat(r)?;
        Ok(
            ReadInput::seekable(Cursor::new(self.bytes.clone()), self.bytes.len() as u64)
                .with_binding(ReadBinding {
                    identity: e.identity.unwrap(),
                    size: self.bytes.len() as u64,
                    modified: e.modified.unwrap(),
                }),
        )
    }
}
struct Metadata;
impl MetadataProvider for Metadata {
    fn open(
        &mut self,
        _: &EntryObservation,
    ) -> std::result::Result<Box<dyn MetadataFields>, MetadataError> {
        Err(MetadataError::LegacyOpen {
            kind: "synthetic".into(),
            detail: "untagged".into(),
        })
    }
}
struct Ai {
    duet: bool,
    fail: bool,
}
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        if self.duet {
            AiProfile::Configured {
                bundle_id: "fake-model".into(),
                available: true,
            }
        } else {
            AiProfile::Disabled
        }
    }
    fn classify(&mut self, _: &str) -> std::result::Result<ClassifierFact, String> {
        if self.fail {
            Err("failed".into())
        } else {
            Ok(ClassifierFact::Accept)
        }
    }
    fn ner(&mut self, _: &str) -> std::result::Result<Vec<String>, String> {
        if self.fail {
            Err("failed".into())
        } else {
            Ok(vec!["אמן".into(), "אמן".into()])
        }
    }
}
fn fixture(duet: bool) -> (JobPlanV2, PlanningStorage, Resources, ExecutionScope) {
    let mut storage = PlanningStorage {
        entries: vec![
            entry("source", "source", EntryKind::Directory, None),
            entry("target", "target", EntryKind::Directory, None),
            entry("song", "song.mp3", EntryKind::File, Some("source")),
        ],
        bytes: b"audio".to_vec(),
    };
    let mut ai = Ai { duet, fail: false };
    let request = JobRequestV2 {
        mode: JobMode::Sort,
        source: reference("source"),
        target: Some(reference("target")),
        settings: Settings {
            duet_mode: duet,
            ..Settings::default()
        },
        alias_rows: if duet {
            vec![]
        } else {
            vec![vec!["song".into(), "אמן".into()]]
        },
        resource_ids: vec!["aliases".into()],
        adapter_versions: vec!["fake-v1".into()],
        ai_profile: ai.profile(),
    };
    let plan = plan_job(&request, &mut storage, &mut Metadata, &mut ai).unwrap();
    let resources = Resources {
        resource_ids: request.resource_ids,
        adapter_versions: request.adapter_versions,
        ai_profile: request.ai_profile,
        resource_hashes: vec![digest(b"aliases")],
        adapter_hashes: vec![digest(b"fake-v1")],
        model_hash: duet.then(|| digest(b"model")),
    };
    let scope = ExecutionScope {
        inspected_plan: plan.binding_sha256.clone(),
        source_root: identity("source"),
        target_root: identity("target"),
        inspected_sources: vec![SourceEvidence {
            observation: storage.entries[2].clone(),
            sha256: digest(b"audio"),
        }],
        resources: resources.clone(),
        allowed_modes: vec![TransferMode::Move],
    };
    (plan, storage, resources, scope)
}
fn binding(duet: bool) -> ExecutionBinding {
    let (p, mut s, r, scope) = fixture(duet);
    validate_execution(
        &p,
        &mut s,
        &mut Metadata,
        &mut Ai { duet, fail: false },
        &r,
        &scope,
    )
    .unwrap()
}
#[derive(Clone, Default)]
struct Sink {
    bytes: Vec<u8>,
    durable: Vec<u8>,
    boundary: usize,
    fail: Option<usize>,
    partial: bool,
}
impl Sink {
    fn hit(&mut self) -> Result<()> {
        let at = self.boundary;
        self.boundary += 1;
        if self.fail == Some(at) {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
}
impl JournalStore for Sink {
    fn read_all(&mut self) -> Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        self.hit()?;
        if self.partial {
            self.bytes.extend(&bytes[..bytes.len() / 2]);
            self.durable = self.bytes.clone();
            return Err(E::Io);
        }
        self.bytes.extend(bytes);
        self.hit()
    }
    fn sync(&mut self) -> Result<()> {
        self.hit()?;
        self.durable = self.bytes.clone();
        self.hit()
    }
}
#[derive(Clone, Default)]
struct Fake {
    source: Option<ObservedObject>,
    destinations: HashMap<String, ObservedObject>,
    temp: Option<ObservedObject>,
    effects: Vec<&'static str>,
    boundary: usize,
    fail: Option<usize>,
}
impl Fake {
    fn new() -> Self {
        Self {
            source: Some(ObservedObject {
                identity: identity("song"),
                sha256: digest(b"audio"),
            }),
            ..Self::default()
        }
    }
    fn hit(&mut self) -> Result<()> {
        let at = self.boundary;
        self.boundary += 1;
        if self.fail == Some(at) {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
    fn allocate(&mut self, op: usize) -> Result<ObjectIdentity> {
        self.hit()?;
        let id = identity(&format!("temp-{op}"));
        self.temp = Some(ObservedObject {
            identity: id.clone(),
            sha256: String::new(),
        });
        self.effects.push("allocate");
        self.hit()?;
        Ok(id)
    }
    fn write_sync(&mut self) -> Result<()> {
        self.hit()?;
        self.temp.as_mut().unwrap().sha256 = digest(b"audio");
        self.effects.push("write_sync");
        self.hit()
    }
    fn publish(&mut self, id: &str) -> Result<()> {
        self.hit()?;
        if self.destinations.contains_key(id) {
            return Err(E::Stale);
        }
        self.destinations
            .insert(id.into(), self.temp.take().unwrap());
        self.effects.push("publish");
        self.hit()
    }
    fn rename(&mut self, id: &str) -> Result<()> {
        self.hit()?;
        if self.destinations.contains_key(id) {
            return Err(E::Stale);
        }
        self.destinations
            .insert(id.into(), self.source.take().ok_or(E::Stale)?);
        self.effects.push("rename");
        self.hit()
    }
    fn verify(&mut self, id: &str) -> Result<()> {
        self.hit()?;
        if self
            .destinations
            .get(id)
            .is_none_or(|d| d.sha256 != digest(b"audio"))
        {
            return Err(E::Stale);
        }
        self.hit()
    }
    fn retire(&mut self) -> Result<()> {
        self.hit()?;
        if self.source.as_ref().map(|s| &s.identity) != Some(&identity("song")) {
            return Err(E::Stale);
        }
        self.source = None;
        self.effects.push("retire");
        self.hit()
    }
}
impl RecoveryStorage for Fake {
    fn source(&mut self, r: &StorageRef) -> Result<Option<ObservedObject>> {
        assert_eq!(*r, reference("song"));
        Ok(self.source.clone())
    }
    fn destination(&mut self, d: &DestinationEvidence) -> Result<Option<ObservedObject>> {
        Ok(self.destinations.get(&d.id).cloned())
    }
}
fn event(b: &ExecutionBinding, op: usize, phase: Phase, id: Option<ObjectIdentity>) -> Event {
    let o = &b.operations()[op];
    Event {
        operation: op,
        destination_id: o.destination.id.clone(),
        expected_source: o.source.observation.identity.clone().unwrap(),
        observed_identity: id,
        sha256: o.source.sha256.clone(),
        phase,
    }
}
fn log(
    j: &mut PhysicalJournalV2,
    s: &mut Sink,
    b: &ExecutionBinding,
    op: usize,
    p: Phase,
    id: Option<ObjectIdentity>,
) -> Result<()> {
    j.append(s, event(b, op, p, id))
}
fn run(b: &ExecutionBinding, s: &mut Sink, f: &mut Fake, atomic: bool) -> Result<()> {
    let mut j = PhysicalJournalV2::create(b, "job", s)?;
    for op in b.operations() {
        log(&mut j, s, b, op.id, Phase::Prepared, None)?;
        let id = if atomic {
            log(
                &mut j,
                s,
                b,
                op.id,
                Phase::AtomicRenamePrepared,
                Some(identity("song")),
            )?;
            f.rename(&op.destination.id)?;
            log(
                &mut j,
                s,
                b,
                op.id,
                Phase::AtomicRenameObserved,
                Some(identity("song")),
            )?;
            identity("song")
        } else {
            log(&mut j, s, b, op.id, Phase::TempAllocationPrepared, None)?;
            let id = f.allocate(op.id)?;
            log(&mut j, s, b, op.id, Phase::TempAllocated, Some(id.clone()))?;
            log(
                &mut j,
                s,
                b,
                op.id,
                Phase::BytesWritePrepared,
                Some(id.clone()),
            )?;
            f.write_sync()?;
            log(&mut j, s, b, op.id, Phase::BytesSynced, Some(id.clone()))?;
            log(
                &mut j,
                s,
                b,
                op.id,
                Phase::DestinationPublishPrepared,
                Some(id.clone()),
            )?;
            f.publish(&op.destination.id)?;
            log(
                &mut j,
                s,
                b,
                op.id,
                Phase::DestinationPublished,
                Some(id.clone()),
            )?;
            id
        };
        f.verify(&op.destination.id)?;
        log(
            &mut j,
            s,
            b,
            op.id,
            Phase::DestinationVerified,
            Some(id.clone()),
        )?;
        log(&mut j, s, b, op.id, Phase::Committed, Some(id))?;
    }
    if !atomic && b.operations()[0].mode == TransferMode::Move {
        let op = b.operations().last().unwrap().id;
        log(
            &mut j,
            s,
            b,
            op,
            Phase::SourceRetirementPrepared,
            Some(identity("song")),
        )?;
        f.retire()?;
        log(
            &mut j,
            s,
            b,
            op,
            Phase::SourceRetired,
            Some(identity("song")),
        )?;
    }
    Ok(())
}
#[test]
fn binding_requires_exact_scope_fresh_bytes_roots_resources_and_plan() {
    let (p, mut s, r, scope) = fixture(false);
    assert!(!p.executable);
    assert!(
        validate_execution(
            &p,
            &mut s,
            &mut Metadata,
            &mut Ai {
                duet: false,
                fail: false
            },
            &r,
            &scope
        )
        .is_ok()
    );
    for change in 0..7 {
        let mut p = p.clone();
        let mut s = s.clone();
        let mut r = r.clone();
        let mut scope = scope.clone();
        match change {
            0 => p.operations[0].depends_on.push(0),
            1 => scope.inspected_plan = digest(b"wrong"),
            2 => scope.target_root = identity("other"),
            3 => s.entries[2].identity = Some(identity("replacement")),
            4 => s.bytes = b"wrong".to_vec(),
            5 => r.adapter_hashes[0] = digest(b"other"),
            _ => scope.allowed_modes.clear(),
        };
        assert!(
            validate_execution(
                &p,
                &mut s,
                &mut Metadata,
                &mut Ai {
                    duet: false,
                    fail: false
                },
                &r,
                &scope
            )
            .is_err(),
            "change {change}"
        );
    }
}
#[test]
fn every_journal_and_storage_before_after_boundary_is_recoverable_without_effect_replay() {
    for (duet, atomic, copy) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, false, true),
    ] {
        let b = if copy {
            copy_binding(duet)
        } else {
            binding(duet)
        };
        let mut complete = Sink::default();
        let mut effects = Fake::new();
        run(&b, &mut complete, &mut effects, atomic).unwrap();
        if copy {
            assert!(effects.source.is_some());
            assert!(!effects.effects.contains(&"retire"));
        }
        for is_journal in [true, false] {
            for boundary in 0..if is_journal {
                complete.boundary
            } else {
                effects.boundary
            } {
                let mut s = Sink::default();
                let mut f = Fake::new();
                if is_journal {
                    s.fail = Some(boundary)
                } else {
                    f.fail = Some(boundary)
                };
                assert!(run(&b, &mut s, &mut f, atomic).is_err());
                let before = f.effects.clone();
                // Both possible persistence outcomes after a failed append/flush.
                for bytes in [&s.bytes, &s.durable] {
                    if let Ok(j) = PhysicalJournalV2::open(&b, "job", bytes) {
                        let first = recover_job(&b, &j, &mut f).unwrap();
                        let second = recover_job(&b, &j, &mut f).unwrap();
                        assert_eq!(first, second);
                        for op in &first.operations {
                            if op.status == RecoveryStatus::Committed {
                                assert!(
                                    f.destinations
                                        .contains_key(&b.operations()[op.operation].destination.id)
                                );
                            }
                        }
                    }
                }
                assert_eq!(before, f.effects);
                assert!(
                    f.effects.iter().filter(|e| **e == "publish").count() <= b.operations().len()
                );
                if f.source.is_none() {
                    assert_eq!(f.destinations.len(), b.operations().len());
                    assert!(
                        f.destinations
                            .values()
                            .all(|d| d.sha256 == digest(b"audio"))
                    );
                }
            }
        }
    }
}
#[test]
fn wrong_event_identity_destination_digest_duplicate_and_out_of_order_are_rejected() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    for change in 0..5 {
        let mut e = event(&b, 0, Phase::Prepared, None);
        match change {
            0 => e.expected_source = identity("other"),
            1 => e.destination_id = "other".into(),
            2 => e.sha256 = digest(b"wrong"),
            3 => e.operation = 9,
            _ => e.phase = Phase::Committed,
        };
        assert!(j.append(&mut s, e).is_err());
    }
    log(&mut j, &mut s, &b, 0, Phase::Prepared, None).unwrap();
    assert!(log(&mut j, &mut s, &b, 0, Phase::Prepared, None).is_err());
}
#[test]
fn header_middle_sequence_hash_job_and_incomplete_tail_fail_closed() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut f = Fake::new();
    run(&b, &mut s, &mut f, false).unwrap();
    assert!(PhysicalJournalV2::open(&b, "other", &s.bytes).is_err());
    for index in [10, s.bytes.iter().position(|x| *x == b'\n').unwrap() + 15] {
        let mut bytes = s.bytes.clone();
        bytes[index] ^= 1;
        assert!(PhysicalJournalV2::open(&b, "job", &bytes).is_err());
    }
    let lines: Vec<_> = s.bytes.split_inclusive(|b| *b == b'\n').collect();
    let mut duplicate = lines[0].to_vec();
    duplicate.extend(lines[1]);
    duplicate.extend(lines[1]);
    assert!(PhysicalJournalV2::open(&b, "job", &duplicate).is_err());
    for cut in 1..lines.last().unwrap().len() {
        let bytes = &s.bytes[..s.bytes.len() - cut];
        let mut j = PhysicalJournalV2::open(&b, "job", bytes).unwrap();
        assert!(!j.incomplete_tail().is_empty());
        let saved = j.incomplete_tail().to_vec();
        let report = recover_job(&b, &j, &mut f).unwrap();
        assert!(
            report
                .operations
                .iter()
                .all(|o| o.status == RecoveryStatus::NeedsRecovery)
        );
        let before = s.bytes.clone();
        assert_eq!(
            log(&mut j, &mut s, &b, 0, Phase::Failed, None),
            Err(E::IncompleteTail)
        );
        assert_eq!(s.bytes, before);
        assert_eq!(j.incomplete_tail(), saved);
    }
}
#[test]
fn distinct_duet_destinations_are_required_before_source_retirement() {
    let b = binding(true);
    assert_eq!(b.operations().len(), 2);
    assert_ne!(
        b.operations()[0].destination.id,
        b.operations()[1].destination.id
    );
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
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
        let id = (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
            .then(|| identity("temp"));
        log(&mut j, &mut s, &b, 0, phase, id).unwrap();
    }
    assert!(
        log(
            &mut j,
            &mut s,
            &b,
            0,
            Phase::SourceRetirementPrepared,
            Some(identity("song"))
        )
        .is_err()
    );
    assert!(
        log(
            &mut j,
            &mut s,
            &b,
            0,
            Phase::Committed,
            Some(identity("temp"))
        )
        .is_err()
    );
    log(&mut j, &mut s, &b, 1, Phase::Prepared, None).unwrap();
    assert!(
        log(
            &mut j,
            &mut s,
            &b,
            1,
            Phase::AtomicRenamePrepared,
            Some(identity("song"))
        )
        .is_err()
    );
}
#[test]
fn publication_adoption_requires_exact_owned_identity_and_digest() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut f = Fake::new();
    run(&b, &mut s, &mut f, false).unwrap();
    let lines: Vec<_> = s.bytes.split_inclusive(|x| *x == b'\n').collect();
    // Through DestinationPublishPrepared: publish occurred but no result record.
    let bytes = lines[..7].concat();
    let j = PhysicalJournalV2::open(&b, "job", &bytes).unwrap();
    f.source = Fake::new().source;
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::EffectProvenAwaitingJournal
    );
    for identity_wrong in [true, false] {
        let mut changed = f.clone();
        let d = changed.destinations.values_mut().next().unwrap();
        if identity_wrong {
            d.identity = identity("unrelated")
        } else {
            d.sha256 = digest(b"other")
        };
        assert_eq!(
            recover_job(&b, &j, &mut changed).unwrap().operations[0].status,
            RecoveryStatus::NeedsRecovery
        );
    }
    f.destinations.clear();
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::NeedsRecovery
    );
}
#[test]
fn atomic_rename_missing_post_event_is_proven_only_by_bound_source_identity() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut f = Fake::new();
    run(&b, &mut s, &mut f, true).unwrap();
    let lines: Vec<_> = s.bytes.split_inclusive(|x| *x == b'\n').collect();
    let j = PhysicalJournalV2::open(&b, "job", &lines[..3].concat()).unwrap();
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::EffectProvenAwaitingJournal
    );
    f.source = Fake::new().source;
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::NeedsRecovery
    );
}
#[test]
fn failed_partial_append_poison_preserves_original_bytes() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    s.partial = true;
    assert_eq!(
        log(&mut j, &mut s, &b, 0, Phase::Prepared, None),
        Err(E::Io)
    );
    let bytes = s.bytes.clone();
    assert_eq!(
        log(&mut j, &mut s, &b, 0, Phase::Prepared, None),
        Err(E::NeedsRecovery)
    );
    assert_eq!(bytes, s.bytes);
    let j = PhysicalJournalV2::open(&b, "job", &bytes).unwrap();
    assert!(j.needs_recovery());
}
#[test]
fn cancellation_after_destination_commit_blocks_retirement_and_dependent_work() {
    let b = binding(true);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    for op in 0..2 {
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
            let id = (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
                .then(|| identity(&format!("temp-{op}")));
            log(&mut j, &mut s, &b, op, phase, id).unwrap();
        }
    }
    log(&mut j, &mut s, &b, 0, Phase::Cancelled, None).unwrap();
    assert!(
        log(
            &mut j,
            &mut s,
            &b,
            1,
            Phase::SourceRetirementPrepared,
            Some(identity("song"))
        )
        .is_err()
    );
}
#[test]
fn cancelled_predecessor_cannot_start_dependent_operation() {
    let b = binding(true);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
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
        let id = (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
            .then(|| identity("temp-0"));
        log(&mut j, &mut s, &b, 0, phase, id).unwrap();
    }
    log(&mut j, &mut s, &b, 0, Phase::Cancelled, None).unwrap();
    assert!(log(&mut j, &mut s, &b, 1, Phase::Prepared, None).is_err());
}
#[test]
fn fresh_host_recovery_scope_works_after_source_is_gone_but_cannot_append() {
    let b = binding(false);
    let (_, _, _, scope) = fixture(false);
    let mut s = Sink::default();
    let mut f = Fake::new();
    run(&b, &mut s, &mut f, true).unwrap();
    assert!(f.source.is_none());
    let grant = RecoveryScope {
        original_scope: scope,
        inspected_binding_sha256: b.sha256().unwrap(),
    };
    let mut j = PhysicalJournalV2::open_for_recovery(&grant, "job", &s.bytes).unwrap();
    assert_eq!(
        recover_job(j.binding(), &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::Committed
    );
    assert_eq!(
        log(&mut j, &mut s, &b, 0, Phase::Cancelled, None),
        Err(E::Scope)
    );
    let mut wrong = grant.clone();
    wrong.inspected_binding_sha256 = digest(b"wrong");
    assert!(PhysicalJournalV2::open_for_recovery(&wrong, "job", &s.bytes).is_err());
    let mut wrong = grant;
    wrong.original_scope.allowed_modes.clear();
    assert!(PhysicalJournalV2::open_for_recovery(&wrong, "job", &s.bytes).is_err());
}
#[test]
fn identity_collision_with_root_cannot_be_claimed_as_owned_temp() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    log(&mut j, &mut s, &b, 0, Phase::Prepared, None).unwrap();
    assert!(
        log(
            &mut j,
            &mut s,
            &b,
            0,
            Phase::TempAllocated,
            Some(identity("target"))
        )
        .is_err()
    );
}
#[test]
fn missing_effect_and_unrelated_occupants_are_never_adopted() {
    let b = binding(false);
    let mut s = Sink::default();
    let j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    let mut f = Fake::new();
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::ReadyForExplicitResume
    );
    f.destinations.insert(
        b.operations()[0].destination.id.clone(),
        ObservedObject {
            identity: identity("unrelated"),
            sha256: digest(b"audio"),
        },
    );
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::NeedsRecovery
    );
    f.source = None;
    f.destinations.clear();
    assert_eq!(
        recover_job(&b, &j, &mut f).unwrap().operations[0].status,
        RecoveryStatus::NeedsRecovery
    );
}
#[test]
fn rehashed_wrong_sequence_previous_plan_or_operation_are_rejected() {
    let b = binding(false);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    log(&mut j, &mut s, &b, 0, Phase::Prepared, None).unwrap();
    let lines: Vec<_> = s.bytes.split_inclusive(|x| *x == b'\n').collect();
    let original = std::str::from_utf8(lines[1]).unwrap();
    let body = &original[8..original.rfind(",\"sha256\":").unwrap()];
    for (from, to) in [
        ("\"sequence\":1".to_string(), "\"sequence\":99".to_string()),
        (
            format!(
                "\"previous\":\"{}\"",
                serde_json::from_slice::<serde_json::Value>(lines[1]).unwrap()["body"]["previous"]
                    .as_str()
                    .unwrap()
            ),
            "\"previous\":\"wrong\"".into(),
        ),
        (
            format!("\"plan_sha256\":\"{}\"", b.plan_sha256()),
            "\"plan_sha256\":\"wrong\"".into(),
        ),
        ("\"operation\":0".into(), "\"operation\":99".into()),
    ] {
        let changed = body.replace(&from, &to);
        assert_ne!(changed, body);
        let mut bytes = lines[0].to_vec();
        bytes.extend(
            format!(
                "{{\"body\":{},\"sha256\":\"{}\"}}\n",
                changed,
                digest(changed.as_bytes())
            )
            .as_bytes(),
        );
        assert!(PhysicalJournalV2::open(&b, "job", &bytes).is_err());
    }
}

#[test]
fn configured_model_failure_and_changed_model_digest_block_binding() {
    let (p, mut s, r, scope) = fixture(true);
    assert!(
        validate_execution(
            &p,
            &mut s,
            &mut Metadata,
            &mut Ai {
                duet: true,
                fail: true
            },
            &r,
            &scope
        )
        .is_err()
    );
    let mut changed = r;
    changed.model_hash = Some(digest(b"wrong"));
    assert!(
        validate_execution(
            &p,
            &mut s,
            &mut Metadata,
            &mut Ai {
                duet: true,
                fail: false
            },
            &changed,
            &scope
        )
        .is_err()
    );
}
#[test]
fn source_absence_after_retirement_intent_is_ambiguous_without_post_event() {
    let b = binding(true);
    let mut s = Sink::default();
    let mut f = Fake::new();
    run(&b, &mut s, &mut f, false).unwrap();
    let lines: Vec<_> = s.bytes.split_inclusive(|x| *x == b'\n').collect();
    let j = PhysicalJournalV2::open(&b, "job", &lines[..lines.len() - 1].concat()).unwrap();
    assert!(
        recover_job(&b, &j, &mut f)
            .unwrap()
            .operations
            .iter()
            .all(|o| o.status == RecoveryStatus::NeedsRecovery)
    );
    let j = PhysicalJournalV2::open(&b, "job", &s.bytes).unwrap();
    assert!(
        recover_job(&b, &j, &mut f)
            .unwrap()
            .operations
            .iter()
            .all(|o| o.status == RecoveryStatus::Committed)
    );
    f.destinations.remove(&b.operations()[1].destination.id);
    assert!(
        recover_job(&b, &j, &mut f)
            .unwrap()
            .operations
            .iter()
            .all(|o| o.status == RecoveryStatus::NeedsRecovery)
    );
}
#[test]
fn fail_open_classifier_policy_cannot_authorize_execution() {
    struct ArtistMetadata;
    struct ArtistFields;
    impl MetadataProvider for ArtistMetadata {
        fn open(
            &mut self,
            _: &EntryObservation,
        ) -> std::result::Result<Box<dyn MetadataFields>, MetadataError> {
            Ok(Box::new(ArtistFields))
        }
    }
    impl MetadataFields for ArtistFields {
        fn get(&mut self, f: LegacyField) -> std::result::Result<LegacyItem, FieldError> {
            let text = if f == LegacyField::Artist {
                "אמן מוכר"
            } else {
                ""
            };
            Ok(LegacyItem {
                values: vec![LegacyScalar::Text(text.into())],
                truthy: !text.is_empty(),
                value: Ok(LegacyScalar::Text(text.into())),
            })
        }
    }
    let (p, mut s, r, mut scope) = fixture(true);
    let mut ai = Ai {
        duet: true,
        fail: true,
    };
    let p = plan_job(&p.request, &mut s, &mut ArtistMetadata, &mut ai).unwrap();
    assert!(
        !p.operations.is_empty(),
        "released classifier fail-open remains visible in planning"
    );
    scope.inspected_plan = p.binding_sha256.clone();
    assert_eq!(
        validate_execution(&p, &mut s, &mut ArtistMetadata, &mut ai, &r, &scope).unwrap_err(),
        E::Unsupported
    );
}

#[test]
fn review_cancelled_dependency_still_allows_new_publish_intent() {
    let b = binding(true);
    assert_eq!(b.operations()[1].depends_on, vec![0]);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
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
        log(
            &mut j,
            &mut s,
            &b,
            0,
            phase,
            (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
                .then(|| identity("temp-0")),
        )
        .unwrap();
    }
    log(&mut j, &mut s, &b, 1, Phase::Prepared, None).unwrap();
    log(&mut j, &mut s, &b, 0, Phase::Cancelled, None).unwrap();
    // No in-flight effect exists for dependent op 1 yet. It should be blocked.
    let after_cancel = log(&mut j, &mut s, &b, 1, Phase::TempAllocationPrepared, None);
    assert_eq!(
        after_cancel,
        Err(E::InvalidTransition),
        "a prepared dependent must not start new effects after its predecessor is cancelled"
    );
}

#[test]
fn review_invalid_dependency_allows_new_atomic_rename_intent() {
    let (p, mut storage, resources, mut scope) = fixture(false);
    storage.entries.push(entry(
        "song2",
        "song other.mp3",
        EntryKind::File,
        Some("source"),
    ));
    let p = plan_job(
        &p.request,
        &mut storage,
        &mut Metadata,
        &mut Ai {
            duet: false,
            fail: false,
        },
    )
    .unwrap();
    scope.inspected_plan = p.binding_sha256.clone();
    scope.inspected_sources.push(SourceEvidence {
        observation: storage.entries[3].clone(),
        sha256: digest(b"audio"),
    });
    let b = validate_execution(
        &p,
        &mut storage,
        &mut Metadata,
        &mut Ai {
            duet: false,
            fail: false,
        },
        &resources,
        &scope,
    )
    .unwrap();
    assert_eq!(b.operations().len(), 2);
    assert_eq!(b.operations()[1].depends_on, vec![0]);
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    for phase in [
        Phase::Prepared,
        Phase::AtomicRenamePrepared,
        Phase::AtomicRenameObserved,
        Phase::DestinationVerified,
        Phase::Committed,
    ] {
        log(
            &mut j,
            &mut s,
            &b,
            0,
            phase,
            (phase != Phase::Prepared).then(|| identity("song")),
        )
        .unwrap();
    }
    log(&mut j, &mut s, &b, 1, Phase::Prepared, None).unwrap();
    log(&mut j, &mut s, &b, 0, Phase::NeedsRecovery, None).unwrap();
    assert_eq!(
        log(
            &mut j,
            &mut s,
            &b,
            1,
            Phase::AtomicRenamePrepared,
            Some(identity("song2"))
        ),
        Err(E::InvalidTransition),
        "a new irreversible intent after predecessor invalidation must fail closed"
    );
}

fn review_two_moves() -> ExecutionBinding {
    review_two_mode(false)
}
fn review_two_mode(copy: bool) -> ExecutionBinding {
    let (mut p, mut storage, resources, mut scope) = fixture(false);
    p.request.settings.copy_mode = copy;
    scope.allowed_modes = vec![if copy {
        TransferMode::Copy
    } else {
        TransferMode::Move
    }];
    storage.entries.push(entry(
        "song2",
        "song other.mp3",
        EntryKind::File,
        Some("source"),
    ));
    let p = plan_job(
        &p.request,
        &mut storage,
        &mut Metadata,
        &mut Ai {
            duet: false,
            fail: false,
        },
    )
    .unwrap();
    assert_eq!(
        p.predicted_effects
            .iter()
            .any(|e| matches!(e, PredictedEffect::SourceUnavailable { operation: 0, .. })),
        !copy
    );
    scope.inspected_plan = p.binding_sha256.clone();
    scope.inspected_sources.push(SourceEvidence {
        observation: storage.entries[3].clone(),
        sha256: digest(b"audio"),
    });
    validate_execution(
        &p,
        &mut storage,
        &mut Metadata,
        &mut Ai {
            duet: false,
            fail: false,
        },
        &resources,
        &scope,
    )
    .unwrap()
}
fn review_committed_destination(b: &ExecutionBinding) -> (PhysicalJournalV2, Sink) {
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(b, "job", &mut s).unwrap();
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
        log(
            &mut j,
            &mut s,
            b,
            0,
            phase,
            (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
                .then(|| identity("temp-0")),
        )
        .unwrap();
    }
    (j, s)
}
#[test]
fn review_dependent_move_starts_before_prerequisite_source_retirement() {
    let b = review_two_moves();
    let (mut j, mut s) = review_committed_destination(&b);
    assert_eq!(
        log(&mut j, &mut s, &b, 1, Phase::Prepared, None),
        Err(E::InvalidTransition),
        "copy completion is not prerequisite move completion"
    );
}
#[test]
fn review_recovery_allows_dependent_resume_before_prerequisite_source_retirement() {
    struct BothSourcesPresent {
        first_dest: String,
    }
    impl RecoveryStorage for BothSourcesPresent {
        fn source(&mut self, r: &StorageRef) -> Result<Option<ObservedObject>> {
            let StorageRef::TestOpaque { id } = r else {
                panic!()
            };
            Ok(Some(ObservedObject {
                identity: identity(id),
                sha256: digest(b"audio"),
            }))
        }
        fn destination(&mut self, d: &DestinationEvidence) -> Result<Option<ObservedObject>> {
            Ok((d.id == self.first_dest).then(|| ObservedObject {
                identity: identity("temp-0"),
                sha256: digest(b"audio"),
            }))
        }
    }
    let b = review_two_moves();
    let (j, _) = review_committed_destination(&b);
    let report = recover_job(
        &b,
        &j,
        &mut BothSourcesPresent {
            first_dest: b.operations()[0].destination.id.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        report.operations[1].status,
        RecoveryStatus::NeedsRecovery,
        "source removal predicted by predecessor has not occurred"
    );
}
fn copy_binding(duet: bool) -> ExecutionBinding {
    let (mut p, mut storage, resources, mut scope) = fixture(duet);
    p.request.settings.copy_mode = true;
    let p = plan_job(
        &p.request,
        &mut storage,
        &mut Metadata,
        &mut Ai { duet, fail: false },
    )
    .unwrap();
    scope.inspected_plan = p.binding_sha256.clone();
    scope.allowed_modes = vec![TransferMode::Copy];
    validate_execution(
        &p,
        &mut storage,
        &mut Metadata,
        &mut Ai { duet, fail: false },
        &resources,
        &scope,
    )
    .unwrap()
}
#[test]
fn in_flight_observations_survive_invalidation_but_next_effect_does_not() {
    for (intent, observation, next) in [
        (
            Phase::TempAllocationPrepared,
            Phase::TempAllocated,
            Phase::BytesWritePrepared,
        ),
        (
            Phase::BytesWritePrepared,
            Phase::BytesSynced,
            Phase::DestinationPublishPrepared,
        ),
        (
            Phase::DestinationPublishPrepared,
            Phase::DestinationPublished,
            Phase::SourceRetirementPrepared,
        ),
    ] {
        let b = binding(true);
        let (mut j, mut s) = review_committed_destination(&b);
        for phase in [
            Phase::Prepared,
            Phase::TempAllocationPrepared,
            Phase::TempAllocated,
            Phase::BytesWritePrepared,
            Phase::BytesSynced,
            Phase::DestinationPublishPrepared,
        ] {
            log(
                &mut j,
                &mut s,
                &b,
                1,
                phase,
                (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
                    .then(|| identity("temp-1")),
            )
            .unwrap();
            if phase == intent {
                break;
            }
        }
        log(&mut j, &mut s, &b, 0, Phase::Cancelled, None).unwrap();
        log(&mut j, &mut s, &b, 1, observation, Some(identity("temp-1"))).unwrap();
        if observation == Phase::DestinationPublished {
            log(
                &mut j,
                &mut s,
                &b,
                1,
                Phase::DestinationVerified,
                Some(identity("temp-1")),
            )
            .unwrap();
            log(
                &mut j,
                &mut s,
                &b,
                1,
                Phase::Committed,
                Some(identity("temp-1")),
            )
            .unwrap();
        }
        let id = if next == Phase::SourceRetirementPrepared {
            identity("song")
        } else {
            identity("temp-1")
        };
        assert_eq!(
            log(&mut j, &mut s, &b, 1, next, Some(id)),
            Err(E::InvalidTransition)
        );
        PhysicalJournalV2::open(&b, "job", &s.bytes).unwrap();
    }
}
#[test]
fn copy_dependency_needs_destination_commit_but_move_needs_source_retirement() {
    for copy in [true, false] {
        let b = review_two_mode(copy);
        let (mut j, mut s) = review_committed_destination(&b);
        if copy {
            log(&mut j, &mut s, &b, 1, Phase::Prepared, None).unwrap();
            assert_eq!(
                log(
                    &mut j,
                    &mut s,
                    &b,
                    0,
                    Phase::SourceRetirementPrepared,
                    Some(identity("song"))
                ),
                Err(E::InvalidTransition)
            );
        } else {
            assert_eq!(
                log(&mut j, &mut s, &b, 1, Phase::Prepared, None),
                Err(E::InvalidTransition)
            );
            log(
                &mut j,
                &mut s,
                &b,
                0,
                Phase::SourceRetirementPrepared,
                Some(identity("song")),
            )
            .unwrap();
            log(
                &mut j,
                &mut s,
                &b,
                0,
                Phase::SourceRetired,
                Some(identity("song")),
            )
            .unwrap();
            log(&mut j, &mut s, &b, 1, Phase::Prepared, None).unwrap();
        }
    }
}
#[test]
fn in_flight_atomic_observation_survives_predecessor_invalidation() {
    let b = review_two_moves();
    let mut s = Sink::default();
    let mut j = PhysicalJournalV2::create(&b, "job", &mut s).unwrap();
    for (op, id) in [(0, "song"), (1, "song2")] {
        for phase in [
            Phase::Prepared,
            Phase::AtomicRenamePrepared,
            Phase::AtomicRenameObserved,
            Phase::DestinationVerified,
            Phase::Committed,
        ] {
            if op == 1 && phase == Phase::AtomicRenameObserved {
                log(&mut j, &mut s, &b, 0, Phase::NeedsRecovery, None).unwrap();
            }
            log(
                &mut j,
                &mut s,
                &b,
                op,
                phase,
                (phase != Phase::Prepared).then(|| identity(id)),
            )
            .unwrap();
        }
    }
    PhysicalJournalV2::open(&b, "job", &s.bytes).unwrap();
}
#[test]
fn in_flight_retirement_observation_survives_duet_cancellation() {
    let b = binding(true);
    let mut s = Sink::default();
    let mut f = Fake::new();
    run(&b, &mut s, &mut f, false).unwrap();
    let lines: Vec<_> = s.bytes.split_inclusive(|x| *x == b'\n').collect();
    let prefix = lines[..lines.len() - 1].concat();
    s.bytes = prefix.clone();
    s.durable = prefix.clone();
    let mut j = PhysicalJournalV2::open(&b, "job", &prefix).unwrap();
    log(&mut j, &mut s, &b, 0, Phase::Cancelled, None).unwrap();
    log(
        &mut j,
        &mut s,
        &b,
        1,
        Phase::SourceRetired,
        Some(identity("song")),
    )
    .unwrap();
    PhysicalJournalV2::open(&b, "job", &s.bytes).unwrap();
}
fn duet_then_later_source() -> ExecutionBinding {
    let (p, mut storage, resources, mut scope) = fixture(true);
    storage.entries.push(entry(
        "song2",
        "song other.mp3",
        EntryKind::File,
        Some("source"),
    ));
    let p = plan_job(
        &p.request,
        &mut storage,
        &mut Metadata,
        &mut Ai {
            duet: true,
            fail: false,
        },
    )
    .unwrap();
    scope.inspected_plan = p.binding_sha256.clone();
    scope.inspected_sources.push(SourceEvidence {
        observation: storage.entries[3].clone(),
        sha256: digest(b"audio"),
    });
    validate_execution(
        &p,
        &mut storage,
        &mut Metadata,
        &mut Ai {
            duet: true,
            fail: false,
        },
        &resources,
        &scope,
    )
    .unwrap()
}
fn commit_copy_destination(
    j: &mut PhysicalJournalV2,
    s: &mut Sink,
    b: &ExecutionBinding,
    op: usize,
) {
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
        log(
            j,
            s,
            b,
            op,
            phase,
            (!matches!(phase, Phase::Prepared | Phase::TempAllocationPrepared))
                .then(|| identity(&format!("temp-{op}"))),
        )
        .unwrap();
    }
}
#[test]
fn duet_exception_is_complete_cohort_only_and_not_later_traversal_permission() {
    let b = duet_then_later_source();
    assert_eq!(b.operations().len(), 4);
    assert_eq!(b.operations()[0].duet_cohort, Some(vec![0, 1]));
    assert_eq!(b.operations()[1].duet_cohort, Some(vec![0, 1]));
    assert_eq!(b.operations()[2].duet_cohort, Some(vec![2, 3]));
    let (mut j, mut s) = review_committed_destination(&b);
    // The remaining exact cohort member may copy before source retirement.
    commit_copy_destination(&mut j, &mut s, &b, 1);
    // A later source/traversal dependency cannot use that exception.
    assert_eq!(
        log(&mut j, &mut s, &b, 2, Phase::Prepared, None),
        Err(E::InvalidTransition)
    );
    log(
        &mut j,
        &mut s,
        &b,
        1,
        Phase::SourceRetirementPrepared,
        Some(identity("song")),
    )
    .unwrap();
    log(
        &mut j,
        &mut s,
        &b,
        1,
        Phase::SourceRetired,
        Some(identity("song")),
    )
    .unwrap();
    log(&mut j, &mut s, &b, 2, Phase::Prepared, None).unwrap();
}
#[test]
fn failed_or_cancelled_duet_cohort_blocks_remaining_copy_and_retirement() {
    for terminal in [Phase::Failed, Phase::Cancelled, Phase::NeedsRecovery] {
        let b = binding(true);
        let (mut j, mut s) = review_committed_destination(&b);
        log(&mut j, &mut s, &b, 0, terminal, None).unwrap();
        assert_eq!(
            log(&mut j, &mut s, &b, 1, Phase::Prepared, None),
            Err(E::InvalidTransition)
        );
        assert_eq!(
            log(
                &mut j,
                &mut s,
                &b,
                0,
                Phase::SourceRetirementPrepared,
                Some(identity("song"))
            ),
            Err(E::InvalidTransition)
        );
    }
}
#[test]
fn recovery_requires_live_source_disappearance_for_move_but_not_copy_dependencies() {
    struct Observations {
        source_absent: bool,
        destinations: HashMap<String, ObservedObject>,
    }
    impl RecoveryStorage for Observations {
        fn source(&mut self, r: &StorageRef) -> Result<Option<ObservedObject>> {
            let StorageRef::TestOpaque { id } = r else {
                panic!()
            };
            Ok(
                (!self.source_absent || id != "song").then(|| ObservedObject {
                    identity: identity(id),
                    sha256: digest(b"audio"),
                }),
            )
        }
        fn destination(&mut self, d: &DestinationEvidence) -> Result<Option<ObservedObject>> {
            Ok(self.destinations.get(&d.id).cloned())
        }
    }
    for copy in [false, true] {
        let b = review_two_mode(copy);
        let (mut j, mut s) = review_committed_destination(&b);
        if !copy {
            log(
                &mut j,
                &mut s,
                &b,
                0,
                Phase::SourceRetirementPrepared,
                Some(identity("song")),
            )
            .unwrap();
            log(
                &mut j,
                &mut s,
                &b,
                0,
                Phase::SourceRetired,
                Some(identity("song")),
            )
            .unwrap();
        }
        for absent in [false, true] {
            let mut storage = Observations {
                source_absent: absent,
                destinations: HashMap::from([(
                    b.operations()[0].destination.id.clone(),
                    ObservedObject {
                        identity: identity("temp-0"),
                        sha256: digest(b"audio"),
                    },
                )]),
            };
            let expected = if absent != copy {
                RecoveryStatus::ReadyForExplicitResume
            } else {
                RecoveryStatus::NeedsRecovery
            };
            assert_eq!(
                recover_job(&b, &j, &mut storage).unwrap().operations[1].status,
                expected
            );
        }
    }
}
#[test]
fn recovery_distinguishes_remaining_duet_copy_from_later_source() {
    struct Observations {
        retired: bool,
        destinations: HashMap<String, ObservedObject>,
    }
    impl RecoveryStorage for Observations {
        fn source(&mut self, r: &StorageRef) -> Result<Option<ObservedObject>> {
            let StorageRef::TestOpaque { id } = r else {
                panic!()
            };
            Ok((!self.retired || id != "song").then(|| ObservedObject {
                identity: identity(id),
                sha256: digest(b"audio"),
            }))
        }
        fn destination(&mut self, d: &DestinationEvidence) -> Result<Option<ObservedObject>> {
            Ok(self.destinations.get(&d.id).cloned())
        }
    }
    let b = duet_then_later_source();
    let (mut j, mut s) = review_committed_destination(&b);
    let mut storage = Observations {
        retired: false,
        destinations: HashMap::from([(
            b.operations()[0].destination.id.clone(),
            ObservedObject {
                identity: identity("temp-0"),
                sha256: digest(b"audio"),
            },
        )]),
    };
    let report = recover_job(&b, &j, &mut storage).unwrap();
    assert_eq!(
        report.operations[1].status,
        RecoveryStatus::ReadyForExplicitResume
    );
    assert_eq!(report.operations[2].status, RecoveryStatus::NeedsRecovery);
    commit_copy_destination(&mut j, &mut s, &b, 1);
    storage.destinations.insert(
        b.operations()[1].destination.id.clone(),
        ObservedObject {
            identity: identity("temp-1"),
            sha256: digest(b"audio"),
        },
    );
    assert_eq!(
        recover_job(&b, &j, &mut storage).unwrap().operations[2].status,
        RecoveryStatus::NeedsRecovery
    );
    log(
        &mut j,
        &mut s,
        &b,
        1,
        Phase::SourceRetirementPrepared,
        Some(identity("song")),
    )
    .unwrap();
    log(
        &mut j,
        &mut s,
        &b,
        1,
        Phase::SourceRetired,
        Some(identity("song")),
    )
    .unwrap();
    storage.retired = true;
    assert_eq!(
        recover_job(&b, &j, &mut storage).unwrap().operations[2].status,
        RecoveryStatus::ReadyForExplicitResume
    );
}
