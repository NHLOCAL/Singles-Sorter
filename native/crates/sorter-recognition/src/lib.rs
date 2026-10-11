//! Explicit, original-only read-only recognition adapter. No downloads or pickle.
//! Models and tokenizer stay on the loading/calling thread; no cache or batching.
use released_ner_text::{
    assets::ReleasedAssets,
    inference::{CancellationToken, InferenceError, Limits as NerLimits},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sorter_classifier::{ClassifierError, Model};
use sorter_core::{
    ClassifierFact,
    job_plan::{
        AbortKind, AiProfile, PlanningAbort, PlanningBoundary, PlanningControl, RecognitionError,
        RecognitionProvider,
    },
};
use std::{path::Path, sync::Arc};

pub const BUNDLE_ID: &str = "original-v14-ner-3.7.5";
pub const RUNTIME_ID: &str =
    "original-native-readonly-v1:ner-bounded-f32:secondary-bounded-f64:pcre2-10.46-static32-no-jit";
const NER_TENSORS: &str = "84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745";
const NER_TOKENIZER: &str = "33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_inference_calls: usize,
    pub ner_input_bytes: usize,
    pub ner_input_chars: usize,
    pub ner_tokens: usize,
    pub ner_work_units: usize,
    pub ner_tensor_bytes: usize,
    pub classifier_input_bytes: usize,
    pub classifier_input_chars: usize,
    pub classifier_work: usize,
    pub classifier_allocation_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        let n = NerLimits::default();
        let c = sorter_classifier::Limits::default();
        Self {
            max_inference_calls: 10_000,
            ner_input_bytes: n.input_bytes,
            ner_input_chars: n.input_chars,
            ner_tokens: n.tokens,
            ner_work_units: n.work_units,
            ner_tensor_bytes: n.tensor_bytes,
            classifier_input_bytes: c.max_input_bytes,
            classifier_input_chars: c.max_input_chars,
            classifier_work: c.max_work,
            classifier_allocation_bytes: c.max_allocation_bytes,
        }
    }
}
impl Limits {
    fn ner(&self) -> NerLimits {
        NerLimits {
            input_bytes: self.ner_input_bytes,
            input_chars: self.ner_input_chars,
            tokens: self.ner_tokens,
            work_units: self.ner_work_units,
            tensor_bytes: self.ner_tensor_bytes,
            ..NerLimits::default()
        }
    }
    fn classifier(&self) -> sorter_classifier::Limits {
        sorter_classifier::Limits {
            max_input_bytes: self.classifier_input_bytes,
            max_input_chars: self.classifier_input_chars,
            max_work: self.classifier_work,
            max_allocation_bytes: self.classifier_allocation_bytes,
        }
    }
    pub fn validate(&self) -> Result<(), RecognitionError> {
        let max = Self::default();
        macro_rules! bounded { ($($f:ident),*) => { $(if self.$f>max.$f { return Err(abort(AbortKind::ResourceLimit, concat!("Invalid limit: ",stringify!($f)))); })* }; }
        bounded!(
            max_inference_calls,
            ner_input_bytes,
            ner_input_chars,
            ner_tokens,
            ner_work_units,
            ner_tensor_bytes,
            classifier_input_bytes,
            classifier_input_chars,
            classifier_work,
            classifier_allocation_bytes
        );
        Ok(())
    }
}
/// One monotonic signal reaches the planner, NER and secondary analyzer.
#[derive(Clone, Default)]
pub struct JobCancellation(Arc<CancellationToken>);
impl JobCancellation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.cancel()
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}
impl PlanningControl for JobCancellation {
    fn checkpoint(&self, _: PlanningBoundary) -> Result<(), PlanningAbort> {
        if self.is_cancelled() {
            Err(PlanningAbort::cancelled())
        } else {
            Ok(())
        }
    }
}
fn abort(kind: AbortKind, detail: impl Into<String>) -> RecognitionError {
    RecognitionError::Abort(PlanningAbort {
        kind,
        detail: detail.into(),
    })
}
fn ner_error(e: InferenceError) -> RecognitionError {
    match e {
        InferenceError::Cancelled => abort(AbortKind::Cancelled, "Recognition cancelled"),
        InferenceError::ResourceLimit { resource, limit } => abort(
            AbortKind::ResourceLimit,
            format!("NER {resource} exceeds {limit}"),
        ),
        InferenceError::Allocation { resource } => abort(
            AbortKind::Allocation,
            format!("NER allocation failed: {resource}"),
        ),
        InferenceError::Model(s) => RecognitionError::Model(s),
    }
}
fn classifier_error(e: ClassifierError) -> RecognitionError {
    match e {
        ClassifierError::Cancelled => abort(AbortKind::Cancelled, "Recognition cancelled"),
        ClassifierError::ResourceLimit(s) => {
            abort(AbortKind::ResourceLimit, format!("Classifier limit: {s}"))
        }
        ClassifierError::Allocation => abort(AbortKind::Allocation, "Classifier allocation failed"),
        ClassifierError::Model(e) => RecognitionError::Model(format!("Original classifier: {e:?}")),
    }
}

