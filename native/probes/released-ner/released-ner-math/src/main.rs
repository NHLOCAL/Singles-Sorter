use released_ner_probe::*;
use serde_json::{Value, json};
use std::{fs, path::Path};
fn flatten(v: &Value, out: &mut Vec<f32>) {
    match v {
        Value::Array(a) => {
            for x in a {
                flatten(x, out)
            }
        }
        Value::Number(n) => out.push(n.as_f64().unwrap() as f32),
        _ => panic!("invalid numeric array"),
    }
}
fn diff(a: &[f32], b: &[f32]) -> Value {
    assert_eq!(a.len(), b.len());
    let mut max = 0f32;
    let mut violations = 0;
    let mut bits = 0;
    for (&a, &b) in a.iter().zip(b) {
        assert!(a.is_finite() && b.is_finite());
        let d = (a - b).abs();
        max = max.max(d);
        if d > 1e-5 + 1e-5 * b.abs() {
            violations += 1;
        }
        if a.to_bits() != b.to_bits() {
            bits += 1;
        }
    }
    json!({"max_abs":max,"diagnostic_atol_rtol_1e_5_violations":violations,"bit_differences":bits,"values":a.len()})
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let dir = Path::new(
        args.get(1)
            .map(String::as_str)
            .ok_or("usage: released-ner-probe EXPORT_DIRECTORY [FIXTURES]")?,
    );
    let m = Model::load(dir)?;
    let fixtures_path = args
        .get(2)
        .map(|s| Path::new(s).to_path_buf())
        .unwrap_or_else(|| dir.join("fixtures.json"));
    let fixtures: Value = serde_json::from_slice(&fs::read(fixtures_path)?)?;
    let mut reports = vec![];
    let mut all_exact = true;
    for f in fixtures["fixtures"].as_array().unwrap() {
        let text = f["text"].as_str().unwrap();
        let tokens: Vec<Token> = serde_json::from_value(f["tokens"].clone())?;
        validate_tokens(&tokens)?;
        let buckets: Vec<[[u32; 4]; 4]> = serde_json::from_value(f["buckets"].clone())?;
        assert_eq!(buckets.len(), tokens.len());
        let out = m.forward(&buckets);
        let mut stage_reports = serde_json::Map::new();
        for (name, expected) in f["stages"].as_object().unwrap() {
            let key = if name == "projection" {
                "projected".to_string()
            } else if name.starts_with("cnn_") {
                format!("{name}_padded")
            } else {
                name.clone()
            };
            let actual = &out.stages[&key];
            assert_eq!(json!(actual.shape), expected["shape"], "{name} shape");
            let mut values = vec![];
            flatten(&expected["values"], &mut values);
            stage_reports.insert(name.clone(), diff(&actual.data, &values));
        }
        let mut s = State::default();
        let mut spans = vec![];
        let mut traces = vec![];
        let mut path_exact = true;
        let expected_states = f["states"].as_array().unwrap();
        assert_eq!(expected_states.len(), tokens.len());
        for expected in expected_states {
            let context = s.context(tokens.len());
            let valid = s.valid(&tokens);
            let logits = m.logits(&out, context);
            let action = choose(&logits, &valid)?;
            let exact = json!(context) == expected["context"]
                && json!(valid) == expected["valid"]
                && json!(action) == expected["action"];
            path_exact &= exact;
            let mut refs = vec![];
            flatten(&expected["logits"], &mut refs);
            let mut ranked: Vec<_> = (0..22).filter(|&i| valid[i]).map(|i| logits[i]).collect();
            ranked.sort_by(|a, b| b.total_cmp(a));
            let margin = if ranked.len() > 1 {
                Some(ranked[0] - ranked[1])
            } else {
                None
            };
            traces.push(json!({"context":context,"valid":valid,"action":action,"action_name":ACTIONS[action],"oracle_action":expected["action"],"exact_state_and_action":exact,"margin":margin,"logits_diff":diff(&logits,&refs)}));
            s.apply(action, &tokens, text, &mut spans)?;
        }
        let spans_exact = json!(spans) == f["spans"];
        all_exact &= path_exact && spans_exact;
        reports.push(json!({"id":f["id"],"text":text,"stages":stage_reports,"states":traces,"spans":spans,"exact_path":path_exact,"exact_spans":spans_exact}));
    }
    let forced_path = dir.join("forced_states.json");
    let mut forced_cases = 0;
    if forced_path.exists() {
        let forced: Value = serde_json::from_slice(&fs::read(forced_path)?)?;
        for case in forced["cases"].as_array().unwrap() {
            let tokens: Vec<Token> = serde_json::from_value(case["tokens"].clone())?;
            validate_tokens(&tokens)?;
            let text = case["text"].as_str().unwrap();
            let mut state = State::default();
            let mut spans = vec![];
            for action in case["prefix"].as_array().unwrap() {
                let index = ACTIONS
                    .iter()
                    .position(|&s| Some(s) == action.as_str())
                    .unwrap();
                state.apply(index, &tokens, text, &mut spans)?;
            }
            assert_eq!(json!(state.cursor == tokens.len()), case["final"]);
            if state.cursor < tokens.len() {
                assert_eq!(json!(state.context(tokens.len())), case["context"]);
                assert_eq!(json!(state.valid(&tokens)), case["valid"]);
                let oracle_valid: Vec<bool> = serde_json::from_value(case["valid"].clone())?;
                assert_eq!(
                    choose(&[0.; 22], &state.valid(&tokens))?,
                    oracle_valid.iter().position(|&b| b).unwrap()
                );
            }
            forced_cases += 1;
        }
    }
    let result = json!({"scope":"synthetic original-model scalar float32 proof; oracle token features; not production parity","exact_paths_and_spans":all_exact,"forced_state_cases_passed":forced_cases,"fixtures":reports});
    println!("{}", serde_json::to_string_pretty(&result)?);
    if !all_exact {
        std::process::exit(2)
    }
    Ok(())
}
