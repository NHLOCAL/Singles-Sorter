//! Retained preview -> one-shot native execution. The renderer never supplies
//! paths, settings, operation lists, resource hashes, or serialized authority.
use super::*;
use sorter_core::discovery::{ReadInput, StorageError};
#[cfg(all(windows, feature = "selected-windows-execution"))]
#[path = "execution_album.rs"]
mod album;
#[cfg(all(windows, feature = "selected-windows-execution"))]
#[path = "execution_repair.rs"]
mod repair;

pub(super) struct ReadAdapter<'a>(pub Box<dyn ReadStorage + 'a>);
impl ReadStorage for ReadAdapter<'_> {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<String>,
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
    /// Display-only leaf from a successful current-file observation, never a plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_filename: Option<String>,
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
pub struct AlbumSummary {
    pub completed: usize,
    pub child_names_changed: usize,
    pub created_directories: usize,
    pub removed_directories: usize,
    pub retained_directories: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExecutionResult {
    #[cfg(feature = "selected-windows-execution")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_summary: Option<sorter_executor::legacy_summary::LegacySummary>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub albums: Option<AlbumSummary>,
}
#[derive(Clone, Debug, Serialize)]
pub struct VerificationProgress {
    pub completed_operations: usize,
    pub total_operations: usize,
    pub bytes_read: u64,
    pub elapsed_ms: u64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<VerificationProgress>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryDirectory {
    pub reference: StorageRef,
    pub role: &'static str,
    pub status: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryInspection {
    pub observed_directories: usize,
    pub pending_directories: usize,
    pub rejected_directories: usize,
    pub incomplete_tail: bool,
    pub transfers: Vec<RecoveryItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pending_operations: Vec<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub directory_evidence: Vec<RecoveryDirectory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_issues: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_records_complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append_blocked: Option<bool>,
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
        windows_album::{AlbumScope, prepare_album_scope, prepare_album_move_streams, supported_album_plan},
        windows_job::{
            CopiedMoveScope, SelectedWindowsJob, prepare_copied_move_scope,
            supported_copied_move_plan, supported_plan,
        },
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
        Noop {
            target: PathBuf,
            resources: Resources,
        },
        Sort {
            target: PathBuf,
            scope: ExecutionScope,
        },
        CopiedMove {
            target: PathBuf,
            journal: PathBuf,
            journal_identity: sorter_core::discovery::ObjectIdentity,
            scope: Box<CopiedMoveScope>,
        },
        Album {
            target: PathBuf,
            journal: Option<(PathBuf, sorter_core::discovery::ObjectIdentity)>,
            scope: Box<AlbumScope>,
        },
        Repair(RepairScope),
    }
    impl PreparedSelection {
        #[cfg(test)]
        pub(super) fn test_album_scope(&self) -> Option<&AlbumScope> {
            match &self.scope {
                PreparedScope::Album { scope, .. } => Some(scope),
                _ => None,
            }
        }
        fn resources(&self) -> &Resources {
            match &self.scope {
                PreparedScope::Noop { resources, .. } => resources,
                PreparedScope::Sort { scope, .. } => &scope.resources,
                PreparedScope::CopiedMove { scope, .. } => &scope.execution_scope().resources,
                PreparedScope::Repair(scope) => scope.resources(),
                PreparedScope::Album { scope, .. } => scope.resources(),
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
    pub(super) const COPIED_MOVE_NOTICE: &str = "סגור תוכנות או סנכרון שמשנים את המקור או היעד עד סיום ההעברה. המקור יוסר רק לאחר אימות כל היעדים של אותו קובץ.";
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn inspect_selection(
        plan: &JobPlanV2,
        source: &Root,
        target: Option<&Root>,
        personal: Option<&[u8]>,
        ai: RequestedAi,
        journal_directory: Option<&Path>,
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
            if plan.operations.iter().any(|operation| {
                matches!(
                    operation.kind,
                    sorter_core::job_plan::OperationKind::AlbumTransfer { .. }
                )
            }) {
                supported_album_plan(plan).map_err(storage_error)?;
                let mut scope = prepare_album_scope(plan, storage, &resources, &cancellation.cancelled)
                    .map_err(storage_error)?;
                let mut retained_journal = None;
                if !plan.request.settings.copy_mode
                    && !matches!((&scope.execution_scope().source_root, &scope.execution_scope().target_root), (sorter_core::discovery::ObjectIdentity::Windows {volume:a,..},sorter_core::discovery::ObjectIdentity::Windows {volume:b,..}) if a == b)
                {
                    let journal = journal_directory.ok_or_else(|| {
                        HostError::new("JOURNAL_UNAVAILABLE", "Application journal directory is unavailable")
                    })?;
                    let identity = inspect_root(journal)?.identity.ok_or_else(|| {
                        HostError::new("JOURNAL_UNAVAILABLE", "Journal root identity missing")
                    })?;
                    let grant = |path: &Path, identity| {
                        WindowsSelectedRootGrant::new(path.to_owned(), identity).map_err(storage_error)
                    };
                    let roots = WindowsMutationStorage::for_selected_roots(
                        grant(&source.path, scope.execution_scope().source_root.clone())?,
                        grant(&target.path, scope.execution_scope().target_root.clone())?,
                        grant(journal, identity.clone())?,
                    ).map_err(storage_error)?;
                    prepare_album_move_streams(&mut scope, &roots, &cancellation.cancelled)
                        .map_err(storage_error)?;
                    // The preview grants never escape or create a journal. Apply
                    // receives retained data and independently reopens every root.
                    retained_journal = Some((journal.to_owned(), identity));
                }
                return Ok(PreparedSelection {
                    source: source.path.clone(),
                    personal: personal.unwrap_or_default().to_vec(),
                    ai,
                    scope: PreparedScope::Album {
                        target: target.path.clone(),
                        journal: retained_journal,
                        scope: Box::new(scope),
                    },
                });
            }
            if sorter_executor::no_op::is_clean_sort_noop(plan) {
                return Ok(PreparedSelection {
                    source: source.path.clone(),
                    personal: personal.unwrap_or_default().to_vec(),
                    ai,
                    scope: PreparedScope::Noop {
                        target: target.path.clone(),
                        resources,
                    },
                });
            }
            if supported_plan(plan).is_err() {
                supported_copied_move_plan(plan).map_err(storage_error)?;
                let journal = journal_directory.ok_or_else(|| {
                    HostError::new(
                        "JOURNAL_UNAVAILABLE",
                        "Application journal directory is unavailable",
                    )
                })?;
                let journal_identity = inspect_root(journal)?.identity.ok_or_else(|| {
                    HostError::new("JOURNAL_UNAVAILABLE", "Journal root identity missing")
                })?;
                let grant =
                    |path: &Path, identity: Option<sorter_core::discovery::ObjectIdentity>| {
                        WindowsSelectedRootGrant::new(
                            path.to_owned(),
                            identity.ok_or_else(|| {
                                HostError::new("ROOT_CHANGED", "Selected root identity missing")
                            })?,
                        )
                        .map_err(storage_error)
                    };
                let roots = WindowsMutationStorage::for_selected_roots(
                    grant(&source.path, source.observed.identity.clone())?,
                    grant(&target.path, target.observed.identity.clone())?,
                    grant(journal, Some(journal_identity.clone()))?,
                )
                .map_err(storage_error)?;
                let scope = prepare_copied_move_scope(
                    plan,
                    storage,
                    &roots,
                    &resources,
                    &cancellation.cancelled,
                )
                .map_err(storage_error)?;
                return Ok(PreparedSelection {
                    source: source.path.clone(),
                    personal: personal.unwrap_or_default().to_vec(),
                    ai,
                    scope: PreparedScope::CopiedMove {
                        target: target.path.clone(),
                        journal: journal.to_owned(),
                        journal_identity,
                        scope: Box::new(scope),
                    },
                });
            }
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
                notice: selected.as_ref().and_then(|selection| match &selection.scope {
                    PreparedScope::Album { journal: Some(_), .. } => Some("סגור תוכנות או סנכרון שמשנים את המקור או היעד עד סיום ההעברה. קובצי המקור יוסרו רק אחרי אימות ההעתקה; תיקיות המקור יוסרו רק כשהן ריקות.".into()),
                    PreparedScope::Album { .. } => Some("לפני מיון אלבומים, סגור תוכנות או סנכרון שעלולים לשנות את המקור או היעד עד לסיום הפעולה.".into()),
                    PreparedScope::CopiedMove { .. } => Some(COPIED_MOVE_NOTICE.into()),
                    _ => None,
                }),
            }),
            selected,
        ))
    }
    pub(super) fn journal_grant(
        directory: PathBuf,
        retained_identity: Option<sorter_core::discovery::ObjectIdentity>,
    ) -> Result<WindowsSelectedRootGrant> {
        // A copied job already bound this role at preview. Never adopt a newly
        // observed identity between its comparison and the native root open.
        let identity = match retained_identity {
            Some(identity) => identity,
            None => inspect_root(&directory)?.identity.ok_or_else(|| {
                HostError::new("JOURNAL_UNAVAILABLE", "Journal root identity missing")
            })?,
        };
        WindowsSelectedRootGrant::new(directory, identity).map_err(storage_error)
    }
    impl Host {
        pub(super) fn verification_control(
            &self,
            id: &str,
            cancellation: &Cancellation,
        ) -> sorter_executor::inspection_control::InspectionControl<'static> {
            let host = self.clone();
            let job_id = id.to_owned();
            #[cfg(test)]
            let verification_observer = self.verification_observer.clone();
            sorter_executor::inspection_control::InspectionControl::new(
                cancellation.cancelled.clone(),
                move |progress| {
                    if let Ok(mut inner) = host.lock()
                        && let Some(snapshot) = inner
                            .job
                            .as_mut()
                            .filter(|job| job.snapshot.job_id == job_id)
                            .and_then(|job| job.execution.as_mut())
                    {
                        snapshot.operation = None;
                        snapshot.verification = Some(VerificationProgress {
                            completed_operations: progress.completed_operations,
                            total_operations: progress.total_operations,
                            bytes_read: progress.bytes_read,
                            elapsed_ms: progress.elapsed_ms,
                        });
                    }
                    #[cfg(test)]
                    if let Some(observer) = &verification_observer {
                        observer(&progress);
                    }
                },
            )
        }

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
                verification: None,
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
            let recognition_result =
                recognition(&selected.ai, self.model_directory.as_deref(), cancellation);
            if cancellation.is_cancelled() {
                self.execution_error(id, "cancelled", "הביצוע בוטל לפני תחילת ההעברה.");
                return Ok(());
            }
            let (mut recognition, model_resources, _) = recognition_result?;
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
            let (target, scope, copied, retained_journal) = match selected.scope {
                PreparedScope::Noop { target, .. } => {
                    let reads =
                        sorter_executor::windows_copy::WindowsPlanningStorage::for_album_preview(
                            &selected.source,
                            &target,
                            sorter_io::desktop::WindowsDirectoryReadLimits::ALBUM_DEFAULT,
                            &cancellation.cancelled,
                        )
                        .map_err(storage_error)?;
                    let mut storage = SharedReadStorage::new(reads);
                    let mut metadata_storage = storage.clone();
                    let mut metadata = StorageMetadataProvider {
                        storage: &mut metadata_storage,
                        reader: MetadataReader,
                    };
                    let fresh = plan_job_controlled(
                        &plan.request,
                        &mut storage,
                        &mut metadata,
                        recognition.as_mut(),
                        cancellation,
                    );
                    if cancellation.is_cancelled() {
                        self.execution_error(id, "cancelled", "הבדיקה בוטלה. לא בוצעו שינויים.");
                        return Ok(());
                    }
                    let fresh = fresh.map_err(|error| HostError::new(&error.code, error.detail))?;
                    if fresh.binding_sha256 != plan.binding_sha256
                        || !sorter_executor::no_op::is_clean_sort_noop(&fresh)
                    {
                        return Err(HostError::new(
                            "PLAN_CHANGED",
                            "Files or resources changed; inspect a fresh plan",
                        ));
                    }
                    let mut inner = self.lock()?;
                    if let Some(snapshot) = inner
                        .job
                        .as_mut()
                        .filter(|job| job.snapshot.job_id == id)
                        .and_then(|job| job.execution.as_mut())
                    {
                        snapshot.status = "complete";
                        snapshot.journal_path = None;
                        snapshot.operation = None;
                        snapshot.result = Some(ExecutionResult {
                            legacy_summary: Some(
                                sorter_executor::legacy_summary::LegacySummary::default(),
                            ),
                            source_items: 0,
                            committed_destinations: 0,
                            moved_sources: 0,
                            failed_operations: 0,
                            blocked_operations: 0,
                            recovery_operations: 0,
                            cancelled_operations: 0,
                            items: vec![],
                            repair: None,
                            albums: None,
                        });
                    }
                    return Ok(());
                }
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
                PreparedScope::Album { target, journal, scope } => {
                    if let Some((path, identity)) = &journal
                        && (path != &directory || inspect_root(&directory)?.identity.as_ref() != Some(identity))
                    {
                        return Err(HostError::new("JOURNAL_CHANGED", "Journal storage changed; inspect a fresh plan"));
                    }
                    return self.execute_album(
                        id,
                        plan,
                        &selected.source,
                        &target,
                        directory,
                        &scope,
                        recognition.as_mut(),
                        cancellation,
                        journal.map(|(_, identity)| identity),
                    );
                }
                PreparedScope::Sort { target, scope } => (target, scope, None, None),
                PreparedScope::CopiedMove {
                    target,
                    journal,
                    journal_identity,
                    scope,
                } => {
                    if directory != journal
                        || inspect_root(&directory)?.identity.as_ref() != Some(&journal_identity)
                    {
                        return Err(HostError::new(
                            "JOURNAL_CHANGED",
                            "Journal storage changed; inspect a fresh plan",
                        ));
                    }
                    (
                        target,
                        scope.execution_scope().clone(),
                        Some(scope),
                        Some(journal_identity),
                    )
                }
            };
            // Startup profile access already established this application directory.
            // Never create a path selected relative to a music tree here.
            let journal = journal_grant(directory.clone(), retained_journal)?;
            let grant = |path, identity| {
                WindowsSelectedRootGrant::new(path, identity).map_err(storage_error)
            };
            let roots = WindowsMutationStorage::for_selected_roots(
                grant(selected.source.clone(), scope.source_root.clone())?,
                grant(target.clone(), scope.target_root.clone())?,
                journal,
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
            let prepared_job = if let Some(copied) = &copied {
                SelectedWindowsJob::prepare_copied_move(
                    roots,
                    plan,
                    &mut metadata,
                    recognition.as_mut(),
                    &scope.resources,
                    copied,
                    &cancellation.cancelled,
                )
            } else {
                SelectedWindowsJob::prepare(
                    roots,
                    plan,
                    &mut metadata,
                    recognition.as_mut(),
                    &scope.resources,
                    &scope,
                )
            };
            if cancellation.is_cancelled() {
                self.execution_error(id, "cancelled", "הביצוע בוטל לפני תחילת ההעברה.");
                return Ok(());
            }
            let mut job = prepared_job.map_err(storage_error)?;
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
            let (recovered, recovery_error) = if job.journal_locations().is_empty() {
                (None, None)
            } else if cancellation.is_cancelled() {
                (None, Some("VERIFICATION_CANCELLED".into()))
            } else {
                let control = self.verification_control(id, cancellation);
                match job.inspect_recovery_controlled(&recovery_scope, &control) {
                    Ok(report) => {
                        let inspection = project_recovery(report);
                        let incomplete = !inspection.pending_operations.is_empty()
                            || inspection.pending_directories > 0
                            || inspection.rejected_directories > 0
                            || inspection.incomplete_tail
                            || inspection.terminal_records_complete != Some(true)
                            || inspection.append_blocked == Some(true)
                            || inspection.metadata_issues.is_some_and(|count| count > 0);
                        (
                            Some(inspection),
                            incomplete.then(|| "TRANSFER_RECONCILIATION_INCOMPLETE".into()),
                        )
                    }
                    Err(error) => (None, Some(error.to_string())),
                }
            };
            if let Ok(mut inner) = self.lock() {
                if let Some(e) = inner
                    .job
                    .as_mut()
                    .filter(|j| j.snapshot.job_id == id)
                    .and_then(|j| j.execution.as_mut())
                {
                    e.recovery = recovered;
                    e.recovery_error = recovery_error;
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
                        let legacy_summary =
                            sorter_executor::legacy_summary::LegacySummary::from_receipts(
                                &plan.operations,
                                &result.committed_operations,
                                &job.created_directory_receipts()
                                    .iter()
                                    .map(|(reference, _)| reference.clone())
                                    .collect::<Vec<_>>(),
                                0,
                                &[],
                            );
                        let mut projected = project(result);
                        projected.legacy_summary = Some(legacy_summary);
                        e.result = Some(projected);
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
    fn project_recovery(
        report: sorter_executor::windows_job::SelectedRecoveryReport,
    ) -> RecoveryInspection {
        use sorter_executor::recovery::{CopyMetadataRecoveryStatus, RecoveryStatus};
        let append_blocked = report
            .transfers
            .as_ref()
            .is_none_or(|transfer| transfer.append_blocked);
        let mut pending_operations = vec![];
        let mut metadata_issues = 0;
        let transfers = report
            .transfers
            .map(|transfer| {
                transfer
                    .operations
                    .into_iter()
                    .map(|operation| {
                        if operation.status != RecoveryStatus::Committed {
                            pending_operations.push(operation.operation);
                        }
                        if !matches!(
                            operation.copy_metadata,
                            CopyMetadataRecoveryStatus::NotRequested
                                | CopyMetadataRecoveryStatus::Verified
                        ) {
                            metadata_issues += 1;
                        }
                        RecoveryItem {
                            operation: operation.operation,
                            status: format!("{:?}", operation.status),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        RecoveryInspection {
            pending_operations,
            directory_evidence: vec![],
            metadata_issues: Some(metadata_issues),
            terminal_records_complete: Some(report.directories.complete),
            append_blocked: Some(append_blocked),
            observed_directories: report.directories.observed.len(),
            pending_directories: report.directories.pending.len(),
            rejected_directories: report.directories.rejected.len(),
            incomplete_tail: report.directories.incomplete_tail,
            transfers,
        }
    }
    pub(super) fn status(result: &JobResult) -> &'static str {
        match result.status {
            JobStatus::Complete => "complete",
            JobStatus::Partial => "partial",
            JobStatus::Cancelled => "cancelled",
            JobStatus::NeedsRecovery => "needs_recovery",
        }
    }
    pub(super) fn project(result: JobResult) -> ExecutionResult {
        ExecutionResult {
            legacy_summary: None,
            source_items: result.source_items,
            committed_destinations: result.committed_destinations,
            moved_sources: result.moved_sources,
            failed_operations: result.failed_operations,
            blocked_operations: result.blocked_operations,
            recovery_operations: result.recovery_operations,
            cancelled_operations: result.cancelled_operations,
            repair: None,
            albums: None,
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

#[cfg(all(test, windows, feature = "selected-windows-execution"))]
#[path = "execution_noop_tests.rs"]
mod no_op_tests;

#[cfg(all(test, windows, feature = "selected-windows-execution"))]
#[path = "execution_copied_move_tests.rs"]
mod copied_move_tests;
