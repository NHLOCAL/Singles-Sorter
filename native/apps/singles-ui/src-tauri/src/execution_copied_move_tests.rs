//! Actual source/target volumes and auxiliary streams, exclusively owned fixtures.
use super::*;
use serde_json::json;
use std::fs;

#[test]
fn cross_volume_host_move_retains_preview_streams_and_exact_journal_root() {
    for action in ["apply", "stale_stream", "stale_journal"] {
        let tree = tempfile::tempdir().unwrap();
        let target_tree = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
        let source = tree.path().canonicalize().unwrap().join("source");
        let journal = tree.path().canonicalize().unwrap().join("journal");
        let target = target_tree.path().canonicalize().unwrap().join("target");
        for path in [&source, &target, &journal] { fs::create_dir(path).unwrap(); }
        let name = "אברהם פריד - owned.mp3";
        let audio = include_bytes!("../../../../tests/fixtures/media/blank.mp3");
        let file = source.join(name);
        let stream = source.join(format!("{name}:notes"));
        fs::write(&file, audio).unwrap();
        fs::write(&stream, b"owned auxiliary").unwrap();
        let source_identity = inspect_root(&source).unwrap().identity.unwrap();
        let target_identity = inspect_root(&target).unwrap().identity.unwrap();
        match (&source_identity, &target_identity) {
            (sorter_core::discovery::ObjectIdentity::Windows { volume: a, .. }, sorter_core::discovery::ObjectIdentity::Windows { volume: b, .. }) => assert_ne!(a, b, "CI must supply two actual volumes"),
            _ => panic!("Expected actual Windows identities"),
        }
        let host = Host::default();
        host.set_journal_directory(journal.clone()).unwrap();
        let source_token = host.select_path(Role::Source, source.clone()).unwrap().token;
        let target_token = host.select_path(Role::Target, target.clone()).unwrap().token;
        let request = serde_json::from_value(json!({"mode":"Sort","source_token":source_token,"target_token":target_token,
            "settings":{"copy_mode":false,"main_folder_only":true,"singles_folder":false,"abc_sort":false,"exist_only":false,"duet_mode":false},"ai_profile":"Disabled"})).unwrap();
        let job = host.preview_with_personal(request, Some(vec![])).unwrap();
        let preview = host.snapshot(&job.job_id).unwrap();
        assert_eq!(preview.status, "completed", "{preview:?}");
        assert!(preview.apply.as_ref().is_some_and(|ready| ready.supported && ready.notice.is_some()), "{:?}", preview.apply);
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
        if action == "stale_stream" {
            let modified = fs::metadata(&file).unwrap().modified().unwrap();
            fs::write(&stream, b"changed stream!").unwrap();
            fs::File::options().write(true).open(&file).unwrap()
                .set_times(fs::FileTimes::new().set_modified(modified)).unwrap();
        } else if action == "stale_journal" {
            fs::rename(&journal, tree.path().join("old-journal")).unwrap();
            fs::create_dir(&journal).unwrap();
        }
        host.start_apply(&job.job_id, &preview.plan_id.unwrap(), vec![], host.claim().unwrap()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        let outcome = loop {
            let value = host.execution_snapshot(&job.job_id).unwrap();
            if !value.is_running() { break value; }
            assert!(std::time::Instant::now() < deadline, "copied Move host timed out");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        if action == "apply" {
            assert_eq!(outcome.status, "complete", "{outcome:?}");
            assert!(outcome.recovery_error.is_none(), "{outcome:?}");
            assert!(outcome.verification.as_ref().is_some_and(|progress| progress.bytes_read > 0 && progress.completed_operations == progress.total_operations), "{outcome:?}");
            let result = outcome.result.unwrap();
            assert_eq!((result.committed_destinations, result.moved_sources), (1, 1));
            let destination = target.join("אברהם פריד").join(name);
            assert_eq!(fs::read(&destination).unwrap(), audio);
            assert_eq!(fs::read(target.join("אברהם פריד").join(format!("{name}:notes"))).unwrap(), b"owned auxiliary");
            assert!(!file.exists());
        } else {
            assert_eq!(outcome.status, "failed", "{outcome:?}");
            assert!(outcome.result.is_none());
            assert_eq!(fs::read(&file).unwrap(), audio);
            assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
            assert_eq!(fs::read_dir(&journal).unwrap().count(), 0);
            if action == "stale_journal" { assert_eq!(fs::read_dir(tree.path().join("old-journal")).unwrap().count(), 0); }
        }
        loop {
            if let Ok(_guard) = host.claim() { break; }
            assert!(std::time::Instant::now() < deadline, "host did not release operation ownership");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
