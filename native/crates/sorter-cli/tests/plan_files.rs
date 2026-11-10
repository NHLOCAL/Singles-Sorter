#![cfg(any(windows, target_os = "linux"))]
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::SystemTime,
};

fn run(input: &[u8], cwd: &Path, exit: i32) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
        .args(["plan-files", "--json"])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let input = input.to_owned();
    let writer = std::thread::spawn(move || {
        if let Err(e) = stdin.write_all(&input) {
            assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe);
        }
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["result"]["executable"], false);
    assert_eq!(value["result"]["inference_executed"], false);
    assert_eq!(output.stderr.is_empty(), exit == 0);
    value
}
fn tree() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    for dir in ["source", "target", "unrelated", "preferences"] {
        fs::create_dir(t.path().join(dir)).unwrap();
    }
    fs::write(
        t.path().join("preferences/settings.json"),
        b"owned preference sentinel",
    )
    .unwrap();
    fs::write(
        t.path().join("source/אברהם_פריד.mp3"),
        include_bytes!("../../../tests/fixtures/media/tagged.mp3"),
    )
    .unwrap();
    t
}
fn request(t: &Path, mode: &str) -> Value {
    json!({"schema_version":2,"mode":mode,"source":t.join("source"),"target":if mode=="Sort" {Some(t.join("target"))} else {None},"settings":sorter_core::Settings::default(),"ai_profile":"Disabled","personal_aliases":null})
}
type Manifest = Vec<(PathBuf, Vec<u8>, u64, SystemTime)>;
fn manifest(root: &Path) -> Manifest {
    fn walk(p: &Path, out: &mut Manifest) {
        let m = fs::symlink_metadata(p).unwrap();
        out.push((
            p.to_owned(),
            if m.is_file() {
                fs::read(p).unwrap()
            } else {
                vec![]
            },
            m.len(),
            m.modified().unwrap(),
        ));
        if m.is_dir() {
            for e in fs::read_dir(p).unwrap() {
                walk(&e.unwrap().path(), out);
            }
        }
    }
    let mut out = vec![];
    walk(root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
#[test]
fn sort_and_targetless_fix_use_real_metadata_and_leave_everything_unchanged() {
    let t = tree();
    let before = manifest(t.path());
    for mode in ["Sort", "FixNames"] {
        let v = run(
            &serde_json::to_vec(&request(t.path(), mode)).unwrap(),
            &t.path().join("unrelated"),
            0,
        );
        let p = &v["result"]["plan"];
        assert_eq!(p["schema_version"], 2);
        assert_eq!(p["executable"], false);
        assert!(!p["operations"].as_array().unwrap().is_empty());
        let metadata = p["metadata_observations"].as_array().unwrap();
        assert!(metadata.iter().any(|o| o.get("Get").is_some()));
        assert!(metadata.iter().all(|o| {
            o.get("Open")
                .is_none_or(|v| v["result"].get("Ok").is_some())
        }));
        assert!(p["binding_sha256"].as_str().unwrap().len() == 64);
        assert_eq!(v["result"]["resources"]["sources"][0]["records"], 1196);
        assert!(
            v["result"]["capabilities"]["metadata_coverage"]["suffixes"]
                .as_array()
                .unwrap()
                .len()
                >= 12
        );
        assert_eq!(manifest(t.path()), before);
    }
}
#[test]
fn configured_ai_never_silently_becomes_disabled() {
    let t = tree();
    let mut r = request(t.path(), "Sort");
    r["ai_profile"] = json!({"Configured":{"bundle_id":"requested-model"}});
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 3);
    assert_eq!(v["error"]["code"], "RECOGNITION_UNAVAILABLE");
    assert_eq!(v["result"]["blockers"], json!(["RECOGNITION_UNAVAILABLE"]));
    assert!(v["result"]["plan"].is_null());
}
#[test]
fn missing_target_and_source_fail_explicitly() {
    let t = tree();
    let mut r = request(t.path(), "Sort");
    r["target"] = Value::Null;
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 2);
    assert_eq!(v["error"]["code"], "TARGET_REQUIRED");
    r = request(t.path(), "FixNames");
    r["source"] = json!(t.path().join("missing"));
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 1);
    assert!(v["error"]["code"].as_str().unwrap().starts_with("STORAGE_"));
}
#[test]
fn malformed_resource_width_is_retained_but_bad_utf8_is_fatal() {
    let t = tree();
    let path = t.path().join("personal.csv");
    let mut r = request(t.path(), "Sort");
    r["personal_aliases"] = json!(path);
    fs::write(&path, b"malformed\n").unwrap();
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 0);
    assert_eq!(v["result"]["resources"]["diagnostics"][0]["width"], 1);
    assert!(
        v["result"]["plan"]["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b.as_str().unwrap().contains("ALIAS"))
    );
    assert_eq!(
        v["result"]["plan"]["request"]["alias_rows"]
            .as_array()
            .unwrap()
            .last()
            .unwrap(),
        &json!(["malformed"])
    );
    fs::write(&path, [0xff]).unwrap();
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 2);
    assert_eq!(v["error"]["code"], "RESOURCE_INVALID");
    fs::remove_file(&path).unwrap();
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 1);
    assert_eq!(v["error"]["code"], "RESOURCE_READ_FAILED");
}
#[test]
fn strict_v2_rejects_unknown_duplicate_wrong_version_and_relative_paths() {
    let t = tree();
    let mut r = request(t.path(), "FixNames");
    r["schema_version"] = json!(1);
    assert_eq!(
        run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
        "UNSUPPORTED_SCHEMA"
    );
    r["schema_version"] = json!(2);
    r["source"] = json!("source");
    assert_eq!(
        run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
        "INVALID_INPUT"
    );
    r = request(t.path(), "FixNames");
    r["apply"] = json!(true);
    run(&serde_json::to_vec(&r).unwrap(), t.path(), 2);
    let s = serde_json::to_string(&request(t.path(), "FixNames"))
        .unwrap()
        .replacen('{', "{\"schema_version\":2,", 1);
    run(s.as_bytes(), t.path(), 2);
}