pub struct OriginalRecognition {
    ner: ReleasedAssets,
    classifier: Model,
    limits: Limits,
    cancellation: JobCancellation,
    calls: usize,
    executed: bool,
    diagnostics: Vec<String>,
}
impl OriginalRecognition {
    pub fn load(
        directory: &Path,
        bundle_id: &str,
        limits: Limits,
        cancellation: JobCancellation,
    ) -> Result<Self, RecognitionError> {
        if bundle_id != BUNDLE_ID || !directory.is_absolute() {
            return Err(RecognitionError::Model(
                "Unsupported original bundle profile".into(),
            ));
        }
        limits.validate()?;
        cancellation.checkpoint(PlanningBoundary::Start)?;
        let ner = ReleasedAssets::load(directory).map_err(|e| match e {
            released_ner_probe::LoadError::Allocation { .. } => {
                abort(AbortKind::Allocation, "NER asset allocation failed")
            }
            released_ner_probe::LoadError::TooLarge { .. } => {
                abort(AbortKind::ResourceLimit, "NER asset size limit exceeded")
            }
            _ => RecognitionError::Model("Original NER bundle validation failed".into()),
        })?;
        cancellation.checkpoint(PlanningBoundary::Start)?;
        let classifier = Model::load_directory(directory, &|| cancellation.is_cancelled())
            .map_err(classifier_error)?;
        cancellation.checkpoint(PlanningBoundary::Start)?;
        Ok(Self {
            ner,
            classifier,
            limits,
            cancellation,
            calls: 0,
            executed: false,
            diagnostics: vec![],
        })
    }
    pub fn inference_executed(&self) -> bool {
        self.executed
    }
    pub fn inference_calls(&self) -> usize {
        self.calls
    }
    pub fn resource_ids(&self) -> Vec<String> {
        vec![
            format!("original-ner-tensors:sha256:{NER_TENSORS}"),
            format!("original-ner-tokenizer:sha256:{NER_TOKENIZER}"),
            format!(
                "original-secondary-weights:sha256:{}",
                sorter_classifier::WEIGHTS_SHA256
            ),
            format!(
                "original-secondary-vocabulary:sha256:{}",
                sorter_classifier::VOCAB_SHA256
            ),
        ]
    }
    pub fn adapter_versions(&self) -> Vec<String> {
        // Includes fixed PCRE2 budgets, not only the caller-tightened limits.
        let n = self.limits.ner();
        let limits = serde_json::to_vec(&self.limits).expect("integer-only limits");
        vec![
            RUNTIME_ID.into(),
            format!(
                "native-runtime-build:{}:{}:pcre2-archive-sha256:{}",
                env!("RECOGNITION_TARGET"),
                env!("RECOGNITION_PROFILE"),
                env!("RECOGNITION_PCRE2_SHA256")
            ),
            "original-model-source:78d001aee5ce49628bdffebd34540fa83ba32c12".into(),
            format!("recognition-limits:sha256:{:x}", Sha256::digest(limits)),
            format!(
                "ner-regex-limits:{}:{}:{}:{}:{}",
                n.regex_match, n.regex_depth, n.regex_heap_kib, n.regex_calls, n.regex_callouts
            ),
        ]
    }
    fn begin(&mut self) -> Result<(), RecognitionError> {
        self.cancellation
            .checkpoint(PlanningBoundary::Recognition)?;
        if self.calls >= self.limits.max_inference_calls {
            return Err(abort(
                AbortKind::ResourceLimit,
                "Job inference-call budget exceeded",
            ));
        }
        self.calls += 1;
        self.executed = true;
        Ok(())
    }
    fn diagnostic(&mut self, scope: &str) {
        // Bound reporting memory; aggregate repeated ordinary failures.
        let message = format!("ORIGINAL_CLASSIFIER_FAIL_OPEN:{scope}");
        if !self.diagnostics.contains(&message) {
            self.diagnostics.push(message)
        }
    }
}
impl RecognitionProvider for OriginalRecognition {
    fn profile(&self) -> AiProfile {
        AiProfile::Configured {
            bundle_id: BUNDLE_ID.into(),
            available: true,
        }
    }
    fn classify(&mut self, text: &str) -> Result<ClassifierFact, RecognitionError> {
        self.begin()?;
        let result = self
            .classifier
            .predict(text, self.limits.classifier(), &|| {
                self.cancellation.is_cancelled()
            })
            .map_err(classifier_error);
        self.cancellation
            .checkpoint(PlanningBoundary::RecognitionAcceptance)?;
        match result {
            Ok(p) => Ok(if p.class == 0 {
                ClassifierFact::Accept
            } else {
                ClassifierFact::Reject
            }),
            Err(e) => {
                if matches!(e, RecognitionError::Model(_)) {
                    self.diagnostic("metadata_or_span")
                }
                Err(e)
            }
        }
    }
    fn ner(&mut self, text: &str) -> Result<Vec<String>, RecognitionError> {
        self.begin()?;
        let result = self
            .ner
            .infer(text, &self.limits.ner(), &self.cancellation.0)
            .map_err(ner_error)?;
        self.cancellation
            .checkpoint(PlanningBoundary::RecognitionAcceptance)?;
        let cancellation = self.cancellation.clone();
        filter_singers(result.spans, |text| self.classify(text), &cancellation)
    }
    fn diagnostics(&self) -> Vec<String> {
        self.diagnostics.clone()
    }
    fn inference_executed(&self) -> bool {
        self.executed
    }
}

