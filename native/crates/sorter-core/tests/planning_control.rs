//! Cancellation is job control, not released classifier/scan-item policy.
use serde_json::json;
use sorter_core::{ClassifierFact, Settings, discovery::*, job_plan::*};
use std::{cell::RefCell, collections::HashMap};
#[allow(dead_code)]
#[path = "support/album_fixture.rs"]
mod support;
use support::{Provider, entry};

#[derive(Clone)]
struct Storage(Vec<EntryObservation>);
fn key(reference: &StorageRef) -> &str {
    let StorageRef::TestOpaque { id } = reference else {
        panic!()
    };
    id
}
impl ReadStorage for Storage {
    fn stat(&mut self, reference: &StorageRef) -> Result<EntryObservation, StorageError> {
        Ok(self
            .0
            .iter()
            .find(|e| &e.reference == reference)
            .unwrap()
            .clone())
    }
    fn children(&mut self, reference: &StorageRef) -> Result<Vec<EntryObservation>, StorageError> {
        Ok(self
            .0
            .iter()
            .filter(|e| {
                key(&e.reference)
                    .rsplit_once('/')
                    .is_some_and(|(p, _)| p == key(reference))
            })
            .cloned()
            .collect())
    }
    fn open_read(&mut self, _: &StorageRef) -> Result<ReadInput, StorageError> {
        panic!("No audio reads")
    }
}
fn profile() -> AiProfile {
    AiProfile::Configured {
        bundle_id: "test".into(),
        available: true,
    }
}
fn fixture() -> (JobRequestV2, Storage, Provider) {
    let entries = [
        ("source", EntryKind::Directory),
        ("target", EntryKind::Directory),
        ("source/good.mp3", EntryKind::File),
        ("source/bad.mp3", EntryKind::File),
        ("source/skipped.mp3", EntryKind::File),
        ("source/later", EntryKind::Directory),
        ("source/later/after.mp3", EntryKind::File),
    ]
    .into_iter()
    .map(|(p, k)| entry(p, k))
    .collect();
    (
        JobRequestV2 {
            mode: JobMode::Sort,
            source: StorageRef::TestOpaque {
                id: "source".into(),
            },
            target: Some(StorageRef::TestOpaque {
                id: "target".into(),
            }),
            settings: Settings::default(),
            alias_rows: vec![],
            resource_ids: vec![],
            adapter_versions: vec![],
            ai_profile: profile(),
        },
        Storage(entries),
        Provider(HashMap::new()),
    )
}
struct Ai {
    error: Option<RecognitionError>,
    classify: bool,
    calls: Vec<String>,
    cancel: Option<CancellationToken>,
}
impl Ai {
    fn new(error: Option<RecognitionError>) -> Self {
        Self {
            error,
            classify: false,
            calls: vec![],
            cancel: None,
        }
    }
}
impl RecognitionProvider for Ai {
    fn profile(&self) -> AiProfile {
        profile()
    }
    fn classify(&mut self, text: &str) -> Result<ClassifierFact, RecognitionError> {
        self.calls.push(format!("classify:{text}"));
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        match &self.error {
            Some(e) => Err(e.clone()),
            None => Ok(ClassifierFact::Accept),
        }
    }
    fn ner(&mut self, text: &str) -> Result<Vec<String>, RecognitionError> {
        self.calls.push(text.into());
        if let Some(token) = &self.cancel {
            token.cancel();
        }
        if !self.classify
            && text == "bad"
            && let Some(e) = &self.error
        {
            return Err(e.clone());
        }
        Ok(vec!["זמר".into()])
    }
}
fn abort(kind: AbortKind) -> PlanningAbort {
    PlanningAbort {
        kind,
        detail: "injected abort".into(),
    }
}
fn code(kind: AbortKind) -> &'static str {
    match kind {
        AbortKind::Cancelled => "PLANNING_CANCELLED",
        AbortKind::ResourceLimit => "PLANNING_RESOURCE_LIMIT",
        AbortKind::Allocation => "PLANNING_ALLOCATION",
    }
}
fn ordinary() -> [RecognitionError; 2] {
    [
        RecognitionError::Model("model failed".into()),
        RecognitionError::Legacy("legacy failed".into()),
    ]
}

