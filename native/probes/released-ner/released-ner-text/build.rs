use std::{env, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=PCRE2_STATIC_DIR");
    let target = env::var("TARGET").expect("Cargo TARGET");
    let dir = PathBuf::from(env::var_os("PCRE2_STATIC_DIR").expect(
        "PCRE2_STATIC_DIR is required; build the pinned 32-bit static library first. No host-library fallback.",
    ));
    assert!(dir.is_absolute(), "PCRE2_STATIC_DIR must be absolute");
    let (name, file) = if target == "x86_64-pc-windows-msvc" {
        ("pcre2-32-static", "pcre2-32-static.lib")
    } else if target == "x86_64-unknown-linux-gnu" {
        ("pcre2-32", "libpcre2-32.a")
    } else {
        panic!("target {target} has not passed this probe; Android is unfinished")
    };
    let library = dir.join(file);
    assert!(library.is_file(), "missing pinned static library: {}", library.display());
    println!("cargo:rerun-if-changed={}", library.display());
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=static={name}");
}
