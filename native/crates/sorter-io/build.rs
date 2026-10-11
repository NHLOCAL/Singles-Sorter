use std::{env, path::PathBuf, process::Command};
fn run(command: &mut Command) {
    let status = command
        .status()
        .expect("CMake is required: install an official CMake distribution or set CMAKE");
    assert!(
        status.success(),
        "native metadata build failed: {command:?}"
    );
}
fn main() {
    println!("cargo:rerun-if-changed=bridge");
    println!("cargo:rerun-if-changed=dependencies");
    println!("cargo:rerun-if-env-changed=SORTER_DEPENDENCY_CACHE");
    println!("cargo:rerun-if-env-changed=SORTER_DEPENDENCIES_OFFLINE");
    println!("cargo:rerun-if-env-changed=CMAKE");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");
    println!("cargo:rustc-check-cfg=cfg(native_metadata_bridge)");
    let target = env::var("TARGET").unwrap();
    let host = env::var("HOST").unwrap();
    // Never claim a cross-target binary is validated or link host objects into it.
    // Native MSVC has its own CMake recipe. Execution acceptance is a CI gate.
    let msvc = target == "x86_64-pc-windows-msvc";
    let linux = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux");
    if host != target || (!linux && !msvc) {
        return;
    }
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("taglib");
    let cmake = env::var_os("CMAKE").unwrap_or_else(|| "cmake".into());
    let mut configure = Command::new(&cmake);
    configure
        .arg("-S")
        .arg("bridge")
        .arg("-B")
        .arg(&output)
        .arg("-DCMAKE_BUILD_TYPE=Release");
    if msvc {
        let features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        let runtime = if features.split(',').any(|f| f == "crt-static") {
            "MultiThreaded"
        } else {
            "MultiThreadedDLL"
        };
        configure.arg(format!("-DCMAKE_MSVC_RUNTIME_LIBRARY={runtime}"));
        // CMake's default installed Visual Studio generator selects a native x64
        // compiler. The build never injects a host GNU archive into Windows.
        configure.args(["-A", "x64"]);
    }
    run(&mut configure);
    run(Command::new(&cmake).arg("--build").arg(&output).args([
        "--config",
        "Release",
        "--target",
        "sorter_metadata",
        "--parallel",
        "2",
    ]));
    let sources = std::fs::read_to_string(output.join("dependency-sources.txt"))
        .expect("CMake must record the verified dependency source locations");
    let mut sources = sources.lines();
    println!(
        "cargo:rustc-env=SORTER_TAGLIB_SOURCE_DIR={}",
        sources.next().unwrap()
    );
    println!(
        "cargo:rustc-env=SORTER_ZLIB_SOURCE_DIR={}",
        sources.next().unwrap()
    );
    println!("cargo:rustc-cfg=native_metadata_bridge");
    println!(
        "cargo:rustc-link-search=native={}",
        output.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=sorter_metadata");
    println!("cargo:rustc-link-lib=static=tag");
    println!("cargo:rustc-link-lib=static=sorter_zlib");
    if !msvc {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
