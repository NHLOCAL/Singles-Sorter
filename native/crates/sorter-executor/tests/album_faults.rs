use sorter_core::{ClassifierFact, Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{
    ExecutionError as E, album::*, binding::*, digest, physical_journal::*, recovery::*,
    results::*, transfer::*,
};
use sorter_storage_contracts::*;
use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
fn id(s: &str) -> ObjectIdentity {
    ObjectIdentity::Test { id: s.into() }
}
fn reference(s: &str) -> StorageRef {
    StorageRef::TestOpaque { id: s.into() }
}
fn entry(s: &str, parent: Option<&str>, name: &str, kind: EntryKind) -> EntryObservation {
    EntryObservation {
        reference: reference(s),
        identity: Some(id(s)),
        parent_identity: parent.map(id),
        name: name.into(),
        kind,
        size: Some(5),
        modified: Some("v1".into()),
        capabilities: Capabilities {
            read: Some(true),
            list: Some(true),
            seek: Some(true),
        },
    }
}
#[derive(Clone)]
struct PlanStorage {
    entries: Vec<EntryObservation>,
}
impl ReadStorage for PlanStorage {
    fn stat(&mut self, r: &StorageRef) -> Result<EntryObservation, StorageError> {
        self.entries
            .iter()
            .find(|e| &e.reference == r)
            .cloned()
            .ok_or_else(|| StorageError::new(StorageErrorKind::NotFound, "absent"))
    }
    fn children(&mut self, r: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        let parent = self.stat(r)?.identity;
        Ok(self
            .entries
            .iter()
            .filter(|e| e.parent_identity == parent)
            .cloned()
            .collect())
    }
    fn open_read(&mut self, r: &StorageRef) -> Result<ReadInput, StorageError> {
        let e = self.stat(r)?;
        Ok(
            ReadInput::seekable(Cursor::new(b"audio".to_vec()), 5).with_binding(ReadBinding {
                identity: e.identity.unwrap(),
                size: 5,
                modified: e.modified.unwrap(),
            }),
        )
    }
}
struct Meta;
impl MetadataProvider for Meta {
    fn open(&mut self, _: &EntryObservation) -> Result<Box<dyn MetadataFields>, MetadataError> {
        Ok(Box::new(Meta))
    }
}
impl MetadataFields for Meta {
    fn get(&mut self, field: LegacyField) -> Result<LegacyItem, FieldError> {
        let value = match field {
            LegacyField::Album => "Album",
            LegacyField::Artist => "אברהם פריד",
            _ => "",
        };
        let scalar = LegacyScalar::Text(value.into());
        Ok(LegacyItem {
            values: vec![scalar.clone()],
            truthy: !value.is_empty(),
            value: Ok(scalar),
        })
    }
}
struct Ai;
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        AiProfile::Disabled
    }
    fn classify(&mut self, _: &str) -> Result<ClassifierFact, RecognitionError> {
        Ok(ClassifierFact::Reject)
    }
    fn ner(&mut self, _: &str) -> Result<Vec<String>, RecognitionError> {
        Ok(vec![])
    }
}
fn fixture(
    mut mutate: impl FnMut(&mut PlanStorage, &mut JobRequestV2),
) -> (JobPlanV2, PlanStorage, Resources, ExecutionScope) {
    let mut s = PlanStorage {
        entries: vec![
            entry("source", None, "source", EntryKind::Directory),
            entry("target", None, "target", EntryKind::Directory),
            entry("album", Some("source"), "album", EntryKind::Directory),
        ],
    };
    for (i, name) in ["1.mp3", "2.mp3", "3.mp3", "cover.jpg"].iter().enumerate() {
        s.entries.push(entry(
            &format!("file{i}"),
            Some("album"),
            name,
            EntryKind::File,
        ));
    }
    let mut request = JobRequestV2 {
        mode: JobMode::Sort,
        source: reference("source"),
        target: Some(reference("target")),
        settings: Settings {
            copy_mode: true,
            ..Settings::default()
        },
        alias_rows: vec![],
        resource_ids: vec![],
        adapter_versions: vec![],
        ai_profile: AiProfile::Disabled,
    };
    mutate(&mut s, &mut request);
    let plan = plan_job(&request, &mut s, &mut Meta, &mut Ai).unwrap();
    let resources = Resources {
        resource_ids: vec![],
        adapter_versions: vec![],
        ai_profile: AiProfile::Disabled,
        resource_hashes: vec![],
        adapter_hashes: vec![],
        model_hash: None,
    };
    let scope = ExecutionScope {
        inspected_plan: plan.binding_sha256.clone(),
        source_root: id("source"),
        target_root: id("target"),
        inspected_sources: s
            .entries
            .iter()
            .filter(|e| e.kind == EntryKind::File)
            .map(|e| SourceEvidence {
                observation: e.clone(),
                sha256: digest(b"audio"),
            })
            .collect(),
        resources: resources.clone(),
        allowed_modes: vec![TransferMode::Copy],
    };
    (plan, s, resources, scope)
}
fn binding() -> (AlbumBinding, ExecutionScope) {
    let (p, mut s, r, scope) = fixture(|_, _| {});
    let b = validate_album_copy(&p, &mut s, &mut Meta, &mut Ai, &r, &scope).unwrap();
    (b, scope)
}
#[derive(Default, Clone)]
struct Sink {
    bytes: Vec<u8>,
    count: usize,
    fail: Option<usize>,
    partial: bool,
    cancel: Option<usize>,
    flag: Option<Arc<AtomicBool>>,
}
impl Sink {
    fn hit(&mut self) -> sorter_executor::Result<()> {
        self.count += 1;
        if self.cancel == Some(self.count) {
            self.flag.as_ref().unwrap().store(true, Ordering::Release);
        }
        if self.fail == Some(self.count) {
            Err(E::Io)
        } else {
            Ok(())
        }
    }
}
impl JournalStore for Sink {
    fn read_all(&mut self) -> sorter_executor::Result<Vec<u8>> {
        Ok(self.bytes.clone())
    }
    fn append(&mut self, bytes: &[u8]) -> sorter_executor::Result<()> {
        if self.partial && self.fail == Some(self.count + 1) {
            self.bytes.extend(&bytes[..bytes.len() / 2]);
            self.count += 1;
            return Err(E::Io);
        }
        self.hit()?;
        self.bytes.extend(bytes);
        Ok(())
    }
    fn sync(&mut self) -> sorter_executor::Result<()> {
        self.hit()
    }
}
#[derive(Clone)]
struct Temp {
    bytes: Vec<u8>,
    obs: MutationObservation,
}
fn observation(
    identity: ObjectIdentity,
    parent: ObjectIdentity,
    name: MutationName,
) -> MutationObservation {
    let ObjectIdentity::Test { id: s } = &identity else {
        panic!()
    };
    MutationObservation {
        reference: reference(s),
        identity,
        parent_identity: parent,
        actual_name: name,
        size: Some(5),
    }
}
fn error() -> MutationError {
    MutationError::new(MutationErrorKind::Io, "fault")
}
struct Fake {
    bound: AlbumBinding,
    directories: HashMap<Vec<String>, MutationObservation>,
    published: HashMap<String, Temp>,
    sources: Vec<SourceEvidence>,
    flag: Arc<AtomicBool>,
    trace: Vec<String>,
    fail: Option<usize>,
    cancel: Option<usize>,
    sequence: usize,
    lose_directory: bool,
    wrong_directory: bool,
    unknown_capability: bool,
    bad_parent: bool,
}
impl Fake {
    fn new(b: &AlbumBinding, flag: Arc<AtomicBool>) -> Self {
        let directories = b
            .directories()
            .iter()
            .filter_map(|d| {
                d.existing.as_ref().map(|e| {
                    (
                        d.components.clone(),
                        observation(
                            e.identity.clone().unwrap(),
                            e.parent_identity.clone().unwrap_or(id("outside")),
                            MutationName::Provider(e.name.clone()),
                        ),
                    )
                })
            })
            .collect();
        Self {
            bound: b.clone(),
            directories,
            published: HashMap::new(),
            sources: b.children().cloned().collect(),
            flag,
            trace: vec![],
            fail: None,
            cancel: None,
            sequence: 0,
            lose_directory: false,
            wrong_directory: false,
            unknown_capability: false,
            bad_parent: false,
        }
    }
    fn hit(&mut self, at: &str) -> Result<(), MutationError> {
        self.trace.push(at.into());
        let n = self.trace.len();
        if self.cancel == Some(n) {
            self.flag.store(true, Ordering::Release);
        }
        if self.fail == Some(n) {
            Err(error())
        } else {
            Ok(())
        }
    }
    fn path(&self, parent: &ObjectIdentity) -> Vec<String> {
        self.directories
            .iter()
            .find(|(_, o)| &o.identity == parent)
            .unwrap()
            .0
            .clone()
    }
}
impl MutationStorage for Fake {
    type Parent = ObjectIdentity;
    type SourceLease = Cursor<Vec<u8>>;
    type OwnedTemp = Temp;
    type Published = String;
    fn capabilities(&self) -> MutationCapabilities {
        MutationCapabilities {
            exclusive_create: Capability::Supported,
            no_replace_publish: Capability::Supported,
            content_verification: Capability::Supported,
            ..Default::default()
        }
    }
    fn acquire_source(
        &mut self,
        _: &Self::Parent,
        e: &ExpectedSource,
    ) -> Result<Self::SourceLease, MutationError> {
        self.hit("acquire_source")?;
        if !self.sources.iter().any(|s| {
            s.observation.identity.as_ref() == Some(&e.identity) && s.sha256 == digest(b"audio")
        }) {
            return Err(error());
        }
        Ok(Cursor::new(b"audio".to_vec()))
    }
    fn create_temp(
        &mut self,
        parent: &Self::Parent,
        _: &str,
        flag: &AtomicBool,
    ) -> MutationOutcome<Temp> {
        if let Err(e) = self.hit("create_temp") {
            return MutationOutcome::NoEffect(e);
        }
        if flag.load(Ordering::Acquire) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Cancelled,
                "cancelled",
            ));
        }
        self.sequence += 1;
        MutationOutcome::Applied(Temp {
            bytes: vec![],
            obs: observation(
                id(&format!("temp{}", self.sequence)),
                parent.clone(),
                MutationName::Provider(format!("tmp{}", self.sequence)),
            ),
        })
    }
    fn publish_no_replace(
        &mut self,
        temp: &mut Temp,
        parent: &Self::Parent,
        name: &MutationName,
        _: &AtomicBool,
    ) -> MutationOutcome<String> {
        if let Err(e) = self.hit("publish") {
            return MutationOutcome::NoEffect(e);
        }
        let key = format!("{name:?}");
        if self.published.contains_key(&key) {
            return MutationOutcome::NoEffect(error());
        }
        let mut t = temp.clone();
        t.obs.parent_identity = parent.clone();
        t.obs.actual_name = name.clone();
        self.published.insert(key.clone(), t);
        MutationOutcome::Applied(key)
    }
    fn rename_no_replace(
        &mut self,
        _: &mut Self::SourceLease,
        _: &Self::Parent,
        _: &MutationName,
        _: &AtomicBool,
    ) -> MutationOutcome<String> {
        panic!("album Copy must never rename")
    }
    fn verify(
        &mut self,
        p: &String,
        d: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        self.hit("verify")?;
        let t = &self.published[p];
        assert_eq!(t.bytes.len() as u64, d.length);
        assert_eq!(digest(&t.bytes), digest(b"audio"));
        Ok(t.obs.clone())
    }
    fn retire_source(
        &mut self,
        _: &mut Self::SourceLease,
        _: &AtomicBool,
    ) -> MutationOutcome<RetirementOutcome> {
        panic!("album Copy must never retire")
    }
    fn inspect(&mut self, p: &String) -> Result<MutationObservation, MutationError> {
        self.hit("inspect")?;
        Ok(self.published[p].obs.clone())
    }
}
impl TransferStorage for Fake {
    fn source_parent(
        &mut self,
        s: &SourceEvidence,
        _: &ObjectIdentity,
    ) -> Result<Self::Parent, MutationError> {
        self.hit("source_parent")?;
        Ok(s.observation.parent_identity.clone().unwrap())
    }
    fn destination_parent(
        &mut self,
        d: &DestinationEvidence,
        _: &ObjectIdentity,
    ) -> Result<Self::Parent, MutationError> {
        self.hit("destination_parent")?;
        Ok(if self.bad_parent {
            id("foreign")
        } else {
            self.directories[&d.components[..d.components.len() - 1]]
                .identity
                .clone()
        })
    }
    fn parent_identity(&self, p: &Self::Parent) -> ObjectIdentity {
        p.clone()
    }
    fn temp_observation(&self, t: &Temp) -> MutationObservation {
        t.obs.clone()
    }
    fn rewind_source(&mut self, s: &mut Self::SourceLease) -> Result<(), MutationError> {
        self.hit("rewind")?;
        s.set_position(0);
        Ok(())
    }
    fn read_source(&mut self, s: &mut Self::SourceLease, b: &mut [u8]) -> io::Result<usize> {
        self.hit("read").map_err(io::Error::other)?;
        s.read(b)
    }
    fn write_temp(&mut self, t: &mut Temp, b: &[u8]) -> io::Result<usize> {
        self.hit("write").map_err(io::Error::other)?;
        t.bytes.extend(b);
        Ok(b.len())
    }
    fn sync_temp(&mut self, _: &mut Temp) -> Result<(), MutationError> {
        self.hit("sync_temp")
    }
    fn verify_temp(
        &mut self,
        t: &Temp,
        _: &ContentDigest,
    ) -> Result<MutationObservation, MutationError> {
        self.hit("verify_temp")?;
        Ok(t.obs.clone())
    }
    fn rename_eligible(&self, _: &Self::SourceLease, _: &Self::Parent) -> bool {
        false
    }
}
impl OwnedDirectoryStorage for Fake {
    type OwnedDirectory = (Vec<String>, MutationObservation);
    fn directory_capabilities(&self) -> DirectoryCapabilities {
        DirectoryCapabilities {
            exclusive_owned_create: if self.unknown_capability {
                Capability::Unknown
            } else {
                Capability::Supported
            },
            held_ancestry: Capability::Supported,
            stable_child_set: Capability::Supported,
            durable_directory_sync: Capability::Supported,
        }
    }
    fn create_owned_directory(
        &mut self,
        parent: &Self::Parent,
        name: &MutationName,
        flag: &AtomicBool,
    ) -> MutationOutcome<Self::OwnedDirectory> {
        if let Err(e) = self.hit("mkdir") {
            return MutationOutcome::NoEffect(e);
        }
        if flag.load(Ordering::Acquire) {
            return MutationOutcome::NoEffect(MutationError::new(
                MutationErrorKind::Cancelled,
                "cancelled",
            ));
        }
        let MutationName::Provider(name) = name else {
            panic!()
        };
        let mut path = self.path(parent);
        path.push(name.clone());
        if self.directories.contains_key(&path) {
            return MutationOutcome::NoEffect(error());
        }
        self.sequence += 1;
        let mut obs = observation(
            id(&format!("dir{}", self.sequence)),
            parent.clone(),
            MutationName::Provider(name.clone()),
        );
        if self.wrong_directory {
            obs.identity = id("source");
        }
        self.directories.insert(path.clone(), obs.clone());
        if self.lose_directory {
            return MutationOutcome::PendingUnknown(MutationError::new(
                MutationErrorKind::UnknownOutcome,
                "response lost",
            ));
        }
        if let Err(e) = self.hit("mkdir_applied") {
            return MutationOutcome::PendingUnknown(e.with_observation(obs));
        }
        MutationOutcome::Applied((path, obs))
    }
    fn directory_observation(&self, d: &Self::OwnedDirectory) -> MutationObservation {
        d.1.clone()
    }
    fn sync_owned_directory(&mut self, _: &Self::OwnedDirectory) -> Result<(), MutationError> {
        self.hit("sync_directory")
    }
    fn owned_directory_parent(&self, d: &Self::OwnedDirectory) -> Self::Parent {
        d.1.identity.clone()
    }
}
impl AlbumStorage for Fake {
    type AlbumLease = ();
    fn acquire_album(&mut self, b: &AlbumBinding) -> Result<(), MutationError> {
        self.hit("acquire_album")?;
        if self.sources != b.children().cloned().collect::<Vec<_>>() {
            return Err(error());
        }
        for d in b.directories() {
            match (&d.existing, self.directories.get(&d.components)) {
                (None, None) => {}
                (Some(e), Some(o)) if e.identity.as_ref() == Some(&o.identity) => {}
                _ => return Err(error()),
            }
        }
        Ok(())
    }
    fn existing_album_parent(
        &mut self,
        _: &(),
        index: usize,
    ) -> Result<Self::Parent, MutationError> {
        self.hit("existing_parent")?;
        Ok(
            self.directories[&self.bound.directories()[index].components]
                .identity
                .clone(),
        )
    }
    fn check_album(&mut self, _: &(), b: &AlbumBinding) -> Result<(), MutationError> {
        self.hit("check_album")?;
        if self.sources == b.children().cloned().collect::<Vec<_>>() {
            Ok(())
        } else {
            Err(error())
        }
    }
}
impl RecoveryStorage for Fake {
    fn source(&mut self, r: &StorageRef) -> sorter_executor::Result<Option<ObservedObject>> {
        Ok(self
            .sources
            .iter()
            .find(|s| &s.observation.reference == r)
            .map(|s| ObservedObject {
                identity: s.observation.identity.clone().unwrap(),
                sha256: s.sha256.clone(),
            }))
    }
    fn destination(
        &mut self,
        d: &DestinationEvidence,
    ) -> sorter_executor::Result<Option<ObservedObject>> {
        let key = format!(
            "{:?}",
            MutationName::Provider(d.components.last().unwrap().clone())
        );
        Ok(self.published.get(&key).map(|t| ObservedObject {
            identity: t.obs.identity.clone(),
            sha256: digest(&t.bytes),
        }))
    }
}
impl AlbumRecoveryStorage for Fake {
    fn album_source(
        &mut self,
        b: &AlbumBinding,
    ) -> sorter_executor::Result<Option<(EntryObservation, Vec<SourceEvidence>)>> {
        Ok(Some((b.source().clone(), self.sources.clone())))
    }
    fn directory(
        &mut self,
        b: &AlbumBinding,
        index: usize,
    ) -> sorter_executor::Result<Option<MutationObservation>> {
        Ok(self
            .directories
            .get(&b.directories()[index].components)
            .cloned())
    }
}
fn run(b: &AlbumBinding, f: &mut Fake, s: &mut Sink) -> (AlbumJournal, AlbumResult) {
    let mut j = AlbumJournal::create(b, "job", s).unwrap();
    let flag = f.flag.clone();
    let r = run_album_copy(b, f, &mut j, s, &flag, &mut |_| {}).unwrap();
    (j, r)
}
fn reopen(b: &AlbumBinding, scope: &ExecutionScope, s: &Sink) -> AlbumJournal {
    AlbumJournal::open_for_recovery(
        &RecoveryScope {
            original_scope: scope.clone(),
            inspected_binding_sha256: b.sha256().unwrap(),
        },
        "job",
        &s.bytes,
    )
    .unwrap()
}
#[test]
fn copies_every_flat_child_preserving_sources_and_reconciles() {
    let (b, scope) = binding();
    let flag = Arc::new(AtomicBool::new(false));
    let mut f = Fake::new(&b, flag);
    let original = f.sources.clone();
    let mut s = Sink::default();
    let (j, r) = run(&b, &mut f, &mut s);
    assert_eq!(r.status, JobStatus::Complete);
    assert_eq!(r.transfers.unwrap().committed_destinations, 4);
    assert_eq!(f.sources, original);
    assert_eq!(f.published.len(), 4);
    assert_eq!(r.directories.len(), 2);
    let mut opened = reopen(&b, &scope, &s);
    let report = reconcile_album(&opened, &mut f).unwrap();
    assert_eq!(report.status, RecoveryStatus::Committed);
    assert!(report.append_blocked);
    assert_eq!(
        run_album_copy(
            &opened.binding().clone(),
            &mut f,
            &mut opened,
            &mut s,
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .unwrap_err(),
        E::Scope
    );
    assert!(matches!(j.events().last(), Some(AlbumEvent::Complete)));
}
#[test]
fn every_storage_failure_preserves_sources_and_stops_success_claims() {
    let (b, scope) = binding();
    let flag = Arc::new(AtomicBool::new(false));
    let mut baseline = Fake::new(&b, flag);
    run(&b, &mut baseline, &mut Sink::default());
    for at in 1..=baseline.trace.len() {
        let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
        f.fail = Some(at);
        let original = f.sources.clone();
        let mut s = Sink::default();
        let (_, r) = run(&b, &mut f, &mut s);
        assert_ne!(
            r.status,
            JobStatus::Complete,
            "{at}: {:?}",
            baseline.trace[at - 1]
        );
        assert_eq!(f.sources, original);
        assert!(f.published.len() <= 4);
        let j = reopen(&b, &scope, &s);
        assert_ne!(
            reconcile_album(&j, &mut f).unwrap().status,
            RecoveryStatus::Committed
        );
    }
}
#[test]
fn cancellation_at_every_storage_boundary_stops_new_work_and_preserves_sources() {
    let (b, _) = binding();
    let mut baseline = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    run(&b, &mut baseline, &mut Sink::default());
    for at in 1..=baseline.trace.len() {
        let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
        f.cancel = Some(at);
        let original = f.sources.clone();
        let (_, r) = run(&b, &mut f, &mut Sink::default());
        assert_ne!(
            r.status,
            JobStatus::Complete,
            "{at}: {:?}",
            baseline.trace[at - 1]
        );
        assert_eq!(f.sources, original);
    }
}
#[test]
fn every_journal_append_and_sync_failure_is_fail_closed_including_nested_records() {
    let (b, scope) = binding();
    let mut baseline = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    let mut bs = Sink::default();
    run(&b, &mut baseline, &mut bs);
    for partial in [false, true] {
        for at in 3..=bs.count {
            let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
            let original = f.sources.clone();
            let mut s = Sink {
                fail: Some(at),
                partial,
                ..Default::default()
            };
            let (j, r) = run(&b, &mut f, &mut s);
            assert_eq!(r.status, JobStatus::NeedsRecovery, "{partial}/{at}");
            assert_eq!(f.sources, original);
            assert!(j.needs_recovery());
            let opened = reopen(&b, &scope, &s);
            let report = reconcile_album(&opened, &mut f).unwrap();
            // A sync failure may occur after the entire Complete record persisted.
            // On restart exact retained durable evidence can prove completion; the
            // failed live invocation nevertheless always reported NeedsRecovery.
            if report.status == RecoveryStatus::Committed {
                assert!(matches!(opened.events().last(), Some(AlbumEvent::Complete)));
                assert_eq!(f.published.len(), 4);
            }
            assert!(report.append_blocked);
        }
    }
}
#[test]
fn cancellation_during_each_journal_boundary_never_discards_an_effect() {
    let (b, _) = binding();
    let mut baseline = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    let mut bs = Sink::default();
    run(&b, &mut baseline, &mut bs);
    for at in 3..=bs.count {
        let flag = Arc::new(AtomicBool::new(false));
        let mut f = Fake::new(&b, flag.clone());
        let mut s = Sink {
            cancel: Some(at),
            flag: Some(flag),
            ..Default::default()
        };
        let (_, r) = run(&b, &mut f, &mut s);
        assert_eq!(f.sources.len(), 4);
        if r.status == JobStatus::Complete {
            assert_eq!(f.published.len(), 4);
            assert!(at >= bs.count - 1);
        }
        assert_eq!(r.directories.len() + 1, f.directories.len());
    }
}
#[test]
fn directory_response_loss_never_adopts_found_same_name() {
    let (b, scope) = binding();
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    f.lose_directory = true;
    let mut s = Sink::default();
    let (j, r) = run(&b, &mut f, &mut s);
    assert_eq!(r.status, JobStatus::NeedsRecovery);
    assert_eq!(f.directories.len(), 2);
    assert!(f.published.is_empty());
    assert!(
        !j.events()
            .iter()
            .any(|e| matches!(e, AlbumEvent::DirectoryCreated { .. }))
    );
    let report = reconcile_album(&reopen(&b, &scope, &s), &mut f).unwrap();
    assert_eq!(report.directories[1], RecoveryStatus::NeedsRecovery);
}
#[test]
fn incorrect_directory_identity_is_preserved_but_never_authorizes_children() {
    let (b, scope) = binding();
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    f.wrong_directory = true;
    let mut s = Sink::default();
    let (j, r) = run(&b, &mut f, &mut s);
    assert_eq!(r.status, JobStatus::NeedsRecovery);
    assert!(f.published.is_empty());
    assert!(r.directories.iter().any(|o| o.identity == id("source")));
    assert!(matches!(
        j.events().last(),
        Some(AlbumEvent::Stopped {
            observed: Some(_),
            ..
        })
    ));
    assert_ne!(
        reconcile_album(&reopen(&b, &scope, &s), &mut f)
            .unwrap()
            .status,
        RecoveryStatus::Committed
    );
}
#[test]
fn child_set_mutation_and_foreign_directory_after_binding_stop_before_effects() {
    let (b, _) = binding();
    for change in 0..5 {
        let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
        match change {
            0 => {
                f.sources.pop();
            }
            1 => f.sources.swap(0, 1),
            2 => f.sources[0].observation.identity = Some(id("changed")),
            3 => {
                let path = b.directories()[1].components.clone();
                f.directories.insert(
                    path,
                    observation(
                        id("foreign"),
                        id("target"),
                        MutationName::Provider("אברהם פריד".into()),
                    ),
                );
            }
            _ => f.sources[0].sha256 = digest(b"changed"),
        }
        let (_, r) = run(&b, &mut f, &mut Sink::default());
        assert_ne!(r.status, JobStatus::Complete);
        assert!(!f.trace.iter().any(|s| s == "mkdir" || s == "create_temp"));
    }
}
#[test]
fn partial_children_remain_visible_and_later_children_are_blocked() {
    let (b, scope) = binding();
    let mut base = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    run(&b, &mut base, &mut Sink::default());
    let second = base
        .trace
        .iter()
        .enumerate()
        .filter(|(_, s)| s.as_str() == "write")
        .nth(1)
        .unwrap()
        .0
        + 1;
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    f.fail = Some(second);
    let mut s = Sink::default();
    let (_, r) = run(&b, &mut f, &mut s);
    assert_eq!(r.status, JobStatus::Partial);
    assert_eq!(f.published.len(), 1);
    let child = r.transfers.unwrap();
    assert_eq!(child.committed_destinations, 1);
    assert_eq!(child.blocked_operations, 2);
    assert_eq!(f.sources.len(), 4);
    assert_eq!(
        reconcile_album(&reopen(&b, &scope, &s), &mut f)
            .unwrap()
            .status,
        RecoveryStatus::NeedsRecovery
    );
}
#[test]
fn final_directory_identity_is_rechecked_by_streaming_driver() {
    let (b, _) = binding();
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    f.bad_parent = true;
    let (_, r) = run(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Partial);
    assert!(f.published.is_empty());
    assert!(!f.trace.iter().any(|s| s == "create_temp"));
}
#[test]
fn unknown_directory_capability_and_initial_cancellation_have_no_effect() {
    let (b, _) = binding();
    for cancel in [false, true] {
        let mut f = Fake::new(&b, Arc::new(AtomicBool::new(cancel)));
        f.unknown_capability = true;
        let (_, r) = run(&b, &mut f, &mut Sink::default());
        assert_ne!(r.status, JobStatus::Complete);
        assert!(f.trace.is_empty());
    }
}
#[test]
fn unsupported_album_shapes_are_executor_gaps_not_changed_planner_decisions() {
    for variant in 0..5 {
        let (p, mut s, r, scope) = fixture(|s, req| match variant {
            0 => req.settings.copy_mode = false,
            1 => s.entries.push(entry(
                "nested",
                Some("album"),
                "extras",
                EntryKind::Directory,
            )),
            2 => s.entries.push(entry(
                "link",
                Some("album"),
                "link",
                EntryKind::LinkOrReparse,
            )),
            3 => s.entries[6].name = "cover?.jpg".into(),
            _ => {
                s.entries[6].name = "CON.jpg".into();
            }
        });
        let original = serde_json::to_vec(&p).unwrap();
        assert!(
            validate_album_copy(&p, &mut s, &mut Meta, &mut Ai, &r, &scope).is_err(),
            "{variant}"
        );
        assert_eq!(serde_json::to_vec(&p).unwrap(), original);
    }
}
#[test]
fn existing_required_artist_parent_is_bound_but_new_album_is_never_adopted() {
    let (p, mut s, r, scope) = fixture(|s, _| {
        s.entries.push(entry(
            "artist",
            Some("target"),
            "אברהם פריד",
            EntryKind::Directory,
        ))
    });
    let b = validate_album_copy(&p, &mut s, &mut Meta, &mut Ai, &r, &scope).unwrap();
    assert!(b.directories()[1].existing.is_some());
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    let (_, r) = run(&b, &mut f, &mut Sink::default());
    assert_eq!(r.status, JobStatus::Complete);
    assert_eq!(r.directories.len(), 1);
}
#[test]
fn restart_header_only_incomplete_tail_foreign_identity_and_source_replacement() {
    let (b, scope) = binding();
    let mut s = Sink::default();
    AlbumJournal::create(&b, "job", &mut s).unwrap();
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    let opened = reopen(&b, &scope, &s);
    assert_eq!(
        reconcile_album(&opened, &mut f).unwrap().directories[1],
        RecoveryStatus::ReadyForExplicitResume
    );
    s.bytes.extend(b"{incomplete");
    let opened = reopen(&b, &scope, &s);
    assert_eq!(opened.incomplete_tail(), b"{incomplete");
    assert!(opened.needs_recovery());
    assert_eq!(
        reconcile_album(&opened, &mut f).unwrap().status,
        RecoveryStatus::NeedsRecovery
    );
    let mut s = Sink::default();
    run(&b, &mut f, &mut s);
    let opened = reopen(&b, &scope, &s);
    let key = b.directories().last().unwrap().components.clone();
    let original = f.directories[&key].identity.clone();
    f.directories.get_mut(&key).unwrap().identity = id("foreign");
    assert_eq!(
        reconcile_album(&opened, &mut f).unwrap().status,
        RecoveryStatus::NeedsRecovery
    );
    f.directories.get_mut(&key).unwrap().identity = original;
    f.sources[0].sha256 = digest(b"foreign");
    assert_eq!(
        reconcile_album(&opened, &mut f).unwrap().status,
        RecoveryStatus::NeedsRecovery
    );
}
#[test]
fn provenance_tampering_and_child_binding_bypass_are_rejected() {
    let (b, scope) = binding();
    let mut f = Fake::new(&b, Arc::new(AtomicBool::new(false)));
    let mut s = Sink::default();
    let (j, _) = run(&b, &mut f, &mut s);
    let child_bytes: Vec<_> = j
        .events()
        .iter()
        .flat_map(|e| match e {
            AlbumEvent::TransferRecord { bytes } => bytes.clone(),
            _ => vec![],
        })
        .collect();
    let header: serde_json::Value =
        serde_json::from_slice(child_bytes.split(|b| *b == b'\n').next().unwrap()).unwrap();
    let digest = header["body"]["binding_sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut child = PhysicalJournalV2::open_for_recovery(
        &RecoveryScope {
            original_scope: scope.clone(),
            inspected_binding_sha256: digest,
        },
        "job",
        &child_bytes,
    )
    .unwrap();
    let cb = child.binding().clone();
    assert!(
        run_job(
            &cb,
            &mut f,
            &mut child,
            &mut Sink::default(),
            &AtomicBool::new(false),
            &mut |_| {}
        )
        .is_err()
    );
    let mut wrong_scope = scope;
    wrong_scope.inspected_plan = "foreign".into();
    assert!(
        AlbumJournal::open_for_recovery(
            &RecoveryScope {
                original_scope: wrong_scope,
                inspected_binding_sha256: b.sha256().unwrap()
            },
            "job",
            &s.bytes
        )
        .is_err()
    );
    let end = s.bytes.iter().position(|b| *b == b'\n').unwrap() + 1;
    s.bytes.insert(end, b'\n');
    assert!(
        AlbumJournal::open_for_recovery(
            &RecoveryScope {
                original_scope: binding().1,
                inspected_binding_sha256: b.sha256().unwrap()
            },
            "job",
            &s.bytes
        )
        .is_err()
    );
}
