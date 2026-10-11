//! Synthetic failures must be ordinary typed errors, never unwinds.
use released_ner_probe::{LoadError, Model};
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
            "ner-loader-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn manifest(&self, bytes: &[u8]) {
        fs::write(self.0.join("manifest.json"), bytes).unwrap();
    }
    fn load_error(&self) -> LoadError {
        match std::panic::catch_unwind(|| Model::load(&self.0)) {
            Ok(Err(e)) => e,
            Ok(Ok(_)) => panic!("unexpected accepted model"),
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
fn missing_and_malformed_are_errors() {
    let d = Dir::new();
    assert!(matches!(d.load_error(), LoadError::Io { .. }));
    for bytes in [b"{".as_slice(), b"[]", b"null", b"\xff", b"{}"] {
        d.manifest(bytes);
        let _ = d.load_error();
    }
}
#[test]
fn oversized_manifest_is_rejected_before_parsing() {
    let d = Dir::new();
    let f = fs::File::create(d.0.join("manifest.json")).unwrap();
    f.set_len(1 << 32).unwrap();
    assert!(matches!(d.load_error(), LoadError::TooLarge { .. }));
}
#[test]
fn unsupported_contract_never_panics() {
    let d = Dir::new();
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../model-lock.json")).unwrap();
    for (key, value) in [
        ("schema", serde_json::json!("future")),
        ("tensors", serde_json::json!([])),
        ("parameter_bytes", serde_json::json!(u64::MAX)),
        ("actions", serde_json::json!([])),
    ] {
        let mut m = original.clone();
        m[key] = value;
        d.manifest(&serde_json::to_vec(&m).unwrap());
        assert!(matches!(d.load_error(), LoadError::Unsupported { .. }));
    }
}
#[test]
fn weights_are_bounded_sized_and_hashed() {
    let d = Dir::new();
    d.manifest(include_bytes!("../model-lock.json"));
    let p = d.0.join("tensors.f32");
    fs::write(&p, [0; 4]).unwrap();
    assert!(matches!(d.load_error(), LoadError::Length { .. }));
    let f = fs::File::create(&p).unwrap();
    f.set_len(3_829_977).unwrap();
    assert!(matches!(d.load_error(), LoadError::TooLarge { .. }));
    f.set_len(3_829_976).unwrap();
    assert!(matches!(d.load_error(), LoadError::Hash { .. }));
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn fifo_is_rejected_without_waiting_for_a_writer() {
    let d = Dir::new();
    let status = std::process::Command::new("mkfifo")
        .arg(d.0.join("manifest.json"))
        .status()
        .unwrap();
    assert!(status.success());
    assert!(matches!(d.load_error(), LoadError::NotRegular { .. }));
}

#[test]
fn malformed_tensor_ranges_and_deep_json_are_errors() {
    let d = Dir::new();
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../model-lock.json")).unwrap();
    for (key, value) in [
        ("shape", serde_json::json!([u64::MAX, u64::MAX])),
        ("offset_bytes", serde_json::json!(u64::MAX)),
        ("nbytes", serde_json::json!(u64::MAX)),
        ("dtype", serde_json::json!(">f4")),
        ("node", serde_json::json!(-1)),
        ("sha256", serde_json::json!(null)),
    ] {
        let mut m = original.clone();
        m["tensors"][0][key] = value;
        d.manifest(&serde_json::to_vec(&m).unwrap());
        assert!(matches!(d.load_error(), LoadError::Unsupported { .. }));
    }
    d.manifest("[".repeat(1000).as_bytes());
    assert!(matches!(d.load_error(), LoadError::Json { .. }));
}
