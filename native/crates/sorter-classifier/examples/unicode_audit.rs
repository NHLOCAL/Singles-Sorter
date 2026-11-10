//! NEW exhaustive synthetic Unicode oracle checksum, not a corpus/model gate.
use sorter_classifier::{Limits, analyze as bounded_analyze, lower as bounded_lower};
fn analyze(text: &str) -> Vec<String> {
    bounded_analyze(text, Limits::default(), &|| false).unwrap()
}
fn lower(text: &str) -> String {
    bounded_lower(text, Limits::default(), &|| false).unwrap()
}
use sha2::{Digest, Sha256};
fn add(h: &mut Sha256, s: &str) {
    h.update((s.len() as u32).to_le_bytes());
    h.update(s.as_bytes());
}
fn main() {
    let mut simple = Sha256::new();
    let mut before = Sha256::new();
    let mut after = Sha256::new();
    let mut word = Sha256::new();
    let mut count = 0u32;
    for cp in 0..=0x10ffff {
        if let Some(c) = char::from_u32(cp) {
            count += 1;
            add(&mut simple, &lower(&c.to_string()));
            add(&mut before, &lower(&format!("A{c}Σ")));
            add(&mut after, &lower(&format!("AΣ{c}A")));
            let features = analyze(&format!("{c}{c}"));
            word.update((features.len() as u32).to_le_bytes());
            for s in features {
                add(&mut word, &s)
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"unicode_scalars":count,"lower":format!("{:x}",simple.finalize()),"sigma_before":format!("{:x}",before.finalize()),"sigma_after":format!("{:x}",after.finalize()),"word_analyzer":format!("{:x}",word.finalize())})
    );
}
