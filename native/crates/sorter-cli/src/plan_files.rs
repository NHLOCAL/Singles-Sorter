//! Strict v2 real-file preview transport. No selected-tree write API is used.
use super::{Failure, ProtocolError, read_request};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sorter_core::{
    Settings,
    job_plan::{
        AiProfile, DisabledRecognition, JobMode, JobPlanV2, JobRequestV2, PlanningBoundary,
        PlanningControl, RecognitionProvider, UncontrolledPlanning, plan_job_controlled,
    },
};
#[cfg(windows)]
use sorter_io::desktop::DesktopStorage as Desktop;
#[cfg(target_os = "linux")]
use sorter_io::desktop::LinuxTestStorage as Desktop;
use sorter_io::{
    metadata::{MetadataReader, StorageMetadataProvider},
    resources::{AliasSource, load_aliases},
    storage::{EntryKind, ReadStorage, SharedReadStorage, StorageError, StorageErrorKind},
};
use std::{
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

const BUNDLED: &[u8] =
    include_bytes!("../../../tests/fixtures/full_contract/released-singer-list.csv");
const COVERAGE: &str = include_str!("../../../tests/fixtures/media/metadata-coverage.json");
const MAX_RESOURCE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub(super) schema_version: u16,
    pub(super) mode: JobMode,
    pub(super) source: PathBuf,
    pub(super) target: Option<PathBuf>,
    pub(super) settings: Settings,
    pub(super) ai_profile: RequestedAi,
    pub(super) personal_aliases: Option<PathBuf>,
}
#[cfg(feature = "original-ai")]
type RequestedLimits = sorter_recognition::Limits;
#[cfg(not(feature = "original-ai"))]
type RequestedLimits = serde::de::IgnoredAny;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) enum RequestedAi {
    Disabled,
    Configured {
        bundle_id: String,
        model_directory: Option<PathBuf>,
        #[serde(default)]
        limits: Option<RequestedLimits>,
    },
}
#[derive(Serialize)]
pub(super) struct ResultPayload {
    command: &'static str,
    pub(super) inference_executed: bool,
    executable: bool,
    pub(super) plan: Option<JobPlanV2>,
    resources: Option<Value>,
    capabilities: Value,
    blockers: Vec<String>,
}
impl ResultPayload {
    fn empty() -> Self {
        Self {
            command: "plan-files",
            inference_executed: false,
            executable: false,
            plan: None,
            resources: None,
            capabilities: capabilities(),
            blockers: vec![],
        }
    }
}
pub(super) fn capabilities() -> Value {
    json!({
        "schema_versions": [2], "commands": ["plan-files"],
        "input_provenance": "real_read_only_observations",
        "storage_backend": if cfg!(windows) {"windows_read_only_runtime_acceptance_pending"} else if cfg!(target_os="linux") {"linux_test_only"} else {"unsupported"},
        "metadata_bridge_compiled": MetadataReader::is_available(),
        "metadata_coverage": serde_json::from_str::<Value>(COVERAGE).expect("embedded reviewed coverage JSON"),
        "metadata_coverage_sha256": hash(COVERAGE.as_bytes()),
        "native_inference": cfg!(feature="original-ai"), "physical_apply": false,
        "native_inference_scope": "explicit_verified_original_bundle_read_only_preview",
        "model_assets_embedded": false, "bundle_validation": "both models verified locally for each configured job",
        "cooperative_cancellation": "library_job_token_only; no CLI Ctrl-C handler",
        "char35_enabled": false, "ui_inference": false,
        "android_provider": false, "windows_runtime_validated": false,
        "csv_parsing": true, "directory_traversal": cfg!(any(windows, target_os="linux")),
        "album_preview": true, "fix_names_preview": true,
        "ai_profiles": ["intentionally_disabled", "original-v14-ner-3.7.5_requires_explicit_verified_directory"],
        "path_input": "absolute UTF-8 desktop paths; Windows observations retain UTF-16 identity; no Android URI conversion",
        "resource_limit_bytes": MAX_RESOURCE_BYTES,
        "writes_selected_trees_or_preferences": false,
        "access_time_may_change": true
    })
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn validate_path(path: &Path) -> Result<(), Failure> {
    // Path::components normalizes interior ".", so check the original text too.
    let invalid_text = path.to_str().is_none_or(|raw| {
        raw.contains('\0')
            || raw
                .split(|c| c == '/' || (cfg!(windows) && c == '\\'))
                .any(|part| matches!(part, "." | ".."))
    });
    if invalid_text
        || !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
    {
        return Err(Failure::new(
            2,
            "INVALID_INPUT",
            "Paths must be absolute UTF-8, without NUL or dot components; CWD is never a resource search path",
        ));
    }
    Ok(())
}
fn storage_failure(e: StorageError) -> Failure {
    Failure::new(
        if matches!(
            e.kind,
            StorageErrorKind::NotDirectory | StorageErrorKind::InvalidReference
        ) {
            2
        } else {
            1
        },
        &format!("STORAGE_{:?}", e.kind),
        e.detail,
    )
}
#[cfg(any(windows, target_os = "linux"))]
pub(super) fn resource_bytes(path: &Path) -> Result<Vec<u8>, Failure> {
    validate_path(path)?;
    let parent = path.parent().ok_or_else(|| {
        Failure::new(
            2,
            "RESOURCE_INVALID",
            "Personal aliases must name a regular CSV file",
        )
    })?;
    let read_error = |e: StorageError| Failure::new(1, "RESOURCE_READ_FAILED", e.to_string());
    let mut storage = Desktop::new(&[parent.to_owned()]).map_err(read_error)?;
    let reference = storage.reference(path).map_err(read_error)?;
    let entry = storage.stat(&reference).map_err(read_error)?;
    if entry.kind != EntryKind::File {
        return Err(Failure::new(
            2,
            "RESOURCE_INVALID",
            "Personal aliases must be a regular CSV file",
        ));
    }
    let input = storage.open_read(&reference).map_err(read_error)?;
    if input.bound() > MAX_RESOURCE_BYTES {
        return Err(Failure::new(
            2,
            "RESOURCE_TOO_LARGE",
            "Personal aliases exceed 16 MiB",
        ));
    }
    let mut bytes = Vec::new();
    input
        .take(MAX_RESOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Failure::new(1, "RESOURCE_READ_FAILED", e.to_string()))?;
    if bytes.len() as u64 > MAX_RESOURCE_BYTES {
        return Err(Failure::new(
            2,
            "RESOURCE_TOO_LARGE",
            "Personal aliases exceed 16 MiB",
        ));
    }
    Ok(bytes)
}

type ConfiguredRecognition = (
    Box<dyn RecognitionProvider>,
    Box<dyn PlanningControl>,
    Vec<String>,
    Vec<String>,
);
pub(super) fn recognition(request: &RequestedAi) -> Result<ConfiguredRecognition, Failure> {
    match request {
        RequestedAi::Disabled => Ok((
            Box::new(DisabledRecognition),
            Box::new(UncontrolledPlanning),
            vec![],
            vec![],
        )),
        RequestedAi::Configured {
            bundle_id,
            model_directory,
            limits,
        } => {
            let unavailable = || {
                Failure::new(
                    3,
                    "RECOGNITION_UNAVAILABLE",
                    "Configured recognition requires the original-ai build, profile original-v14-ner-3.7.5 and an explicit verified original model directory",
                )
            };
            let directory = model_directory.as_deref().ok_or_else(unavailable)?;
            validate_path(directory)?;
            #[cfg(not(feature = "original-ai"))]
            {
                let _ = (bundle_id, limits);
                Err(unavailable())
            }
            #[cfg(feature = "original-ai")]
            {
                use sorter_recognition::{BUNDLE_ID, JobCancellation, Limits, OriginalRecognition};
                if bundle_id != BUNDLE_ID {
                    return Err(unavailable());
                }
                let limits: Limits = limits.clone().unwrap_or_default();
                let control = JobCancellation::new();
                let provider =
                    OriginalRecognition::load(directory, bundle_id, limits, control.clone())
                        .map_err(|e| match e {
                            sorter_core::job_plan::RecognitionError::Abort(abort) => {
                                let e: sorter_core::job_plan::JobError = abort.into();
                                Failure::new(1, &e.code, e.detail)
                            }
                            _ => Failure::new(
                                3,
                                "RECOGNITION_UNAVAILABLE",
                                "Both original models must pass complete local asset validation",
                            ),
                        })?;
                let resources = provider.resource_ids();
                let adapters = provider.adapter_versions();
                Ok((Box::new(provider), Box::new(control), resources, adapters))
            }
        }
    }
}

fn execute() -> Result<ResultPayload, Failure> {
    let request: Request = serde_json::from_slice(&read_request()?).map_err(|e| {
        Failure::new(
            2,
            "INVALID_INPUT",
            format!("Expected one strict v2 UTF-8 JSON request: {e}"),
        )
    })?;
    execute_request(&request)
}
pub(super) fn execute_request(request: &Request) -> Result<ResultPayload, Failure> {
    if request.schema_version != 2 {
        return Err(Failure::new(
            2,
            "UNSUPPORTED_SCHEMA",
            "plan-files requires schema_version 2",
        ));
    }
    validate_path(&request.source)?;
    if let Some(target) = &request.target {
        validate_path(target)?;
    }
    if request.mode == JobMode::Sort && request.target.is_none() {
        return Err(Failure::new(
            2,
            "TARGET_REQUIRED",
            "Sort requires an existing target directory",
        ));
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    return Err(Failure::unsupported(
        "Real-file desktop storage is unavailable on this platform",
    ));
    #[cfg(any(windows, target_os = "linux"))]
    {
        let personal = request
            .personal_aliases
            .as_deref()
            .map(resource_bytes)
            .transpose()?;
        let bundle = load_aliases(BUNDLED, personal.as_deref())
            .map_err(|e| Failure::new(2, "RESOURCE_INVALID", e.to_string()))?;
        let source_name = |s: AliasSource| match s {
            AliasSource::Bundled => "bundled",
            AliasSource::Personal => "personal",
        };
        let resources = json!({
            "sources": bundle.source_hashes.iter().map(|s|json!({"source":source_name(s.source),"sha256":s.sha256,"bytes":s.bytes,"records":s.records})).collect::<Vec<_>>(),
            "diagnostics": bundle.diagnostics.iter().map(|d|json!({"code":d.code,"source":source_name(d.source),"index":d.index,"record":d.record,"width":d.width,"blocks_apply":d.blocks_apply})).collect::<Vec<_>>()
        });
        let mut roots = vec![request.source.clone()];
        if let Some(target) = &request.target {
            roots.push(target.clone());
        }
        let mut desktop = Desktop::new(&roots).map_err(storage_failure)?;
        let source = desktop
            .reference(&request.source)
            .map_err(storage_failure)?;
        let target = request
            .target
            .as_deref()
            .map(|p| desktop.reference(p))
            .transpose()
            .map_err(storage_failure)?;
        #[cfg(all(windows, feature = "selected-windows-execution"))]
        let desktop: Box<dyn ReadStorage> = if let Some(target) = &request.target {
            Box::new(
                sorter_executor::windows_copy::WindowsPlanningStorage::for_preview(
                    &request.source,
                    target,
                )
                .map_err(|e| Failure::new(1, "STORAGE_ERROR", e.to_string()))?,
            )
        } else if request.mode == JobMode::FixNames {
            Box::new(
                sorter_executor::windows_copy::WindowsPlanningStorage::for_repair_preview(
                    &request.source,
                )
                .map_err(|e| Failure::new(1, "STORAGE_ERROR", e.to_string()))?,
            )
        } else {
            Box::new(desktop)
        };
        #[cfg(not(all(windows, feature = "selected-windows-execution")))]
        let desktop: Box<dyn ReadStorage> = Box::new(desktop);
        let mut storage = SharedReadStorage::new(ReadAdapter(desktop));
        let mut metadata_storage = storage.clone();
        let mut metadata = StorageMetadataProvider {
            storage: &mut metadata_storage,
            reader: MetadataReader,
        };
        let (mut recognition, control, recognition_resources, recognition_adapters) =
            recognition(&request.ai_profile)?;
        let mut core = JobRequestV2 {
            mode: request.mode,
            source,
            target,
            settings: request.settings.clone(),
            alias_rows: bundle.ordered_rows.into_iter().map(|r| r.fields).collect(),
            resource_ids: bundle
                .source_hashes
                .iter()
                .map(|s| format!("{}:sha256:{}", source_name(s.source), s.sha256))
                .collect(),
            adapter_versions: vec![
                format!("sorter-cli:{}:plan-files-v2", env!("CARGO_PKG_VERSION")),
                format!(
                    "storage-v2:{}",
                    if cfg!(windows) {
                        "windows-runtime-unvalidated"
                    } else {
                        "linux-test-only"
                    }
                ),
                "taglib:2.3.2:3ca2d8afaa7f1cf7f6ed10e511ebc368bfacd6dcaa3dbfa690b89e502e8963dc"
                    .into(),
                "music-tag-projection:0.4.3:read-only-v2".into(),
                format!("metadata-coverage:sha256:{}", hash(COVERAGE.as_bytes())),
                "recognition:intentionally-disabled:no-model".into(),
            ],
            ai_profile: recognition.profile(),
        };
        if !matches!(core.ai_profile, AiProfile::Disabled) {
            core.adapter_versions
                .retain(|s| s != "recognition:intentionally-disabled:no-model");
        }
        core.resource_ids.extend(recognition_resources);
        core.adapter_versions.extend(recognition_adapters);
        let outcome = plan_job_controlled(
            &core,
            &mut storage,
            &mut metadata,
            recognition.as_mut(),
            control.as_ref(),
        );
        let executed = recognition.inference_executed();
        let plan = outcome.map_err(|e| {
            let mut f = Failure::new(
                if matches!(
                    e.code.as_str(),
                    "TARGET_REQUIRED"
                        | "SOURCE_NOT_DIRECTORY"
                        | "TARGET_NOT_DIRECTORY"
                        | "SAME_ROOT"
                ) {
                    2
                } else {
                    1
                },
                &e.code,
                e.detail,
            );
            f.inference_executed = executed;
            f
        })?;
        control.checkpoint(PlanningBoundary::Result).map_err(|e| {
            let e: sorter_core::job_plan::JobError = e.into();
            let mut f = Failure::new(1, &e.code, e.detail);
            f.inference_executed = executed;
            f
        })?;
        let mut result = ResultPayload::empty();
        result.inference_executed = executed;
        result.blockers = plan.blockers.clone();
        result.plan = Some(plan);
        result.resources = Some(resources);
        Ok(result)
    }
}
pub(super) fn run() -> ExitCode {
    emit(execute())
}
pub(super) fn emit(response: Result<ResultPayload, Failure>) -> ExitCode {
    #[derive(Serialize)]
    struct Envelope {
        schema_version: u16,
        status: &'static str,
        result: ResultPayload,
        error: Option<ProtocolError>,
    }
    let (exit, envelope) = match response {
        Ok(result) => (
            0,
            Envelope {
                schema_version: 2,
                status: "ok",
                result,
                error: None,
            },
        ),
        Err(f) => {
            let _ = writeln!(io::stderr().lock(), "{}: {}", f.error.code, f.error.message);
            let mut result = ResultPayload::empty();
            result.inference_executed = f.inference_executed;
            result.blockers.push(f.error.code.clone());
            (
                f.exit,
                Envelope {
                    schema_version: 2,
                    status: "error",
                    result,
                    error: Some(f.error),
                },
            )
        }
    };
    let encoded = serde_json::to_vec(&envelope).map(|mut b| {
        b.push(b'\n');
        b
    });
    let write = encoded.map_err(io::Error::other).and_then(|b| {
        let mut out = io::stdout().lock();
        out.write_all(&b).and_then(|()| out.flush())
    });
    if let Err(e) = write {
        let _ = writeln!(
            io::stderr().lock(),
            "INTERNAL_ERROR: Cannot emit response: {e}"
        );
        return ExitCode::from(1);
    }
    ExitCode::from(exit)
}

struct ReadAdapter(Box<dyn ReadStorage>);
impl ReadStorage for ReadAdapter {
    fn stat(
        &mut self,
        reference: &sorter_core::discovery::StorageRef,
    ) -> Result<sorter_core::discovery::EntryObservation, StorageError> {
        self.0.stat(reference)
    }
    fn children(
        &mut self,
        reference: &sorter_core::discovery::StorageRef,
    ) -> Result<Vec<sorter_core::discovery::EntryObservation>, StorageError> {
        self.0.children(reference)
    }
    fn open_read(
        &mut self,
        reference: &sorter_core::discovery::StorageRef,
    ) -> Result<sorter_core::discovery::ReadInput, StorageError> {
        self.0.open_read(reference)
    }
}
