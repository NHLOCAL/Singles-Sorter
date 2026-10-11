use std::io::{self, BufRead};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let tokenizer = if args.get(1).is_some_and(|s| s == "tokenize") {
        let v = serde_json::from_str(include_str!("../tokenizer.json")).unwrap();
        Some(released_ner_text::tokenizer::Tokenizer::from_json(&v).unwrap())
    } else {
        None
    };
    for line in io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let text = value["text"].as_str().unwrap();
        if let Some(ref tokenizer) = tokenizer {
            let ts = tokenizer.tokenize(text).unwrap();
            let out:Vec<_>=ts.iter().map(|t|{let f=released_ner_text::features(&t.text,t.norm_override.as_deref());serde_json::json!({"text":t.text,"offset":t.offset,"whitespace":t.whitespace,"sent_start":t.sent_start,"strings":f.strings,"ids":f.ids.map(|i|i.to_string()),"hashes":f.hashes,"buckets":f.buckets})}).collect();
            println!("{}", serde_json::json!(out));
        } else {
            let f = released_ner_text::features(text, value["norm_override"].as_str());
            println!(
                "{}",
                serde_json::json!({"strings":f.strings,"ids":f.ids.map(|i|i.to_string()),"hashes":f.hashes,"buckets":f.buckets,"lower":released_ner_text::lower(text)})
            )
        }
    }
}
