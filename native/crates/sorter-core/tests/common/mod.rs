//! Test-only bridges to immutable, source-generated evidence. Never regenerate expectations here.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const RELEASE_SHA256: &str = "d7ce4a2e543a2a4782036c5e55d396a1eb9fd7e4601104ad9d97b485ba095536";
pub const ALIASES_SHA256: &str = "dbffac487b7c4ccc787725b8b2e869f5a5b5bb62e5dfe95f12a223bcb4d5ec61";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseCase {
    pub id: String,
    pub op: String,
    pub input: Value,
    pub expected: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedAliasCorpus {
    pub baseline: String,
    pub csv_sha256: String,
    pub aliases: Vec<[String; 2]>,
    pub cases: Vec<OrderedAliasCase>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedAliasCase {
    pub text: String,
    pub kind: String,
    pub expected: Option<String>,
}

pub fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(path).expect("frozen fixture must be present")
}

fn verified_fixture(name: &str, expected_hash: &str) -> Vec<u8> {
    let bytes = fixture_bytes(name);
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        expected_hash,
        "frozen fixture identity changed: {name}"
    );
    bytes
}

pub fn load_release_contract() -> Vec<ReleaseCase> {
    let bytes = verified_fixture("release-contract.jsonl", RELEASE_SHA256);
    let text = std::str::from_utf8(&bytes).expect("release corpus must be UTF-8");
    let cases: Vec<ReleaseCase> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("invalid frozen release case"))
        .collect();
    assert_eq!(cases.len(), 574, "release corpus case count changed");
    cases
}

pub fn load_ordered_aliases() -> OrderedAliasCorpus {
    let bytes = verified_fixture("ordered-aliases.json", ALIASES_SHA256);
    let corpus: OrderedAliasCorpus =
        serde_json::from_slice(&bytes).expect("invalid frozen alias corpus");
    assert_eq!(corpus.aliases.len(), 1_196, "frozen alias count changed");
    assert_eq!(
        corpus.cases.len(),
        2_409,
        "ordered alias case count changed"
    );
    corpus
}
