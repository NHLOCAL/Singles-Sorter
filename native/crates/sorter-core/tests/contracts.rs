mod common;

use std::collections::BTreeMap;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sorter_core::contract::*;
use sorter_core::validate_classify_request;

fn request() -> ClassifyRequest {
    ClassifyRequest {
        schema_version: 1,
        context: PolicyContext {
            ruleset_id: "singles-v14.0".into(),
            settings: Settings::default(),
            aliases: vec![
                Alias {
                    source: "אמן קצר".into(),
                    target: "ראשון".into(),
                },
                Alias {
                    source: "אמן קצר ארוך".into(),
                    target: "שני".into(),
                },
                Alias {
                    source: "אמן קצר".into(),
                    target: "ראשון".into(),
                },
            ],
            recognition: RecognitionFacts::Disabled,
        },
        song: SongFacts {
            source_id: "song-1".into(),
            filename: "אמן קצר.mp3".into(),
            metadata: MetadataFacts::Present {
                artist: "אמן".into(),
                title: "שיר".into(),
            },
        },
    }
}

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let bytes = serde_json::to_vec(value).unwrap();
    assert_eq!(&serde_json::from_slice::<T>(&bytes).unwrap(), value);
}

#[test]
fn settings_preserve_all_six_release_defaults() {
    let settings = Settings::default();
    assert!(!settings.copy_mode);
    assert!(!settings.main_folder_only);
    assert!(settings.singles_folder);
    assert!(!settings.exist_only);
    assert!(!settings.abc_sort);
    assert!(!settings.duet_mode);
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        json!({
            "copy_mode": false, "main_folder_only": false, "singles_folder": true,
            "exist_only": false, "abc_sort": false, "duet_mode": false
        })
    );
}

#[test]
fn partial_settings_use_defaults_without_overriding_explicit_false() {
    assert_eq!(
        serde_json::from_value::<Settings>(json!({})).unwrap(),
        Settings::default()
    );
    let settings: Settings = serde_json::from_value(json!({"singles_folder": false})).unwrap();
    assert!(!settings.singles_folder);
    assert!(!settings.copy_mode);
}

#[test]
fn request_round_trip_preserves_alias_order_duplicates_and_exact_text_keys() {
    let mut value = request();
    value.context.recognition = RecognitionFacts::Configured {
        bundle_id: "frozen-model".into(),
        health: RecognitionHealth::Healthy,
        classifier: BTreeMap::from([
            ("אמן ".into(), ClassifierFact::Accept),
            ("אמן".into(), ClassifierFact::Reject),
        ]),
        ner: BTreeMap::from([(
            "כותרת מקורית".into(),
            NerFact::Artists(vec!["שני".into(), "ראשון".into(), "שני".into()]),
        )]),
    };
    round_trip(&value);
    let serialized = serde_json::to_value(&value).unwrap();
    assert_eq!(
        serialized["context"]["aliases"][0],
        serialized["context"]["aliases"][2]
    );
    assert_eq!(
        serialized["context"]["recognition"]["ner"]["כותרת מקורית"],
        json!({
            "kind": "artists", "value": ["שני", "ראשון", "שני"]
        })
    );
}

#[test]
fn schema_one_is_valid_but_other_schema_versions_are_rejected() {
    let mut value = request();
    assert!(validate_classify_request(&value).is_ok());
    for schema_version in [0, 2, u16::MAX] {
        value.schema_version = schema_version;
        assert_eq!(
            validate_classify_request(&value).unwrap_err().code,
            ErrorCode::UnsupportedSchema
        );
    }
}

#[test]
fn unknown_ruleset_is_rejected_without_falling_back() {
    let mut value = request();
    value.context.ruleset_id = "singles-dev".into();
    assert_eq!(
        validate_classify_request(&value).unwrap_err().code,
        ErrorCode::UnsupportedRuleset
    );
}