#[test]
fn ai_profile_is_required_and_caller_cannot_claim_available_inference() {
    let t = tree();
    let mut r = request(t.path(), "FixNames");
    r.as_object_mut().unwrap().remove("ai_profile");
    run(&serde_json::to_vec(&r).unwrap(), t.path(), 2);
    r["ai_profile"] = json!({"Configured":{"bundle_id":"not-loaded","available":true}});
    run(&serde_json::to_vec(&r).unwrap(), t.path(), 2);
}
#[test]
fn non_directory_roots_and_same_sort_root_are_not_plans() {
    let t = tree();
    let before = manifest(t.path());
    let mut r = request(t.path(), "Sort");
    r["target"] = r["source"].clone();
    assert_eq!(
        run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
        "SAME_ROOT"
    );
    assert_eq!(manifest(t.path()), before);
    #[cfg(windows)]
    {
        // Distinct lexical spellings must retain the same full native identity.
        r["target"] = json!(t.path().join("SOURCE"));
        assert_eq!(
            run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
            "SAME_ROOT"
        );
        assert_eq!(manifest(t.path()), before);
    }
    r = request(t.path(), "FixNames");
    r["source"] = json!(t.path().join("source/אברהם_פריד.mp3"));
    assert_eq!(
        run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
        "STORAGE_NotDirectory"
    );
    assert_eq!(manifest(t.path()), before);
}
#[cfg(target_os = "linux")]
#[test]
fn symlink_selected_source_is_blocked_and_external_tree_unchanged() {
    let t = tree();
    std::os::unix::fs::symlink(t.path().join("source"), t.path().join("linked")).unwrap();
    let mut r = request(t.path(), "FixNames");
    r["source"] = json!(t.path().join("linked"));
    let before = manifest(t.path());
    let v = run(&serde_json::to_vec(&r).unwrap(), t.path(), 1);
    assert!(v["error"]["code"].as_str().unwrap().starts_with("STORAGE_"));
    assert_eq!(manifest(t.path()), before);
}

