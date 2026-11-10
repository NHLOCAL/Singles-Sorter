use released_ner_text::inference::{CancellationToken, InferenceError, Limits, RequestControl};
use released_ner_text::{regex::Regex, tokenizer::Tokenizer};
fn tokenizer() -> Tokenizer {
    Tokenizer::from_json(&serde_json::from_str(include_str!("../tokenizer.json")).unwrap()).unwrap()
}
#[test]
fn input_and_token_caps_are_explicit_and_never_truncate() {
    let t = tokenizer();
    let cancel = CancellationToken::new();
    for (s, bytes, chars, tokens, expected) in [
        ("🎵", 3, 1, 1, "input bytes"),
        ("🎵a", 5, 1, 2, "input characters"),
        ("a b", 3, 3, 1, "tokens"),
    ] {
        let limits = Limits {
            input_bytes: bytes,
            input_chars: chars,
            tokens,
            ..Limits::default()
        };
        let c = RequestControl::new(&limits, &cancel).unwrap();
        assert!(
            matches!(t.tokenize_bounded(s, &c), Err(InferenceError::ResourceLimit { resource, .. }) if resource == expected)
        );
    }
    let limits = Limits {
        input_bytes: 4,
        input_chars: 1,
        tokens: 1,
        ..Limits::default()
    };
    let c = RequestControl::new(&limits, &cancel).unwrap();
    assert_eq!(
        t.tokenize_bounded("🎵", &c).unwrap(),
        t.tokenize("🎵").unwrap()
    );
}
#[test]
fn explicit_regex_limits_and_smaller_request_after_larger() {
    let regex = Regex::new("(a+)+$").unwrap();
    let cancel = CancellationToken::new();
    let limits = Limits::default();
    let c = RequestControl::new(&limits, &cancel).unwrap();
    assert_eq!(regex.search_bounded("aaaa", &c).unwrap(), Some((0, 4)));
    for (limits, expected) in [
        (
            Limits {
                regex_match: 0,
                ..Limits::default()
            },
            "regex match",
        ),
        (
            Limits {
                regex_depth: 0,
                ..Limits::default()
            },
            "regex depth",
        ),
        (
            Limits {
                regex_heap_kib: 0,
                ..Limits::default()
            },
            "regex heap KiB",
        ),
        (
            Limits {
                regex_calls: 0,
                ..Limits::default()
            },
            "regex_calls",
        ),
        (
            Limits {
                regex_callouts: 0,
                ..Limits::default()
            },
            "regex_callouts",
        ),
    ] {
        let c = RequestControl::new(&limits, &cancel).unwrap();
        assert!(
            matches!(regex.search_bounded("aaaa", &c), Err(InferenceError::ResourceLimit { resource, .. }) if resource == expected),
            "{expected}"
        );
    }
    let c = RequestControl::new(&limits, &cancel).unwrap();
    assert_eq!(regex.search_bounded("aaaa", &c).unwrap(), Some((0, 4)));
}
#[test]
fn cancellation_is_distinct_and_tokenizer_can_be_reused() {
    let t = tokenizer();
    let limits = Limits::default();
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    let invalid = Limits {
        input_bytes: usize::MAX,
        ..Limits::default()
    };
    assert!(matches!(
        RequestControl::new(&invalid, &cancelled),
        Err(InferenceError::Cancelled)
    ));
    assert!(matches!(
        RequestControl::new(&limits, &cancelled),
        Err(InferenceError::Cancelled)
    ));
    let cancel = CancellationToken::new();
    let c = RequestControl::new(&limits, &cancel).unwrap();
    cancel.cancel();
    assert!(matches!(
        t.tokenize_bounded("אבג", &c),
        Err(InferenceError::Cancelled)
    ));
    let fresh = CancellationToken::new();
    let c = RequestControl::new(&limits, &fresh).unwrap();
    assert_eq!(
        t.tokenize_bounded("אבג", &c).unwrap(),
        t.tokenize("אבג").unwrap()
    );
}
#[test]
fn aggregate_regex_budget_is_shared_across_calls() {
    let regex = Regex::new("a").unwrap();
    let cancel = CancellationToken::new();
    let limits = Limits {
        regex_calls: 1,
        ..Limits::default()
    };
    let c = RequestControl::new(&limits, &cancel).unwrap();
    assert_eq!(regex.search_bounded("a", &c).unwrap(), Some((0, 1)));
    assert!(matches!(
        regex.search_bounded("a", &c),
        Err(InferenceError::ResourceLimit { .. })
    ));
}
#[test]
#[ignore = "requires verified ORIGINAL_NER_EXPORT"]
fn full_original_parity_cancellation_reuse_and_resource_rejection() {
    use released_ner_probe::{choose, State, Token};
    use released_ner_text::{assets::ReleasedAssets, features, is_space};
    let path = std::env::var("ORIGINAL_NER_EXPORT").expect("ORIGINAL_NER_EXPORT");
    let assets = ReleasedAssets::load(std::path::Path::new(&path)).unwrap();
    let limits = Limits::default();
    let cancel = CancellationToken::new();
    let fixtures: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::path::Path::new(&path).join("fixtures.json")).unwrap(),
    )
    .unwrap();
    // The export's exact fixture layout is checked below; no private corpus used.
    let docs = fixtures["fixtures"].as_array().unwrap();
    for doc in docs {
        let text = doc["text"].as_str().unwrap();
        let out = assets.infer(text, &limits, &cancel).unwrap();
        let lexical = assets.tokenizer().tokenize(text).unwrap();
        let hashes: Vec<_> = lexical
            .iter()
            .map(|t| features(&t.text, t.norm_override.as_deref()).hashes)
            .collect();
        let tokens: Vec<_> = lexical
            .iter()
            .map(|t| Token {
                start: t.offset,
                end: t.offset + t.text.chars().count(),
                is_space: !t.text.is_empty() && t.text.chars().all(is_space),
                sent_start: t.sent_start,
                ent_iob: 0,
            })
            .collect();
        let forward = assets.model().forward_fast(&hashes);
        let mut state = State::default();
        let mut spans = vec![];
        let mut actions = vec![];
        let mut margins = vec![];
        while state.cursor < tokens.len() {
            let valid = state.valid(&tokens);
            let logits = assets.model().logits(&forward, state.context(tokens.len()));
            let action = choose(&logits, &valid).unwrap();
            let other = logits
                .iter()
                .enumerate()
                .filter(|&(i, _)| valid[i] && i != action)
                .map(|(_, v)| *v)
                .max_by(f32::total_cmp);
            margins.push(other.map(|v| (logits[action] - v).to_bits()));
            actions.push(action);
            state.apply(action, &tokens, text, &mut spans).unwrap();
        }
        assert_eq!(out.actions, actions);
        assert_eq!(out.spans, spans);
        assert_eq!(
            out.margins
                .iter()
                .map(|v| v.map(f32::to_bits))
                .collect::<Vec<_>>(),
            margins
        );
        assert_eq!(
            serde_json::to_value(&out.tokens).unwrap(),
            serde_json::to_value(&tokens).unwrap()
        );
    }
    let at_token_limit = "a ".repeat(1023) + "a";
    let started = std::time::Instant::now();
    let output = assets
        .infer(&at_token_limit, &limits, &CancellationToken::new())
        .unwrap();
    assert_eq!(output.tokens.len(), 1024);
    assert_eq!(output.actions.len(), 1024);
    eprintln!("full 1024-token request elapsed {:?}", started.elapsed());
    let at_character_limit = "a".repeat(4096);
    let output = assets
        .infer(&at_character_limit, &limits, &CancellationToken::new())
        .unwrap();
    assert_eq!(output.tokens.len(), 1);
    assert_eq!(output.tokens[0].end, 4096);
    for _ in 0..3 {
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert!(matches!(
            assets.infer("אבג", &limits, &cancelled),
            Err(InferenceError::Cancelled)
        ));
        assert!(assets
            .infer("אבג", &limits, &CancellationToken::new())
            .is_ok());
    }
    let tiny = Limits {
        tensor_bytes: 0,
        ..Limits::default()
    };
    assert!(matches!(
        assets.infer("אבג", &tiny, &cancel),
        Err(InferenceError::ResourceLimit { .. })
    ));
    assert!(assets
        .infer("אבג", &limits, &CancellationToken::new())
        .is_ok());
}