#[test]
fn ordinary_classifier_errors_still_fail_open_with_original_artist() {
    for error in ordinary() {
        let (request, mut storage, mut metadata) = fixture();
        for e in &storage.0 {
            if e.kind == EntryKind::File {
                metadata
                    .0
                    .insert(key(&e.reference).into(), json!({"artist":"זמר מוכר"}));
            }
        }
        let mut ai = Ai::new(Some(error));
        ai.classify = true;
        let plan = plan_job(&request, &mut storage, &mut metadata, &mut ai).unwrap();
        assert_eq!(plan.operations.len(), 4);
        assert!(
            plan.decisions
                .iter()
                .all(|d| !matches!(d, DecisionV2::ItemError { .. }))
        );
        assert!(
            plan.decisions
                .iter()
                .filter_map(|d| match d {
                    DecisionV2::Single { artists, .. } => Some(artists),
                    _ => None,
                })
                .all(|a| a == &["זמר מוכר"])
        );
        assert_eq!(ai.calls.len(), 4);
        assert!(ai.calls.iter().all(|s| s.starts_with("classify:")));
    }
}
#[test]
fn ordinary_ner_errors_skip_rest_of_scan_item_but_not_later_visit() {
    for error in ordinary() {
        let (request, mut storage, mut metadata) = fixture();
        let message = error.to_string();
        let mut ai = Ai::new(Some(error));
        let plan = plan_job(&request, &mut storage, &mut metadata, &mut ai).unwrap();
        assert_eq!(ai.calls, ["good", "bad", "after"]);
        assert_eq!(plan.operations.len(), 2);
        assert_eq!(
            plan.decisions
                .iter()
                .filter(|d| matches!(d, DecisionV2::ItemError { error, .. } if error == &message))
                .count(),
            1
        );
        verify_job_plan_binding(&plan).unwrap();
    }
}
#[test]
fn every_typed_provider_abort_discards_previously_recognized_files() {
    for kind in [
        AbortKind::Cancelled,
        AbortKind::ResourceLimit,
        AbortKind::Allocation,
    ] {
        for classifier in [false, true] {
            let (request, mut storage, mut metadata) = fixture();
            if classifier {
                metadata
                    .0
                    .insert("source/bad.mp3".into(), json!({"artist":"זמר מוכר"}));
            }
            let mut ai = Ai::new(Some(RecognitionError::Abort(abort(kind))));
            let error = plan_job(&request, &mut storage, &mut metadata, &mut ai).unwrap_err();
            assert_eq!(error.code, code(kind));
            assert_eq!(error.detail, "injected abort");
            assert_eq!(ai.calls.len(), 2, "Abort must prevent later recognition");
        }
    }
}
struct TraceControl {
    seen: RefCell<Vec<PlanningBoundary>>,
    fail: Option<(usize, AbortKind)>,
}
impl PlanningControl for TraceControl {
    fn checkpoint(&self, stage: PlanningBoundary) -> Result<(), PlanningAbort> {
        let mut seen = self.seen.borrow_mut();
        let index = seen.len();
        seen.push(stage);
        if let Some((target, kind)) = self.fail
            && target == index
        {
            Err(abort(kind))
        } else {
            Ok(())
        }
    }
}
#[test]
fn abort_at_every_observed_boundary_returns_no_plan_and_retry_has_no_stale_state() {
    for mode in [JobMode::Sort, JobMode::FixNames] {
        let (mut request, storage, _) = fixture();
        request.mode = mode;
        let trace = TraceControl {
            seen: RefCell::new(vec![]),
            fail: None,
        };
        let baseline = plan_job_controlled(
            &request,
            &mut storage.clone(),
            &mut Provider(HashMap::new()),
            &mut Ai::new(None),
            &trace,
        )
        .unwrap();
        let stages = trace.seen.into_inner();
        for expected in [
            PlanningBoundary::Start,
            PlanningBoundary::Collection,
            PlanningBoundary::Visit,
            PlanningBoundary::File,
            PlanningBoundary::Binding,
            PlanningBoundary::Result,
        ] {
            assert!(stages.contains(&expected), "missing {expected:?}");
        }
        if mode == JobMode::Sort {
            for expected in [
                PlanningBoundary::Recognition,
                PlanningBoundary::RecognitionAcceptance,
                PlanningBoundary::DeferredPlanning,
            ] {
                assert!(stages.contains(&expected));
            }
        }
        for (index, stage) in stages.iter().enumerate() {
            for kind in [
                AbortKind::Cancelled,
                AbortKind::ResourceLimit,
                AbortKind::Allocation,
            ] {
                let mut storage = storage.clone();
                let mut metadata = Provider(HashMap::new());
                let mut ai = Ai::new(None);
                let control = TraceControl {
                    seen: RefCell::new(vec![]),
                    fail: Some((index, kind)),
                };
                let error =
                    plan_job_controlled(&request, &mut storage, &mut metadata, &mut ai, &control)
                        .unwrap_err();
                assert_eq!(error.code, code(kind), "{stage:?} at {index}");
                assert_eq!(control.seen.borrow().len(), index + 1);
                let retried = plan_job_controlled(
                    &request,
                    &mut storage,
                    &mut metadata,
                    &mut ai,
                    &CancellationToken::new(),
                )
                .unwrap();
                assert_eq!(
                    serde_json::to_value(&retried).unwrap(),
                    serde_json::to_value(&baseline).unwrap()
                );
                verify_job_plan_binding(&retried).unwrap();
            }
        }
    }
}
#[test]
fn cancelled_tokens_remain_cancelled_and_late_results_are_never_accepted() {
    for classifier in [false, true] {
        for error in [
            None,
            Some(RecognitionError::Model(
                "ordinary failure after cancellation".into(),
            )),
        ] {
            let (request, mut storage, mut metadata) = fixture();
            if classifier {
                metadata
                    .0
                    .insert("source/good.mp3".into(), json!({"artist":"זמר מוכר"}));
            }
            let token = CancellationToken::new();
            let mut ai = Ai::new(error);
            ai.cancel = Some(token.clone());
            assert_eq!(
                plan_job_controlled(&request, &mut storage, &mut metadata, &mut ai, &token)
                    .unwrap_err()
                    .code,
                "PLANNING_CANCELLED"
            );
            assert_eq!(ai.calls.len(), 1);
            ai.cancel = None;
            ai.error = None;
            assert_eq!(
                plan_job_controlled(&request, &mut storage, &mut metadata, &mut ai, &token)
                    .unwrap_err()
                    .code,
                "PLANNING_CANCELLED"
            );
            assert_eq!(ai.calls.len(), 1);
            assert!(
                plan_job_controlled(
                    &request,
                    &mut storage,
                    &mut metadata,
                    &mut ai,
                    &CancellationToken::new()
                )
                .is_ok()
            );
        }
    }
}