#[test]
fn disabled_and_failed_configured_recognition_remain_distinct_valid_facts() {
    let disabled = request();
    let mut failed = request();
    failed.context.recognition = RecognitionFacts::Configured {
        bundle_id: "required-bundle".into(),
        health: RecognitionHealth::Failed,
        classifier: BTreeMap::from([("אמן".into(), ClassifierFact::Unavailable)]),
        ner: BTreeMap::from([(
            "שיר".into(),
            NerFact::Error {
                code: "MODEL_RUNTIME_FAILURE".into(),
            },
        )]),
    };
    assert!(validate_classify_request(&disabled).is_ok());
    assert!(validate_classify_request(&failed).is_ok());
    assert_ne!(disabled.context.recognition, failed.context.recognition);
    assert_eq!(
        serde_json::to_value(&disabled.context.recognition).unwrap(),
        json!({"kind": "disabled"})
    );
    assert_eq!(
        serde_json::to_value(&failed.context.recognition).unwrap()["health"],
        "failed"
    );
    round_trip(&failed);
}

#[test]
fn input_validation_rejects_empty_song_identity_filename_and_configured_bundle_id() {
    let mut value = request();
    value.song.source_id.clear();
    assert_eq!(
        validate_classify_request(&value).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    value = request();
    value.song.filename.clear();
    assert_eq!(
        validate_classify_request(&value).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    value = request();
    value.context.recognition = RecognitionFacts::Configured {
        bundle_id: String::new(),
        health: RecognitionHealth::Failed,
        classifier: BTreeMap::new(),
        ner: BTreeMap::new(),
    };
    assert_eq!(
        validate_classify_request(&value).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn unknown_fields_are_rejected_at_every_request_level() {
    let original = serde_json::to_value(request()).unwrap();
    for pointer in [
        "",
        "/context",
        "/context/settings",
        "/context/aliases/0",
        "/context/recognition",
        "/song",
        "/song/metadata",
    ] {
        let mut value = original.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future_option".into(), json!(true));
        assert!(
            serde_json::from_value::<ClassifyRequest>(value).is_err(),
            "ignored field at {pointer}"
        );
    }
    let plan = json!({
        "schema_version": 1, "context": original["context"], "songs": [original["song"]],
        "target": {"root_id": "target-1", "existing_directories": [], "existing_files": []}
    });
    for pointer in ["", "/target", "/songs/0"] {
        let mut value = plan.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("album".into(), json!(true));
        assert!(
            serde_json::from_value::<PlanRequest>(value).is_err(),
            "ignored field at {pointer}"
        );
    }
}

#[test]
fn unknown_fields_and_tags_are_rejected_in_recognition_payloads() {
    let cases = [
        json!({"kind": "disabled", "bundle_id": "silent-fallback"}),
        json!({"kind": "configured", "bundle_id": "bundle", "health": "healthy", "classifier": {}, "ner": {}, "future_option": true}),
        json!({"kind": "configured", "bundle_id": "bundle", "health": "healthy", "classifier": {"a": {"kind": "accept", "code": "unexpected"}}, "ner": {}}),
        json!({"kind": "configured", "bundle_id": "bundle", "health": "healthy", "classifier": {"a": {"kind": "error", "code": "known", "retry": true}}, "ner": {}}),
        json!({"kind": "configured", "bundle_id": "bundle", "health": "healthy", "classifier": {}, "ner": {"a": {"kind": "artists", "value": ["a"], "score": 1}}}),
        json!({"kind": "configured", "bundle_id": "bundle", "health": "healthy", "classifier": {}, "ner": {"a": {"kind": "error", "value": {"code": "known", "retry": true}}}}),
        json!({"kind": "auto"}),
    ];
    for value in cases {
        assert!(
            serde_json::from_value::<RecognitionFacts>(value.clone()).is_err(),
            "accepted {value}"
        );
    }
    assert!(
        serde_json::from_value::<MetadataFacts>(
            json!({"kind": "read_error", "code": "READ", "title": "ignored"})
        )
        .is_err()
    );
}

#[test]
fn repair_mode_fixture_bridge_preserves_legacy_unknown_mode_fallback() {
    assert_eq!(RepairMode::from_legacy("heb"), RepairMode::Hebrew);
    assert_eq!(RepairMode::from_legacy("jib"), RepairMode::Jibrish);
    for value in ["auto", "", "HEB", "unexpected"] {
        assert_eq!(RepairMode::from_legacy(value), RepairMode::Auto);
    }
    for (mode, wire) in [
        (RepairMode::Hebrew, "hebrew"),
        (RepairMode::Jibrish, "jibrish"),
        (RepairMode::Auto, "auto"),
    ] {
        assert_eq!(serde_json::to_value(mode).unwrap(), wire);
        round_trip(&mode);
    }
}

#[test]
fn error_codes_are_stable_uppercase_strings() {
    for (code, wire) in [
        (ErrorCode::InvalidInput, "INVALID_INPUT"),
        (ErrorCode::UnsupportedSchema, "UNSUPPORTED_SCHEMA"),
        (ErrorCode::UnsupportedRuleset, "UNSUPPORTED_RULESET"),
        (ErrorCode::UnsupportedCapability, "UNSUPPORTED_CAPABILITY"),
        (ErrorCode::MissingInferenceFact, "MISSING_INFERENCE_FACT"),
        (ErrorCode::InferenceError, "INFERENCE_ERROR"),
        (ErrorCode::InputTooLarge, "INPUT_TOO_LARGE"),
        (
            ErrorCode::UnsupportedTargetShape,
            "UNSUPPORTED_TARGET_SHAPE",
        ),
        (ErrorCode::StalePlan, "STALE_PLAN"),
        (ErrorCode::JournalCorrupt, "JOURNAL_CORRUPT"),
        (
            ErrorCode::InvalidJournalTransition,
            "INVALID_JOURNAL_TRANSITION",
        ),
    ] {
        assert_eq!(serde_json::to_value(code).unwrap(), wire);
        assert_eq!(code.to_string(), wire);
        let error = EngineError {
            code,
            message: "explicit diagnostic".into(),
        };
        assert_eq!(error.to_string(), format!("{wire}: explicit diagnostic"));
        round_trip(&error);
    }
}

#[test]
fn decision_and_plan_round_trip_preserve_duplicates_trace_and_supplied_provenance() {
    let decision = Decision {
        provenance: Provenance::ProvidedFacts,
        artists: Some(vec!["שני".into(), "ראשון".into(), "שני".into()]),
        selected_by: Some(DecisionSource::FilenameAlias),
        trace: vec![
            DecisionEvent::AliasSelected {
                source: DecisionSource::FilenameAlias,
                index: 2,
            },
            DecisionEvent::MetadataReadObserved { failed: true },
            DecisionEvent::ClassifierRequested {
                text: "אמן ".into(),
            },
            DecisionEvent::NerRequested {
                text: "אמן".into()
            },
        ],
        warnings: vec![Diagnostic {
            code: "CLASSIFIER_FAIL_OPEN".into(),
            message: "supplied classifier failed".into(),
        }],
        apply_blockers: vec![Diagnostic {
            code: "RECOGNITION_UNHEALTHY".into(),
            message: "configured recognition failed".into(),
        }],
    };
    round_trip(&decision);
    assert_eq!(
        serde_json::to_value(&decision).unwrap()["provenance"],
        "supplied_facts"
    );
    assert_eq!(
        serde_json::to_value(&decision).unwrap()["trace"][0],
        json!({
            "kind": "alias_selected", "source": "filename_alias", "index": 2
        })
    );
    let plan = Plan {
        schema_version: 1,
        provenance: Provenance::ProvidedFacts,
        plan_id: "sha256:plan".into(),
        request_hash: "sha256:request".into(),
        ruleset_id: "singles-v14.0".into(),
        items: vec![PlannedSong {
            operation_id: "operation-0001".into(),
            source_id: "song-1".into(),
            decision,
            destinations: vec![DestinationIntent {
                destination_id: "destination-0001".into(),
                components: vec!["שני".into(), "סינגלים".into(), "שיר.mp3".into()],
                disposition: DestinationDisposition::Required,
            }],
            copy_mode: false,
        }],
        warnings: vec![],
        executable: false,
    };
    round_trip(&plan);
    let wire = serde_json::to_value(plan).unwrap();
    assert_eq!(wire["provenance"], "supplied_facts");
    assert_eq!(wire["executable"], false);
    assert_eq!(
        wire["items"][0]["decision"]["artists"],
        json!(["שני", "ראשון", "שני"])
    );
    let mut missing_provenance = wire.clone();
    missing_provenance
        .as_object_mut()
        .unwrap()
        .remove("provenance");
    assert!(serde_json::from_value::<Plan>(missing_provenance).is_err());
    let mut decision_without_provenance = wire["items"][0]["decision"].clone();
    decision_without_provenance
        .as_object_mut()
        .unwrap()
        .remove("provenance");
    assert!(serde_json::from_value::<Decision>(decision_without_provenance).is_err());
    assert!(serde_json::from_value::<Provenance>(json!("verified_filesystem")).is_err());
}

#[test]
fn all_decision_sources_dispositions_and_fact_variants_have_documented_tags() {
    for (value, expected) in [
        (DecisionSource::FilenameAlias, "filename_alias"),
        (DecisionSource::MetadataArtistAlias, "metadata_artist_alias"),
        (
            DecisionSource::MetadataArtistClassifier,
            "metadata_artist_classifier",
        ),
        (DecisionSource::TitleAlias, "title_alias"),
        (DecisionSource::FilenameNer, "filename_ner"),
        (DecisionSource::TitleNer, "title_ner"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (DestinationDisposition::Required, "required"),
        (
            DestinationDisposition::SkippedExistingOnly,
            "skipped_existing_only",
        ),
        (DestinationDisposition::Blocked, "blocked"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (ClassifierFact::Accept, json!({"kind": "accept"})),
        (ClassifierFact::Reject, json!({"kind": "reject"})),
        (ClassifierFact::Unavailable, json!({"kind": "unavailable"})),
        (
            ClassifierFact::Error { code: "E".into() },
            json!({"kind": "error", "code": "E"}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&value).unwrap(), expected);
        round_trip(&value);
    }
    for (value, expected) in [
        (
            NerFact::Artists(vec![]),
            json!({"kind": "artists", "value": []}),
        ),
        (NerFact::Unavailable, json!({"kind": "unavailable"})),
        (
            NerFact::Error { code: "E".into() },
            json!({"kind": "error", "value": {"code": "E"}}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&value).unwrap(), expected);
        round_trip(&value);
    }
    round_trip(&MetadataFacts::ReadError {
        code: "TAG_READ_FAILED".into(),
    });
    round_trip(&TargetSnapshot {
        root_id: "logical".into(),
        existing_directories: vec![vec!["אמן".into()], vec!["אמן".into()]],
        existing_files: vec![vec!["אמן".into(), "שיר.mp3".into()]],
    });
}

#[test]
fn frozen_fixture_loaders_verify_identities_and_counts() {
    let release = common::load_release_contract();
    let aliases = common::load_ordered_aliases();
    assert_eq!(release.len(), 574);
    assert_eq!(aliases.aliases.len(), 1_196);
    assert_eq!(aliases.cases.len(), 2_409);
    assert_eq!(aliases.baseline, "78d001aee5ce49628bdffebd34540fa83ba32c12");
    assert_eq!(
        aliases.csv_sha256,
        "7d2b0c806a18043c16b7baa99ebd174955bad1f8b70f606ee8f56e9559d4d391"
    );
    assert_eq!(release[0].id, "match-exact");
    assert_eq!(release[0].op, "match");
    assert_eq!(release[0].input["text"], "דוד");
    assert_eq!(release[0].expected, true);
    assert_eq!(aliases.cases[0].kind, "bare_alias");
    assert_eq!(aliases.cases[0].text, "אבי אילסון");
    assert_eq!(aliases.cases[0].expected.as_deref(), Some("אבי אילסון"));
    let manifest: Value = serde_json::from_slice(&common::fixture_bytes("manifest.json")).unwrap();
    assert_eq!(manifest["release_corpus"]["total_cases"], 574);
    assert_eq!(manifest["ordered_alias_corpus"]["aliases"], 1196);
    assert_eq!(manifest["ordered_alias_corpus"]["total_cases"], 2409);
    assert_eq!(
        manifest["release_corpus"]["source_runtime"]["unicode"],
        "15.0.0"
    );
}
