//! Adapter for frozen legacy stub inputs, not a second implementation of policy.
use std::collections::BTreeMap;

use serde_json::{Value, json};
use sorter_core::*;

use crate::common::ReleaseCase;

pub fn request(case: &ReleaseCase) -> ClassifyRequest {
    let input = &case.input;
    let aliases: Vec<[String; 2]> = serde_json::from_value(input["aliases"].clone()).unwrap();
    let recognition = if input["ai_enabled"].as_bool().unwrap() {
        // The historical classifier stub is a constant and the NER stub defaults
        // to [] for any unknown key. Supply their finite reference call keys;
        // a wrong/new production request must still fail or mismatch the trace.
        let calls: Vec<[String; 2]> =
            serde_json::from_value(case.expected["calls"].clone()).unwrap();
        let mut classifier = BTreeMap::new();
        let mut ner = BTreeMap::new();
        for [operation, text] in calls {
            match operation.as_str() {
                "classify" => {
                    classifier.insert(
                        text,
                        if input["accept_artist"].as_bool().unwrap() {
                            ClassifierFact::Accept
                        } else {
                            ClassifierFact::Reject
                        },
                    );
                }
                "ner" => {
                    let artists = input["ner"].get(&text).cloned().unwrap_or(json!([]));
                    ner.insert(
                        text,
                        NerFact::Artists(serde_json::from_value(artists).unwrap()),
                    );
                }
                other => panic!("unknown frozen operation {other}"),
            }
        }
        RecognitionFacts::Configured {
            bundle_id: "frozen-fixture-stub".into(),
            health: RecognitionHealth::Healthy,
            classifier,
            ner,
        }
    } else {
        RecognitionFacts::Disabled
    };
    ClassifyRequest {
        schema_version: SCHEMA_VERSION,
        context: PolicyContext {
            ruleset_id: RULESET_ID.into(),
            settings: Settings {
                duet_mode: input["duet_mode"].as_bool().unwrap(),
                ..Settings::default()
            },
            aliases: aliases
                .into_iter()
                .map(|[source, target]| Alias { source, target })
                .collect(),
            recognition,
        },
        song: SongFacts {
            source_id: case.id.clone(),
            filename: input["filename"].as_str().unwrap().into(),
            metadata: if input["metadata_error"].as_bool().unwrap() {
                MetadataFacts::ReadError {
                    code: "FROZEN_TAG_READ_ERROR".into(),
                }
            } else {
                MetadataFacts::Present {
                    artist: input["artist"].as_str().unwrap().into(),
                    title: input["title"].as_str().unwrap().into(),
                }
            },
        },
    }
}

pub fn legacy_result(decision: &Decision) -> Value {
    let calls: Vec<Value> = decision
        .trace
        .iter()
        .filter_map(|event| match event {
            DecisionEvent::ClassifierRequested { text } => Some(json!(["classify", text])),
            DecisionEvent::NerRequested { text } => Some(json!(["ner", text])),
            _ => None,
        })
        .collect();
    json!({"artists": decision.artists, "calls": calls})
}
