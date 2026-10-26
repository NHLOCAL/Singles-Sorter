#![allow(dead_code)]
use sorter_core::{ClassifierFact, Settings, discovery::*, job_plan::*, metadata::*};
use sorter_executor::{ExecutionError as E, Result, binding::*, digest, physical_journal::*};
use std::io::Cursor;
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
    fn classify(&mut self, _: &str) -> std::result::Result<ClassifierFact, RecognitionError> {
        if self.fail {
            Err("failed".into())
        } else {
            Ok(ClassifierFact::Accept)
        }
    }
    fn ner(&mut self, _: &str) -> std::result::Result<Vec<String>, RecognitionError> {
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
pub fn binding(duet: bool) -> ExecutionBinding {
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
pub struct Sink {
    pub bytes: Vec<u8>,
    durable: Vec<u8>,
    pub boundary: usize,
    pub fail: Option<usize>,
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

pub fn binding_with(duet: bool, copy: bool, bytes: &[u8]) -> ExecutionBinding {
    let (mut plan, mut storage, resources, mut scope) = fixture(duet);
    storage.bytes = bytes.to_vec();
    storage.entries[2].size = Some(bytes.len() as u64);
    plan.request.settings.copy_mode = copy;
    plan = plan_job(
        &plan.request,
        &mut storage,
        &mut Metadata,
        &mut Ai { duet, fail: false },
    )
    .unwrap();
    scope.inspected_plan = plan.binding_sha256.clone();
    scope.inspected_sources = vec![SourceEvidence {
        observation: storage.entries[2].clone(),
        sha256: digest(bytes),
    }];
    scope.allowed_modes = vec![if copy {
        TransferMode::Copy
    } else {
        TransferMode::Move
    }];
    validate_execution(
        &plan,
        &mut storage,
        &mut Metadata,
        &mut Ai { duet, fail: false },
        &resources,
        &scope,
    )
    .unwrap()
}

pub fn recovery_scope(duet: bool) -> RecoveryScope {
    RecoveryScope {
        original_scope: fixture(duet).3,
        inspected_binding_sha256: binding(duet).sha256().unwrap(),
    }
}
