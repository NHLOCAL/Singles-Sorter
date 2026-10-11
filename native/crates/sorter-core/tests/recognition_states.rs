use sorter_core::*;
use std::collections::BTreeMap;

fn request() -> ClassifyRequest {
    ClassifyRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings::default(),
            aliases: vec![],
            recognition: RecognitionFacts::Configured {
                bundle_id: "required-local-bundle".into(),
                health: RecognitionHealth::Healthy,
                classifier: BTreeMap::from([(" דוד  ישראל ".into(), ClassifierFact::Accept)]),
                ner: BTreeMap::from([("unknown".into(), NerFact::Artists(vec![]))]),
            },
        },
        song: SongFacts {
            source_id: "supplied-song".into(),
            filename: "unknown.mp3".into(),
            metadata: MetadataFacts::Present {
                artist: " דוד  ישראל ".into(),
                title: "".into(),
            },
        },
    }
}
fn classifier(request: &mut ClassifyRequest, fact: ClassifierFact) {
    let RecognitionFacts::Configured { classifier, .. } = &mut request.context.recognition else {
        panic!()
    };
    classifier.insert(" דוד  ישראל ".into(), fact);
}
fn ner(request: &mut ClassifyRequest, fact: NerFact) {
    let RecognitionFacts::Configured { ner, .. } = &mut request.context.recognition else {
        panic!()
    };
    ner.insert("unknown".into(), fact);
}
#[test]
fn explicit_classifier_failures_keep_raw_legacy_acceptance_with_evidence() {
    for fact in [
        ClassifierFact::Unavailable,
        ClassifierFact::Error {
            code: "PREDICTION_FAILED".into(),
        },
    ] {
        let mut input = request();
        classifier(&mut input, fact);
        let decision = classify(&input).unwrap();
        assert_eq!(decision.artists, Some(vec![" דוד  ישראל ".into()]));
        assert_eq!(
            decision.selected_by,
            Some(DecisionSource::MetadataArtistClassifier)
        );
        assert_eq!(
            decision.trace,
            vec![
                DecisionEvent::MetadataReadObserved { failed: false },
                DecisionEvent::ClassifierRequested {
                    text: " דוד  ישראל ".into()
                }
            ]
        );
        assert_eq!(decision.warnings[0].code, "CLASSIFIER_FAIL_OPEN");
        assert!(
            !decision.apply_blockers.is_empty(),
            "required configured failure must block future apply"
        );
    }
}
#[test]
fn unused_failed_bundle_does_not_block_a_filename_alias_decision() {
    let mut input = request();
    input.context.aliases.push(Alias {
        source: "unknown".into(),
        target: "known".into(),
    });
    let RecognitionFacts::Configured {
        health,
        classifier,
        ner,
        ..
    } = &mut input.context.recognition
    else {
        panic!()
    };
    *health = RecognitionHealth::Failed;
    classifier.clear();
    ner.clear();
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, Some(vec!["known".into()]));
    assert!(decision.apply_blockers.is_empty());
    assert!(
        decision
            .warnings
            .iter()
            .any(|d| d.code == "RECOGNITION_UNHEALTHY")
    );
    assert!(!decision.trace.iter().any(|event| matches!(
        event,
        DecisionEvent::ClassifierRequested { .. } | DecisionEvent::NerRequested { .. }
    )));
}
#[test]
fn failed_bundle_and_classifier_fail_open_preserve_artist_and_block_apply() {
    let mut input = request();
    classifier(&mut input, ClassifierFact::Unavailable);
    let RecognitionFacts::Configured { health, .. } = &mut input.context.recognition else {
        panic!()
    };
    *health = RecognitionHealth::Failed;
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, Some(vec![" דוד  ישראל ".into()]));
    assert!(
        decision
            .warnings
            .iter()
            .any(|d| d.code == "CLASSIFIER_FAIL_OPEN")
    );
    assert!(
        decision
            .apply_blockers
            .iter()
            .any(|d| d.code == "RECOGNITION_UNHEALTHY")
    );
}
#[test]
fn deliberately_disabled_recognition_does_not_accept_arbitrary_artist() {
    let mut input = request();
    input.context.recognition = RecognitionFacts::Disabled;
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, None);
    assert_eq!(decision.selected_by, None);
    assert_eq!(
        decision.trace,
        vec![DecisionEvent::MetadataReadObserved { failed: false }]
    );
    assert!(decision.apply_blockers.is_empty());
    assert!(decision.warnings.is_empty());
}
#[test]
fn missing_classifier_exact_key_is_not_manufactured_as_unavailable() {
    let mut input = request();
    let RecognitionFacts::Configured { classifier, .. } = &mut input.context.recognition else {
        panic!()
    };
    classifier.clear();
    classifier.insert("דוד ישראל".into(), ClassifierFact::Accept);
    assert_eq!(
        classify(&input).unwrap_err().code,
        ErrorCode::MissingInferenceFact
    );
}
#[test]
fn missing_filename_or_title_ner_exact_key_errors() {
    let mut input = request();
    classifier(&mut input, ClassifierFact::Reject);
    let RecognitionFacts::Configured { ner: facts, .. } = &mut input.context.recognition else {
        panic!()
    };
    facts.clear();
    assert_eq!(
        classify(&input).unwrap_err().code,
        ErrorCode::MissingInferenceFact
    );
    ner(&mut input, NerFact::Artists(vec![]));
    input.song.metadata = MetadataFacts::Present {
        artist: "".into(),
        title: " exact title ".into(),
    };
    assert_eq!(
        classify(&input).unwrap_err().code,
        ErrorCode::MissingInferenceFact
    );
}
#[test]
fn unavailable_ner_keeps_legacy_empty_result_and_blocks_apply() {
    let mut input = request();
    classifier(&mut input, ClassifierFact::Reject);
    ner(&mut input, NerFact::Unavailable);
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, None);
    assert_eq!(decision.selected_by, None);
    assert_eq!(decision.warnings[0].code, "NER_UNAVAILABLE");
    assert!(!decision.apply_blockers.is_empty());
}
#[test]
fn ner_invocation_error_propagates_inference_error() {
    let mut input = request();
    classifier(&mut input, ClassifierFact::Reject);
    ner(
        &mut input,
        NerFact::Error {
            code: "NLP_CRASH".into(),
        },
    );
    let error = classify(&input).unwrap_err();
    assert_eq!(error.code, ErrorCode::InferenceError);
    assert!(error.message.contains("NLP_CRASH"));
}
#[test]
fn raw_ner_artists_keep_invalid_strings_entity_order_and_duplicates() {
    let mut input = request();
    classifier(&mut input, ClassifierFact::Reject);
    ner(
        &mut input,
        NerFact::Artists(vec![
            "".into(),
            "לא ידוע".into(),
            "English".into(),
            "English".into(),
        ]),
    );
    let decision = classify(&input).unwrap();
    assert_eq!(
        decision.artists,
        Some(vec![
            "".into(),
            "לא ידוע".into(),
            "English".into(),
            "English".into()
        ])
    );
    assert_eq!(decision.selected_by, Some(DecisionSource::FilenameNer));
    assert!(decision.apply_blockers.is_empty());
}
#[test]
fn legacy_path_stem_and_sanitization_failure_are_explicit() {
    let mut input = request();
    input.context.recognition = RecognitionFacts::Disabled;
    input.song.filename = "<>:*?".into();
    assert_eq!(classify(&input).unwrap_err().code, ErrorCode::InvalidInput);
    input.song.filename = "מייל מיוזיק".into();
    input.context.aliases = vec![Alias {
        source: "מייל מיוזיק".into(),
        target: "kept original".into(),
    }];
    assert_eq!(
        classify(&input).unwrap().artists,
        Some(vec!["kept original".into()])
    );
}
#[test]
fn invalid_versioned_envelope_is_rejected_before_decisions() {
    let mut input = request();
    input.schema_version = 2;
    assert_eq!(
        classify(&input).unwrap_err().code,
        ErrorCode::UnsupportedSchema
    );
    input.schema_version = 1;
    input.context.ruleset_id = "other".into();
    assert_eq!(
        classify(&input).unwrap_err().code,
        ErrorCode::UnsupportedRuleset
    );
}

