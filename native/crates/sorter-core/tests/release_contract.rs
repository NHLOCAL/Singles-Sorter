#[allow(dead_code)]
mod common;
#[path = "common/policy_bridge.rs"]
mod policy_bridge;

use serde_json::json;
use sorter_core::*;

fn cases(operation: &str, expected_count: usize, check: impl Fn(&common::ReleaseCase)) {
    let corpus = common::load_release_contract();
    let selected: Vec<_> = corpus.iter().filter(|case| case.op == operation).collect();
    assert_eq!(selected.len(), expected_count);
    for case in selected {
        check(case);
    }
}

#[test]
fn all_298_frozen_exact_matching_cases() {
    cases("match", 298, |case| {
        assert_eq!(
            json!(exact_match(
                case.input["text"].as_str().unwrap(),
                case.input["artist"].as_str().unwrap()
            )),
            case.expected,
            "{}",
            case.id
        );
    });
}
#[test]
fn all_38_frozen_cleaning_cases() {
    cases("clean", 38, |case| {
        assert_eq!(
            json!(clean(case.input["text"].as_str().unwrap())),
            case.expected,
            "{}",
            case.id
        );
    });
}
#[test]
fn all_27_frozen_sanitizing_cases() {
    cases("sanitize", 27, |case| {
        assert_eq!(
            json!(sanitize(case.input["text"].as_str().unwrap())),
            case.expected,
            "{}",
            case.id
        );
    });
}
#[test]
fn all_46_frozen_artist_validation_cases() {
    cases("check_artist", 46, |case| {
        assert_eq!(
            json!(check_artist(case.input["text"].as_str().unwrap())),
            case.expected,
            "{}",
            case.id
        );
    });
}
#[test]
fn all_20_frozen_encoding_repair_cases() {
    cases("fix_jibrish", 20, |case| {
        assert_eq!(
            json!(fix_jibrish(
                case.input["text"].as_str().unwrap(),
                RepairMode::from_legacy(case.input["mode"].as_str().unwrap())
            )),
            case.expected,
            "{}",
            case.id
        );
    });
}
#[test]
fn all_57_frozen_routes_and_ordered_inference_call_traces() {
    cases("route", 57, |case| {
        let decision = classify(&policy_bridge::request(case))
            .unwrap_or_else(|error| panic!("{}: {error}", case.id));
        assert_eq!(
            policy_bridge::legacy_result(&decision),
            case.expected,
            "{}",
            case.id
        );
        assert_eq!(decision.provenance, Provenance::ProvidedFacts);
        assert!(decision.apply_blockers.is_empty(), "{}", case.id);
    });
}
#[test]
fn policy_corpus_scope_is_486_cases_and_88_targets_remain_for_planner() {
    let corpus = common::load_release_contract();
    assert_eq!(corpus.iter().filter(|c| c.op != "target").count(), 486);
    assert_eq!(corpus.iter().filter(|c| c.op == "target").count(), 88);
}
#[test]
fn unicode_fifteen_and_legacy_quirks_have_independent_anchors() {
    assert!(!check_artist("   "));
    assert!(check_artist("-"));
    assert!(!check_artist("דוד\tישראל"));
    assert!(!check_artist("דָוד"));
    assert!(exact_match("\u{1c}ודוד", "דוד"));
    assert!(!exact_match("דוד\u{11f04}", "דוד")); // Unicode 15 Kawi letter.
    assert!(exact_match("דוד\u{11f00}", "דוד")); // Kawi combining mark is not \w.
    assert!(!exact_match("דוד²", "דוד")); // Python \w includes numeric category No.
    assert!(exact_match("דוד\u{200c}", "דוד"));
    assert!(exact_match("דוד שיר", "דוד "));
    assert!(!exact_match("דוד ", "דוד "));
    assert!(exact_match("דוד", ""));
    assert!(!exact_match("", ""));
    assert!(exact_match("xדוד", "דוד"));
    assert!(!exact_match("וודוד", "דוד"));
    assert_eq!(clean("x\u{1c}y"), "x y");
    assert_eq!(clean("x\u{200b}y"), "x\u{200b}y");
    assert_eq!(clean("a___b x"), "ab x");
    assert_eq!(clean("a---b"), "a---b");
    assert_eq!(sanitize(&"😀".repeat(256)).unwrap().chars().count(), 255);
    assert_eq!(fix_jibrish("a אà", RepairMode::Auto), "a àà");
}
