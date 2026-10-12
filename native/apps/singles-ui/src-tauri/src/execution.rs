//! Retained preview -> one-shot native execution. The renderer never supplies
//! paths, settings, operation lists, resource hashes, or serialized authority.
use super::*;
use sorter_core::discovery::{ReadInput, StorageError};
#[cfg(all(windows, feature = "selected-windows-execution"))]
#[path = "execution_repair.rs"]
mod repair;

pub(super) struct ReadAdapter(pub Box<dyn ReadStorage>);
impl ReadStorage for ReadAdapter {
    fn stat(
        &mut self,
        reference: &StorageRef,
    ) -> std::result::Result<EntryObservation, StorageError> {
        self.0.stat(reference)
    }
    fn children(
        &mut self,
        reference: &StorageRef,
    ) -> std::result::Result<Vec<EntryObservation>, StorageError> {
        self.0.children(reference)
    }
    fn open_read(
        &mut self,
        reference: &StorageRef,
    ) -> std::result::Result<ReadInput, StorageError> {
        self.0.open_read(reference)
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ApplyReadiness {
    pub supported: bool,
    pub reason: Option<String>,
    pub operation_count: usize,
    pub copy_mode: bool,
    pub source: String,
    pub target: Option<String>,
    pub mode: JobMode,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionItem {
    pub operation: usize,
    pub status: &'static str,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairOutcome>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RepairOutcome {
    pub renamed: bool,
    pub repaired: bool,
    pub unchanged: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct RepairSummary {
    pub total_files: usize,
    pub processed_files: usize,
    pub renamed_files: usize,
    pub repaired_files: usize,
    pub unchanged_files: usize,
    pub unsupported_files: usize,
    pub failed_files: usize,
    pub retained_backup_bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionResult {
    pub source_items: usize,
    pub committed_destinations: usize,
    pub moved_sources: usize,
    pub failed_operations: usize,
    pub blocked_operations: usize,
    pub recovery_operations: usize,
    pub cancelled_operations: usize,
    pub items: Vec<ExecutionItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairSummary>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionSnapshot {
    pub host_schema: u16,
    pub job_id: String,
    pub plan_id: String,
    pub status: &'static str,
    pub operation_count: usize,
    pub operation: Option<usize>,
    pub bytes_written: u64,
    pub source_length: u64,
    pub journal_path: Option<String>,
    pub error: Option<String>,
    pub result: Option<ExecutionResult>,
    pub recovery: Option<RecoveryInspection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_error: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryInspection {
    pub observed_directories: usize,
    pub pending_directories: usize,
    pub rejected_directories: usize,
    pub incomplete_tail: bool,
    pub transfers: Vec<RecoveryItem>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryItem {
    pub operation: usize,
    pub status: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct RetainedExecution {
    pub preview: JobSnapshot,
    pub execution: ExecutionSnapshot,
}
impl ExecutionSnapshot {
    pub(super) fn is_running(&self) -> bool {
        matches!(self.status, "preparing" | "running")
    }
}
impl Host {
    pub fn latest_execution(&self) -> Result<Option<RetainedExecution>> {
        let inner = self.lock()?;
        Ok(inner.job.as_ref().and_then(|job| {
            job.execution.as_ref().map(|execution| RetainedExecution {
                preview: job.snapshot.clone(),
                execution: execution.clone(),
            })
        }))
    }
    pub fn set_journal_directory(&self, directory: PathBuf) -> Result<()> {
        validate_path(&directory)?;
        self.lock()?.journal_directory = Some(directory);
        Ok(())
    }
    pub fn execution_snapshot(&self, id: &str) -> Result<ExecutionSnapshot> {
        self.lock()?
            .job
            .as_ref()
            .filter(|job| job.snapshot.job_id == id)
            .ok_or_else(|| HostError::new("UNKNOWN_JOB", "Job is no longer retained"))?
            .execution
            .clone()
            .ok_or_else(|| HostError::new("APPLY_NOT_STARTED", "Execution was not started"))
    }
    pub fn cancel_apply(&self, id: &str) -> Result<()> {
        let inner = self.lock()?;
        let job = inner
            .job
            .as_ref()
            .filter(|job| job.snapshot.job_id == id)
            .ok_or_else(|| HostError::new("UNKNOWN_JOB", "Job is no longer retained"))?;
        if job
            .execution
            .as_ref()
            .is_some_and(ExecutionSnapshot::is_running)
        {
            job.cancellation.cancel();
        }
        Ok(())
    }
    #[cfg(not(all(windows, feature = "selected-windows-execution")))]
    pub fn start_apply(&self, _: &str, _: &str, _: Vec<u8>, _: BusyGuard) -> Result<JobId> {
        Err(HostError::new(
            "APPLY_UNAVAILABLE",
            "ביצוע קבצים אינו זמין במארח זה.",
        ))
    }
}
#[cfg(all(windows, feature = "selected-windows-execution"))]
mod selected {
    use super::*;
    use sorter_executor::{
        binding::{ExecutionScope, Resources, prepare_scope},
        results::{ItemStatus, JobResult, JobStatus},
        windows_job::{SelectedWindowsJob, supported_plan},
        windows_repair::{RepairScope, prepare_repair_scope, supported_repair_plan},
    };
    use sorter_io::desktop::windows_mutation::{WindowsMutationStorage, WindowsSelectedRootGrant};
    #[derive(Clone)]
    pub(crate) struct PreparedSelection {
        source: PathBuf,
        personal: Vec<u8>,
        ai: RequestedAi,
        scope: PreparedScope,
    }
    #[derive(Clone)]
    enum PreparedScope {
        Sort {
            target: PathBuf,
            scope: ExecutionScope,
        },
        Repair(RepairScope),
    }
    impl PreparedSelection {
        fn resources(&self) -> &Resources {
            match &self.scope {
                PreparedScope::Sort { scope, .. } => &scope.resources,
                PreparedScope::Repair(scope) => scope.resources(),
            }
        }
    }
    fn digest(value: &[u8]) -> String {
        format!("{:x}", Sha256::digest(value))
    }
    // IDs come exclusively from verified resources/loaders in this process.
    // Adapter evidence hashes descriptors, not executable binary bytes. The
    // model hash is the ordered four-artifact ID aggregate, not a fifth asset.
    fn evidence(plan: &JobPlanV2) -> Result<Resources> {
        let resource_hashes = plan
            .request
            .resource_ids
            .iter()
            .map(|id| {
                id.rsplit_once(":sha256:")
                    .map(|(_, hash)| hash.to_owned())
                    .filter(|hash| {
                        hash.len() == 64
                            && hash
                                .bytes()
                                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    })
                    .ok_or_else(|| {
                        HostError::new(
                            "RESOURCE_INVALID",
                            "Resource has no verified content digest",
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let models = plan
            .request
            .resource_ids
            .iter()
            .filter(|id| id.starts_with("original-"))
            .collect::<Vec<_>>();
        Ok(Resources {
            resource_ids: plan.request.resource_ids.clone(),
            adapter_versions: plan.request.adapter_versions.clone(),
            ai_profile: plan.request.ai_profile.clone(),
            resource_hashes,
            adapter_hashes: plan
                .request
                .adapter_versions
                .iter()
                .map(|s| digest(s.as_bytes()))
                .collect(),
            model_hash: (!matches!(plan.request.ai_profile, AiProfile::Disabled))
                .then(|| digest(&serde_json::to_vec(&models).expect("string identifiers"))),
        })
    }
    pub(crate) fn inspect_selection(
        plan: &JobPlanV2,
        source: &Root,
        target: Option<&Root>,
        personal: Option<&[u8]>,
        ai: RequestedAi,
        storage: &mut dyn ReadStorage,
        cancellation: &Cancellation,
    ) -> Result<(Option<ApplyReadiness>, Option<PreparedSelection>)> {
        let inspected = (|| {
            let resources = evidence(plan)?;
            if plan.request.mode == JobMode::FixNames {
                supported_repair_plan(plan).map_err(storage_error)?;
                let scope =
                    prepare_repair_scope(plan, storage, &resources, &cancellation.cancelled)
                        .map_err(storage_error)?;
                return Ok(PreparedSelection {
                    source: source.path.clone(),
                    personal: personal.unwrap_or_default().to_vec(),
                    ai,
                    scope: PreparedScope::Repair(scope),
                });
            }
            let target =
                target.ok_or_else(|| HostError::new("TARGET_REQUIRED", "בחר תיקיית יעד למיון."))?;
            supported_plan(plan).map_err(storage_error)?;
            let scope = prepare_scope(plan, storage, &resources, &cancellation.cancelled)
                .map_err(storage_error)?;
            if !plan.request.settings.copy_mode
                && !matches!((&scope.source_root, &scope.target_root), (sorter_core::discovery::ObjectIdentity::Windows {volume:a,..},sorter_core::discovery::ObjectIdentity::Windows {volume:b,..}) if a == b)
            {
                return Err(HostError::new(
                    "CROSS_VOLUME_MOVE_UNAVAILABLE",
                    "העברה בין כוננים אינה זמינה עדיין. אפשר לבחור העתקה ולבדוק מחדש.",
                ));
            }
            Ok(PreparedSelection {
                source: source.path.clone(),
                personal: personal.unwrap_or_default().to_vec(),
                ai,
                scope: PreparedScope::Sort {
                    target: target.path.clone(),
                    scope,
                },
            })
        })();
        if cancellation.is_cancelled() {
            return Err(HostError::new("PLANNING_CANCELLED", "הבדיקה בוטלה."));
        }
        let (selected, reason) = match inspected {
            Ok(selected) => (Some(selected), None),
            Err(error) => (
                None,
                Some(if error.code == "CROSS_VOLUME_MOVE_UNAVAILABLE" {
                    error.message
                } else {
                    format!(
                        "התוכנית כוללת פעולות שאינן זמינות לביצוע בטוח במארח זה ({})",
                        error.message
                    )
                }),
            ),
        };
        Ok((
            Some(ApplyReadiness {
                supported: selected.is_some(),
                reason,
                operation_count: plan.operations.len(),
                copy_mode: plan.request.settings.copy_mode,
                source: source.selection.display_name.clone(),
                target: target.map(|root| root.selection.display_name.clone()),
                mode: plan.request.mode,
            }),
            selected,
        ))
    }
    impl Host {
        pub fn start_apply(
            &self,
            id: &str,
            plan_id: &str,
            personal: Vec<u8>,
            guard: BusyGuard,
        ) -> Result<JobId> {
            let mut inner = self.lock()?;
            if guard.lifecycle != self.shared.lifecycle.load(Ordering::Acquire) {
                return Err(HostError::new(
                    "PLANNING_CANCELLED",
                    "The owning view was closed or reloaded",
                ));
            }
            let directory = inner.journal_directory.clone().ok_or_else(|| {
                HostError::new(
                    "JOURNAL_UNAVAILABLE",
                    "Application journal directory is unavailable",
                )
            })?;
            let job = inner
                .job
                .as_mut()
                .filter(|job| job.snapshot.job_id == id)
                .ok_or_else(|| HostError::new("UNKNOWN_JOB", "Plan is no longer retained"))?;
            if job.snapshot.status != "completed"
                || job.snapshot.plan_id.as_deref() != Some(plan_id)
                || job.execution.is_some()
            {
                return Err(HostError::new(
                    "PLAN_CHANGED",
                    "יש לבצע בדיקה חדשה לפני הביצוע.",
                ));
            }
            let selected = job.selected.as_ref().ok_or_else(|| {
                HostError::new("APPLY_UNAVAILABLE", "תוכנית זו אינה זמינה לביצוע.")
            })?;
            if selected.personal != personal {
                return Err(HostError::new(
                    "LIST_CHANGED",
                    "רשימת האמנים השתנתה. יש לבדוק מחדש.",
                ));
            }
            let selected = job.selected.take().expect("checked retained selection");
            let plan = job._plan.as_ref().expect("completed retained plan").clone();
            let cancellation = Cancellation::default();
            job.cancellation = cancellation.clone();
            job.execution = Some(ExecutionSnapshot {
                host_schema: 1,
                job_id: id.into(),
                plan_id: plan_id.into(),
                status: "preparing",
                operation_count: plan.operations.len(),
                operation: None,
                bytes_written: 0,
                source_length: 0,
                journal_path: Some(directory.to_string_lossy().into_owned()),
                error: None,
                result: None,
                recovery: None,
                recovery_error: None,
            });
            drop(inner);
            let host = self.clone();
            let job_id = id.to_owned();
            let returned_id = job_id.clone();
            let spawned = std::thread::Builder::new()
                .name("selected-windows-execution".into())
                .spawn(move || {
                    let _guard = guard;
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        host.execute_selected(&job_id, &plan, selected, directory, &cancellation)
                    }));
                    if result.is_err() {
                        host.execution_error(
                            &job_id,
                            "needs_recovery",
                            "הפעולה הופסקה באופן בלתי צפוי. יש לבדוק את היומן לפני פעולה נוספת.",
                        );
                    } else if let Ok(Err(error)) = result {
                        host.execution_error(&job_id, "failed", &error.message);
                    }
                });
            if let Err(error) = spawned {
                self.execution_error(id, "failed", &error.to_string());
            }
            Ok(JobId {
                job_id: returned_id,
            })
        }
        pub(super) fn execution_error(&self, id: &str, status: &'static str, error: &str) {
            if let Ok(mut inner) = self.lock() {
                if let Some(e) = inner
                    .job
                    .as_mut()
                    .filter(|j| j.snapshot.job_id == id)
                    .and_then(|j| j.execution.as_mut())
                {
                    e.status = status;
                    e.error = Some(error.into());
                }
            }
        }
        fn execute_selected(
            &self,
            id: &str,
            plan: &JobPlanV2,
            selected: PreparedSelection,
            directory: PathBuf,
            cancellation: &Cancellation,
        ) -> Result<()> {
            if cancellation.is_cancelled() {
                self.execution_error(id, "cancelled", "הביצוע בוטל לפני הפעלתו.");
                return Ok(());
            }
            let (mut recognition, model_resources, _) =
                recognition(&selected.ai, self.model_directory.as_deref(), cancellation)?;
            let aliases = load_aliases(BUNDLED, Some(&selected.personal)).map_err(storage_error)?;
            let mut ids = aliases
                .source_hashes
                .iter()
                .map(|s| {
                    format!(
                        "{}:sha256:{}",
                        match s.source {
                            sorter_io::resources::AliasSource::Bundled => "bundled",
                            sorter_io::resources::AliasSource::Personal => "personal",
                        },
                        s.sha256
                    )
                })
                .collect::<Vec<_>>();
            ids.extend(model_resources);
            if ids != selected.resources().resource_ids
                || recognition.profile() != selected.resources().ai_profile
            {
                return Err(HostError::new(
                    "RESOURCE_CHANGED",
                    "Resources changed; inspect a fresh plan",
                ));
            }
            let (target, scope) = match selected.scope {
                PreparedScope::Repair(scope) => {
                    return self.execute_repair(
                        id,
                        plan,
                        &selected.source,
                        directory,
                        &scope,
                        recognition.as_mut(),
                        cancellation,
                    );
                }
                PreparedScope::Sort { target, scope } => (target, scope),
            };
            // Startup profile access already established this application directory.
            // Never create a path selected relative to a music tree here.
            let journal = inspect_root(&directory)?;
            let grant = |path, identity| {
                WindowsSelectedRootGrant::new(path, identity).map_err(storage_error)
            };
            let roots = WindowsMutationStorage::for_selected_roots(
                grant(selected.source.clone(), scope.source_root.clone())?,
                grant(target.clone(), scope.target_root.clone())?,
                grant(
                    directory.clone(),
                    journal.identity.ok_or_else(|| {
                        HostError::new("JOURNAL_UNAVAILABLE", "Journal root identity missing")
                    })?,
                )?,
            )
            .map_err(storage_error)?;
            let mut reads = sorter_executor::windows_copy::WindowsPlanningStorage::for_preview(
                &selected.source,
                &target,
            )
            .map_err(storage_error)?;
            let mut metadata = StorageMetadataProvider {
                storage: &mut reads,
                reader: MetadataReader,
            };
            let mut job = SelectedWindowsJob::prepare(
                roots,
                plan,
                &mut metadata,
                recognition.as_mut(),
                &scope.resources,
                &scope,
            )
            .map_err(storage_error)?;
            let journal_path = directory.to_string_lossy().into_owned();
            let recovery_scope = sorter_executor::binding::RecoveryScope {
                original_scope: scope.clone(),
                inspected_binding_sha256: job.binding_sha256().map_err(storage_error)?,
            };
            if let Ok(mut inner) = self.lock() {
                if let Some(e) = inner
                    .job
                    .as_mut()
                    .filter(|j| j.snapshot.job_id == id)
                    .and_then(|j| j.execution.as_mut())
                {
                    e.status = "running";
                    e.journal_path = Some(journal_path);
                }
            }
            let result = job.run(&cancellation.cancelled, &mut |progress| {
                if let Ok(mut inner) = self.lock() {
                    if let Some(e) = inner
                        .job
                        .as_mut()
                        .filter(|j| j.snapshot.job_id == id)
                        .and_then(|j| j.execution.as_mut())
                    {
                        e.operation = Some(progress.operation);
                        e.bytes_written = progress.bytes_written;
                        e.source_length = progress.source_length;
                    }
                }
            });
            let recovered = if job.journal_locations().is_empty() {
                None
            } else {
                job.inspect_recovery(&recovery_scope)
                    .ok()
                    .map(|report| RecoveryInspection {
                        observed_directories: report.directories.observed.len(),
                        pending_directories: report.directories.pending.len(),
                        rejected_directories: report.directories.rejected.len(),
                        incomplete_tail: report.directories.incomplete_tail,
                        transfers: report
                            .transfers
                            .map(|transfers| {
                                transfers
                                    .operations
                                    .into_iter()
                                    .map(|operation| RecoveryItem {
                                        operation: operation.operation,
                                        status: format!("{:?}", operation.status),
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
            };
            if let Ok(mut inner) = self.lock() {
                if let Some(e) = inner
                    .job
                    .as_mut()
                    .filter(|j| j.snapshot.job_id == id)
                    .and_then(|j| j.execution.as_mut())
                {
                    e.recovery = recovered;
                }
            }
            match result {
                Ok(result) => {
                    let mut inner = self.lock()?;
                    if let Some(e) = inner
                        .job
                        .as_mut()
                        .filter(|j| j.snapshot.job_id == id)
                        .and_then(|j| j.execution.as_mut())
                    {
                        e.status = status(&result);
                        e.result = Some(project(result));
                        e.operation = None;
                    }
                }
                Err(error) => self.execution_error(
                    id,
                    if job.journal_locations().is_empty() && cancellation.is_cancelled() {
                        "cancelled"
                    } else {
                        "needs_recovery"
                    },
                    &format!("הביצוע נעצר: {error}. יש לבדוק את יומן הפעולה."),
                ),
            }
            Ok(())
        }
    }
    fn status(result: &JobResult) -> &'static str {
        match result.status {
            JobStatus::Complete => "complete",
            JobStatus::Partial => "partial",
            JobStatus::Cancelled => "cancelled",
            JobStatus::NeedsRecovery => "needs_recovery",
        }
    }
    fn project(result: JobResult) -> ExecutionResult {
        ExecutionResult {
            source_items: result.source_items,
            committed_destinations: result.committed_destinations,
            moved_sources: result.moved_sources,
            failed_operations: result.failed_operations,
            blocked_operations: result.blocked_operations,
            recovery_operations: result.recovery_operations,
            cancelled_operations: result.cancelled_operations,
            repair: None,
            items: result
                .items
                .into_iter()
                .map(|i| ExecutionItem {
                    operation: i.operation,
                    status: match i.status {
                        ItemStatus::Complete => "complete",
                        ItemStatus::SourceRetained => "source_retained",
                        ItemStatus::Failed => "failed",
                        ItemStatus::Cancelled => "cancelled",
                        ItemStatus::Blocked => "blocked",
                        ItemStatus::NeedsRecovery => "needs_recovery",
                    },
                    detail: i.detail,
                    repair: None,
                })
                .collect(),
        }
    }
}
#[cfg(all(windows, feature = "selected-windows-execution"))]
pub(super) use selected::{PreparedSelection, inspect_selection};