#[test]
fn failed_bundle_blocks_a_reached_successful_classifier_fact_without_changing_artist() {
    let mut input = request();
    let RecognitionFacts::Configured { health, .. } = &mut input.context.recognition else {
        panic!()
    };
    *health = RecognitionHealth::Failed;
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, Some(vec![" דוד  ישראל ".into()]));
    assert_eq!(
        decision.selected_by,
        Some(DecisionSource::MetadataArtistClassifier)
    );
    assert_eq!(decision.apply_blockers[0].code, "RECOGNITION_UNHEALTHY");
}

#[test]
fn unavailable_filename_ner_continues_to_title_and_title_errors_propagate() {
    let mut input = request();
    input.song.metadata = MetadataFacts::Present {
        artist: "".into(),
        title: "title".into(),
    };
    ner(&mut input, NerFact::Unavailable);
    let RecognitionFacts::Configured { ner: facts, .. } = &mut input.context.recognition else {
        panic!()
    };
    facts.insert(
        "title".into(),
        NerFact::Artists(vec!["raw".into(), "raw".into()]),
    );
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, Some(vec!["raw".into(), "raw".into()]));
    assert_eq!(decision.selected_by, Some(DecisionSource::TitleNer));
    assert_eq!(
        decision.trace,
        vec![
            DecisionEvent::MetadataReadObserved { failed: false },
            DecisionEvent::NerRequested {
                text: "unknown".into()
            },
            DecisionEvent::NerRequested {
                text: "title".into()
            },
        ]
    );
    assert!(!decision.apply_blockers.is_empty());
    let RecognitionFacts::Configured { ner: facts, .. } = &mut input.context.recognition else {
        panic!()
    };
    facts.insert(
        "title".into(),
        NerFact::Error {
            code: "TITLE_NLP_CRASH".into(),
        },
    );
    assert_eq!(
        classify(&input).unwrap_err().code,
        ErrorCode::InferenceError
    );
}

