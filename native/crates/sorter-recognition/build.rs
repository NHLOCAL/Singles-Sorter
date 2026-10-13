//! Bind the actually linked PCRE2 archive, not a caller's model/runtime label.
use sha2::{Digest, Sha256};
use std::{env, fs::File, io::Read, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=PCRE2_STATIC_DIR");
    let target = env::var("TARGET").expect("Cargo target");
    // Explicit experimental build seam; this does not enable Android app capabilities.
    println!("cargo:rerun-if-env-changed=SINGLES_SORTER_ANDROID_PROBE");
    if target.ends_with("-linux-android") {
        assert_eq!(
            env::var("SINGLES_SORTER_ANDROID_PROBE").as_deref(),
            Ok("1"),
            "Android inference is an opt-in probe, not a supported app runtime"
        );
    }
    let profile = env::var("PROFILE").expect("Cargo profile");
    let dir =
        PathBuf::from(env::var_os("PCRE2_STATIC_DIR").expect("pinned static PCRE2 is required"));
    assert!(dir.is_absolute());
    let filename = match target.as_str() {
        "x86_64-pc-windows-msvc" => "pcre2-32-static.lib",
        "x86_64-unknown-linux-gnu" | "x86_64-linux-android" | "aarch64-linux-android" => {
            "libpcre2-32.a"
        }
        _ => panic!("Original recognition target unsupported"),
    };
    let path = dir.join(filename);
    let mut file = File::open(&path).expect("static PCRE2 archive");
    assert!(file.metadata().unwrap().is_file());
    let mut digest = Sha256::new();
    let mut bytes = [0u8; 8192];
    loop {
        let n = file.read(&mut bytes).expect("read static PCRE2 archive");
        if n == 0 {
            break;
        }
        digest.update(&bytes[..n]);
    }
    println!("cargo:rerun-if-changed={}", path.display());
    println!(
        "cargo:rustc-env=RECOGNITION_PCRE2_SHA256={:x}",
        digest.finalize()
    );
    println!("cargo:rustc-env=RECOGNITION_TARGET={target}");
    println!("cargo:rustc-env=RECOGNITION_PROFILE={profile}");
}
