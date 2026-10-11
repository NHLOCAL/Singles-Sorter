use super::*;
use std::time::{Duration, Instant};
fn request(source: String, target: Option<String>, ai_profile: RequestedAi) -> PreviewRequest {
    PreviewRequest {
        mode: if target.is_some() {
            JobMode::Sort
        } else {
            JobMode::FixNames
        },
        source_token: source,
        target_token: target,
        settings: serde_json::from_value(serde_json::to_value(Settings::default()).unwrap())
            .unwrap(),
        ai_profile,
    }
}
fn running(host: &Host, id: &str) -> Cancellation {
    let cancellation = Cancellation::default();
    host.lock().unwrap().job = Some(CompletedJob {
        snapshot: JobSnapshot {
            host_schema: 1,
            job_id: id.into(),
            status: "running",
            inference_executed: None,
            plan_id: None,
            total_rows: 0,
            blockers: vec![],
            warnings: vec![],
            error: None,
        },
        rows: vec![],
        _plan: None,
        cancellation: cancellation.clone(),
    });
    cancellation
}
fn terminal(host: &Host, id: &str) -> JobSnapshot {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let snapshot = host.snapshot(id).unwrap();
        if snapshot.status != "running" && !host.shared.busy.load(Ordering::Acquire) {
            return snapshot;
        }
        assert!(
            Instant::now() < deadline,
            "job failed to reach terminal state"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn cancel_is_scoped_monotonic_and_no_partial_page_exists() {
    let host = Host::default();
    let token = running(&host, "job-1");
    assert_eq!(host.cancel("unknown").unwrap_err().code, "UNKNOWN_JOB");
    assert!(!token.is_cancelled());
    assert_eq!(
        host.page("job-1", 0, 1).err().unwrap().code,
        "JOB_NOT_COMPLETED"
    );
    host.cancel("job-1").unwrap();
    host.cancel("job-1").unwrap();
    assert!(token.checkpoint(PlanningBoundary::Start).is_err());
    #[cfg(feature = "original-ai")]
    assert!(token.recognition.is_cancelled());
    assert_eq!(host.snapshot("job-1").unwrap().status, "running");
    host.finish("job-1", Err(HostError::new("OTHER", "late error")), token)
        .unwrap();
    let snapshot = host.snapshot("job-1").unwrap();
    assert_eq!(snapshot.status, "cancelled");
    assert!(snapshot.plan_id.is_none());
    assert_eq!(snapshot.total_rows, 0);
    host.cancel("job-1").unwrap();
}
#[test]
fn obsolete_worker_cannot_replace_newer_job_and_reload_cancels_owner() {
    let host = Host::default();
    let old = running(&host, "old");
    let current = running(&host, "current");
    host.finish("old", Err(HostError::new("OLD", "old")), old)
        .unwrap();
    assert_eq!(host.snapshot("current").unwrap().status, "running");
    host.cancel_active();
    assert!(current.is_cancelled());
    host.finish("current", Err(HostError::new("FAILED", "failed")), current)
        .unwrap();
    assert_eq!(host.snapshot("current").unwrap().status, "cancelled");
}
#[test]
fn feature_off_or_missing_models_never_advertise_availability() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::with_model_directory(directory.path().join("missing"));
    assert!(!host.model_status().available);
}
#[test]
fn async_job_releases_busy_for_repeat_after_completion() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::default();
    let source = host
        .select_path(Role::Source, directory.path().to_owned())
        .unwrap();
    for _ in 0..3 {
        let job = host
            .start(
                request(source.token.clone(), None, RequestedAi::Disabled),
                vec![],
                host.claim().unwrap(),
            )
            .unwrap();
        assert_eq!(terminal(&host, &job.job_id).status, "completed");
        assert_eq!(host.page(&job.job_id, 0, 1).unwrap().total, 0);
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
#[cfg(feature = "original-ai")]
#[test]
#[ignore = "requires explicit pinned original asset bundle"]
fn original_models_real_plan_cancel_corruption_and_read_only() {
    let bundle =
        PathBuf::from(std::env::var_os("ORIGINAL_AI_BUNDLE").expect("explicit verified bundle"));
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("models");
    std::fs::create_dir(&models).unwrap();
    let files = [
        "manifest.json",
        "tensors.f32",
        "tokenizer.json",
        "classifier-v14.weights.bin",
        "classifier-v14.vocab.txt",
    ];
    for name in files {
        std::fs::copy(bundle.join(name), models.join(name)).unwrap();
    }
    let source = directory.path().join("source");
    let target = directory.path().join("target");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/media/tagged.mp3");
    let bytes = std::fs::read(fixture).unwrap();
    let song = source.join("שירת הלב.mp3");
    std::fs::write(&song, &bytes).unwrap();
    let host = Host::with_model_directory(models.clone());
    assert!(host.model_status().available);
    let s = host.select_path(Role::Source, source.clone()).unwrap();
    let t = host.select_path(Role::Target, target.clone()).unwrap();
    let make = || {
        request(
            s.token.clone(),
            Some(t.token.clone()),
            RequestedAi::Configured,
        )
    };
    let job = host.start(make(), vec![], host.claim().unwrap()).unwrap();
    let snapshot = terminal(&host, &job.job_id);
    assert_eq!(snapshot.status, "completed", "{:?}", snapshot.error);
    assert!(
        snapshot.inference_executed == Some(true),
        "fixture must exercise original inference"
    );
    {
        let inner = host.lock().unwrap();
        let plan = inner.job.as_ref().unwrap()._plan.as_ref().unwrap();
        assert!(!plan.executable);
        assert!(matches!(
            plan.request.ai_profile,
            AiProfile::Configured {
                available: true,
                ..
            }
        ));
        assert!(
            plan.request
                .resource_ids
                .iter()
                .any(|r| r.starts_with("original-ner-tensors:"))
        );
        assert!(
            plan.request
                .resource_ids
                .iter()
                .any(|r| r.starts_with("original-secondary-weights:"))
        );
        assert!(
            !plan
                .request
                .adapter_versions
                .iter()
                .any(|r| r.contains("intentionally-disabled"))
        );
    }
    let job = host.start(make(), vec![], host.claim().unwrap()).unwrap();
    host.cancel(&job.job_id).unwrap();
    host.cancel(&job.job_id).unwrap();
    assert_eq!(terminal(&host, &job.job_id).status, "cancelled");
    assert!(host.page(&job.job_id, 0, 1).is_err());
    for name in files {
        let path = models.join(name);
        let original = std::fs::read(&path).unwrap();
        for corrupt in [false, true] {
            if corrupt {
                std::fs::write(&path, b"corrupt").unwrap();
            } else {
                std::fs::remove_file(&path).unwrap();
            }
            assert!(!host.model_status().available, "{name} corrupt={corrupt}");
            let job = host.start(make(), vec![], host.claim().unwrap()).unwrap();
            let snapshot = terminal(&host, &job.job_id);
            assert_eq!(snapshot.status, "failed");
            assert_eq!(snapshot.blockers, ["RECOGNITION_UNAVAILABLE"]);
            assert!(snapshot.plan_id.is_none());
            assert_eq!(snapshot.total_rows, 0);
            std::fs::write(&path, &original).unwrap();
        }
    }
    assert_eq!(std::fs::read(&song).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(source).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(target).unwrap().count(), 0);
}

#[test]
fn reload_before_start_reply_cannot_launch_orphan_worker() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::default();
    let source = host
        .select_path(Role::Source, directory.path().to_owned())
        .unwrap();
    let guard = host.claim().unwrap();
    host.cancel_active();
    assert_eq!(
        host.start(
            request(source.token, None, RequestedAi::Disabled),
            vec![],
            guard
        )
        .err()
        .unwrap()
        .code,
        "PLANNING_CANCELLED"
    );
    assert!(host.lock().unwrap().job.is_none());
    assert!(host.claim().is_ok());
}
#[test]
fn lifecycle_cancel_also_signals_startup_model_probe() {
    let host = Host::default();
    let cancellation = Cancellation::default();
    host.lock().unwrap().model_probe = Some(cancellation.clone());
    host.cancel_active();
    assert!(cancellation.is_cancelled());
}
