//! Native application authority. Paths never arrive through renderer job tokens.
//! The renderer receives display projections, never an executable plan.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sorter_core::{
    Settings,
    discovery::{EntryKind, EntryObservation, StorageRef},
    job_plan::{
        AiProfile, DecisionV2, DisabledRecognition, JobMode, JobPlanV2, JobRequestV2,
        OperationKind, PlanningAbort, PlanningBoundary, PlanningControl, RecognitionProvider,
        plan_job_controlled,
    },
};
#[cfg(windows)]
use sorter_io::desktop::DesktopStorage as Desktop;
#[cfg(target_os = "linux")]
use sorter_io::desktop::LinuxTestStorage as Desktop;
use sorter_io::{
    metadata::{MetadataReader, StorageMetadataProvider},
    resources::load_aliases,
    storage::{ReadStorage, SharedReadStorage},
};
use std::{
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

#[path = "execution.rs"]
mod execution;
pub use execution::{ApplyReadiness, ExecutionSnapshot, RetainedExecution};

const BUNDLED: &[u8] =
    include_bytes!("../../../../tests/fixtures/full_contract/released-singer-list.csv");
const COVERAGE: &[u8] = include_bytes!("../../../../tests/fixtures/media/metadata-coverage.json");
const MAX_PAGE: usize = 200;
pub const FOOTER_URL: &str =
    "https://nhlocal.github.io/shir-bot?utm_source=singles_sorter_program&utm_medium=desktop";

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum ProjectLink {
    Source,
    License,
}
impl ProjectLink {
    pub fn url(self) -> &'static str {
        match self {
            Self::Source => "https://github.com/NHLOCAL/Singles-Sorter",
            Self::License => "https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HostError {
    pub code: String,
    pub message: String,
}
impl HostError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
type Result<T> = std::result::Result<T, HostError>;
fn storage_error(e: impl std::fmt::Display) -> HostError {
    HostError::new("STORAGE_ERROR", e.to_string())
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum Role {
    Source,
    Target,
}
#[derive(Clone, Debug, Serialize)]
pub struct RootCapabilities {
    read: bool,
    list: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct RootSelection {
    pub token: String,
    pub display_name: String,
    pub capabilities: RootCapabilities,
    pub platform: &'static str,
}
#[derive(Clone)]
struct Root {
    role: Role,
    selection: RootSelection,
    path: PathBuf,
    observed: EntryObservation,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum RequestedAi {
    Disabled,
    Configured,
}
/// No serde defaults here: the IPC snapshot must explicitly carry all six values.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobSettings {
    pub copy_mode: bool,
    pub main_folder_only: bool,
    pub singles_folder: bool,
    pub exist_only: bool,
    pub abc_sort: bool,
    pub duet_mode: bool,
}
impl From<JobSettings> for Settings {
    fn from(s: JobSettings) -> Self {
        Self {
            copy_mode: s.copy_mode,
            main_folder_only: s.main_folder_only,
            singles_folder: s.singles_folder,
            exist_only: s.exist_only,
            abc_sort: s.abc_sort,
            duet_mode: s.duet_mode,
        }
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRequest {
    pub mode: JobMode,
    pub source_token: String,
    pub target_token: Option<String>,
    pub settings: JobSettings,
    pub ai_profile: RequestedAi,
}
#[derive(Clone, Serialize)]
pub struct JobId {
    pub job_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct JobSnapshot {
    pub host_schema: u16,
    pub job_id: String,
    pub status: &'static str,
    /// Evidence only for an accepted complete plan; absent for aborted/failed jobs.
    pub inference_executed: Option<bool>,
    pub plan_id: Option<String>,
    pub total_rows: usize,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply: Option<ApplyReadiness>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DisplayRow {
    pub id: String,
    pub source: String,
    pub kind: String,
    pub destination: Option<String>,
    pub reason: String,
    pub blockers: Vec<String>,
    pub operation_ids: Vec<usize>,
    pub artist: Option<String>,
    pub developer_only: bool,
    /// Display classification only; never consulted by execution.
    pub no_action: bool,
    /// Display-only relationship; not an independent file operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_in_album: Option<usize>,
}
#[derive(Serialize)]
pub struct PlanPage {
    pub rows: Vec<DisplayRow>,
    pub total: usize,
    pub skipped: usize,
}
struct Planned {
    plan: JobPlanV2,
    inference_executed: bool,
    apply: Option<ApplyReadiness>,
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    selected: Option<execution::PreparedSelection>,
}
struct CompletedJob {
    snapshot: JobSnapshot,
    rows: Vec<DisplayRow>,
    _plan: Option<JobPlanV2>,
    execution: Option<ExecutionSnapshot>,
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    selected: Option<execution::PreparedSelection>,
    cancellation: Cancellation,
}
#[derive(Clone, Default)]
struct Cancellation {
    cancelled: Arc<AtomicBool>,
    #[cfg(feature = "original-ai")]
    recognition: sorter_recognition::JobCancellation,
}
impl Cancellation {
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        #[cfg(feature = "original-ai")]
        self.recognition.cancel();
    }
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}
impl PlanningControl for Cancellation {
    fn checkpoint(&self, _: PlanningBoundary) -> std::result::Result<(), PlanningAbort> {
        if self.is_cancelled() {
            Err(PlanningAbort::cancelled())
        } else {
            Ok(())
        }
    }
}
#[derive(Default)]
struct Inner {
    roots: Vec<Root>,
    job: Option<CompletedJob>,
    model_probe: Option<Cancellation>,
    journal_directory: Option<PathBuf>,
}
#[derive(Default)]
struct Shared {
    inner: Mutex<Inner>,
    busy: AtomicBool,
    serial: AtomicU64,
    lifecycle: AtomicU64,
}
#[derive(Clone, Default)]
pub struct Host {
    shared: Arc<Shared>,
    model_directory: Option<PathBuf>,
}
pub struct BusyGuard {
    shared: Arc<Shared>,
    lifecycle: u64,
}
impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.shared.busy.store(false, Ordering::Release);
    }
}
pub fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux_test"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "unsupported"
    }
}
impl Host {
    pub fn claim(&self) -> Result<BusyGuard> {
        let lifecycle = self.shared.lifecycle.load(Ordering::Acquire);
        self.shared
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| HostError::new("BUSY", "A picker or preview is already running"))?;
        Ok(BusyGuard {
            shared: self.shared.clone(),
            lifecycle,
        })
    }
    fn next(&self, prefix: &str) -> String {
        format!(
            "{prefix}-{}",
            self.shared.serial.fetch_add(1, Ordering::Relaxed)
        )
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Inner>> {
        self.shared.inner.lock().map_err(|_| {
            HostError::new(
                "HOST_STATE_ERROR",
                "Host state unavailable; restart the preview application",
            )
        })
    }
    pub fn select_path(&self, role: Role, path: PathBuf) -> Result<RootSelection> {
        validate_path(&path)?;
        let observed = inspect_root(&path)?;
        let selection = RootSelection {
            token: self.next("root"),
            display_name: path.to_string_lossy().into_owned(),
            capabilities: RootCapabilities {
                read: observed.capabilities.read == Some(true),
                list: observed.capabilities.list == Some(true),
            },
            platform: platform(),
        };
        let mut inner = self.lock()?;
        inner.roots.retain(|r| r.role != role);
        inner.roots.push(Root {
            role,
            selection: selection.clone(),
            path,
            observed,
        });
        inner.job = None;
        Ok(selection)
    }
    pub fn with_model_directory(directory: PathBuf) -> Self {
        Self {
            model_directory: Some(directory),
            ..Self::default()
        }
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        let inner = self.lock()?;
        let job = inner
            .job
            .as_ref()
            .filter(|j| j.snapshot.job_id == id)
            .ok_or_else(|| HostError::new("UNKNOWN_JOB", "Job is no longer retained"))?;
        if job.snapshot.status == "running" {
            job.cancellation.cancel();
        }
        Ok(())
    }
    pub fn cancel_active(&self) {
        if let Ok(inner) = self.lock() {
            self.shared.lifecycle.fetch_add(1, Ordering::AcqRel);
            if let Some(probe) = &inner.model_probe {
                probe.cancel();
            }
            if let Some(job) = &inner.job {
                if job.snapshot.status == "running"
                    || job.execution.as_ref().is_some_and(|e| e.is_running())
                {
                    job.cancellation.cancel();
                }
            }
        }
    }
    fn prepare(&self, request: &PreviewRequest) -> Result<(Root, Option<Root>)> {
        #[cfg(not(feature = "original-ai"))]
        if request.ai_profile == RequestedAi::Configured {
            return Err(HostError::new(
                "RECOGNITION_UNAVAILABLE",
                "This build does not include original AI",
            ));
        }
        let (source, target) = {
            let inner = self.lock()?;
            let find = |token: &str, role: Role| {
                inner
                    .roots
                    .iter()
                    .find(|r| r.selection.token == token && r.role == role)
                    .cloned()
                    .ok_or_else(|| {
                        HostError::new(
                            "INVALID_ROOT_TOKEN",
                            "Root token is stale, unknown, or has the wrong role",
                        )
                    })
            };
            let source = find(&request.source_token, Role::Source)?;
            let target = if request.mode == JobMode::Sort {
                Some(find(
                    request.target_token.as_deref().ok_or_else(|| {
                        HostError::new(
                            "TARGET_REQUIRED",
                            "Sort requires a selected target directory",
                        )
                    })?,
                    Role::Target,
                )?)
            } else {
                None
            };
            (source, target)
        };
        Ok((source, target))
    }
    pub fn start(
        &self,
        request: PreviewRequest,
        personal: Vec<u8>,
        guard: BusyGuard,
    ) -> Result<JobId> {
        let (source, target) = self.prepare(&request)?;
        let job_id = self.next("job");
        let cancellation = Cancellation::default();
        let mut inner = self.lock()?;
        if guard.lifecycle != self.shared.lifecycle.load(Ordering::Acquire) {
            return Err(HostError::new(
                "PLANNING_CANCELLED",
                "The owning view was closed or reloaded",
            ));
        }
        inner.job = Some(CompletedJob {
            snapshot: JobSnapshot {
                host_schema: 1,
                job_id: job_id.clone(),
                status: "running",
                inference_executed: None,
                plan_id: None,
                total_rows: 0,
                blockers: vec![],
                warnings: vec![],
                error: None,
                apply: None,
            },
            rows: vec![],
            _plan: None,
            execution: None,
            #[cfg(all(windows, feature = "selected-windows-execution"))]
            selected: None,
            cancellation: cancellation.clone(),
        });
        drop(inner);
        let host = self.clone();
        let id = job_id.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("read-only-plan".into())
            .spawn(move || {
                let _guard = guard;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    real_plan(
                        &source,
                        target.as_ref(),
                        request,
                        Some(&personal),
                        host.model_directory.as_deref(),
                        &cancellation,
                    )
                }))
                .unwrap_or_else(|_| Err(HostError::new("WORKER_FAILED", "Planning worker failed")));
                let _ = host.finish(&id, result, cancellation);
            })
        {
            self.finish(
                &job_id,
                Err(HostError::new("WORKER_FAILED", error.to_string())),
                Cancellation::default(),
            )?;
        }
        Ok(JobId { job_id })
    }
    fn finish(
        &self,
        job_id: &str,
        result: Result<Planned>,
        cancellation: Cancellation,
    ) -> Result<()> {
        let mut inner = self.lock()?;
        // Cancellation and result acceptance share the lock: a late success never
        // revives a cancelled job, and an obsolete worker never replaces a newer job.
        if inner
            .job
            .as_ref()
            .is_some_and(|job| job.snapshot.job_id != job_id)
        {
            return Ok(());
        }
        let result = if cancellation.is_cancelled() {
            Err(HostError::new(
                "PLANNING_CANCELLED",
                "הבדיקה בוטלה. לא בוצעו שינויים בקבצים.",
            ))
        } else {
            result
        };
        let job = match result {
            Ok(Planned {
                plan,
                inference_executed,
                apply,
                #[cfg(all(windows, feature = "selected-windows-execution"))]
                selected,
            }) => {
                let rows = project_rows(&plan);
                CompletedJob {
                    snapshot: JobSnapshot {
                        host_schema: 1,
                        job_id: job_id.into(),
                        status: "completed",
                        inference_executed: Some(inference_executed),
                        plan_id: Some(plan.binding_sha256.clone()),
                        total_rows: rows.len(),
                        blockers: plan.blockers.clone(),
                        warnings: plan.warnings.clone(),
                        error: None,
                        apply,
                    },
                    rows,
                    _plan: Some(plan),
                    execution: None,
                    #[cfg(all(windows, feature = "selected-windows-execution"))]
                    selected,
                    cancellation,
                }
            }
            Err(error) => CompletedJob {
                snapshot: JobSnapshot {
                    host_schema: 1,
                    job_id: job_id.into(),
                    status: if error.code == "PLANNING_CANCELLED" {
                        "cancelled"
                    } else {
                        "failed"
                    },
                    inference_executed: None,
                    plan_id: None,
                    total_rows: 0,
                    blockers: vec![error.code],
                    warnings: vec![],
                    error: Some(error.message),
                    apply: None,
                },
                rows: vec![],
                _plan: None,
                execution: None,
                #[cfg(all(windows, feature = "selected-windows-execution"))]
                selected: None,
                cancellation,
            },
        };
        inner.job = Some(job);
        Ok(())
    }
    #[cfg(test)]
    pub fn preview(&self, request: PreviewRequest) -> Result<JobId> {
        self.preview_with_personal(request, None)
    }
    #[cfg(test)]
    pub fn preview_with_personal(
        &self,
        request: PreviewRequest,
        personal: Option<Vec<u8>>,
    ) -> Result<JobId> {
        let (source, target) = self.prepare(&request)?;
        let job_id = self.next("job");
        let cancellation = Cancellation::default();
        let result = real_plan(
            &source,
            target.as_ref(),
            request,
            personal.as_deref(),
            self.model_directory.as_deref(),
            &cancellation,
        );
        self.finish(&job_id, result, cancellation)?;
        Ok(JobId { job_id })
    }
    pub fn snapshot(&self, id: &str) -> Result<JobSnapshot> {
        let inner = self.lock()?;
        Ok(inner
            .job
            .as_ref()
            .filter(|j| j.snapshot.job_id == id)
            .ok_or_else(|| HostError::new("UNKNOWN_JOB", "Preview is no longer retained"))?
            .snapshot
            .clone())
    }
    #[cfg(test)]
    pub fn page(&self, id: &str, offset: usize, limit: usize) -> Result<PlanPage> {
        self.page_view_options(id, offset, limit, true, true)
    }
    #[cfg(test)]
    pub fn page_view(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
        developer_mode: bool,
    ) -> Result<PlanPage> {
        self.page_view_options(id, offset, limit, developer_mode, developer_mode)
    }
    pub fn page_view_options(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
        developer_mode: bool,
        show_inactive: bool,
    ) -> Result<PlanPage> {
        if limit == 0 || limit > MAX_PAGE {
            return Err(HostError::new(
                "INVALID_PAGE",
                "Page size must be 1 through 200",
            ));
        }
        let inner = self.lock()?;
        let job = inner
            .job
            .as_ref()
            .filter(|j| j.snapshot.job_id == id)
            .ok_or_else(|| HostError::new("UNKNOWN_JOB", "Preview is no longer retained"))?;
        if job.snapshot.status != "completed" {
            return Err(HostError::new(
                "JOB_NOT_COMPLETED",
                "Only completed jobs have result pages",
            ));
        }
        let effects = DisplayEffects::from_execution(job.execution.as_ref());
        let projected: Vec<_> = job
            .rows
            .iter()
            .filter(|row| developer_mode || !row.developer_only)
            .map(|row| (row, effects.inactive(row)))
            .collect();
        let skipped = projected.iter().filter(|(_, no_action)| *no_action).count();
        let visible: Vec<_> = projected
            .into_iter()
            .filter(|(_, no_action)| show_inactive || !*no_action)
            .collect();
        if offset > visible.len() {
            return Err(HostError::new(
                "INVALID_PAGE",
                "Offset exceeds the available rows",
            ));
        }
        let total = visible.len();
        Ok(PlanPage {
            rows: visible
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(row, no_action)| {
                    let mut row = row.clone();
                    row.no_action = no_action;
                    row
                })
                .collect(),
            total,
            skipped,
        })
    }
}
// This is display evidence, never an execution grant. Build once per page so a
// large terminal job does not rescan every item for every row under the host lock.
enum DisplayEffects {
    Preview,
    Unknown,
    Observed {
        inactive: std::collections::HashMap<usize, bool>,
        complete: std::collections::HashMap<usize, bool>,
    },
}
impl DisplayEffects {
    fn from_execution(execution: Option<&ExecutionSnapshot>) -> Self {
        let Some(execution) = execution.filter(|value| !value.is_running()) else {
            return Self::Preview;
        };
        let Some(result) = &execution.result else {
            return Self::Unknown;
        };
        let mut operations = std::collections::HashMap::new();
        let mut complete = std::collections::HashMap::new();
        let verified = execution.recovery_error.is_none()
            && !matches!(execution.status, "failed" | "needs_recovery" | "cancelled");
        for item in &result.items {
            complete
                .entry(item.operation)
                .and_modify(|all: &mut bool| *all &= verified && item.status == "complete")
                .or_insert(verified && item.status == "complete");
            let changed = item
                .repair
                .as_ref()
                .is_some_and(|repair| repair.renamed || repair.repaired);
            let inactive = !changed
                && (item.status == "blocked"
                    || (item.status == "complete" && item.repair.is_some()));
            operations
                .entry(item.operation)
                .and_modify(|all: &mut bool| *all &= inactive)
                .or_insert(inactive);
        }
        Self::Observed {
            inactive: operations,
            complete,
        }
    }
    fn inactive(&self, row: &DisplayRow) -> bool {
        match self {
            Self::Preview => row.no_action,
            Self::Unknown => false,
            Self::Observed {
                inactive: operations,
                complete,
            } => {
                if let Some(album) = row.included_in_album {
                    return row.no_action && complete.get(&album) == Some(&true);
                }
                if row.operation_ids.is_empty() {
                    return row.no_action;
                }
                // Missing items never prove no effects. Multiple items for one
                // operation must all have observed no-effect outcomes.
                row.operation_ids
                    .iter()
                    .all(|operation| operations.get(operation) == Some(&true))
            }
        }
    }
}
fn validate_path(path: &Path) -> Result<()> {
    let bad = path.to_str().is_none_or(|raw| {
        raw.len() > 32768
            || raw.contains('\0')
            || raw
                .split(|c| c == '/' || (cfg!(windows) && c == '\\'))
                .any(|p| matches!(p, "." | ".."))
    });
    if bad
        || !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(HostError::new(
            "INVALID_PATH",
            "Select an absolute UTF-8 directory path without dot components",
        ));
    }
    Ok(())
}
#[cfg(any(windows, target_os = "linux"))]
fn inspect_root(path: &Path) -> Result<EntryObservation> {
    let mut storage = Desktop::new(&[path.to_owned()]).map_err(storage_error)?;
    let reference = storage.reference(path).map_err(storage_error)?;
    let observed = storage.stat(&reference).map_err(storage_error)?;
    if observed.kind != EntryKind::Directory || observed.identity.is_none() {
        return Err(HostError::new(
            "INVALID_ROOT",
            "A stable directory identity is required",
        ));
    }
    Ok(observed)
}
#[cfg(not(any(windows, target_os = "linux")))]
fn inspect_root(_path: &Path) -> Result<EntryObservation> {
    Err(HostError::new(
        "PLATFORM_UNAVAILABLE",
        "Android SAF root selection is not integrated yet",
    ))
}
#[cfg(any(windows, target_os = "linux"))]
fn real_plan(
    source: &Root,
    target: Option<&Root>,
    request: PreviewRequest,
    personal: Option<&[u8]>,
    model_directory: Option<&Path>,
    cancellation: &Cancellation,
) -> Result<Planned> {
    let mut roots = vec![source.path.clone()];
    if let Some(t) = target {
        roots.push(t.path.clone());
    }
    let mut desktop = Desktop::new(&roots).map_err(storage_error)?;
    for root in std::iter::once(source).chain(target) {
        let reference = desktop.reference(&root.path).map_err(storage_error)?;
        let current = desktop.stat(&reference).map_err(storage_error)?;
        if current.identity != root.observed.identity || current.kind != EntryKind::Directory {
            return Err(HostError::new(
                "ROOT_CHANGED",
                "A selected directory was replaced; select it again",
            ));
        }
    }
    let source_ref = desktop.reference(&source.path).map_err(storage_error)?;
    let target_ref = target
        .map(|t| desktop.reference(&t.path))
        .transpose()
        .map_err(storage_error)?;
    let aliases = load_aliases(BUNDLED, personal).map_err(storage_error)?;
    let (mut recognition, model_resources, model_adapters) =
        recognition(&request.ai_profile, model_directory, cancellation)?;
    let requested_ai = request.ai_profile.clone();
    let mut request = JobRequestV2 {
        mode: request.mode,
        source: source_ref,
        target: target_ref,
        settings: request.settings.into(),
        alias_rows: aliases.ordered_rows.into_iter().map(|r| r.fields).collect(),
        resource_ids: aliases
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
            .collect(),
        adapter_versions: vec![
            format!("singles-ui-host:{}:read-only-v1", env!("CARGO_PKG_VERSION")),
            format!("storage:{}:runtime-acceptance-pending", platform()),
            "taglib:2.3.2:3ca2d8afaa7f1cf7f6ed10e511ebc368bfacd6dcaa3dbfa690b89e502e8963dc".into(),
            format!("metadata-coverage:sha256:{:x}", Sha256::digest(COVERAGE)),
            "recognition:intentionally-disabled:no-model".into(),
        ],
        ai_profile: recognition.profile(),
    };
    request.resource_ids.extend(model_resources);
    if request.ai_profile != AiProfile::Disabled {
        request.adapter_versions.pop();
    }
    request.adapter_versions.extend(model_adapters);
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    let desktop: Box<dyn ReadStorage + '_> = if let Some(target) = target {
        Box::new(
            sorter_executor::windows_copy::WindowsPlanningStorage::for_album_preview(
                &source.path,
                &target.path,
                sorter_io::desktop::WindowsDirectoryReadLimits::ALBUM_DEFAULT,
                &cancellation.cancelled,
            )
            .map_err(storage_error)?,
        )
    } else if request.mode == JobMode::FixNames {
        Box::new(
            sorter_executor::windows_copy::WindowsPlanningStorage::for_repair_preview(&source.path)
                .map_err(storage_error)?,
        )
    } else {
        Box::new(desktop)
    };
    #[cfg(not(all(windows, feature = "selected-windows-execution")))]
    let desktop: Box<dyn ReadStorage> = Box::new(desktop);
    let mut storage = SharedReadStorage::new(execution::ReadAdapter(desktop));
    let mut metadata_storage = storage.clone();
    let mut metadata = StorageMetadataProvider {
        storage: &mut metadata_storage,
        reader: MetadataReader,
    };
    let plan = plan_job_controlled(
        &request,
        &mut storage,
        &mut metadata,
        recognition.as_mut(),
        cancellation,
    )
    .map_err(|e| HostError::new(&e.code, e.detail))?;
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    let (apply, selected) = execution::inspect_selection(
        &plan,
        source,
        target,
        personal,
        requested_ai,
        &mut storage,
        cancellation,
    )?;
    #[cfg(not(all(windows, feature = "selected-windows-execution")))]
    let apply = {
        let _ = requested_ai;
        None
    };
    Ok(Planned {
        plan,
        inference_executed: recognition.inference_executed(),
        apply,
        #[cfg(all(windows, feature = "selected-windows-execution"))]
        selected,
    })
}
#[cfg(not(any(windows, target_os = "linux")))]
fn real_plan(
    _source: &Root,
    _target: Option<&Root>,
    _request: PreviewRequest,
    _personal: Option<&[u8]>,
    _model_directory: Option<&Path>,
    _cancellation: &Cancellation,
) -> Result<Planned> {
    Err(HostError::new(
        "PLATFORM_UNAVAILABLE",
        "No native Android provider is integrated",
    ))
}
type Recognition = (Box<dyn RecognitionProvider>, Vec<String>, Vec<String>);
fn recognition(
    profile: &RequestedAi,
    directory: Option<&Path>,
    cancellation: &Cancellation,
) -> Result<Recognition> {
    if *profile == RequestedAi::Disabled {
        return Ok((Box::new(DisabledRecognition), vec![], vec![]));
    }
    #[cfg(feature = "original-ai")]
    {
        let directory = directory.ok_or_else(|| {
            HostError::new(
                "RECOGNITION_UNAVAILABLE",
                "Original model directory missing",
            )
        })?;
        let provider = sorter_recognition::OriginalRecognition::load(directory, sorter_recognition::BUNDLE_ID,
            sorter_recognition::Limits::default(), cancellation.recognition.clone()).map_err(|error| match error {
                sorter_core::job_plan::RecognitionError::Abort(abort) => {
                    let error: sorter_core::job_plan::JobError = abort.into();
                    HostError::new(&error.code, error.detail)
                }
                _ => HostError::new("RECOGNITION_UNAVAILABLE", "המודלים המקוריים חסרים או לא תקינים. הבדיקה לא הופעלה; אין מעבר אוטומטי לעבודה ללא AI."),
            })?;
        let resources = provider.resource_ids();
        let adapters = provider.adapter_versions();
        Ok((Box::new(provider), resources, adapters))
    }
    #[cfg(not(feature = "original-ai"))]
    {
        let _ = (directory, cancellation);
        Err(HostError::new(
            "RECOGNITION_UNAVAILABLE",
            "This build does not include original AI",
        ))
    }
}
#[derive(Serialize)]
pub struct ModelStatus {
    pub available: bool,
    pub message: String,
    pub platform: &'static str,
}
impl Host {
    pub fn model_status(&self, guard: &BusyGuard) -> Result<ModelStatus> {
        let cancellation = Cancellation::default();
        {
            let mut inner = self.lock()?;
            if guard.lifecycle != self.shared.lifecycle.load(Ordering::Acquire) {
                return Err(HostError::new(
                    "PLANNING_CANCELLED",
                    "The owning view was closed or reloaded",
                ));
            }
            inner.model_probe = Some(cancellation.clone());
        }
        let result = recognition(
            &RequestedAi::Configured,
            self.model_directory.as_deref(),
            &cancellation,
        );
        if let Ok(mut inner) = self.lock() {
            inner.model_probe = None;
        }
        Ok(ModelStatus {
            available: result.is_ok(),
            platform: platform(),
            message: match result {
                Ok(_) => {
                    "שני מודלי הזיהוי המקוריים אומתו. הקבצים נבדקים מחדש לפני כל בדיקה.".into()
                }
                Err(error) => {
                    if cfg!(feature = "original-ai") {
                        error.message
                    } else {
                        "מהדורה זו אינה כוללת את מודלי ה־AI המקוריים. בחר במפורש עבודה ללא AI או פתח את המהדורה הכוללת אותם.".into()
                    }
                }
            },
        })
    }
}
fn entry_display(entry: &EntryObservation) -> String {
    match &entry.reference {
        StorageRef::WindowsPath { utf16 } => String::from_utf16_lossy(utf16),
        // Opaque IDs remain authority only, not filenames. Android display names
        // and Linux test names are never reconstructed into storage references.
        _ => entry.name.clone(),
    }
}
fn operation_source(kind: &OperationKind) -> &EntryObservation {
    match kind {
        OperationKind::AlbumTransfer { source, .. }
        | OperationKind::SingleTransfer { source, .. } => source,
        OperationKind::FixFile(f) => &f.source,
    }
}
// Exact planned removals only. This projection never grants an operation or
// treats a predicted removal as an observed successful transfer.
fn album_omissions(plan: &JobPlanV2) -> std::collections::HashMap<&StorageRef, usize> {
    let albums = plan
        .operations
        .iter()
        .filter(|operation| matches!(operation.kind, OperationKind::AlbumTransfer { .. }))
        .map(|operation| operation.id)
        .collect::<std::collections::HashSet<_>>();
    plan.predicted_effects
        .iter()
        .filter_map(|effect| match effect {
            sorter_core::job_plan::PredictedEffect::SourceUnavailable { operation, source }
                if albums.contains(operation) =>
            {
                Some((source, *operation))
            }
            _ => None,
        })
        .collect()
}
fn included_album(
    decision: &DecisionV2,
    omissions: &std::collections::HashMap<&StorageRef, usize>,
) -> Option<usize> {
    let DecisionV2::Vanished { read } = decision else {
        return None;
    };
    let operation = *omissions.get(&read.source.reference)?;
    (!read.virtually_present
        && operation < read.after_operations
        && read.depends_on.contains(&operation))
    .then_some(operation)
}

