//! Standalone native JSONL proof. Errors are explicit; no truncation or fallback.
use released_ner_probe::{choose, validate_tokens, Model, State, Token, ACTIONS};
use released_ner_text::{features, is_space, tokenizer::Tokenizer};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead},
    path::Path,
};
fn infer(m: &Model, t: &Tokenizer, text: &str) -> Result<Value, String> {
    let text_tokens = t.tokenize(text)?;
    let tokens: Vec<_> = text_tokens
        .iter()
        .map(|t| Token {
            start: t.offset,
            end: t.offset + t.text.chars().count(),
            is_space: !t.text.is_empty() && t.text.chars().all(is_space),
            sent_start: t.sent_start,
            ent_iob: 0,
        })
        .collect();
    validate_tokens(&tokens)?;
    let hashes: Vec<_> = text_tokens
        .iter()
        .map(|t| features(&t.text, t.norm_override.as_deref()).hashes)
        .collect();
    let forward = m.forward(&hashes);
    let mut state = State::default();
    let mut spans = vec![];
    let mut actions = vec![];
    let mut margins = vec![];
    while state.cursor < tokens.len() {
        let valid = state.valid(&tokens);
        let logits = m.logits(&forward, state.context(tokens.len()));
        let action = choose(&logits, &valid)?;
        let other = logits
            .iter()
            .enumerate()
            .filter(|&(i, _)| valid[i] && i != action)
            .map(|(_, v)| *v)
            .max_by(f32::total_cmp);
        margins.push(other.map(|v| logits[action] - v));
        actions.push(ACTIONS[action]);
        state.apply(action, &tokens, text, &mut spans)?;
    }
    Ok(json!({"actions":actions,"spans":spans,"margins":margins,"tokens":tokens}))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let dir = args
        .get(1)
        .ok_or("usage: raw-ner EXPORT_DIRECTORY; JSONL {text:string} on stdin")?;
    let m = Model::load(Path::new(dir))?;
    let tokenizer =
        Tokenizer::from_json(&serde_json::from_str(include_str!("../../tokenizer.json"))?)?;
    for line in io::stdin().lock().lines() {
        let result = (|| {
            let v: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let text = v["text"].as_str().ok_or("missing string text")?;
            if v.as_object().is_none_or(|o| o.len() != 1) {
                return Err("raw text only; preset entities/other inputs unsupported".into());
            }
            infer(&m, &tokenizer, text)
        })();
        match result {
            Ok(v) => println!("{v}"),
            Err(e) => println!("{}", json!({"error":e})),
        }
    }
    Ok(())
}
