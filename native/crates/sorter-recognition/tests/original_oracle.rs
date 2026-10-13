//! Opt-in generated-original-asset tests; no weights or private inputs committed.
use released_ner_text::{
    assets::ReleasedAssets,
    inference::{CancellationToken, Limits as NerLimits},
};
use serde_json::{Value, json};
use sorter_core::job_plan::{RecognitionError, RecognitionProvider};
use sorter_recognition::{BUNDLE_ID, JobCancellation, Limits, OriginalRecognition};
use std::{fs, path::PathBuf};
fn bundle() -> PathBuf {
    std::env::var_os("ORIGINAL_AI_BUNDLE")
        .expect("explicit original combined bundle required")
        .into()
}
#[test]
#[ignore = "requires freshly verified original assets and generated original NER oracle"]
fn original_paths_and_combined_gate_under_application_release_profile() {
    let dir = bundle();
    let oracle = PathBuf::from(
        std::env::var_os("ORIGINAL_NER_ORACLE").expect("generated fixtures.json required"),
    );
    let fixtures: Value = serde_json::from_slice(&fs::read(oracle).unwrap()).unwrap();
    let assets = ReleasedAssets::load(&dir).unwrap();
    let secondary = sorter_classifier::Model::load_directory(&dir, &|| false).unwrap();
    let mut combined =
        OriginalRecognition::load(&dir, BUNDLE_ID, Limits::default(), JobCancellation::new())
            .unwrap();
    assert!(!combined.inference_executed());
    let mut actions = 0;
    let cases = fixtures["fixtures"].as_array().unwrap();
    assert!(cases.len() >= 20, "must run full platform oracle suite");
    for f in cases {
        let text = f["text"].as_str().unwrap();
        let result = assets
            .infer(text, &NerLimits::default(), &CancellationToken::new())
            .unwrap();
        let expected_actions: Vec<usize> = f["states"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["action"].as_u64().unwrap() as usize)
            .collect();
        assert_eq!(result.actions, expected_actions, "actions: {}", f["id"]);
        assert_eq!(json!(result.spans), f["spans"], "spans: {}", f["id"]);
        let expected_tokens:Vec<Value>=f["tokens"].as_array().unwrap().iter().map(|t|json!({"start":t["start"],"end":t["end"],"is_space":t["is_space"],"sent_start":t["sent_start"],"ent_iob":t["ent_iob"]})).collect();
        assert_eq!(
            json!(result.tokens),
            json!(expected_tokens),
            "token metadata: {}",
            f["id"]
        );
        assert!(result.margins.iter().flatten().all(|m| m.is_finite()));
        let expected_artists: Vec<String> = result
            .spans
            .iter()
            .filter(|s| s.label == "SINGER")
            .filter(|s| {
                secondary
                    .predict(&s.text, sorter_classifier::Limits::default(), &|| false)
                    .unwrap()
                    .class
                    == 0
            })
            .map(|s| s.text.clone())
            .collect();
        assert_eq!(
            combined.ner(text).unwrap(),
            expected_artists,
            "combined class0 gate: {}",
            f["id"]
        );
        actions += result.actions.len();
    }
    assert!(combined.inference_executed());
    println!(
        "Original application-profile NER oracle: {} documents, {actions} exact actions, exact token metadata/spans and combined class0 gate",
        cases.len()
    );
}
#[test]
#[ignore = "requires verified original combined assets"]
fn cancellation_limits_and_identity_with_real_models() {
    let dir = bundle();
    let token = JobCancellation::new();
    let mut provider =
        OriginalRecognition::load(&dir, BUNDLE_ID, Limits::default(), token.clone()).unwrap();
    token.cancel();
    assert!(matches!(
        provider.ner("ישראל כהן"),
        Err(RecognitionError::Abort(_))
    ));
    assert!(!provider.inference_executed());
    let base = provider.adapter_versions();
    let limited = Limits {
        ner_work_units: 0,
        ..Limits::default()
    };
    let mut provider =
        OriginalRecognition::load(&dir, BUNDLE_ID, limited, JobCancellation::new()).unwrap();
    assert_ne!(base, provider.adapter_versions());
    assert!(matches!(
        provider.ner("ישראל כהן"),
        Err(RecognitionError::Abort(_))
    ));
    assert!(provider.inference_executed());
    let mut provider = OriginalRecognition::load(
        &dir,
        BUNDLE_ID,
        Limits {
            max_inference_calls: 1,
            ..Limits::default()
        },
        JobCancellation::new(),
    )
    .unwrap();
    provider.classify("ישראל כהן").unwrap();
    assert!(matches!(
        provider.classify("ישראל כהן"),
        Err(RecognitionError::Abort(_))
    ));
    let mut retry =
        OriginalRecognition::load(&dir, BUNDLE_ID, Limits::default(), JobCancellation::new())
            .unwrap();
    assert!(retry.classify("ישראל כהן").is_ok());
    assert_eq!(retry.resource_ids().len(), 4);
}
