//! Version 1 supplied-facts contracts. No file, metadata or model access is performed here.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u16 = 1;
pub const RULESET_ID: &str = "singles-v14.0";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub copy_mode: bool,
    pub main_folder_only: bool,
    pub singles_folder: bool,
    pub exist_only: bool,
    pub abc_sort: bool,
    pub duet_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            copy_mode: false,
            main_folder_only: false,
            singles_folder: true,
            exist_only: false,
            abc_sort: false,
            duet_mode: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepairMode {
    Hebrew,
    Jibrish,
    Auto,
}

impl RepairMode {
    /// Map legacy fixture modes without introducing new normalization or validation.
    pub fn from_legacy(mode: &str) -> Self {
        match mode {
            "heb" => Self::Hebrew,
            "jib" => Self::Jibrish,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Alias {
    pub source: String,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SongFacts {
    pub source_id: String,
    pub filename: String,
    pub metadata: MetadataFacts,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetadataFacts {
    Present { artist: String, title: String },
    ReadError { code: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecognitionFacts {
    #[serde(deserialize_with = "deserialize_empty_variant")]
    Disabled,
    Configured {
        bundle_id: String,
        health: RecognitionHealth,
        classifier: BTreeMap<String, ClassifierFact>,
        ner: BTreeMap<String, NerFact>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecognitionHealth {
    Healthy,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClassifierFact {
    #[serde(deserialize_with = "deserialize_empty_variant")]
    Accept,
    #[serde(deserialize_with = "deserialize_empty_variant")]
    Reject,
    #[serde(deserialize_with = "deserialize_empty_variant")]
    Unavailable,
    Error {
        code: String,
    },
}

// Serde's internally-tagged unit visitor discards extra fields, even with
// deny_unknown_fields on the enum. A strict empty object keeps v1 fail-closed.
fn deserialize_empty_variant<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EmptyPayload {}

    EmptyPayload::deserialize(deserializer).map(|_| ())
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NerFact {
    Artists(Vec<String>),
    Unavailable,
    Error { code: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyContext {
    pub ruleset_id: String,
    pub settings: Settings,
    pub aliases: Vec<Alias>,
    pub recognition: RecognitionFacts,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClassifyRequest {
    pub schema_version: u16,
    pub context: PolicyContext,
    pub song: SongFacts,
}

/// Facts were supplied by the caller, not measured or verified by a storage/model adapter.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Provenance {
    #[serde(rename = "supplied_facts")]
    ProvidedFacts,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub provenance: Provenance,
    pub artists: Option<Vec<String>>,
    pub selected_by: Option<DecisionSource>,
    pub trace: Vec<DecisionEvent>,
    pub warnings: Vec<Diagnostic>,
    pub apply_blockers: Vec<Diagnostic>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionSource {
    FilenameAlias,
    MetadataArtistAlias,
    MetadataArtistClassifier,
    TitleAlias,
    FilenameNer,
    TitleNer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DecisionEvent {
    AliasSelected {
        source: DecisionSource,
        index: usize,
    },
    ClassifierRequested {
        text: String,
    },
    NerRequested {
        text: String,
    },
    MetadataReadObserved {
        failed: bool,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetSnapshot {
    pub root_id: String,
    pub existing_directories: Vec<Vec<String>>,
    pub existing_files: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlanRequest {
    pub schema_version: u16,
    pub context: PolicyContext,
    pub songs: Vec<SongFacts>,
    pub target: TargetSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u16,
    pub provenance: Provenance,
    pub plan_id: String,
    pub request_hash: String,
    pub ruleset_id: String,
    pub items: Vec<PlannedSong>,
    pub warnings: Vec<Diagnostic>,
    pub executable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlannedSong {
    pub operation_id: String,
    pub source_id: String,
    pub decision: Decision,
    pub destinations: Vec<DestinationIntent>,
    pub copy_mode: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DestinationIntent {
    pub destination_id: String,
    pub components: Vec<String>,
    pub disposition: DestinationDisposition,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DestinationDisposition {
    Required,
    SkippedExistingOnly,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EngineError {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidInput,
    UnsupportedSchema,
    UnsupportedRuleset,
    UnsupportedCapability,
    MissingInferenceFact,
    InferenceError,
    InputTooLarge,
    UnsupportedTargetShape,
    StalePlan,
    JournalCorrupt,
    InvalidJournalTransition,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::UnsupportedSchema => "UNSUPPORTED_SCHEMA",
            Self::UnsupportedRuleset => "UNSUPPORTED_RULESET",
            Self::UnsupportedCapability => "UNSUPPORTED_CAPABILITY",
            Self::MissingInferenceFact => "MISSING_INFERENCE_FACT",
            Self::InferenceError => "INFERENCE_ERROR",
            Self::InputTooLarge => "INPUT_TOO_LARGE",
            Self::UnsupportedTargetShape => "UNSUPPORTED_TARGET_SHAPE",
            Self::StalePlan => "STALE_PLAN",
            Self::JournalCorrupt => "JOURNAL_CORRUPT",
            Self::InvalidJournalTransition => "INVALID_JOURNAL_TRANSITION",
        })
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EngineError {}

/// Validate the versioned envelope and supplied identities, without running recognition.
pub fn validate_classify_request(request: &ClassifyRequest) -> Result<(), EngineError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(EngineError {
            code: ErrorCode::UnsupportedSchema,
            message: format!(
                "schema_version {} is unsupported; expected {SCHEMA_VERSION}",
                request.schema_version
            ),
        });
    }
    if request.context.ruleset_id != RULESET_ID {
        return Err(EngineError {
            code: ErrorCode::UnsupportedRuleset,
            message: format!(
                "ruleset_id {:?} is unsupported; expected {RULESET_ID}",
                request.context.ruleset_id
            ),
        });
    }
    if request.song.source_id.is_empty() || request.song.filename.is_empty() {
        return Err(EngineError {
            code: ErrorCode::InvalidInput,
            message: "source_id and filename must be non-empty supplied strings".into(),
        });
    }
    if let RecognitionFacts::Configured { bundle_id, .. } = &request.context.recognition
        && bundle_id.is_empty()
    {
        return Err(EngineError {
            code: ErrorCode::InvalidInput,
            message: "configured recognition requires a non-empty bundle_id".into(),
        });
    }
    Ok(())
}
