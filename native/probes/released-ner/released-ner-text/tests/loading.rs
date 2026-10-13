use released_ner_probe::LoadError;
use released_ner_text::{assets::ReleasedAssets, tokenizer::Tokenizer};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ner-tokenizer-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn error(&self) -> LoadError {
        match std::panic::catch_unwind(|| Tokenizer::load(&self.0)) {
            Ok(Err(e)) => e,
            Ok(Ok(_)) => panic!("unexpected accepted tokenizer"),
            Err(_) => panic!("loader unwound"),
        }
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn missing_malformed_truncated_changed_and_oversized_tokenizer() {
    let d = Dir::new();
    assert!(matches!(d.error(), LoadError::Io { .. }));
    let p = d.0.join("tokenizer.json");
    let original = include_bytes!("../tokenizer.json");
    for bytes in [
        b"{".as_slice(),
        b"null",
        &original[..original.len() - 1],
        b"{\"unicode\":\"future\"}",
    ] {
        fs::write(&p, bytes).unwrap();
        assert!(matches!(d.error(), LoadError::Hash { .. }));
    }
    let mut changed = original.to_vec();
    changed[0] ^= 1;
    fs::write(&p, changed).unwrap();
    assert!(matches!(d.error(), LoadError::Hash { .. }));
    let f = fs::File::create(&p).unwrap();
    f.set_len(1 << 32).unwrap();
    assert!(matches!(d.error(), LoadError::TooLarge { .. }));
}
#[test]
fn loaded_tokenizer_preserves_frozen_behavior_after_path_replacement() {
    let d = Dir::new();
    let p = d.0.join("tokenizer.json");
    fs::write(&p, include_bytes!("../tokenizer.json")).unwrap();
    let loaded = Tokenizer::load(&d.0).unwrap();
    let frozen =
        Tokenizer::from_json(&serde_json::from_slice(include_bytes!("../tokenizer.json")).unwrap())
            .unwrap();
    fs::write(&p, b"replaced after validation").unwrap();
    for text in [
        "",
        "אברהם פריד - אבא",
        "Don't stop...",
        "  אבא\nאמא",
        "ΣΟΣ 123 https://example.com",
    ] {
        assert_eq!(
            loaded.tokenize(text).unwrap(),
            frozen.tokenize(text).unwrap()
        );
    }
}
#[test]
#[ignore = "requires ORIGINAL_NER_EXPORT pointing to the independently verified original bundle"]
fn complete_bundle_is_immutable_and_partial_load_fails() {
    let source =
        PathBuf::from(std::env::var_os("ORIGINAL_NER_EXPORT").expect("ORIGINAL_NER_EXPORT"));
    let d = Dir::new();
    for name in ["manifest.json", "tensors.f32", "tokenizer.json"] {
        fs::copy(source.join(name), d.0.join(name)).unwrap();
    }
    let loaded = ReleasedAssets::load(&d.0).unwrap();
    let before = loaded.model().forward_fast(&[]).affine;
    fs::write(d.0.join("tensors.f32"), b"replaced after load").unwrap();
    assert_eq!(before, loaded.model().forward_fast(&[]).affine);
    assert!(!loaded
        .tokenizer()
        .tokenize("אברהם פריד")
        .unwrap()
        .is_empty());
    assert!(ReleasedAssets::load(&d.0).is_err());
    fs::copy(source.join("tensors.f32"), d.0.join("tensors.f32")).unwrap();
    fs::remove_file(d.0.join("tokenizer.json")).unwrap();
    assert!(matches!(
        ReleasedAssets::load(&d.0),
        Err(LoadError::Io { .. })
    ));
}
