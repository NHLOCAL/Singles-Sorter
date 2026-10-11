use crate::job_owner::*;
use crate::profile::*;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
#[cfg(any(windows, target_os = "linux"))]
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn select_root(
    app: tauri::AppHandle,
    state: State<'_, Host>,
    role: Role,
) -> Result<Option<RootSelection>, HostError> {
    let host = state.inner().clone();
    let guard = host.claim()?;
    #[cfg(any(windows, target_os = "linux"))]
    return tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let Some(file) = app.dialog().file().blocking_pick_folder() else {
            return Ok(None);
        };
        let path = file.into_path().map_err(|_| {
            HostError::new("UNSUPPORTED_ROOT", "A desktop directory path is required")
        })?;
        host.select_path(role, path).map(Some)
    })
    .await
    .map_err(|e| HostError::new("WORKER_FAILED", e.to_string()))?;
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (app, guard);
        Err(HostError::new(
            "PLATFORM_UNAVAILABLE",
            "Android SAF selection is not implemented yet",
        ))
    }
}
#[tauri::command]
pub async fn resolve_typed_root(
    state: State<'_, Host>,
    role: Role,
    text: String,
) -> Result<RootSelection, HostError> {
    let host = state.inner().clone();
    let guard = host.claim()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        host.select_path(role, PathBuf::from(text))
    })
    .await
    .map_err(|e| HostError::new("WORKER_FAILED", e.to_string()))?
}
#[tauri::command]
pub async fn start_preview(
    state: State<'_, Host>,
    request: PreviewRequest,
    profile: State<'_, ProfileStore>,
) -> Result<JobId, HostError> {
    let host = state.inner().clone();
    let guard = host.claim()?;
    let profile = profile.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let personal = profile.personal_bytes()?;
        host.start(request, personal, guard)
    })
    .await
    .map_err(|e| HostError::new("WORKER_FAILED", e.to_string()))?
}
#[tauri::command]
pub fn job_snapshot(state: State<'_, Host>, job_id: String) -> Result<JobSnapshot, HostError> {
    state.snapshot(&job_id)
}
#[tauri::command]
pub fn get_plan_page(
    state: State<'_, Host>,
    job_id: String,
    offset: usize,
    limit: usize,
    developer_mode: Option<bool>,
) -> Result<PlanPage, HostError> {
    state.page_view(&job_id, offset, limit, developer_mode.unwrap_or(false))
}
#[tauri::command]
pub async fn get_settings(
    profile: State<'_, ProfileStore>,
) -> Result<sorter_core::Settings, HostError> {
    let profile = profile.inner().clone();
    blocking(move || profile.settings()).await
}
#[tauri::command]
pub async fn get_model_status(state: State<'_, Host>) -> Result<ModelStatus, HostError> {
    let host = state.inner().clone();
    let guard = host.claim()?;
    blocking(move || host.model_status(&guard)).await
}
#[tauri::command]
pub fn cancel_preview(state: State<'_, Host>, job_id: String) -> Result<(), HostError> {
    state.cancel(&job_id)
}

#[tauri::command]
pub async fn open_footer_link(app: tauri::AppHandle) -> Result<(), HostError> {
    use tauri_plugin_opener::OpenerExt;
    tauri::async_runtime::spawn_blocking(move || {
        app.opener()
            .open_url(FOOTER_URL, None::<&str>)
            .map_err(|e| HostError::new("OPEN_LINK_FAILED", e.to_string()))
    })
    .await
    .map_err(|e| HostError::new("WORKER_FAILED", e.to_string()))?
}

