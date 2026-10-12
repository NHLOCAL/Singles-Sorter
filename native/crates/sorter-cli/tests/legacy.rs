//! Real subprocess acceptance for the positional v14-compatible frontend.
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
        .current_dir(cwd)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}
fn tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for directory in ["source", "target", "journal", "app"] {
        fs::create_dir(root.path().join(directory)).unwrap();
    }
    fs::write(
        root.path().join("app/personal-singer-list.csv"),
        "זמר בדיקה,אמן בדיקה\n",
    )
    .unwrap();
    fs::write(
        root.path().join("source/זמר בדיקה - שיר.mp3"),
        include_bytes!("../../../tests/fixtures/media/blank.mp3"),
    )
    .unwrap();
    root
}
#[test]
fn explicit_dry_run_uses_relative_paths_and_personal_csv_without_writes() {
    let root = tree();
    let source = root.path().join("source/זמר בדיקה - שיר.mp3");
    let before = fs::read(&source).unwrap();
    let result = run(
        root.path(),
        &[
            "source",
            "target",
            "-can",
            "--dry-run",
            "--no-ai",
            "-l",
            "ERROR",
        ],
    );
    assert!(result.status.success(), "{result:?}");
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(
        stdout.contains("COPY") && stdout.contains("אמן בדיקה"),
        "{stdout}"
    );
    assert!(stdout.contains("predictions only"));
    assert!(result.stderr.is_empty());
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(fs::read_dir(root.path().join("target")).unwrap().count(), 0);
    assert_eq!(
        fs::read_dir(root.path().join("journal")).unwrap().count(),
        0
    );
    assert!(!root.path().join("logs").exists());
}
#[test]
fn primary_csv_default_override_and_empty_lists_match_the_original_workflow() {
    let root = tree();
    fs::write(root.path().join("app/singer-list.csv"), "זמר בדיקה,ראשי מקומי\n").unwrap();
    fs::write(root.path().join("custom.csv"), "זמר בדיקה,ראשי מפורש\n").unwrap();
    for (extra, expected) in [(vec![], "ראשי מקומי"), (vec!["--singer-list", "custom.csv"], "ראשי מפורש")] {
        let mut args = vec!["source", "target", "-cn", "--dry-run", "--no-ai", "-l", "ERROR"];
        args.extend(extra);
        let result = run(root.path(), &args);
        assert!(result.status.success(), "{result:?}");
        let output = String::from_utf8(result.stdout).unwrap();
        assert!(output.contains(expected), "{output}");
        assert!(!output.contains("אמן בדיקה"), "primary rows precede personal rows: {output}");
    }
    fs::write(root.path().join("app/singer-list.csv"), b"").unwrap();
    fs::write(root.path().join("app/personal-singer-list.csv"), b"").unwrap();
    let result = run(root.path(), &["source", "target", "--dry-run", "--no-ai"]);
    assert_eq!(result.status.code(), Some(2), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stderr).contains("No singer lists found"));
    assert_eq!(fs::read_dir(root.path().join("target")).unwrap().count(), 0);
    assert!(!root.path().join("logs").exists());
}
#[test]
fn argument_failures_never_start_jobs_and_help_does_not_read_stdin() {
    let root = tree();
    for args in [
        vec!["source", "target", "--copy_mode=1"],
        vec!["source", "-l", "debug"],
        vec!["source", "--unknown"],
    ] {
        let result = run(root.path(), &args);
        assert_eq!(result.status.code(), Some(2), "{result:?}");
        assert!(result.stdout.is_empty());
    }
    let result = run(root.path(), &["--help"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    for flag in [
        "--copy_mode",
        "--abc_sort",
        "--exist_only",
        "--no_singles_dir",
        "--main_dir_only",
        "--duet_mode",
        "--fix_names",
        "--log_level",
        "--dry-run",
    ] {
        assert!(help.contains(flag));
    }
    assert_eq!(fs::read_dir(root.path().join("source")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(root.path().join("target")).unwrap().count(), 0);
}
#[cfg(not(all(windows, feature = "selected-windows-execution")))]
#[test]
fn feature_off_never_silently_converts_an_execution_invocation_to_dry_run() {
    let root = tree();
    let result = run(root.path(), &["source", "target", "-c", "--no-ai"]);
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    assert!(result.stdout.is_empty());
    assert_eq!(fs::read_dir(root.path().join("source")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(root.path().join("target")).unwrap().count(), 0);
}
#[cfg(all(windows, feature = "selected-windows-execution"))]
mod selected {
    use super::*;
    #[test]
    fn actual_copy_uses_selected_primary_csv_before_personal_aliases() {
        for explicit in [false, true] {
            let root = tree();
            fs::write(root.path().join("app/singer-list.csv"), "זמר בדיקה,ראשי מקומי\n").unwrap();
            fs::write(root.path().join("custom.csv"), "זמר בדיקה,ראשי מפורש\n").unwrap();
            let mut args = vec!["source", "target", "-cn", "--no-ai", "--journal-directory", "journal"];
            if explicit { args.extend(["--singer-list", "custom.csv"]); }
            let result = run(root.path(), &args);
            assert!(result.status.success(), "{result:?}");
            let artist = if explicit { "ראשי מפורש" } else { "ראשי מקומי" };
            assert_eq!(fs::read(root.path().join("target").join(artist).join("זמר בדיקה - שיר.mp3")).unwrap(), include_bytes!("../../../tests/fixtures/media/blank.mp3"));
            assert!(!root.path().join("target/אמן בדיקה").exists());
        }
    }
    #[test]
    fn positional_copy_and_move_execute_without_any_stdin_handshake() {
        for copying in [true, false] {
            let root = tree();
            let source = root.path().join("source/זמר בדיקה - שיר.mp3");
            let before = fs::read(&source).unwrap();
            let mut args = vec![
                "source",
                "target",
                "-n",
                "--no-ai",
                "--journal-directory",
                "journal",
            ];
            if copying {
                args.push("-c");
            }
            let result = run(root.path(), &args);
            assert!(result.status.success(), "{result:?}");
            assert_eq!(source.exists(), copying);
            assert_eq!(
                fs::read(root.path().join("target/אמן בדיקה/זמר בדיקה - שיר.mp3")).unwrap(),
                before
            );
            assert!(fs::read_dir(root.path().join("journal")).unwrap().count() >= 2);
            assert!(String::from_utf8_lossy(&result.stderr).contains("complete"));
        }
    }
    fn album_audio(track: usize) -> Vec<u8> {
        fn sync(size: usize) -> [u8; 4] { [((size >> 21) & 127) as u8, ((size >> 14) & 127) as u8, ((size >> 7) & 127) as u8, (size & 127) as u8] }
        let mut body = vec![];
        for (id, value) in [(b"TPE1", "שלום עולם".to_owned()), (b"TPE2", "שלום עולם".to_owned()), (b"TALB", "אלבום".to_owned()), (b"TRCK", track.to_string())] {
            let mut payload = vec![3]; payload.extend(value.as_bytes());
            body.extend(id); body.extend(sync(payload.len())); body.extend([0, 0]); body.extend(payload);
        }
        let mut file = b"ID3\x04\x00\x00".to_vec(); file.extend(sync(body.len())); file.extend(body);
        file.extend(include_bytes!("../../../tests/fixtures/media/blank.mp3")); file
    }
    #[test]
    fn positional_album_copy_root_move_and_mixed_move_use_real_metadata() {
        for (copying, root_album) in [(true, false), (false, true), (false, false)] {
            let root = tree(); let source = root.path().join("source");
            fs::write(root.path().join("app/personal-singer-list.csv"), "זמר בדיקה,אמן בדיקה\nשלום עולם,שם ממופה\n").unwrap();
            let album = if root_album {
                fs::remove_file(source.join("זמר בדיקה - שיר.mp3")).unwrap(); source.clone()
            } else { let album = source.join("input-album"); fs::create_dir(&album).unwrap(); album };
            for track in 1..=3 { fs::write(album.join(format!("{track}-שיר.mp3")), album_audio(track)).unwrap(); }
            fs::write(album.join("cover.jpg"), b"owned sidecar").unwrap();
            fs::create_dir(album.join("booklet")).unwrap();
            fs::write(album.join("booklet/readme.txt"), b"owned nested file").unwrap();
            let mut args = vec!["source", "target", "-n", "--no-ai", "--journal-directory", "journal"];
            if copying { args.push("-c"); }
            let result = run(root.path(), &args);
            assert!(result.status.success(), "{result:?}");
            let destination = root.path().join("target/שם ממופה/אלבום");
            for track in 1..=3 { assert_eq!(fs::read(destination.join(format!("{track}-שיר.mp3"))).unwrap(), album_audio(track)); }
            assert_eq!(fs::read(destination.join("cover.jpg")).unwrap(), b"owned sidecar");
            assert_eq!(fs::read(destination.join("booklet/readme.txt")).unwrap(), b"owned nested file");
            assert_eq!(album.exists(), copying);
            let stderr = String::from_utf8(result.stderr).unwrap();
            let actual: serde_json::Value = serde_json::from_str(stderr.lines().find_map(|line| line.split_once(" - MusicSorter - result: ").map(|(_, value)| value)).unwrap()).unwrap();
            assert_eq!(actual["status"], "complete"); assert_eq!(actual["albums_completed"], 1);
            assert_eq!(actual["artist_files"], serde_json::json!([["שלום עולם", 4]]));
            assert_eq!(actual["committed_destinations"], if root_album {5} else {6});
            assert!(actual["recovery_error"].is_null(), "{actual}");
            if !root_album { assert!(root.path().join("target/אמן בדיקה/זמר בדיקה - שיר.mp3").is_file()); }
        }
    }
    #[test]
    fn positional_source_only_fix_retains_original_bytes_and_media() {
        let root = tree();
        fs::remove_file(root.path().join("source/זמר בדיקה - שיר.mp3")).unwrap();
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
        let source = root.path().join("source/batch_repair.mp3");
        fs::write(&source, &original).unwrap();
        let result = run(
            root.path(),
            &["source", "-f", "--no-ai", "--journal-directory", "journal"],
        );
        assert!(result.status.success(), "{result:?}");
        assert!(!source.exists());
        let repaired = fs::read(root.path().join("source/batch repair.mp3")).unwrap();
        assert_ne!(repaired, original);
        assert_eq!(&repaired[137..], audio);
        assert!(
            repaired
                .windows("אבג".len())
                .any(|bytes| bytes == "אבג".as_bytes())
        );
        let backups = fs::read_dir(root.path().join("source"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "backup"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), original);
        assert!(String::from_utf8_lossy(&result.stderr).contains("FixNames"));
    }
    #[test]
    fn explicit_existing_journal_override_and_default_local_appdata_are_bound() {
        let root = tree();
        let output = Command::new(env!("CARGO_BIN_EXE_singles-sorter"))
            .current_dir(root.path())
            .args(["source", "target", "-cn", "--no-ai"])
            .env("LOCALAPPDATA", root.path().join("journal"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(fs::read_dir(root.path().join("journal")).unwrap().count() >= 2);
        let rejected = tree();
        let output = run(
            rejected.path(),
            &[
                "source",
                "target",
                "-c",
                "--no-ai",
                "--journal-directory",
                "source",
            ],
        );
        assert!(!output.status.success());
        assert_eq!(
            fs::read_dir(rejected.path().join("source"))
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            fs::read_dir(rejected.path().join("target"))
                .unwrap()
                .count(),
            0
        );
    }
}
