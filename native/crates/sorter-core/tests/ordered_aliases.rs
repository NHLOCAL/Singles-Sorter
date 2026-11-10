#[allow(dead_code)]
mod common;

use sorter_core::*;

#[test]
fn all_2409_cases_keep_the_first_matching_released_csv_row() {
    let corpus = common::load_ordered_aliases();
    let context = PolicyContext {
        ruleset_id: RULESET_ID.into(),
        settings: Settings::default(),
        aliases: corpus
            .aliases
            .iter()
            .map(|[source, target]| Alias {
                source: source.clone(),
                target: target.clone(),
            })
            .collect(),
        recognition: RecognitionFacts::Disabled,
    };
    for case in corpus.cases {
        let actual = corpus
            .aliases
            .iter()
            .find(|[source, _]| exact_match(&case.text, source))
            .map(|[_, target]| target.as_str());
        let request = ClassifyRequest {
            schema_version: SCHEMA_VERSION,
            context: context.clone(),
            song: SongFacts {
                source_id: "ordered-alias-fixture".into(),
                filename: "0000_0000.mp3".into(),
                metadata: MetadataFacts::Present {
                    artist: case.text.clone(),
                    title: "".into(),
                },
            },
        };
        let decision = classify(&request).unwrap();
        assert_eq!(
            decision.artists,
            case.expected.as_ref().map(|target| vec![target.clone()]),
            "production metadata alias: {} {:?}",
            case.kind,
            case.text
        );
        if case.expected.is_some() {
            assert_eq!(
                decision.selected_by,
                Some(DecisionSource::MetadataArtistAlias)
            );
            assert!(matches!(
                decision.trace.last(),
                Some(DecisionEvent::AliasSelected {
                    source: DecisionSource::MetadataArtistAlias,
                    ..
                })
            ));
        }
        assert_eq!(
            actual,
            case.expected.as_deref(),
            "{}: {:?}",
            case.kind,
            case.text
        );
    }
}

#[test]
fn earlier_short_alias_wins_over_longer_and_personal_duplicate() {
    let request = ClassifyRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings {
                duet_mode: true,
                ..Settings::default()
            },
            aliases: vec![
                Alias {
                    source: "דוד".into(),
                    target: "ראשון".into(),
                },
                Alias {
                    source: "דוד ישראל".into(),
                    target: "ארוך".into(),
                },
                Alias {
                    source: "דוד".into(),
                    target: "אישי".into(),
                },
            ],
            recognition: RecognitionFacts::Disabled,
        },
        song: SongFacts {
            source_id: "anchor".into(),
            filename: "דוד ישראל.mp3".into(),
            metadata: MetadataFacts::ReadError {
                code: "TEST".into(),
            },
        },
    };
    let decision = classify(&request).unwrap();
    assert_eq!(decision.artists, Some(vec!["ראשון".into()]));
    assert_eq!(decision.selected_by, Some(DecisionSource::FilenameAlias));
    assert_eq!(
        decision.trace,
        vec![
            DecisionEvent::AliasSelected {
                source: DecisionSource::FilenameAlias,
                index: 0
            },
            DecisionEvent::MetadataReadObserved { failed: true },
        ]
    );
    assert_eq!(decision.warnings[0].code, "METADATA_READ_ERROR");
}

#[test]
fn selected_index_counts_nonmatching_and_duplicate_rows_verbatim() {
    let request = ClassifyRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings::default(),
            aliases: vec![
                Alias {
                    source: "משה".into(),
                    target: "ignored".into(),
                },
                Alias {
                    source: "משה".into(),
                    target: "duplicate".into(),
                },
                Alias {
                    source: "דוד".into(),
                    target: " raw target ".into(),
                },
            ],
            recognition: RecognitionFacts::Disabled,
        },
        song: SongFacts {
            source_id: "anchor".into(),
            filename: "דוד.mp3".into(),
            metadata: MetadataFacts::Present {
                artist: "משה".into(),
                title: "משה".into(),
            },
        },
    };
    let decision = classify(&request).unwrap();
    assert_eq!(decision.artists, Some(vec![" raw target ".into()]));
    assert_eq!(
        decision.trace,
        vec![
            DecisionEvent::AliasSelected {
                source: DecisionSource::FilenameAlias,
                index: 2
            },
            DecisionEvent::MetadataReadObserved { failed: false },
        ]
    );
}
