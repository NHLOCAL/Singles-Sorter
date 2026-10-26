use sha2::{Digest, Sha256};
use sorter_core::unicode15::{is_space, is_word};

#[test]
fn every_unicode_scalar_matches_frozen_cpython_three_twelve_fifteen_predicates() {
    let mut words = Sha256::new();
    let mut spaces = Sha256::new();
    for codepoint in 0..=0x10ffff {
        let character = char::from_u32(codepoint);
        words.update([u8::from(character.is_some_and(is_word))]);
        spaces.update([u8::from(character.is_some_and(is_space))]);
    }
    // Generated directly from CPython re/str, never from Rust predicates.
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("../../../unicode/unicode15-provenance.json")).unwrap();
    assert_eq!(
        format!("{:x}", words.finalize()),
        provenance["word_membership_sha256"]
    );
    assert_eq!(
        format!("{:x}", spaces.finalize()),
        provenance["space_membership_sha256"]
    );
}

#[test]
fn frozen_unicode_provenance_identifies_the_generator_and_formatted_rust() {
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("../../../unicode/unicode15-provenance.json")).unwrap();
    assert_eq!(provenance["python_implementation"], "CPython");
    assert_eq!(provenance["python_version"], "3.12.14");
    assert_eq!(provenance["unicode_version"], "15.0.0");
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("../../../unicode/generate_unicode15.py"))
        ),
        provenance["generator_sha256"]
    );
    assert_eq!(
        format!(
            "{:x}",
            Sha256::digest(include_bytes!("../src/unicode15.rs"))
        ),
        provenance["generated_rust_sha256"]
    );
}

#[test]
fn every_unicode_scalar_decimal_value_matches_frozen_cpython_three_twelve() {
    let mut digest = Sha256::new();
    for cp in 0..=0x10ffff {
        digest.update([char::from_u32(cp)
            .and_then(sorter_core::unicode15::decimal_digit)
            .unwrap_or(255)]);
    }
    let provenance: serde_json::Value =
        serde_json::from_str(include_str!("../../../unicode/unicode15-provenance.json")).unwrap();
    assert_eq!(
        format!("{:x}", digest.finalize()),
        provenance["decimal_values_sha256"]
    );
    assert_eq!(provenance["decimal_zeroes"], 68);
}
