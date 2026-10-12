//! Actual CLI subprocesses. Every write is beneath an exclusively owned temp root.
use std::process::{Command, Stdio};
#[cfg(not(all(windows, feature = "selected-windows-execution")))]
#[test]
fn unsupported_build_does_not_consume_paths_or_hang_on_stdin() {
    let output = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
        .args(["run-files", "--ndjson"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["event"], "error");
    assert_eq!(response["data"]["code"], "UNSUPPORTED_CAPABILITY");
}
#[cfg(all(windows, feature = "selected-windows-execution"))]
mod selected {
    use super::*;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        io::{BufRead, BufReader, Write},
    };
    fn wave() -> Vec<u8> {
        let samples = 1600u32;
        let mut bytes = vec![];
        bytes.extend(b"RIFF");
        bytes.extend((36 + samples).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(samples.to_le_bytes());
        bytes.resize(44 + samples as usize, 0);
        bytes
    }
    #[test]
    fn copied_move_revalidates_auxiliary_streams_and_journal_after_confirmation() {
        for action in ["apply", "stale_stream", "stale_journal", "cancel"] {
            let root = tempfile::tempdir().unwrap();
            let target_tree = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
            let source = root.path().canonicalize().unwrap().join("source");
            let journal = root.path().canonicalize().unwrap().join("journal");
            let target = target_tree.path().canonicalize().unwrap().join("target");
            for path in [&source, &target, &journal] {
                fs::create_dir(path).unwrap();
            }
            let name = "אברהם פריד - copied.wav";
            let file = source.join(name);
            let stream = source.join(format!("{name}:notes"));
            let bytes = wave();
            fs::write(&file, &bytes).unwrap();
            fs::write(&stream, b"owned auxiliary").unwrap();
            let mut child = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
                .args(["run-files", "--ndjson"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut stdin = child.stdin.take().unwrap();
            let mut output = BufReader::new(child.stdout.take().unwrap());
            let request = json!({"schema_version":1,"journal_directory":journal,"preview":{"schema_version":2,"mode":"Sort","source":source,"target":target,"settings":{"copy_mode":false,"main_folder_only":true,"singles_folder":false,"exist_only":false,"abc_sort":false,"duet_mode":false},"ai_profile":"Disabled","personal_aliases":null}});
            writeln!(stdin, "{request}").unwrap();
            stdin.flush().unwrap();
            let mut line = String::new();
            output.read_line(&mut line).unwrap();
            let planned: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(planned["event"], "planned", "{planned}");
            assert!(planned["data"]["execution_condition"].as_str().is_some());
            let plan = &planned["data"]["plan"];
            let observations = plan["observations"].as_array().unwrap();
            let volume = |role: &str| {
                observations
                    .iter()
                    .find(|entry| entry["reference"] == plan["request"][role])
                    .unwrap()["identity"]["Windows"]["volume"]
                    .as_u64()
                    .unwrap()
            };
            assert_ne!(
                volume("source"),
                volume("target"),
                "CI must exercise two actual volumes"
            );
            assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
            assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
            if action == "stale_stream" {
                let modified = fs::metadata(&file).unwrap().modified().unwrap();
                fs::write(&stream, b"changed stream!").unwrap();
                fs::File::options()
                    .write(true)
                    .open(&file)
                    .unwrap()
                    .set_times(fs::FileTimes::new().set_modified(modified))
                    .unwrap();
            } else if action == "stale_journal" {
                fs::rename(&journal, root.path().join("old-journal")).unwrap();
                fs::create_dir(&journal).unwrap();
            }
            let decision = if action == "cancel" {
                json!({"action":"cancel"})
            } else {
                json!({"action":"apply","plan_id":plan["binding_sha256"]})
            };
            writeln!(stdin, "{decision}").unwrap();
            stdin.flush().unwrap();
            let events = output
                .lines()
                .map(|line| serde_json::from_str::<Value>(&line.unwrap()).unwrap())
                .collect::<Vec<_>>();
            let status = child.wait().unwrap();
            if action == "apply" {
                assert!(status.success(), "{events:?}");
                let result = &events
                    .iter()
                    .find(|event| event["event"] == "result")
                    .unwrap()["data"];
                assert_eq!(result["status"], "complete");
                assert_eq!(result["moved_sources"], 1);
                assert_eq!(result["committed_destinations"], 1);
                assert!(result["recovery_error"].is_null(), "{result}");
                assert_eq!(
                    result["recovery"]["transfers"]["operations"][0]["status"],
                    "Committed"
                );
                let progress = &events
                    .iter()
                    .find(|event| event["event"] == "verification_progress")
                    .unwrap()["data"]["progress"];
                assert!(progress["bytes_read"].as_u64().unwrap() > 0);
                assert_eq!(
                    progress["completed_operations"],
                    progress["total_operations"]
                );
                assert_eq!(
                    fs::read(target.join("אברהם פריד").join(name)).unwrap(),
                    bytes
                );
                assert_eq!(
                    fs::read(target.join("אברהם פריד").join(format!("{name}:notes"))).unwrap(),
                    b"owned auxiliary"
                );
                assert!(!file.exists());
            } else {
                assert_eq!(status.success(), action == "cancel", "{events:?}");
                assert_eq!(fs::read(&file).unwrap(), bytes);
                assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
                assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
                if action == "stale_journal" {
                    assert_eq!(
                        fs::read_dir(root.path().join("old-journal"))
                            .unwrap()
                            .count(),
                        0
                    );
                }
            }
        }
    }
    fn run_case(copy: bool, action: &str) {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("מקור");
        let target = root.path().join("יעד");
        let journal = match action {
            "journal_missing" => target.join("must-not-be-created"),
            "journal_overlap" => target.clone(),
            _ => root.path().join("יומן"),
        };
        for p in [&source, &target] {
            fs::create_dir(p).unwrap();
        }
        if !action.starts_with("journal_") {
            fs::create_dir(&journal).unwrap();
        }
        let names = ["אברהם פריד - בדיקה אחת.wav", "אברהם פריד - בדיקה שניה.wav"];
        let bytes = wave();
        for name in names {
            fs::write(source.join(name), &bytes).unwrap();
        }
        let primary = root.path().join("selected-primary.csv");
        if action == "stale_primary" {
            fs::write(&primary, "אברהם פריד,אברהם פריד\n").unwrap();
        }
        let expected = Sha256::digest(&bytes);
        let mut child = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
            .args(["run-files", "--ndjson"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut request = json!({"schema_version":1,"journal_directory":journal,"preview":{"schema_version":2,"mode":"Sort","source":source,"target":target,"settings":{"copy_mode":copy,"main_folder_only":false,"singles_folder":true,"exist_only":false,"abc_sort":false,"duet_mode":false},"ai_profile":"Disabled","personal_aliases":null}});
        if action == "stale_primary" {
            request["preview"]["primary_aliases"] = json!(primary);
        }
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let planned: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(planned["event"], "planned", "{planned}");
        assert!(fs::read_dir(&target).unwrap().next().is_none());
        if action == "stale" {
            fs::write(source.join(names[0]), b"changed after inspected preview").unwrap();
        }
        if action == "stale_primary" {
            fs::write(&primary, "אברהם פריד,יעד שהשתנה\n").unwrap();
        }
        let decision = if action == "cancel" {
            json!({"action":"cancel"})
        } else {
            json!({"action":"apply","plan_id":planned["data"]["plan"]["binding_sha256"]})
        };
        writeln!(stdin, "{decision}").unwrap();
        stdin.flush().unwrap();
        let events = output
            .lines()
            .map(|line| serde_json::from_str::<Value>(&line.unwrap()).unwrap())
            .collect::<Vec<_>>();
        let status = child.wait().unwrap();
        if action != "apply" {
            if action == "stale_primary" {
                assert!(!status.success(), "{events:?}");
                assert!(
                    events.iter().any(|event| event["event"] == "error"
                        && event["data"]["code"] == "PLAN_CHANGED"),
                    "{events:?}"
                );
                assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
                for name in names {
                    assert_eq!(fs::read(source.join(name)).unwrap(), bytes);
                }
            }
            if action == "journal_missing" {
                assert!(!journal.exists());
            }
            assert!(fs::read_dir(&target).unwrap().next().is_none());
            assert!(source.join(names[1]).exists());
            return;
        }
        assert!(status.success(), "{events:?}");
        let result = events
            .iter()
            .find(|event| event["event"] == "result")
            .unwrap();
        assert_eq!(result["data"]["status"], "complete");
        let decisions = &result["data"]["execution_decisions"];
        assert_eq!(decisions["schema_version"], 1);
        assert_eq!(decisions["basis"], "fresh_apply_preflight");
        assert_eq!(
            decisions["plan_binding_sha256"],
            planned["data"]["plan"]["binding_sha256"]
        );
        assert_eq!(
            decisions["decisions"],
            planned["data"]["plan"]["decisions"]
        );
        let routes = decisions["single_routes"].as_array().unwrap();
        assert_eq!(routes.len(), 2);
        for (id, route) in routes.iter().enumerate() {
            assert_eq!(route["artist"], "אברהם פריד");
            assert_eq!(route["operation_ids"], json!([id]));
            assert_eq!(
                route["source"],
                planned["data"]["plan"]["operations"][id]["kind"]["SingleTransfer"]["source"]["reference"]
            );
        }
        assert_eq!(result["data"]["committed_destinations"], 2);
        assert_eq!(
            result["data"]["legacy_summary"],
            json!({"songs_sorted":2,"artist_folders_created":1,"albums_processed":0,"top_artists":[["אברהם פריד",2]]})
        );
        assert_eq!(result["data"]["committed_operations"], json!([0, 1]));
        assert!(
            !result["data"]["effect_receipts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        for name in names {
            let destination = target.join("אברהם פריד").join("סינגלים").join(name);
            assert_eq!(Sha256::digest(fs::read(destination).unwrap()), expected);
            assert_eq!(source.join(name).exists(), copy);
        }
        assert!(fs::read_dir(journal).unwrap().count() >= 2);
    }
    #[test]
    fn clean_noop_requires_fresh_confirmation_and_never_creates_a_journal() {
        for changed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("source");
            let target = root.path().join("target");
            let journal = root.path().join("journal");
            for path in [&source, &target, &journal] {
                fs::create_dir(path).unwrap();
            }
            fs::write(source.join("notes.txt"), b"owned non-audio").unwrap();
            let mut child = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
                .args(["run-files", "--ndjson"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let mut stdin = child.stdin.take().unwrap();
            let mut output = BufReader::new(child.stdout.take().unwrap());
            writeln!(stdin, "{}", json!({"schema_version":1,"journal_directory":journal,"preview":{"schema_version":2,"mode":"Sort","source":source,"target":target,"settings":{"copy_mode":true},"ai_profile":"Disabled","personal_aliases":null}})).unwrap();
            stdin.flush().unwrap();
            let mut line = String::new();
            output.read_line(&mut line).unwrap();
            let planned: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(planned["event"], "planned", "{planned}");
            assert_eq!(planned["data"]["plan"]["operations"], json!([]));
            if changed {
                fs::write(source.join("אברהם פריד - new.wav"), wave()).unwrap();
            }
            writeln!(
                stdin,
                "{}",
                json!({"action":"apply","plan_id":planned["data"]["plan"]["binding_sha256"]})
            )
            .unwrap();
            stdin.flush().unwrap();
            drop(stdin);
            let events = output
                .lines()
                .map(|line| serde_json::from_str::<Value>(&line.unwrap()).unwrap())
                .collect::<Vec<_>>();
            let status = child.wait().unwrap();
            assert_eq!(status.success(), !changed, "{events:?}");
            if changed {
                assert!(
                    events.iter().any(|event| event["event"] == "error"
                        && event["data"]["code"] == "PLAN_CHANGED")
                );
            } else {
                let result = &events
                    .iter()
                    .find(|event| event["event"] == "result")
                    .unwrap()["data"];
                assert_eq!(result["status"], "complete");
                assert_eq!(result["no_op"], true);
                let decisions = &result["execution_decisions"];
                assert_eq!(decisions["schema_version"], 1);
                assert_eq!(decisions["basis"], "fresh_noop_preflight");
                assert_eq!(decisions["plan_binding_sha256"], result["plan_id"]);
                assert_eq!(
            decisions["decisions"],
            planned["data"]["plan"]["decisions"]
        );
                assert_eq!(decisions["single_routes"], json!([]));
                assert_eq!(
                    result["legacy_summary"],
                    json!({"songs_sorted":0,"artist_folders_created":0,"albums_processed":0,"top_artists":[]})
                );
                assert_eq!(result["effect_receipts"], json!([]));
                assert!(result["journal_directory"].is_null());
            }
            assert_eq!(fs::read_dir(journal).unwrap().count(), 0);
            assert_eq!(fs::read_dir(target).unwrap().count(), 0);
            assert_eq!(
                fs::read(source.join("notes.txt")).unwrap(),
                b"owned non-audio"
            );
        }
    }
    #[test]
    fn actual_copy_and_move_require_exact_inspected_confirmation() {
        run_case(true, "apply");
        run_case(false, "apply");
    }
    #[test]
    fn missing_or_overlapping_journal_does_not_create_music_directories() {
        run_case(true, "journal_missing");
        run_case(true, "journal_overlap");
    }
    #[test]
    fn cancel_and_changed_source_never_apply_inspected_plan() {
        run_case(true, "cancel");
        run_case(true, "stale");
        run_case(true, "stale_primary");
    }
    #[test]
    fn capability_report_identifies_repair_scope_without_cleanup_authority() {
        let output = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
            .args(["capabilities", "--json"])
            .output()
            .unwrap();
        assert!(output.status.success());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let selected = &report["result"]["capabilities"]["selected_execution"];
        assert!(
            selected["modes"]
                .as_array()
                .unwrap()
                .contains(&json!("FixNames"))
        );
        assert_eq!(selected["automatic_backup_cleanup"], false);
        assert_eq!(selected["automatic_replay"], false);
    }
    fn run_repair_case(action: &str) {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("תיקון");
        let journal = root.path().join("יומן תיקון");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&journal).unwrap();
        let audio = include_bytes!("../../../tests/fixtures/media/blank.mp3");
        let payload = [b"\x03".as_slice(), "àáâ".as_bytes()].concat();
        let mut body = b"TPE1".to_vec();
        body.extend([0, 0, 0, payload.len() as u8]);
        body.extend([0, 0]);
        body.extend(payload);
        body.resize(127, 0);
        let mut original = b"ID3\x04\0\0\0\0\0\x7f".to_vec();
        original.extend(body);
        original.extend(audio);
        let path = source.join("owned_repair.mp3");
        fs::write(&path, &original).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
            .args(["run-files", "--ndjson"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let request = json!({"schema_version":1,"journal_directory":journal,"preview":{"schema_version":2,"mode":"FixNames","source":source,"target":null,"settings":{"copy_mode":false,"main_folder_only":false,"singles_folder":true,"exist_only":false,"abc_sort":false,"duet_mode":false},"ai_profile":"Disabled","personal_aliases":null}});
        writeln!(stdin, "{request}").unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let planned: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(planned["event"], "planned", "{planned}");
        assert!(planned["data"]["target"].is_null());
        assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&source).unwrap().count(), 1);
        if action == "stale" {
            original.push(7);
            fs::write(&path, &original).unwrap();
        }
        let decision = if action == "cancel" {
            json!({"action":"cancel"})
        } else {
            json!({"action":"apply","plan_id":planned["data"]["plan"]["binding_sha256"]})
        };
        writeln!(stdin, "{decision}").unwrap();
        stdin.flush().unwrap();
        let events = output
            .lines()
            .map(|line| serde_json::from_str::<Value>(&line.unwrap()).unwrap())
            .collect::<Vec<_>>();
        let status = child.wait().unwrap();
        if action != "apply" {
            assert_eq!(fs::read(&path).unwrap(), original);
            assert_eq!(fs::read_dir(&source).unwrap().count(), 1);
            assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
            return;
        }
        assert!(status.success(), "{events:?}");
        let result = &events
            .iter()
            .find(|event| event["event"] == "result")
            .unwrap()["data"];
        assert_eq!(result["status"], "complete");
        assert_eq!(result["mode"], "FixNames");
        assert_eq!(result["repair"]["renamed_files"], 1);
        assert_eq!(result["repair"]["repaired_files"], 1);
        assert_eq!(result["repair"]["retained_backup_bytes"], original.len());
        assert!(!path.exists());
        let repaired = fs::read(source.join("owned repair.mp3")).unwrap();
        assert_ne!(repaired, original);
        assert_eq!(&repaired[137..], audio);
        assert!(
            repaired
                .windows("אבג".len())
                .any(|bytes| bytes == "אבג".as_bytes())
        );
        let backups = fs::read_dir(&source)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "backup"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), original);
        assert_eq!(
            result["recovery"]["recorded_finished"], true,
            "repair recovery must verify the journal; result={result}; planned={planned}"
        );
        assert!(result["recovery_error"].is_null(), "{result}");
    }
    #[test]
    fn actual_source_only_repair_preserves_original_and_audio() {
        run_repair_case("apply");
    }
    #[test]
    fn source_only_repair_cancel_and_stale_preview_have_no_effects() {
        run_repair_case("cancel");
        run_repair_case("stale");
    }
}
