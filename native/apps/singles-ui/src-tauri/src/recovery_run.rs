//! One read-only worker owns Busy through cleanup. Snapshots carry display data
//! only; a retained resume token changes observation order, never file evidence.
use super::*;
use std::time::Instant;

#[derive(Clone, Debug, Serialize, Default)]
pub struct RecoveryProgress {
    pub stage: String,
    pub operation: Option<usize>,
    pub total_operations: usize,
    pub completed_operations: usize,
    pub records_checked: u64,
    pub observations: u64,
    pub bytes_read: u64,
    pub elapsed_ms: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryRunId {
    pub run_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoverySnapshot {
    pub run_id: String,
    pub kind: &'static str,
    pub status: &'static str,
    pub progress: RecoveryProgress,
    pub page: Option<RecoveryPage>,
    pub report: Option<RecoveryReport>,
    pub error: Option<HostError>,
}
impl RecoverySnapshot {
    fn running(&self) -> bool {
        matches!(self.status, "running" | "cancelling")
    }
}
pub(in crate::job_owner) struct RecoveryRun {
    snapshot: RecoverySnapshot,
    cancellation: Cancellation,
    started: Instant,
    #[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
    identity: Option<(String, String, Option<String>)>,
}
impl RecoveryRun {
    pub(in crate::job_owner) fn cancel(&mut self) {
        if self.snapshot.running() {
            self.cancellation.cancel();
            self.snapshot.status = "cancelling";
        }
    }
}
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
pub(super) enum RecoveryOutput {
    Page(RecoveryPage),
    Report(RecoveryReport),
}
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
struct RecoveryLease {
    host: Host,
    id: String,
    guard: Option<BusyGuard>,
}
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
impl RecoveryLease {
    fn finish(mut self, result: Result<RecoveryOutput>) {
        if let Some(guard) = self.guard.take() {
            self.host.finish_recovery_run(&self.id, result, guard);
        }
    }
}
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
impl Drop for RecoveryLease {
    fn drop(&mut self) {
        if let Some(guard) = self.guard.take() {
            self.host.finish_recovery_run(
                &self.id,
                Err(HostError::new(
                    "WORKER_FAILED",
                    "בדיקת היומן נעצרה. לא בוצעו שינויים.",
                )),
                guard,
            );
        }
    }
}
impl Host {
    pub fn recovery_snapshot(&self, id: &str) -> Result<RecoverySnapshot> {
        let inner = self.lock()?;
        let run = inner
            .recovery
            .run
            .as_ref()
            .filter(|run| run.snapshot.run_id == id)
            .ok_or_else(|| {
                HostError::new(
                    "UNKNOWN_RECOVERY_RUN",
                    "הבדיקה אינה שמורה עוד. יש לפתוח בדיקה חדשה.",
                )
            })?;
        let mut snapshot = run.snapshot.clone();
        if snapshot.running() {
            snapshot.progress.elapsed_ms = u64::try_from(run.started.elapsed().as_millis())
                .map_err(|_| HostError::new("RECOVERY_LIMIT", "משך הבדיקה חרג מהמגבלה."))?;
        }
        Ok(snapshot)
    }
    pub fn cancel_recovery(&self, id: &str) -> Result<()> {
        let mut inner = self.lock()?;
        let run = inner
            .recovery
            .run
            .as_mut()
            .filter(|run| run.snapshot.run_id == id)
            .ok_or_else(|| HostError::new("UNKNOWN_RECOVERY_RUN", "הבדיקה אינה שמורה עוד."))?;
        run.cancel();
        Ok(())
    }
    #[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
    pub(super) fn update_recovery_progress(&self, id: &str, progress: RecoveryProgress) {
        if let Ok(mut inner) = self.lock() {
            if let Some(run) = inner
                .recovery
                .run
                .as_mut()
                .filter(|run| run.snapshot.run_id == id && run.snapshot.running())
            {
                run.snapshot.progress = progress;
            }
        }
    }
    #[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
    pub(super) fn start_recovery_run(
        &self,
        kind: &'static str,
        identity: Option<(String, String, Option<String>)>,
        resume: Option<&str>,
        guard: BusyGuard,
        work: impl FnOnce(&Cancellation, &Host, &str, usize) -> Result<RecoveryOutput> + Send + 'static,
    ) -> Result<RecoveryRunId> {
        let id = self.next("recovery-run");
        let cancellation = Cancellation::default();
        let mut inner = self.lock()?;
        // Close/reload may win after claim but before this start is registered.
        if guard.lifecycle != self.shared.lifecycle.load(Ordering::Acquire) {
            return Err(HostError::new("CANCELLED", "הבדיקה בוטלה לפני שהחלה."));
        }
        let cursor = if let Some(resume) = resume {
            let previous = inner
                .recovery
                .run
                .as_ref()
                .filter(|run| {
                    run.snapshot.run_id == resume
                        && run.identity == identity
                        && kind == "inspect"
                        && matches!(run.snapshot.status, "cancelled" | "failed")
                })
                .ok_or_else(|| {
                    HostError::new(
                        "INVALID_RECOVERY_RESUME",
                        "יש להתחיל בדיקה חדשה עם אותן בחירות.",
                    )
                })?;
            previous.snapshot.progress.operation.unwrap_or(0)
        } else {
            0
        };
        inner.recovery.run = Some(RecoveryRun {
            snapshot: RecoverySnapshot {
                run_id: id.clone(),
                kind,
                status: "running",
                progress: RecoveryProgress {
                    stage: "queued".into(),
                    ..RecoveryProgress::default()
                },
                page: None,
                report: None,
                error: None,
            },
            cancellation: cancellation.clone(),
            started: Instant::now(),
            identity,
        });
        drop(inner);
        let lease = RecoveryLease {
            host: self.clone(),
            id: id.clone(),
            guard: Some(guard),
        };
        // A failed spawn drops the lease and records failure before releasing Busy.
        let _worker = std::thread::Builder::new()
            .name("read-only-restart-inspection".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    work(&cancellation, &lease.host, &lease.id, cursor)
                }))
                .unwrap_or_else(|_| {
                    Err(HostError::new(
                        "WORKER_FAILED",
                        "בדיקת היומן נעצרה. לא בוצעו שינויים.",
                    ))
                });
                lease.finish(result);
            });
        Ok(RecoveryRunId { run_id: id })
    }
    #[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
    fn finish_recovery_run(&self, id: &str, result: Result<RecoveryOutput>, guard: BusyGuard) {
        if let Ok(mut inner) = self.lock() {
            if let Some(run) = inner
                .recovery
                .run
                .as_mut()
                .filter(|run| run.snapshot.run_id == id && run.snapshot.running())
            {
                // Work has returned/unwound, so all read handles are already gone.
                let elapsed = u64::try_from(run.started.elapsed().as_millis());
                let result = match elapsed {
                    Ok(elapsed) => {
                        run.snapshot.progress.elapsed_ms = elapsed;
                        result
                    }
                    Err(_) => Err(HostError::new("RECOVERY_LIMIT", "משך הבדיקה חרג מהמגבלה.")),
                };
                if run.cancellation.is_cancelled()
                    || result
                        .as_ref()
                        .is_err_and(|error| error.code == "CANCELLED")
                {
                    run.snapshot.status = "cancelled";
                    run.snapshot.error = Some(HostError::new(
                        "CANCELLED",
                        "הבדיקה בוטלה. הקבצים שלא נבדקו עדיין דורשים בדיקה; היומנים נשמרו.",
                    ));
                } else {
                    match result {
                        Ok(RecoveryOutput::Page(page)) => {
                            run.snapshot.status = "completed";
                            run.snapshot.page = Some(page);
                        }
                        Ok(RecoveryOutput::Report(report)) => {
                            run.snapshot.status = "completed";
                            run.snapshot.report = Some(report);
                        }
                        Err(error) => {
                            run.snapshot.status = "failed";
                            run.snapshot.error = Some(error);
                        }
                    }
                }
            }
            // A new owner cannot replace the terminal snapshot until cleanup and
            // this release have both happened under the same state lock.
            drop(guard);
        }
    }
}

