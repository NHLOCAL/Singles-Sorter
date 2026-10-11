//! Integrity and labeling checks only. Album/traversal/fix replacement policy is
//! intentionally not implemented by Task 1 and cannot regenerate these goldens.
use serde_json::Value;
use sha2::{Digest, Sha256};

const FIXTURES: [(&str, &[u8]); 3] = [
    (
        "cases.jsonl",
        include_bytes!("../../../tests/fixtures/full_contract/cases.jsonl"),
    ),
    (
        "historical-unsafe.jsonl",
        include_bytes!("../../../tests/fixtures/full_contract/historical-unsafe.jsonl"),
    ),
    (
        "csv-dialect.json",
        include_bytes!("../../../tests/fixtures/full_contract/csv-dialect.json"),
    ),
];

#[test]
fn pinned_release_outputs_match_frozen_manifest() {
    let manifest: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/full_contract/manifest.json"
    ))
    .unwrap();
    assert_eq!(
        manifest["baseline"]["revision"],
        "78d001aee5ce49628bdffebd34540fa83ba32c12"
    );
    assert_eq!(manifest["inputs"]["cases"], 82);
    assert_eq!(manifest["actual_metadata_decoding"], false);
    assert_eq!(manifest["actual_model_inference"], false);
    for (name, contents) in FIXTURES {
        assert_eq!(
            format!("{:x}", Sha256::digest(contents)),
            manifest["artifacts"][name]["sha256"].as_str().unwrap(),
            "{name}"
        );
        assert_eq!(
            contents.len() as u64,
            manifest["artifacts"][name]["bytes"].as_u64().unwrap()
        );
    }
    let inputs = include_bytes!("../../../tests/fixtures/full_contract/inputs.json");
    assert_eq!(
        format!("{:x}", Sha256::digest(inputs)),
        manifest["inputs"]["sha256"].as_str().unwrap()
    );
    let exporter = include_bytes!("../../../tests/reference/export_full_contract.py");
    assert_eq!(
        format!("{:x}", Sha256::digest(exporter)),
        manifest["exporter_sha256"].as_str().unwrap()
    );
}

#[test]
fn unsafe_mechanics_stay_historical_and_separate_from_acceptance() {
    let policy: Vec<Value> = std::str::from_utf8(FIXTURES[0].1)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let unsafe_rows: Vec<Value> = std::str::from_utf8(FIXTURES[1].1)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(policy.len(), 80);
    assert_eq!(unsafe_rows.len(), 2);
    for row in policy {
        assert_ne!(row["op"], "unsafe");
        assert_eq!(row["historical_only"], false);
    }
    for row in unsafe_rows {
        assert_eq!(row["historical_only"], true);
        assert_eq!(row["future_acceptance"], false);
    }
}