#[tauri::command]
pub async fn open_project_link(app: tauri::AppHandle, target: ProjectLink) -> Result<(), HostError> {
    use tauri_plugin_opener::OpenerExt;
    tauri::async_runtime::spawn_blocking(move || {
        app.opener()
            .open_url(target.url(), None::<&str>)
            .map_err(|e| HostError::new("OPEN_LINK_FAILED", e.to_string()))
    })
    .await
    .map_err(|e| HostError::new("WORKER_FAILED", e.to_string()))?
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, HostError> + Send + 'static,
) -> Result<T, HostError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| HostError::new("WORKER_FAILED", e.to_string()))?
}
#[tauri::command]
pub async fn save_main_settings(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
    settings: MainSettings,
) -> Result<sorter_core::Settings, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    blocking(move || {
        let _guard = guard;
        profile.save_main(settings)
    })
    .await
}
#[tauri::command]
pub async fn set_duet(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
    enabled: bool,
) -> Result<sorter_core::Settings, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    blocking(move || {
        let _guard = guard;
        profile.set_duet(enabled)
    })
    .await
}
#[tauri::command]
pub async fn get_personal_list(
    profile: State<'_, ProfileStore>,
) -> Result<PersonalList, HostError> {
    let profile = profile.inner().clone();
    blocking(move || profile.list()).await
}
#[tauri::command]
pub async fn save_personal_list(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
    revision: String,
    rows: Vec<Vec<String>>,
) -> Result<PersonalList, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    blocking(move || {
        let _guard = guard;
        profile.save_list(&revision, rows)
    })
    .await
}
#[tauri::command]
pub async fn select_personal_import(
    app: tauri::AppHandle,
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
) -> Result<Option<ImportSelection>, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    #[cfg(any(windows, target_os = "linux"))]
    return blocking(move || {
        let _guard = guard;
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("CSV", &["csv"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| HostError::new("UNSUPPORTED_PATH", "Desktop CSV path required"))?;
        let filename = path
            .file_name()
            .ok_or_else(|| HostError::new("INVALID_PATH", "CSV file name missing"))?
            .to_string_lossy()
            .into_owned();
        let bytes = read_import_file(&path)?;
        profile.stage_import(filename, bytes).map(Some)
    })
    .await;
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (app, guard, profile);
        Err(HostError::new(
            "PLATFORM_UNAVAILABLE",
            "Android document import not integrated",
        ))
    }
}
#[tauri::command]
pub async fn preview_personal_import(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
    token: String,
    encoding: ImportEncoding,
) -> Result<ImportPreview, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    blocking(move || {
        let _guard = guard;
        profile.preview_import(&token, encoding)
    })
    .await
}
#[tauri::command]
pub async fn confirm_personal_import(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
    token: String,
    encoding: ImportEncoding,
    revision: String,
    edits: Option<Vec<ImportEdit>>,
) -> Result<PersonalList, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    blocking(move || {
        let _guard = guard;
        profile.confirm_import(
            &token,
            encoding,
            &revision,
            edits.as_deref().unwrap_or_default(),
        )
    })
    .await
}
#[derive(Serialize)]
pub struct ExportResult {
    filename: String,
    rows: usize,
}
#[tauri::command]
pub async fn export_personal_list(
    app: tauri::AppHandle,
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
) -> Result<Option<ExportResult>, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    #[cfg(any(windows, target_os = "linux"))]
    return blocking(move || {
        let _guard = guard;
        let list = profile.list()?;
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("CSV", &["csv"])
            .set_file_name("personal-singer-list.csv")
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let path = file
            .into_path()
            .map_err(|_| HostError::new("UNSUPPORTED_PATH", "Desktop export path required"))?;
        let filename = export_new_csv(path, &list.rows)?;
        Ok(Some(ExportResult {
            filename,
            rows: list.rows.len(),
        }))
    })
    .await;
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (app, guard, profile);
        Err(HostError::new(
            "PLATFORM_UNAVAILABLE",
            "Android document export not integrated",
        ))
    }
}
#[tauri::command]
pub async fn get_first_run_status(
    profile: State<'_, ProfileStore>,
) -> Result<FirstRunStatus, HostError> {
    let profile = profile.inner().clone();
    blocking(move || profile.first_run()).await
}
#[tauri::command]
pub async fn mark_first_run_seen(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
) -> Result<FirstRunStatus, HostError> {
    let guard = state.claim()?;
    let profile = profile.inner().clone();
    blocking(move || {
        let _guard = guard;
        profile.mark_first_run()
    })
    .await
}
#[cfg(any(windows, target_os = "linux"))]
fn read_import_file(path: &std::path::Path) -> Result<Vec<u8>, HostError> {
    #[cfg(windows)]
    use sorter_io::desktop::DesktopStorage as Desktop;
    #[cfg(target_os = "linux")]
    use sorter_io::desktop::LinuxTestStorage as Desktop;
    use sorter_io::storage::{EntryKind, ReadStorage};
    use std::io::Read;
    let parent = path
        .parent()
        .ok_or_else(|| HostError::new("INVALID_PATH", "CSV directory missing"))?;
    let mut storage = Desktop::new(&[parent.to_owned()])
        .map_err(|e| HostError::new("IMPORT_FAILED", e.to_string()))?;
    let reference = storage
        .reference(path)
        .map_err(|e| HostError::new("IMPORT_FAILED", e.to_string()))?;
    if storage
        .stat(&reference)
        .map_err(|e| HostError::new("IMPORT_FAILED", e.to_string()))?
        .kind
        != EntryKind::File
    {
        return Err(HostError::new("INVALID_PATH", "CSV must be a regular file"));
    }
    let input = storage
        .open_read(&reference)
        .map_err(|e| HostError::new("IMPORT_FAILED", e.to_string()))?;
    if input.bound() > MAX_CSV_BYTES as u64 {
        return Err(HostError::new("CSV_TOO_LARGE", "CSV exceeds 16 MiB"));
    }
    let mut bytes = vec![];
    input
        .take(MAX_CSV_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| HostError::new("IMPORT_FAILED", e.to_string()))?;
    if bytes.len() > MAX_CSV_BYTES {
        return Err(HostError::new("CSV_TOO_LARGE", "CSV exceeds 16 MiB"));
    }
    Ok(bytes)
}

#[tauri::command]
pub async fn start_apply(
    state: State<'_, Host>,
    profile: State<'_, ProfileStore>,
    job_id: String,
    plan_id: String,
) -> Result<JobId, HostError> {
    let host = state.inner().clone();
    let guard = host.claim()?;
    let profile = profile.inner().clone();
    blocking(move || host.start_apply(&job_id, &plan_id, profile.personal_bytes()?, guard)).await
}
#[tauri::command]
pub fn execution_snapshot(
    state: State<'_, Host>,
    job_id: String,
) -> Result<ExecutionSnapshot, HostError> {
    state.execution_snapshot(&job_id)
}
#[tauri::command]
pub fn cancel_apply(state: State<'_, Host>, job_id: String) -> Result<(), HostError> {
    state.cancel_apply(&job_id)
}

#[tauri::command]
pub fn latest_execution(state: State<'_, Host>) -> Result<Option<RetainedExecution>, HostError> {
    state.latest_execution()
}