#[cfg(all(windows, feature = "selected-windows-execution"))]
impl From<sorter_executor::inspection_control::InspectionProgress> for RecoveryProgress {
    fn from(progress: sorter_executor::inspection_control::InspectionProgress) -> Self {
        Self {
            stage: progress.stage.into(),
            operation: progress.operation,
            total_operations: progress.total_operations,
            completed_operations: progress.completed_operations,
            records_checked: progress.records_checked,
            observations: progress.observations,
            bytes_read: progress.bytes_read,
            elapsed_ms: progress.elapsed_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, time::Duration};
    fn empty_page() -> RecoveryOutput {
        RecoveryOutput::Page(RecoveryPage {
            records: vec![],
            total: 0,
            offset: 0,
            truncated: false,
        })
    }
    fn terminal(host: &Host, id: &str) -> RecoverySnapshot {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = host.recovery_snapshot(id).unwrap();
            if !snapshot.running() {
                return snapshot;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn close_before_start_registration_releases_owner_without_running_work() {
        let host = Host::default();
        let guard = host.claim().unwrap();
        host.cancel_active();
        assert_eq!(
            host.start_recovery_run("list", None, None, guard, |_, _, _, _| panic!(
                "late start ran"
            ))
            .unwrap_err()
            .code,
            "CANCELLED"
        );
        assert!(host.claim().is_ok());
    }
    #[test]
    fn close_cancels_running_inspection_and_terminal_snapshot_is_immutable() {
        let host = Host::default();
        let (entered, started) = mpsc::channel();
        let (release, pending) = mpsc::channel();
        let id = host
            .start_recovery_run(
                "list",
                None,
                None,
                host.claim().unwrap(),
                move |_, _, _, _| {
                    entered.send(()).unwrap();
                    pending.recv().unwrap();
                    Ok(empty_page())
                },
            )
            .unwrap()
            .run_id;
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        host.cancel_active();
        assert_eq!(host.recovery_snapshot(&id).unwrap().status, "cancelling");
        assert!(host.claim().is_err());
        release.send(()).unwrap();
        let snapshot = terminal(&host, &id);
        assert_eq!(snapshot.status, "cancelled");
        assert!(snapshot.page.is_none());
        let before = serde_json::to_value(&snapshot).unwrap();
        host.cancel_recovery(&id).unwrap();
        host.update_recovery_progress(
            &id,
            RecoveryProgress {
                bytes_read: 99,
                ..RecoveryProgress::default()
            },
        );
        assert_eq!(
            serde_json::to_value(host.recovery_snapshot(&id).unwrap()).unwrap(),
            before
        );
        assert!(host.claim().is_ok());
    }
    #[test]
    fn resume_requires_same_tokens_and_stale_cancel_cannot_touch_new_run() {
        let host = Host::default();
        let identity = Some(("record".into(), "source".into(), Some("target".into())));
        let (entered, started) = mpsc::channel();
        let (release, pending) = mpsc::channel();
        let id = host
            .start_recovery_run(
                "inspect",
                identity.clone(),
                None,
                host.claim().unwrap(),
                move |_, host, id, _| {
                    host.update_recovery_progress(
                        id,
                        RecoveryProgress {
                            operation: Some(2),
                            total_operations: 3,
                            ..RecoveryProgress::default()
                        },
                    );
                    entered.send(()).unwrap();
                    pending.recv().unwrap();
                    Ok(empty_page())
                },
            )
            .unwrap()
            .run_id;
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        host.cancel_recovery(&id).unwrap();
        release.send(()).unwrap();
        assert_eq!(terminal(&host, &id).status, "cancelled");
        assert_eq!(
            host.start_recovery_run(
                "inspect",
                Some(("other".into(), "source".into(), Some("target".into()))),
                Some(&id),
                host.claim().unwrap(),
                |_, _, _, _| panic!()
            )
            .unwrap_err()
            .code,
            "INVALID_RECOVERY_RESUME"
        );
        let (entered, started) = mpsc::channel();
        let (release, pending) = mpsc::channel();
        let next = host
            .start_recovery_run(
                "inspect",
                identity,
                Some(&id),
                host.claim().unwrap(),
                move |_, _, _, cursor| {
                    assert_eq!(cursor, 2);
                    entered.send(()).unwrap();
                    pending.recv().unwrap();
                    Ok(RecoveryOutput::Report(RecoveryReport {
                        job_id: "record".into(),
                        state: "needs_attention",
                        observed_directories: 0,
                        pending_directories: 0,
                        rejected_directories: 0,
                        incomplete_tail: false,
                        unverified_directories: 0,
                        observation_limit_reached: false,
                        operations: vec![],
                        evidence: vec![],
                    }))
                },
            )
            .unwrap()
            .run_id;
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(host.cancel_recovery(&id).is_err());
        assert_eq!(host.recovery_snapshot(&next).unwrap().status, "running");
        release.send(()).unwrap();
        assert_eq!(terminal(&host, &next).status, "completed");
    }
    #[test]
    fn panicking_worker_records_failure_and_releases_busy_once() {
        let host = Host::default();
        let id = host
            .start_recovery_run("list", None, None, host.claim().unwrap(), |_, _, _, _| {
                panic!("fixture worker failure")
            })
            .unwrap()
            .run_id;
        assert_eq!(terminal(&host, &id).status, "failed");
        let next = host.claim().unwrap();
        host.cancel_recovery(&id).unwrap();
        assert!(host.claim().is_err());
        drop(next);
        assert!(host.claim().is_ok());
    }
}
