use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const CLASSIFY: &[u8] = include_bytes!("../../../tests/fixtures/protocol/classify-alias.json");
const PLAN: &[u8] = include_bytes!("../../../tests/fixtures/protocol/plan-collisions.json");
const FAIL_OPEN: &[u8] = include_bytes!("../../../tests/fixtures/protocol/classify-fail-open.json");

struct TempTree(PathBuf);

impl TempTree {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "singles-cli-protocol-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

// Real subprocesses own stdin; a writer thread avoids a stdout/stdin deadlock
// when an unsupported command returns without reading the supplied request.
fn run(args: &[&str], input: &[u8], cwd: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_singles-sorter"));
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let input = input.to_vec();
    let writer = std::thread::spawn(move || {
        if let Err(error) = stdin.write_all(&input) {
            assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
        }
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    output
}

fn envelope(output: &Output, exit: i32) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    let text = std::str::from_utf8(&output.stdout).expect("machine stdout is UTF-8");
    let response: Value = serde_json::from_str(text).expect("exactly one stdout JSON value");
    let fields = response.as_object().unwrap();
    assert_eq!(fields.len(), 4, "only the versioned envelope fields");
    assert_eq!(response["schema_version"], 1);
    assert_eq!(response["result"]["inference_executed"], false);
    assert_eq!(response["result"]["executable"], false);
    assert!(text.ends_with('\n'));
    if exit == 0 {
        assert_eq!(response["status"], "ok");
        assert_eq!(response["error"], Value::Null);
        assert!(
            output.stderr.is_empty(),
            "success has no diagnostic chatter"
        );
    } else {
        assert_eq!(response["status"], "error");
        let error = response["error"].as_object().unwrap();
        assert_eq!(error.len(), 2);
        assert!(response["error"]["message"].as_str().unwrap().len() > 1);
        let diagnostics = std::str::from_utf8(&output.stderr).unwrap();
        assert!(diagnostics.contains(response["error"]["code"].as_str().unwrap()));
    }
    response
}

fn error(args: &[&str], input: &[u8], exit: i32, code: &str) -> Value {
    let response = envelope(&run(args, input, None), exit);
    assert_eq!(response["error"]["code"], code);
    assert!(response["result"].get("decision").is_none());
    assert!(response["result"].get("plan").is_none());
    response
}

fn classify_request() -> Value {
    serde_json::from_slice(CLASSIFY).unwrap()
}

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

#[test]
fn classify_exposes_supplied_facts_without_inference_and_preserves_first_alias() {
    let response = envelope(&run(&["classify", "--json"], CLASSIFY, None), 0);
    assert_eq!(response["result"]["command"], "classify");
    assert_eq!(
        response["result"]["decision"]["provenance"],
        "supplied_facts"
    );
    assert_eq!(
        response["result"]["decision"]["artists"],
        json!(["אמן ראשון"])
    );
    assert_eq!(
        response["result"]["decision"]["selected_by"],
        "filename_alias"
    );
    assert_eq!(
        response["result"]["decision"]["trace"],
        json!([
            {"kind":"alias_selected", "source":"filename_alias", "index":0},
            {"kind":"metadata_read_observed", "failed":false}
        ])
    );
}

#[test]
fn plan_preserves_content_ids_collisions_and_non_executability() {
    let response = envelope(&run(&["plan", "--json"], PLAN, None), 0);
    let plan = &response["result"]["plan"];
    assert_eq!(plan["provenance"], "supplied_facts");
    assert_eq!(plan["executable"], false);
    assert_eq!(
        plan["items"][0]["destinations"][0]["components"],
        json!(["אמן ראשון", "סינגלים", "אמן - שיר_1.mp3"])
    );
    assert_eq!(
        plan["items"][1]["destinations"][0]["components"],
        json!(["אמן ראשון", "סינגלים", "אמן - שיר_2.mp3"])
    );
    for field in ["plan_id", "request_hash"] {
        let id = plan[field].as_str().unwrap();
        assert_eq!(id.len(), 64);
        assert!(
            id.bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );
    }
    assert_eq!(response, envelope(&run(&["plan", "--json"], PLAN, None), 0));
}

#[test]
fn capabilities_declares_every_phase_boundary_and_upcoming_protocols() {
    let response = envelope(&run(&["capabilities", "--json"], b"", None), 0);
    let capabilities = &response["result"]["capabilities"];
    assert_eq!(capabilities["input_provenance"], "supplied_facts");
    assert_eq!(capabilities["scope"], "supplied_single_song_facts");
    assert_eq!(capabilities["stdin_request_limit_bytes"], MAX_INPUT_BYTES);
    assert_eq!(capabilities["schema_versions"], json!([1]));
    assert_eq!(capabilities["ruleset_ids"], json!(["singles-v14.0"]));
    assert_eq!(
        capabilities["commands"],
        json!(["classify", "plan", "capabilities"])
    );
    for feature in [
        "native_inference",
        "metadata_decoding",
        "csv_parsing",
        "directory_traversal",
        "album_execution",
        "physical_apply",
    ] {
        assert_eq!(capabilities[feature], false, "{feature}");
    }
    assert_eq!(
        capabilities["upcoming"],
        json!(["ndjson_streaming", "legacy_positional_flags"])
    );
}

#[test]
fn explicit_classifier_failure_retains_fail_open_evidence_and_apply_blocker() {
    let response = envelope(&run(&["classify", "--json"], FAIL_OPEN, None), 0);
    let decision = &response["result"]["decision"];
    assert_eq!(decision["artists"], json!(["אמן חדש"]));
    assert_eq!(decision["selected_by"], "metadata_artist_classifier");
    assert!(
        decision["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "CLASSIFIER_FAIL_OPEN")
    );
    assert!(
        decision["apply_blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|blocker| blocker["code"] == "RECOGNITION_UNHEALTHY")
    );
}

#[test]
fn supplied_ner_artist_order_and_duplicates_are_not_deduplicated() {
    let mut request = classify_request();
    request["context"]["aliases"] = json!([]);
    request["song"]["filename"] = json!("unknown.mp3");
    request["song"]["metadata"] = json!({"kind":"present", "artist":"", "title":""});
    request["context"]["recognition"] = json!({
        "kind":"configured", "bundle_id":"provided-example", "health":"healthy", "classifier":{},
        "ner":{"unknown":{"kind":"artists", "value":["second", "first", "second"]}}
    });
    let response = envelope(&run(&["classify", "--json"], &bytes(&request), None), 0);
    assert_eq!(
        response["result"]["decision"]["artists"],
        json!(["second", "first", "second"])
    );
    assert_eq!(
        response["result"]["decision"]["selected_by"],
        "filename_ner"
    );
}

#[test]
fn malformed_empty_non_utf8_and_multiple_json_values_are_invalid_requests() {
    for input in [b"{".as_slice(), b"", b"\xff", b"null", b"{}\n{}", b"[]"] {
        error(&["classify", "--json"], input, 2, "INVALID_INPUT");
        error(&["plan", "--json"], input, 2, "INVALID_INPUT");
    }
}

#[test]
fn unknown_fields_and_duplicate_fields_are_rejected_without_silent_fallback() {
    let mut request = classify_request();
    request["unexpected"] = json!(true);
    error(
        &["classify", "--json"],
        &bytes(&request),
        2,
        "INVALID_INPUT",
    );
    let input = String::from_utf8(CLASSIFY.to_vec()).unwrap().replacen(
        "\"schema_version\": 1",
        "\"schema_version\": 1, \"schema_version\": 1",
        1,
    );
    error(
        &["classify", "--json"],
        input.as_bytes(),
        2,
        "INVALID_INPUT",
    );
    let mut request = classify_request();
    request["context"]["recognition"]["model_path"] = json!("never-opened.model");
    error(
        &["classify", "--json"],
        &bytes(&request),
        2,
        "INVALID_INPUT",
    );
}

#[test]
fn unsupported_schema_and_ruleset_are_stable_invalid_request_errors() {
    for command in ["classify", "plan"] {
        let base = if command == "classify" {
            CLASSIFY
        } else {
            PLAN
        };
        let mut request: Value = serde_json::from_slice(base).unwrap();
        request["schema_version"] = json!(2);
        error(
            &[command, "--json"],
            &bytes(&request),
            2,
            "UNSUPPORTED_SCHEMA",
        );
        request["schema_version"] = json!(1);
        request["context"]["ruleset_id"] = json!("future-ruleset");
        error(
            &[command, "--json"],
            &bytes(&request),
            2,
            "UNSUPPORTED_RULESET",
        );
    }
}

#[test]
fn absent_configured_facts_are_not_manufactured_as_failed_inference() {
    let mut request = classify_request();
    request["context"]["aliases"] = json!([]);
    request["context"]["recognition"] = json!({
        "kind":"configured", "bundle_id":"required-example", "health":"healthy", "classifier":{}, "ner":{}
    });
    error(
        &["classify", "--json"],
        &bytes(&request),
        2,
        "MISSING_INFERENCE_FACT",
    );
    request["song"]["metadata"] = json!({"kind":"present", "artist":"", "title":""});
    error(
        &["classify", "--json"],
        &bytes(&request),
        2,
        "MISSING_INFERENCE_FACT",
    );
    let mut plan: Value = serde_json::from_slice(PLAN).unwrap();
    plan["context"] = request["context"].clone();
    error(
        &["plan", "--json"],
        &bytes(&plan),
        2,
        "MISSING_INFERENCE_FACT",
    );
}

#[test]
fn supplied_ner_error_is_processing_failure_with_specific_code_not_internal_bug() {
    let mut request = classify_request();
    request["song"]["filename"] = json!("unknown.mp3");
    request["song"]["metadata"] = json!({"kind":"present", "artist":"", "title":""});
    request["context"]["recognition"] = json!({
        "kind":"configured", "bundle_id":"supplied-error", "health":"failed", "classifier":{},
        "ner":{"unknown":{"kind":"error", "value":{"code":"MODEL_RUNTIME_FAILURE"}}}
    });
    error(
        &["classify", "--json"],
        &bytes(&request),
        1,
        "INFERENCE_ERROR",
    );
}

#[test]
fn no_arguments_and_help_only_show_honest_help_without_reading_input() {
    for args in [vec![], vec!["--help"], vec!["-h"], vec!["plan", "--help"]] {
        let output = run(&args, b"not JSON", None);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let help = std::str::from_utf8(&output.stdout).unwrap();
        for text in [
            "classify --json",
            "plan --json",
            "capabilities --json",
            "supplied",
            "unsupported",
        ] {
            assert!(help.contains(text), "help missing {text}");
        }
    }
}

#[test]
fn duplicate_missing_or_unknown_command_arguments_are_invalid() {
    for args in [
        vec!["classify"],
        vec!["plan", "--json", "--json"],
        vec!["classify", "--json", "--verbose"],
        vec!["capabilities", "--json", "extra"],
        vec!["plan", "--help", "--json"],
    ] {
        error(&args, CLASSIFY, 2, "INVALID_INPUT");
    }
}

#[test]
fn an_unknown_top_level_flag_is_invalid_rather_than_an_unsupported_operation() {
    error(&["--bogus"], b"", 2, "INVALID_INPUT");
    error(&["--json"], b"", 2, "INVALID_INPUT");
}

#[cfg(unix)]
#[test]
fn non_utf8_argument_is_rejected_as_json_without_panicking() {
    use std::os::unix::ffi::OsStringExt;

    let output = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
        .arg(std::ffi::OsString::from_vec(vec![0xff]))
        .output()
        .unwrap();
    let response = envelope(&output, 2);
    assert_eq!(response["error"]["code"], "INVALID_INPUT");
}

#[test]
fn unsupported_operations_and_legacy_syntax_never_fall_back_to_singles() {
    for command in [
        "unknown",
        "apply",
        "albums",
        "scan",
        "decode-tags",
        "parse-csv",
        "infer",
        "status",
        "cancel",
        "resume",
        "source-directory",
    ] {
        error(&[command, "--json"], CLASSIFY, 3, "UNSUPPORTED_CAPABILITY");
    }
    error(&["plan", "--ndjson"], PLAN, 3, "UNSUPPORTED_CAPABILITY");
    error(
        &["classify", "--copy_mode"],
        CLASSIFY,
        3,
        "UNSUPPORTED_CAPABILITY",
    );
}

#[test]
fn unsupported_target_shape_returns_capability_error_without_altering_policy() {
    let mut request: Value = serde_json::from_slice(PLAN).unwrap();
    request["context"]["aliases"][0]["target"] = json!("nested/artist");
    error(
        &["plan", "--json"],
        &bytes(&request),
        3,
        "UNSUPPORTED_TARGET_SHAPE",
    );
}

#[test]
fn exact_sixteen_mib_request_is_accepted_including_all_trailing_whitespace() {
    let mut request = CLASSIFY.to_vec();
    request.resize(MAX_INPUT_BYTES, b' ');
    envelope(&run(&["classify", "--json"], &request, None), 0);
}

#[test]
fn oversize_is_rejected_before_any_truncated_prefix_can_be_processed() {
    let mut request = CLASSIFY.to_vec();
    request.resize(MAX_INPUT_BYTES + 1, b' ');
    error(&["classify", "--json"], &request, 2, "INPUT_TOO_LARGE");
    request.fill(b'x');
    error(&["plan", "--json"], &request, 2, "INPUT_TOO_LARGE");
}

fn manifest(root: &Path) -> BTreeMap<PathBuf, Option<(Vec<u8>, String)>> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, Option<(Vec<u8>, String)>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                entries.insert(relative, None);
                visit(root, &path, entries);
            } else {
                let bytes = fs::read(&path).unwrap();
                let hash = format!("{:x}", Sha256::digest(&bytes));
                entries.insert(relative, Some((bytes, hash)));
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

#[test]
fn unrelated_cwd_source_target_hashes_survive_success_failure_and_attempted_apply() {
    let tree = TempTree::new();
    fs::create_dir_all(tree.0.join("source/child")).unwrap();
    fs::create_dir_all(tree.0.join("target/אמן ראשון/סינגלים")).unwrap();
    fs::write(
        tree.0.join("source/אמן - שיר.mp3"),
        b"source sentinel\0\xff",
    )
    .unwrap();
    fs::write(tree.0.join("source/child/.keep"), b"nested sentinel").unwrap();
    fs::write(
        tree.0.join("target/אמן ראשון/סינגלים/אמן - שיר.mp3"),
        b"destination sentinel",
    )
    .unwrap();
    let before = manifest(&tree.0);
    let mut request: Value = serde_json::from_slice(PLAN).unwrap();
    request["songs"][0]["source_id"] = json!(tree.0.join("source/אמן - שיר.mp3").to_str().unwrap());
    request["target"]["root_id"] = json!(tree.0.join("target").to_str().unwrap());
    for copy in [false, true] {
        request["context"]["settings"]["copy_mode"] = json!(copy);
        let response = envelope(
            &run(&["plan", "--json"], &bytes(&request), Some(&tree.0)),
            0,
        );
        assert_eq!(response["result"]["plan"]["items"][0]["copy_mode"], copy);
    }
    envelope(&run(&["classify", "--json"], CLASSIFY, Some(&tree.0)), 0);
    envelope(&run(&["capabilities", "--json"], b"", Some(&tree.0)), 0);
    for (args, input, exit, code) in [
        (
            vec!["apply", "--json"],
            bytes(&request),
            3,
            "UNSUPPORTED_CAPABILITY",
        ),
        (
            vec!["unknown", "--json"],
            bytes(&request),
            3,
            "UNSUPPORTED_CAPABILITY",
        ),
        (
            vec!["classify", "--json"],
            b"{".to_vec(),
            2,
            "INVALID_INPUT",
        ),
    ] {
        let response = envelope(&run(&args, &input, Some(&tree.0)), exit);
        assert_eq!(response["error"]["code"], code);
    }
    assert_eq!(
        before,
        manifest(&tree.0),
        "all relative paths, bytes, and SHA256s preserved"
    );
}

#[cfg(unix)]
#[test]
fn stdin_read_failure_is_a_single_internal_error_envelope() {
    let tree = TempTree::new();
    let output = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
        .args(["classify", "--json"])
        .stdin(fs::File::open(&tree.0).unwrap())
        .output()
        .unwrap();
    let response = envelope(&output, 1);
    assert_eq!(response["error"]["code"], "INTERNAL_ERROR");
}

#[test]
fn v1_classify_and_plan_output_remains_byte_identical_to_task4() {
    for (command, input, expected) in [
        (
            "classify",
            CLASSIFY,
            include_bytes!("../../../tests/fixtures/protocol/classify-alias.v1-output.json")
                .as_slice(),
        ),
        (
            "classify",
            FAIL_OPEN,
            include_bytes!("../../../tests/fixtures/protocol/classify-fail-open.v1-output.json")
                .as_slice(),
        ),
        (
            "plan",
            PLAN,
            include_bytes!("../../../tests/fixtures/protocol/plan-collisions.v1-output.json")
                .as_slice(),
        ),
    ] {
        let output = run(&[command, "--json"], input, None);
        assert!(output.status.success());
        assert_eq!(output.stdout, expected);
    }
}