fn project_rows(plan: &JobPlanV2) -> Vec<DisplayRow> {
    if plan.request.mode == JobMode::FixNames {
        return plan
            .operations
            .iter()
            .map(|op| match &op.kind {
                OperationKind::FixFile(f) => DisplayRow {
                    id: format!("operation-{}", op.id),
                    source: entry_display(&f.source),
                    kind: "FixNames".into(),
                    destination: f.intended_name.clone(),
                    reason: format!(
                        "rename_intended={}; save_intended={}; repairs={}; failure={:?}",
                        f.rename_intended,
                        f.save_intended,
                        f.repairs.len(),
                        f.failure
                    ),
                    blockers: f
                        .blockers
                        .iter()
                        .map(|b| format!("{b:?}"))
                        .chain(
                            f.failure
                                .as_ref()
                                .map(|_| "FIX_METADATA_READ_ERROR".to_owned()),
                        )
                        .collect(),
                    operation_ids: vec![op.id],
                    artist: None,
                    developer_only: false,
                    included_in_album: None,
                    no_action: !f.rename_intended && f.repairs.is_empty() && f.failure.is_none(),
                },
                _ => unreachable!("fix planner only emits fix operations"),
            })
            .collect();
    }
    let mut operations_by_source =
        std::collections::HashMap::<&StorageRef, Vec<&sorter_core::job_plan::OperationV2>>::new();
    for operation in &plan.operations {
        operations_by_source
            .entry(&operation_source(&operation.kind).reference)
            .or_default()
            .push(operation);
    }
    let omissions = album_omissions(plan);
    plan.decisions
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let included_in_album = included_album(d, &omissions);
            let (read, kind, reason) = match d {
                DecisionV2::Album { read, decision } => (
                    read,
                    "Album",
                    if !decision.is_album {
                        "התיקייה אינה אלבום לפי כללי המיון".into()
                    } else if !decision.should_process {
                        "זוהה אלבום שאינו מיועד לעיבוד לפי הכללים".into()
                    } else {
                        format!(
                            "אלבום: {}; אמן: {}",
                            decision.album_name.as_deref().unwrap_or(""),
                            decision.artist_name.as_deref().unwrap_or("")
                        )
                    },
                ),
                DecisionV2::Single { read, artists } => (
                    read,
                    "Single",
                    if artists.is_empty() {
                        "לא נבחר אמן לפי כללי המיון".into()
                    } else {
                        artists.join("; ")
                    },
                ),
                DecisionV2::ItemError { read, error } => (read, "ItemError", error.clone()),
                DecisionV2::Vanished { read } => {
                    (read, "Vanished", "המקור אינו זמין בשלב זה של התכנון".into())
                }
            };
            let mut destinations = vec![];
            let inspection_error = match d {
                DecisionV2::Album { decision, .. } => decision.trace.iter().any(|event| {
                    matches!(
                        event,
                        sorter_core::album::AlbumTraceEvent::ProtectedError { .. }
                            | sorter_core::album::AlbumTraceEvent::FileError { .. }
                    )
                }),
                _ => false,
            };
            let mut blockers = vec![];
            if inspection_error {
                blockers.push("ALBUM_READ_ERROR".into());
            }
            for op in operations_by_source
                .get(&read.source.reference)
                .into_iter()
                .flatten()
            {
                match &op.kind {
                    OperationKind::SingleTransfer { transfer, .. } => {
                        destinations.push(transfer.destination.components.join(" / "));
                        blockers.extend(transfer.blockers.clone());
                    }
                    OperationKind::AlbumTransfer {
                        destination,
                        children,
                        ..
                    } => {
                        destinations.push(destination.components.join(" / "));
                        for c in children {
                            blockers.extend(c.blockers.clone());
                        }
                    }
                    _ => {}
                }
            }
            DisplayRow {
                id: format!("decision-{i}"),
                artist: match d {
                    DecisionV2::Single { artists, .. } => {
                        (!artists.is_empty()).then(|| artists.join("; "))
                    }
                    DecisionV2::Album { decision, .. } => decision.artist_name.clone(),
                    _ => None,
                },
                developer_only: matches!(d, DecisionV2::Album { decision, .. } if !decision.is_album)
                    && !operations_by_source.contains_key(&read.source.reference)
                    && !inspection_error,
                no_action: (included_in_album.is_some()
                    || matches!(d, DecisionV2::Single { .. } | DecisionV2::Album { .. }))
                    && !operations_by_source.contains_key(&read.source.reference)
                    && !inspection_error,
                included_in_album,
                source: entry_display(&read.source),
                kind: kind.into(),
                destination: (!destinations.is_empty()).then(|| destinations.join("; ")),
                reason,
                blockers,
                operation_ids: operations_by_source
                    .get(&read.source.reference)
                    .into_iter()
                    .flatten()
                    .map(|op| op.id)
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> JobSettings {
        serde_json::from_value(serde_json::to_value(Settings::default()).unwrap()).unwrap()
    }
    fn request(source: String, target: Option<String>, mode: JobMode) -> PreviewRequest {
        PreviewRequest {
            mode,
            source_token: source,
            target_token: target,
            settings: settings(),
            ai_profile: RequestedAi::Disabled,
        }
    }
    #[test]
    fn exact_six_settings_required() {
        assert!(serde_json::from_str::<JobSettings>("{}").is_err());
        let mut value = serde_json::to_value(Settings::default()).unwrap();
        value["singles_folder"] = false.into();
        let parsed: JobSettings = serde_json::from_value(value).unwrap();
        assert!(!Settings::from(parsed).singles_folder);
    }
    #[test]
    fn unknown_request_fields_rejected() {
        let mut v = serde_json::json!({"mode":"Sort","source_token":"x","target_token":"y","settings":Settings::default(),"ai_profile":"Disabled"});
        v["apply"] = true.into();
        assert!(serde_json::from_value::<PreviewRequest>(v).is_err());
    }
    #[test]
    fn busy_guard_releases_on_unwind_and_drop() {
        let h = Host::default();
        let guard = h.claim().unwrap();
        assert!(h.claim().is_err());
        drop(guard);
        assert!(h.claim().is_ok());
    }
    #[test]
    fn relative_and_dot_paths_rejected() {
        for path in ["songs", "/tmp/../songs", "/tmp/./songs"] {
            assert!(validate_path(Path::new(path)).is_err());
        }
    }
    #[test]
    fn unknown_tokens_and_configured_ai_fail_closed() {
        let h = Host::default();
        assert_eq!(
            h.preview(request("/tmp".into(), None, JobMode::FixNames))
                .err()
                .unwrap()
                .code,
            "INVALID_ROOT_TOKEN"
        );
        let mut r = request("bogus".into(), None, JobMode::FixNames);
        r.ai_profile = RequestedAi::Configured;
        assert_eq!(
            h.preview(r).err().unwrap().code,
            if cfg!(feature = "original-ai") {
                "INVALID_ROOT_TOKEN"
            } else {
                "RECOGNITION_UNAVAILABLE"
            }
        );
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn real_read_only_empty_fix_and_selected_roles() {
        let d = tempfile::tempdir().unwrap();
        let h = Host::default();
        let s = h.select_path(Role::Source, d.path().into()).unwrap();
        let j = h
            .preview(request(s.token.clone(), None, JobMode::FixNames))
            .unwrap();
        let snapshot = h.snapshot(&j.job_id).unwrap();
        assert_eq!(snapshot.status, "completed");
        assert_eq!(snapshot.total_rows, 0);
        assert!(
            snapshot
                .blockers
                .contains(&"PHYSICAL_APPLY_UNAVAILABLE".into())
        );
        assert!(h.page(&j.job_id, 0, 1).unwrap().rows.is_empty());
        assert!(h.page(&j.job_id, 0, 201).is_err());
        assert!(h.page(&j.job_id, 1, 1).is_err());
        assert_eq!(
            h.preview(request(s.token.clone(), Some(s.token), JobMode::Sort))
                .err()
                .unwrap()
                .code,
            "INVALID_ROOT_TOKEN"
        );
        assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
    }
    #[test]
    fn terminal_display_hides_only_proven_no_effect_items() {
        use execution::{ExecutionItem, ExecutionResult, RepairOutcome};
        let mut row = DisplayRow {
            id: "owned".into(),
            source: "owned.mp3".into(),
            kind: "FixNames".into(),
            destination: None,
            reason: String::new(),
            blockers: vec![],
            operation_ids: vec![0],
            artist: None,
            developer_only: false,
            no_action: false,
            included_in_album: None,
        };
        let mut snapshot = ExecutionSnapshot {
            host_schema: 1,
            job_id: "owned".into(),
            plan_id: "retained".into(),
            status: "complete",
            operation_count: 1,
            operation: None,
            bytes_written: 0,
            source_length: 0,
            journal_path: None,
            error: None,
            recovery: None,
            recovery_error: None,
            result: Some(ExecutionResult {
                #[cfg(feature = "selected-windows-execution")]
                legacy_summary: None,
                source_items: 1,
                committed_destinations: 0,
                moved_sources: 0,
                failed_operations: 0,
                blocked_operations: 0,
                recovery_operations: 0,
                cancelled_operations: 0,
                repair: None,
                albums: None,
                items: vec![ExecutionItem {
                    operation: 0,
                    status: "complete",
                    detail: String::new(),
                    repair: Some(RepairOutcome {
                        renamed: false,
                        repaired: false,
                        unchanged: true,
                        observed_filename: None,
                    }),
                }],
            }),
        };
        let inactive = |row: &DisplayRow, snapshot: &ExecutionSnapshot| {
            DisplayEffects::from_execution(Some(snapshot)).inactive(row)
        };
        assert!(inactive(&row, &snapshot));
        for status in [
            "failed",
            "cancelled",
            "source_retained",
            "needs_recovery",
            "unsupported_tags",
            "metadata_failure",
        ] {
            snapshot.result.as_mut().unwrap().items[0].status = status;
            assert!(!inactive(&row, &snapshot), "{status}");
        }
        snapshot.result.as_mut().unwrap().items[0].status = "blocked";
        assert!(inactive(&row, &snapshot));
        let mut attempted = snapshot.result.as_ref().unwrap().items[0].clone();
        attempted.status = "failed";
        snapshot.result.as_mut().unwrap().items.push(attempted);
        assert!(!inactive(&row, &snapshot));
        snapshot.result.as_mut().unwrap().items.pop();
        snapshot.result.as_mut().unwrap().items[0].status = "complete";
        snapshot.result.as_mut().unwrap().items[0]
            .repair
            .as_mut()
            .unwrap()
            .renamed = true;
        assert!(!inactive(&row, &snapshot));
        snapshot.result.as_mut().unwrap().items[0].status = "blocked";
        assert!(!inactive(&row, &snapshot));
        snapshot.result.as_mut().unwrap().items[0].status = "complete";
        snapshot.result.as_mut().unwrap().items[0].repair = None;
        assert!(!inactive(&row, &snapshot));
        row.operation_ids.push(1);
        assert!(!inactive(&row, &snapshot));
        row.kind = "Vanished".into();
        row.operation_ids.clear();
        row.included_in_album = Some(0);
        row.no_action = true;
        assert!(inactive(&row, &snapshot));
        snapshot.recovery_error = Some("unverified album root".into());
        assert!(!inactive(&row, &snapshot));
        snapshot.recovery_error = None;
        for status in [
            "failed",
            "partial",
            "cancelled",
            "needs_recovery",
            "blocked",
        ] {
            snapshot.result.as_mut().unwrap().items[0].status = status;
            assert!(!inactive(&row, &snapshot), "{status}");
        }
        snapshot.result.as_mut().unwrap().items[0].status = "complete";
        let mut partial = snapshot.result.as_ref().unwrap().items[0].clone();
        partial.status = "partial";
        snapshot.result.as_mut().unwrap().items.push(partial);
        assert!(!inactive(&row, &snapshot));
        snapshot.result.as_mut().unwrap().items.clear();
        assert!(!inactive(&row, &snapshot));
        snapshot.result = None;
        row.no_action = true;
        assert!(!inactive(&row, &snapshot));
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn ordinary_pages_filter_before_offset_and_preserve_errors_and_skips() {
        let directory = tempfile::tempdir().unwrap();
        let host = Host::default();
        let source = host
            .select_path(Role::Source, directory.path().into())
            .unwrap();
        let job = host
            .preview(request(source.token, None, JobMode::FixNames))
            .unwrap();
        {
            let mut inner = host.lock().unwrap();
            inner.job.as_mut().unwrap().rows = (0..120)
                .map(|index| DisplayRow {
                    id: format!("row-{index}"),
                    source: format!("file-{index}"),
                    kind: match index {
                        30 => "Single",
                        60 => "ItemError",
                        90 => "Vanished",
                        _ => "Album",
                    }
                    .into(),
                    destination: None,
                    reason: "technical-only evidence".into(),
                    blockers: vec![],
                    operation_ids: vec![],
                    artist: None,
                    developer_only: !matches!(index, 30 | 60 | 90),
                    included_in_album: None,
                    no_action: !matches!(index, 60 | 90),
                })
                .collect();
        }
        let first = host
            .page_view_options(&job.job_id, 0, 1, false, true)
            .unwrap();
        assert_eq!(first.total, 3);
        assert_eq!(first.skipped, 1);
        assert_eq!(first.rows[0].id, "row-30");
        assert_eq!(
            host.page_view_options(&job.job_id, 1, 1, false, true)
                .unwrap()
                .rows[0]
                .id,
            "row-60"
        );
        assert_eq!(
            host.page_view_options(&job.job_id, 2, 1, false, true)
                .unwrap()
                .rows[0]
                .kind,
            "Vanished"
        );
        assert!(
            host.page_view_options(&job.job_id, 3, 1, false, true)
                .unwrap()
                .rows
                .is_empty()
        );
        assert!(host.page_view(&job.job_id, usize::MAX, 1, false).is_err());
        let technical = host.page_view(&job.job_id, 0, 200, true).unwrap();
        assert_eq!(technical.total, 120);
        assert_eq!(technical.skipped, 118);
        let active = host
            .page_view_options(&job.job_id, 0, 1, false, false)
            .unwrap();
        assert_eq!(active.total, 2);
        assert_eq!(active.skipped, 1);
        assert_eq!(active.rows[0].kind, "ItemError");
        assert_eq!(
            host.page_view_options(&job.job_id, 1, 1, false, false)
                .unwrap()
                .rows[0]
                .kind,
            "Vanished"
        );
        assert_eq!(
            host.page_view_options(&job.job_id, 0, 200, true, false)
                .unwrap()
                .total,
            2
        );
        assert!(
            technical
                .rows
                .iter()
                .all(|row| row.reason == "technical-only evidence")
        );
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn real_metadata_fixture_preview_never_writes() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("מקור");
        let dst = d.path().join("יעד");
        std::fs::create_dir(&src).unwrap();
        std::fs::create_dir(&dst).unwrap();
        // Owned fixture bytes, a real metadata read through the production bridge.
        let file =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/media/tagged.mp3");
        let bytes = std::fs::read(&file).unwrap();
        let name = format!(
            "אברהם פריד - בדיקה.{}",
            file.extension().unwrap().to_string_lossy()
        );
        let song = src.join(name);
        std::fs::write(&song, &bytes).unwrap();
        let h = Host::default();
        let s = h.select_path(Role::Source, src).unwrap();
        let t = h.select_path(Role::Target, dst.clone()).unwrap();
        let job = h
            .preview(request(s.token.clone(), Some(t.token), JobMode::Sort))
            .unwrap();
        let snap = h.snapshot(&job.job_id).unwrap();
        assert_eq!(snap.status, "completed", "{:?}", snap.error);
        assert!(snap.total_rows > 0);
        let page = h.page(&job.job_id, 0, 200).unwrap();
        assert_eq!(page.total, snap.total_rows);
        let simple = h.page_view(&job.job_id, 0, 200, false).unwrap();
        assert_eq!(simple.total, 1);
        assert_eq!(simple.skipped, 0);
        assert_eq!(simple.rows[0].kind, "Single");
        assert_eq!(simple.rows[0].artist.as_deref(), Some("אברהם פריד"));
        assert!(page.rows.iter().any(|row| row.developer_only));
        {
            let inner = h.lock().unwrap();
            let mut plan = inner.job.as_ref().unwrap()._plan.as_ref().unwrap().clone();
            let (read, mut decision) = plan
                .decisions
                .iter()
                .find_map(|item| match item {
                    DecisionV2::Album { read, decision } => Some((read.clone(), decision.clone())),
                    _ => None,
                })
                .unwrap();
            let mut omission_plan = plan.clone();
            let mut album_source = read.source.clone();
            album_source.reference = StorageRef::TestOpaque {
                id: "album parent".into(),
            };
            omission_plan.operations[0].kind = OperationKind::AlbumTransfer {
                source: album_source,
                destination: sorter_core::job_plan::Destination {
                    root: read.source.reference.clone(),
                    components: vec!["owned album".into()],
                },
                children: vec![],
                remove_empty_source: true,
            };
            omission_plan.predicted_effects =
                vec![sorter_core::job_plan::PredictedEffect::SourceUnavailable {
                    operation: 0,
                    source: read.source.reference.clone(),
                }];
            let mut omitted = read.clone();
            omitted.virtually_present = false;
            omitted.after_operations = 1;
            omitted.depends_on = vec![0];
            let project = |read: sorter_core::job_plan::ReadPoint| {
                let mut candidate = omission_plan.clone();
                candidate.decisions = vec![DecisionV2::Vanished { read }];
                project_rows(&candidate).remove(0)
            };
            let covered = project(omitted.clone());
            assert!(covered.no_action);
            assert_eq!(covered.included_in_album, Some(0));
            assert_eq!(covered.kind, "Vanished");
            assert!(covered.operation_ids.is_empty());
            for variation in 0..4 {
                let mut unexpected = omitted.clone();
                match variation {
                    0 => {
                        unexpected.source.reference = StorageRef::TestOpaque { id: "other".into() }
                    }
                    1 => unexpected.after_operations = 0,
                    2 => unexpected.depends_on.clear(),
                    _ => unexpected.virtually_present = true,
                }
                let row = project(unexpected);
                assert!(!row.no_action);
                assert_eq!(row.included_in_album, None);
            }
            omission_plan.operations[0] = plan.operations[0].clone();
            omission_plan.decisions = vec![DecisionV2::Vanished { read: omitted }];
            assert!(!project_rows(&omission_plan)[0].no_action);
            let mut actionable = plan.clone();
            if let OperationKind::SingleTransfer { source, .. } = &mut actionable.operations[0].kind
            {
                *source = read.source.clone();
            }
            assert!(!project_rows(&actionable)[0].developer_only);
            // A recognized album, error or vanished source is never routine noise.
            decision.is_album = true;
            plan.decisions = vec![
                DecisionV2::Album {
                    read: read.clone(),
                    decision,
                },
                DecisionV2::ItemError {
                    read: read.clone(),
                    error: "owned read error".into(),
                },
                DecisionV2::Vanished { read },
            ];
            assert!(project_rows(&plan).iter().all(|row| !row.developer_only));
            let projected = project_rows(&plan);
            assert!(projected[0].no_action);
            assert!(!projected[1].no_action);
            assert!(!projected[2].no_action);
            let mut album = plan.decisions[0].clone();
            if let DecisionV2::Album { read, decision } = &mut album {
                decision.is_album = false;
                for error in [
                    sorter_core::album::AlbumTraceEvent::ProtectedError {
                        kind: "owned error".into(),
                    },
                    sorter_core::album::AlbumTraceEvent::FileError {
                        source: read.source.reference.clone(),
                        kind: "owned file error".into(),
                    },
                ] {
                    decision.trace = vec![error];
                    let test_plan = JobPlanV2 {
                        decisions: vec![DecisionV2::Album {
                            read: read.clone(),
                            decision: decision.clone(),
                        }],
                        ..plan.clone()
                    };
                    let projected = project_rows(&test_plan);
                    assert!(!projected[0].developer_only);
                    assert!(!projected[0].no_action);
                    assert!(projected[0].blockers.contains(&"ALBUM_READ_ERROR".into()));
                }
            }
        }
        {
            use sorter_core::fix_plan::{FixFailure, FixFilePlan};
            let inner = h.lock().unwrap();
            let mut plan = inner.job.as_ref().unwrap()._plan.as_ref().unwrap().clone();
            plan.request.mode = JobMode::FixNames;
            plan.operations.truncate(1);
            let original = operation_source(&plan.operations[0].kind).clone();
            plan.operations[0].kind = OperationKind::FixFile(FixFilePlan {
                source: original,
                intended_name: None,
                rename_intended: false,
                repairs: vec![],
                save_intended: true,
                failure: None,
                blockers: vec![],
                executable: false,
            });
            assert!(project_rows(&plan)[0].no_action);
            if let OperationKind::FixFile(fix) = &mut plan.operations[0].kind {
                fix.rename_intended = true;
            }
            assert!(!project_rows(&plan)[0].no_action);
            if let OperationKind::FixFile(fix) = &mut plan.operations[0].kind {
                fix.rename_intended = false;
                fix.failure = Some(FixFailure::Open(
                    sorter_core::metadata::MetadataError::Parser,
                ));
            }
            assert!(!project_rows(&plan)[0].no_action);
        }
        let root_json = serde_json::to_value(&s).unwrap();
        assert_eq!(
            root_json
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            ["capabilities", "display_name", "platform", "token"]
        );
        assert!(matches!(
            root_json["platform"].as_str(),
            Some("linux_test" | "windows")
        ));
        let snapshot_json = serde_json::to_value(&snap).unwrap();
        assert_eq!(snapshot_json["host_schema"], 1);
        assert_eq!(snapshot_json["status"], "completed");
        assert!(snapshot_json["total_rows"].is_u64());
        assert_eq!(
            serde_json::to_value(&job)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            ["job_id"]
        );
        let page_json = serde_json::to_value(&page).unwrap();
        assert_eq!(
            page_json
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            ["rows", "skipped", "total"]
        );
        for row in page_json["rows"].as_array().unwrap() {
            assert_eq!(
                row.as_object().unwrap().keys().cloned().collect::<Vec<_>>(),
                [
                    "artist",
                    "blockers",
                    "destination",
                    "developer_only",
                    "id",
                    "kind",
                    "no_action",
                    "operation_ids",
                    "reason",
                    "source"
                ]
            );
        }
        assert!(
            page_json["rows"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["operation_ids"].is_array())
        );
        // Explicit fixture authoring only. Remove ephemeral absolute source paths;
        // destination, decision, reason and blockers are untouched engine projections.
        if let Some(output) = std::env::var_os("SINGLES_UPDATE_HOST_FIXTURE") {
            let mut fixture = page_json;
            for row in fixture["rows"].as_array_mut().unwrap() {
                let actual = row["source"].as_str().unwrap();
                let portable = actual.replace(&s.display_name, "<owned-source>");
                row["source"] = portable.replace('\\', "/").into();
            }
            std::fs::write(output, serde_json::to_vec_pretty(&fixture).unwrap()).unwrap();
        }
        assert!(!page.rows.is_empty());
        {
            let inner = h.lock().unwrap();
            let plan = inner.job.as_ref().unwrap()._plan.as_ref().unwrap();
            assert!(!plan.executable);
            assert!(plan.metadata_observations.iter().any(|o| matches!(
                o,
                sorter_core::metadata::MetadataObservation::Open { result: Ok(()), .. }
            )));
        }
        assert_eq!(std::fs::read(&song).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(dst).unwrap().count(), 0);
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn replacing_selection_invalidates_old_token() {
        let d = tempfile::tempdir().unwrap();
        let h = Host::default();
        let old = h.select_path(Role::Source, d.path().into()).unwrap();
        let new = h.select_path(Role::Source, d.path().into()).unwrap();
        assert_ne!(old.token, new.token);
        assert_eq!(
            h.preview(request(old.token, None, JobMode::FixNames))
                .err()
                .unwrap()
                .code,
            "INVALID_ROOT_TOKEN"
        );
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn directory_replacement_after_selection_is_blocked() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("source");
        std::fs::create_dir(&src).unwrap();
        let h = Host::default();
        let root = h.select_path(Role::Source, src.clone()).unwrap();
        std::fs::rename(&src, d.path().join("old")).unwrap();
        std::fs::create_dir(&src).unwrap();
        let job = h
            .preview(request(root.token, None, JobMode::FixNames))
            .unwrap();
        let snap = h.snapshot(&job.job_id).unwrap();
        assert_eq!(snap.status, "failed");
        assert!(snap.blockers.contains(&"ROOT_CHANGED".into()));
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn missing_target_and_empty_sort_have_distinct_failures() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let host = Host::default();
        let s = host
            .select_path(Role::Source, source.path().into())
            .unwrap();
        assert_eq!(
            host.preview(request(s.token.clone(), None, JobMode::Sort))
                .err()
                .unwrap()
                .code,
            "TARGET_REQUIRED"
        );
        let t = host
            .select_path(Role::Target, target.path().into())
            .unwrap();
        let job = host
            .preview(request(s.token.clone(), Some(t.token), JobMode::Sort))
            .unwrap();
        let snapshot = host.snapshot(&job.job_id).unwrap();
        assert_eq!(snapshot.status, "failed");
        assert_eq!(snapshot.blockers, ["SOURCE_EMPTY"]);
    }
    #[test]
    fn ipc_permissions_and_configuration_are_read_only() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
        assert_eq!(config["identifier"], "com.nhlocal.singlesorter.preview");
        assert_eq!(config["bundle"]["active"], false);
        assert_eq!(capability["windows"], serde_json::json!(["main"]));
        assert_eq!(capability["local"], true);
        assert!(capability.get("remote").is_none());
        assert_eq!(
            capability["permissions"],
            serde_json::json!(["allow-preview-commands"])
        );
        let permissions = include_str!("../permissions/preview.toml");
        assert!(!permissions.contains("confirm_apply"));
        assert!(!permissions.contains("shell:"));
        assert!(!permissions.contains("fs:"));
        assert!(!permissions.contains("dialog:"));
        assert!(!permissions.contains("opener:"));
        assert!(permissions.contains("open_footer_link"));
        assert!(permissions.contains("open_project_link"));
        assert_eq!(
            FOOTER_URL,
            "https://nhlocal.github.io/shir-bot?utm_source=singles_sorter_program&utm_medium=desktop"
        );
    }
    #[test]
    fn project_links_accept_named_destinations_only() {
        let source: ProjectLink = serde_json::from_str("\"Source\"").unwrap();
        let license: ProjectLink = serde_json::from_str("\"License\"").unwrap();
        assert_eq!(source.url(), "https://github.com/NHLOCAL/Singles-Sorter");
        assert_eq!(
            license.url(),
            "https://github.com/NHLOCAL/Singles-Sorter/blob/main/LICENSE"
        );
        for value in ["\"https://example.com\"", "\"source\"", "null", "{}"] {
            assert!(serde_json::from_str::<ProjectLink>(value).is_err());
        }
    }
    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn saved_personal_rows_feed_existing_engine_after_bundled_rows() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let target = directory.path().join("target");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        let profile = crate::profile::ProfileStore::new(directory.path().join("profile"));
        let initial = profile.list().unwrap();
        profile
            .save_list(
                &initial.revision,
                vec![vec!["סולןבדיקהייחודי".into(), "תיקייהאישית".into()]],
            )
            .unwrap();
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/media/tagged.mp3");
        std::fs::copy(fixture, source.join("סולןבדיקהייחודי - בדיקה.mp3")).unwrap();
        let host = Host::default();
        let s = host.select_path(Role::Source, source).unwrap();
        let t = host.select_path(Role::Target, target).unwrap();
        let job = host
            .preview_with_personal(
                request(s.token, Some(t.token), JobMode::Sort),
                Some(profile.personal_bytes().unwrap()),
            )
            .unwrap();
        let snapshot = host.snapshot(&job.job_id).unwrap();
        assert_eq!(snapshot.status, "completed", "{:?}", snapshot.error);
        let inner = host.lock().unwrap();
        let plan = inner.job.as_ref().unwrap()._plan.as_ref().unwrap();
        assert!(plan.request.resource_ids[0].starts_with("bundled:sha256:"));
        assert!(plan.request.resource_ids[1].starts_with("personal:sha256:"));
        assert!(plan.operations.iter().any(|o| matches!(&o.kind, OperationKind::SingleTransfer { artist, .. } if artist == "תיקייהאישית")));
        assert!(!plan.executable);
    }
}

#[cfg(test)]
#[path = "job_lifetime_tests.rs"]
mod lifetime_tests;