#[test]
fn python_three_twelve_stems_do_not_use_rust_host_path_rules() {
    for (filename, expected_stem) in [
        (".hidden", ".hidden"),
        ("song.", "song."),
        ("song.live.mp3", "song.live"),
        (".hidden.mp3", ".hidden"),
        (".", ""),
        ("..", ".."),
        ("a..mp3", "a."),
    ] {
        let mut input = request();
        input.song.filename = filename.into();
        input.song.metadata = MetadataFacts::Present {
            artist: "".into(),
            title: "".into(),
        };
        let RecognitionFacts::Configured { ner, .. } = &mut input.context.recognition else {
            panic!()
        };
        ner.clear();
        ner.insert(expected_stem.into(), NerFact::Artists(vec!["raw".into()]));
        let decision = classify(&input).unwrap();
        assert_eq!(decision.artists, Some(vec!["raw".into()]));
        assert_eq!(
            decision.trace[1],
            DecisionEvent::NerRequested {
                text: expected_stem.into()
            }
        );
    }
}

#[test]
fn unused_failed_bundle_does_not_block_metadata_or_title_aliases() {
    for (artist, title, source) in [
        ("דוד", "", DecisionSource::MetadataArtistAlias),
        ("English", "דוד", DecisionSource::TitleAlias),
    ] {
        let mut input = request();
        input.song.metadata = MetadataFacts::Present {
            artist: artist.into(),
            title: title.into(),
        };
        input.context.aliases = vec![Alias {
            source: "דוד".into(),
            target: "raw known".into(),
        }];
        let RecognitionFacts::Configured {
            health,
            classifier,
            ner,
            ..
        } = &mut input.context.recognition
        else {
            panic!()
        };
        *health = RecognitionHealth::Failed;
        classifier.clear();
        ner.clear();
        let decision = classify(&input).unwrap();
        assert_eq!(decision.artists, Some(vec!["raw known".into()]));
        assert_eq!(decision.selected_by, Some(source));
        assert_eq!(
            decision.trace,
            vec![
                DecisionEvent::MetadataReadObserved { failed: false },
                DecisionEvent::AliasSelected { source, index: 0 },
            ]
        );
        assert!(decision.apply_blockers.is_empty());
    }
}

#[test]
fn classifier_rejection_precedes_title_alias_and_preserves_selected_index() {
    let mut input = request();
    classifier(&mut input, ClassifierFact::Reject);
    input.song.metadata = MetadataFacts::Present {
        artist: " דוד  ישראל ".into(),
        title: "מייל מיוזיק àáøäí ôøéã".into(),
    };
    input.context.aliases = vec![
        Alias {
            source: "משה".into(),
            target: "ignored".into(),
        },
        Alias {
            source: "אברהם פריד".into(),
            target: "canonical".into(),
        },
    ];
    let decision = classify(&input).unwrap();
    assert_eq!(decision.artists, Some(vec!["canonical".into()]));
    assert_eq!(decision.selected_by, Some(DecisionSource::TitleAlias));
    assert_eq!(
        decision.trace,
        vec![
            DecisionEvent::MetadataReadObserved { failed: false },
            DecisionEvent::ClassifierRequested {
                text: " דוד  ישראל ".into()
            },
            DecisionEvent::AliasSelected {
                source: DecisionSource::TitleAlias,
                index: 1
            },
        ]
    );
}
