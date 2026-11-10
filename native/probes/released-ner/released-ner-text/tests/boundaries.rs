use released_ner_text::{features, lower, regex::Regex, shape, string_id, tokenizer::Tokenizer};
#[test]
fn lexical_boundaries() {
    assert_eq!(lower("İ ΟΣ ΟΣΑ Ο\u{301}Σ"), "i\u{307} ος οσα ο\u{301}ς");
    assert_eq!(shape(&"X".repeat(99)), "XXXX");
    assert_eq!(shape(&"X".repeat(100)), "LONG");
    assert_eq!(shape("²Ⅷ①𐐀🎵"), "dⅧdX🎵");
    assert_eq!(string_id(""), 0);
    assert_eq!(string_id("NORM"), 67);
    assert_eq!(features("€", None).strings[0], "$");
}
#[test]
fn regex_empty_matches_and_anchors() {
    assert_eq!(
        Regex::new("|a").unwrap().finditer("a").unwrap(),
        vec![(0, 0), (0, 1), (1, 1)]
    );
    assert_eq!(
        Regex::new("$").unwrap().finditer("a\n").unwrap(),
        vec![(1, 1), (2, 2)]
    );
    assert_eq!(
        Regex::new(".").unwrap().finditer("\r\n").unwrap(),
        vec![(0, 1)]
    );
    assert_eq!(
        Regex::new("\\w+").unwrap().search("\u{345}²Ⅷ①").unwrap(),
        Some((1, 4))
    );
    assert_eq!(
        Regex::new("\\d+").unwrap().search("²１２٣").unwrap(),
        Some((1, 4))
    );
    assert_eq!(
        Regex::new("\\s+").unwrap().search("\u{1c}\u{a0}").unwrap(),
        Some((0, 2))
    );
}
#[test]
fn special_and_whitespace_reconstruct() {
    let v = serde_json::from_str(include_str!("../tokenizer.json")).unwrap();
    let t = Tokenizer::from_json(&v).unwrap();
    for s in [
        "",
        " ",
        "  ",
        "\r\n",
        "a  b\t\r\n",
        "\u{a0}",
        "((ומוטי))",
        "ווייס וייס",
    ] {
        let ts = t.tokenize(s).unwrap();
        assert_eq!(
            ts.iter()
                .map(|t| format!("{}{}", t.text, t.whitespace))
                .collect::<String>(),
            s
        );
    }
    assert_eq!(
        t.tokenize("\u{a0}").unwrap()[0].norm_override.as_deref(),
        Some("  ")
    );
}

#[test]
fn modified_contract_rejected() {
    let mut v: serde_json::Value = serde_json::from_str(include_str!("../tokenizer.json")).unwrap();
    v["prefix_search"] = serde_json::json!("^");
    assert!(Tokenizer::from_json(&v).is_err());
}

#[test]
fn second_pass_exception_sentence_start() {
    let v = serde_json::from_str(include_str!("../tokenizer.json")).unwrap();
    let t = Tokenizer::from_json(&v).unwrap();
    for text in ["'א'", "'abc'", "'", " 'א'"] {
        assert!(t
            .tokenize(text)
            .unwrap()
            .iter()
            .all(|t| t.sent_start.is_none()));
    }
    for text in ["א", "\"א\"", "\nא"] {
        let tokens = t.tokenize(text).unwrap();
        assert_eq!(tokens[0].sent_start, Some(true));
        assert!(tokens[1..].iter().all(|t| t.sent_start.is_none()));
    }
}