fn filter_singers(
    spans: Vec<released_ner_probe::Span>,
    mut classify: impl FnMut(&str) -> Result<ClassifierFact, RecognitionError>,
    cancellation: &JobCancellation,
) -> Result<Vec<String>, RecognitionError> {
    let mut artists = Vec::new();
    artists
        .try_reserve_exact(spans.len())
        .map_err(|_| abort(AbortKind::Allocation, "Singer output allocation failed"))?;
    for span in spans {
        cancellation.checkpoint(PlanningBoundary::Recognition)?;
        if span.label != "SINGER" {
            continue;
        }
        match classify(&span.text) {
            Ok(ClassifierFact::Reject) => {}
            Err(RecognitionError::Abort(e)) => return Err(RecognitionError::Abort(e)),
            _ => artists.push(span.text),
        }
        cancellation.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
    }
    cancellation.checkpoint(PlanningBoundary::RecognitionAcceptance)?;
    Ok(artists)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span(label: &str, text: &str) -> released_ner_probe::Span {
        released_ner_probe::Span {
            start: 0,
            end: text.chars().count(),
            label: label.into(),
            text: text.into(),
        }
    }
    #[test]
    fn singer_filter_preserves_order_duplicates_and_ordinary_fail_open() {
        let spans = vec![
            span("SONG", "skip"),
            span("SINGER", "keep"),
            span("SINGER", "reject"),
            span("SINGER", "error"),
            span("SINGER", "keep"),
        ];
        let mut calls = vec![];
        let result = filter_singers(
            spans,
            |text| {
                calls.push(text.to_owned());
                match text {
                    "reject" => Ok(ClassifierFact::Reject),
                    "error" => Err(RecognitionError::Model("ordinary".into())),
                    _ => Ok(ClassifierFact::Accept),
                }
            },
            &JobCancellation::new(),
        )
        .unwrap();
        assert_eq!(calls, ["keep", "reject", "error", "keep"]);
        assert_eq!(result, ["keep", "error", "keep"]);
    }
    #[test]
    fn abort_discards_prior_singers_and_cancellation_after_classifier_is_fatal() {
        let spans = vec![span("SINGER", "first"), span("SINGER", "abort")];
        assert!(matches!(
            filter_singers(
                spans,
                |text| if text == "abort" {
                    Err(abort(AbortKind::ResourceLimit, "bounded"))
                } else {
                    Ok(ClassifierFact::Accept)
                },
                &JobCancellation::new()
            ),
            Err(RecognitionError::Abort(_))
        ));
        let token = JobCancellation::new();
        let t = token.clone();
        assert!(matches!(
            filter_singers(
                vec![span("SINGER", "late")],
                |_| {
                    t.cancel();
                    Ok(ClassifierFact::Accept)
                },
                &token
            ),
            Err(RecognitionError::Abort(_))
        ));
    }
    #[test]
    fn limits_only_tighten() {
        let mut l = Limits::default();
        l.max_inference_calls += 1;
        assert!(matches!(l.validate(), Err(RecognitionError::Abort(_))));
        l.max_inference_calls = 0;
        assert!(l.validate().is_ok());
    }
    #[test]
    fn cancellation_is_monotonic_and_shared() {
        let t = JobCancellation::new();
        let b = t.clone();
        b.cancel();
        assert!(t.checkpoint(PlanningBoundary::Result).is_err());
        assert!(b.is_cancelled());
        assert!(!JobCancellation::new().is_cancelled());
    }
    #[test]
    fn error_kinds_cannot_become_fail_open() {
        assert!(matches!(
            classifier_error(ClassifierError::ResourceLimit("test")),
            RecognitionError::Abort(_)
        ));
        assert!(matches!(
            ner_error(InferenceError::Cancelled),
            RecognitionError::Abort(_)
        ));
    }
}