#[test]
fn album_metadata_and_recursive_predicted_effects_abort_without_partial_plan() {
    for main_folder_only in [false, true] {
        for copy_mode in [false, true] {
            let (mut request, mut storage, _) = fixture();
            request.settings.main_folder_only = main_folder_only;
            request.settings.copy_mode = copy_mode;
            // Place an album under the source so both collection modes visit it.
            storage.0.retain(|e| !key(&e.reference).contains(".mp3"));
            storage.0.extend(
                [
                    ("source/later/one.mp3", EntryKind::File),
                    ("source/later/two.mp3", EntryKind::File),
                    ("source/later/three.mp3", EntryKind::File),
                    ("source/later/art", EntryKind::Directory),
                    ("source/later/art/cover.jpg", EntryKind::File),
                ]
                .into_iter()
                .map(|(p, k)| entry(p, k)),
            );
            let tags: HashMap<_, _> = ["one", "two", "three"]
                .into_iter()
                .map(|name| {
                    (
                        format!("source/later/{name}.mp3"),
                        json!({"album":"אלבום", "artist":"זמר מוכר"}),
                    )
                })
                .collect();
            let trace = TraceControl {
                seen: RefCell::new(vec![]),
                fail: None,
            };
            let baseline = plan_job_controlled(
                &request,
                &mut storage.clone(),
                &mut Provider(tags.clone()),
                &mut Ai::new(None),
                &trace,
            )
            .unwrap();
            assert!(
                baseline
                    .operations
                    .iter()
                    .any(|o| matches!(o.kind, OperationKind::AlbumTransfer { .. }))
            );
            for index in 0..trace.seen.borrow().len() {
                let control = TraceControl {
                    seen: RefCell::new(vec![]),
                    fail: Some((index, AbortKind::Cancelled)),
                };
                let error = plan_job_controlled(
                    &request,
                    &mut storage.clone(),
                    &mut Provider(tags.clone()),
                    &mut Ai::new(None),
                    &control,
                )
                .unwrap_err();
                assert_eq!(error.code, "PLANNING_CANCELLED");
                assert_eq!(control.seen.borrow().len(), index + 1);
            }
            let retry = plan_job_controlled(
                &request,
                &mut storage,
                &mut Provider(tags),
                &mut Ai::new(None),
                &CancellationToken::new(),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(retry).unwrap(),
                serde_json::to_value(baseline).unwrap()
            );
        }
    }
}
#[test]
fn provider_diagnostics_are_integrity_bound() {
    struct DiagnosticAi(Ai);
    impl RecognitionProvider for DiagnosticAi {
        fn profile(&self) -> AiProfile {
            self.0.profile()
        }
        fn classify(&mut self, s: &str) -> Result<ClassifierFact, RecognitionError> {
            self.0.classify(s)
        }
        fn ner(&mut self, s: &str) -> Result<Vec<String>, RecognitionError> {
            self.0.ner(s)
        }
        fn diagnostics(&self) -> Vec<String> {
            vec!["ordinary model diagnostic".into()]
        }
    }
    let (request, mut storage, mut metadata) = fixture();
    let mut plan = plan_job(
        &request,
        &mut storage,
        &mut metadata,
        &mut DiagnosticAi(Ai::new(None)),
    )
    .unwrap();
    assert_eq!(plan.warnings.last().unwrap(), "ordinary model diagnostic");
    verify_job_plan_binding(&plan).unwrap();
    plan.warnings.pop();
    assert_eq!(
        verify_job_plan_binding(&plan).unwrap_err().code,
        "JOB_BINDING_MISMATCH"
    );
}