#[cfg(target_os = "linux")]
#[test]
fn alias_resources_do_not_follow_symlinks() {
    let t = tree();
    fs::write(t.path().join("original.csv"), b"a,b\n").unwrap();
    std::os::unix::fs::symlink(t.path().join("original.csv"), t.path().join("personal.csv"))
        .unwrap();
    let mut r = request(t.path(), "FixNames");
    r["personal_aliases"] = json!(t.path().join("personal.csv"));
    assert_eq!(
        run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
        "RESOURCE_INVALID"
    );
}

#[test]
fn stdin_and_alias_resource_limits_fail_with_v2_envelopes() {
    let t = tree();
    let mut bytes = serde_json::to_vec(&request(t.path(), "FixNames")).unwrap();
    bytes.resize(16 * 1024 * 1024, b' ');
    run(&bytes, t.path(), 0);
    bytes.push(b' ');
    assert_eq!(run(&bytes, t.path(), 2)["error"]["code"], "INPUT_TOO_LARGE");
    assert_eq!(run(&[0xff], t.path(), 2)["error"]["code"], "INVALID_INPUT");
    let csv = t.path().join("large.csv");
    fs::File::create(&csv)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    let mut r = request(t.path(), "FixNames");
    r["personal_aliases"] = json!(csv);
    assert_eq!(
        run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
        "RESOURCE_TOO_LARGE"
    );
}

#[test]
fn plan_files_argument_failures_are_v2_without_reading_stdin() {
    for args in [
        vec!["plan-files"],
        vec!["plan-files", "--json", "--json"],
        vec!["plan-files", "--ndjson"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        let v: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(v["schema_version"], 2);
        assert_eq!(v["result"]["executable"], false);
    }
}

#[test]
fn capability_report_is_explicit_and_metadata_readings_bind_real_resources() {
    let t = tree();
    let v = run(
        &serde_json::to_vec(&request(t.path(), "FixNames")).unwrap(),
        t.path(),
        0,
    );
    let caps = &v["result"]["capabilities"];
    assert_eq!(caps["native_inference"], cfg!(feature = "original-ai"));
    assert_eq!(caps["physical_apply"], false);
    assert_eq!(caps["android_provider"], false);
    assert_eq!(caps["windows_runtime_validated"], false);
    assert_eq!(
        caps["metadata_bridge_compiled"],
        sorter_io::metadata::MetadataReader::is_available()
    );
    #[cfg(target_os = "linux")]
    assert_eq!(caps["storage_backend"], "linux_test_only");
    let p = &v["result"]["plan"];
    let hash = v["result"]["resources"]["sources"][0]["sha256"]
        .as_str()
        .unwrap();
    assert_eq!(
        p["request"]["resource_ids"][0],
        format!("bundled:sha256:{hash}")
    );
    assert!(
        p["request"]["adapter_versions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s
                .as_str()
                .unwrap()
                .ends_with(caps["metadata_coverage_sha256"].as_str().unwrap()))
    );
    assert_eq!(p["request"]["ai_profile"], "Disabled");
}

#[test]
fn dot_components_and_nul_paths_are_rejected_before_storage_access() {
    let t = tree();
    let mut r = request(t.path(), "FixNames");
    for path in [
        format!("{}/./source", t.path().display()),
        format!("{}/source\0", t.path().display()),
    ] {
        r["source"] = json!(path);
        assert_eq!(
            run(&serde_json::to_vec(&r).unwrap(), t.path(), 2)["error"]["code"],
            "INVALID_INPUT"
        );
    }
}
