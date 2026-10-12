//! Interactive one-process execution. A JSON plan from disk is never authority.
use super::{Failure, MAX_INPUT_BYTES};
#[cfg(all(windows, feature = "selected-windows-execution"))]
#[path = "run_albums.rs"]
mod albums;
#[cfg(all(windows, feature = "selected-windows-execution"))]
pub(super) const ALBUM_CONDITION: &str = "Album execution requires that no other application or sync process changes the selected source or target until completion";
#[cfg(all(windows, feature = "selected-windows-execution"))]
pub(super) fn execution_condition(plan: &sorter_core::job_plan::JobPlanV2) -> Option<&'static str> {
    use sorter_core::job_plan::{JobMode, OperationKind};
    if plan.operations.iter().any(|operation| matches!(operation.kind, OperationKind::AlbumTransfer { .. })) {
        Some(ALBUM_CONDITION)
    } else if plan.request.mode == JobMode::Sort && !plan.request.settings.copy_mode
        && sorter_executor::windows_job::supported_plan(plan).is_err()
    {
        Some("Copied Move requires ordinary local NTFS storage and no concurrent writers; each source is retired only after every bound destination is verified")
    } else {
        None
    }
}
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    process::ExitCode,
};

#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Decision {
    Apply { plan_id: String },
    Cancel,
}
fn event(kind: &str, data: Value) -> Result<(), Failure> {
    let mut output = io::stdout().lock();
    let bytes = serde_json::to_vec(&json!({"schema_version":1,"event":kind,"data":data}))
        .map_err(|e| Failure::new(1, "OUTPUT_ERROR", e.to_string()))?;
    output
        .write_all(&bytes)
        .and_then(|()| output.write_all(b"\n"))
        .and_then(|()| output.flush())
        .map_err(|e| Failure::new(1, "OUTPUT_ERROR", e.to_string()))
}
pub(super) fn run() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let _ = event(
                "error",
                json!({"code":failure.error.code,"message":failure.error.message}),
            );
            ExitCode::from(failure.exit)
        }
    }
}
#[cfg(any(test, all(windows, feature = "selected-windows-execution")))]
fn read_requests(
    mut input: impl std::io::BufRead,
    sender: std::sync::mpsc::SyncSender<Result<Vec<u8>, Failure>>,
    cancel: &std::sync::atomic::AtomicBool,
    invalid: &std::sync::atomic::AtomicBool,
) {
    use std::{
        io::{BufRead, Read},
        sync::atomic::Ordering,
    };
    let mut forwarded = 0;
    loop {
        let mut bytes = Vec::new();
        let read = input
            .by_ref()
            .take((MAX_INPUT_BYTES + 1) as u64)
            .read_until(b'\n', &mut bytes);
        match read {
            Ok(0) => break,
            Ok(_) if bytes.len() <= MAX_INPUT_BYTES => {
                if forwarded < 2 {
                    if forwarded == 1
                        && matches!(
                            serde_json::from_slice::<Decision>(&bytes),
                            Ok(Decision::Cancel)
                        )
                    {
                        cancel.store(true, Ordering::Release);
                    }
                    forwarded += 1;
                    if sender.send(Ok(bytes)).is_err() {
                        break;
                    }
                } else {
                    // Only request and confirmation enter the bounded queue.
                    // Later controls can never block behind an undrained message.
                    if !matches!(
                        serde_json::from_slice::<Decision>(&bytes),
                        Ok(Decision::Cancel)
                    ) {
                        invalid.store(true, Ordering::Release);
                    }
                    cancel.store(true, Ordering::Release);
                }
            }
            _ => {
                invalid.store(true, Ordering::Release);
                cancel.store(true, Ordering::Release);
                if forwarded < 2 {
                    let _ = sender.send(Err(Failure::new(
                        2,
                        "INVALID_INPUT",
                        "Each NDJSON request must fit within 16 MiB",
                    )));
                }
                break;
            }
        }
    }
}
#[cfg(not(all(windows, feature = "selected-windows-execution")))]
fn execute() -> Result<(), Failure> {
    // Do not consume any user paths in a build that cannot execute them.
    let _ = MAX_INPUT_BYTES;
    Err(Failure::unsupported(
        "run-files requires Windows and the selected-windows-execution feature; no files were changed",
    ))
}
#[cfg(all(windows, feature = "selected-windows-execution"))]
fn execute() -> Result<(), Failure> {
    use std::sync::{Arc, atomic::AtomicBool, mpsc};
    let cancel = Arc::new(AtomicBool::new(false));
    let reader_cancel = cancel.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let invalid_control = Arc::new(AtomicBool::new(false));
    let reader_invalid = invalid_control.clone();
    std::thread::Builder::new()
        .name("execution-stdin".into())
        .spawn(move || {
            let stdin = io::stdin();
            read_requests(stdin.lock(), tx, &reader_cancel, &reader_invalid);
        })
        .map_err(|e| Failure::new(1, "WORKER_FAILED", e.to_string()))?;
    let request_bytes = rx
        .recv()
        .map_err(|_| Failure::new(2, "INVALID_INPUT", "Expected a run-files request line"))??;
    let request: Request = serde_json::from_slice(&request_bytes)
        .map_err(|e| Failure::new(2, "INVALID_INPUT", e.to_string()))?;
    let source = request.preview.source.clone();
    let target = request.preview.target.clone();
    let journal_directory = request.journal_directory.clone();
    execute_request(
        request,
        &cancel,
        &invalid_control,
        &mut |plan, inference_executed| {
            event(
                "planned",
                json!({"plan":plan,"execution_condition":execution_condition(plan),"inference_executed":inference_executed,"apply_eligible":true,"source":source,"target":target,"journal_directory":journal_directory,"confirmation":{"action":"apply","plan_id":plan.binding_sha256}}),
            )?;
            let decision = rx.recv().map_err(|_| {
                Failure::new(
                    2,
                    "CONFIRMATION_REQUIRED",
                    "stdin ended before explicit confirmation; no files changed",
                )
            })??;
            match serde_json::from_slice::<Decision>(&decision)
                .map_err(|e| Failure::new(2, "INVALID_CONFIRMATION", e.to_string()))?
            {
                Decision::Cancel => Ok(false),
                Decision::Apply { plan_id } if plan_id == plan.binding_sha256 => Ok(true),
                Decision::Apply { .. } => Err(Failure::new(
                    2,
                    "PLAN_CHANGED",
                    "Confirmation must identify the exact retained inspected plan",
                )),
            }
        },
        &mut event,
    )
}

#[cfg(all(windows, feature = "selected-windows-execution"))]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub schema_version: u16,
    pub preview: super::plan_files::Request,
    pub journal_directory: std::path::PathBuf,
}

// Both frontends use the same live retained plan, fresh resource check, root
// grants and journaled backend. The positional invocation authorizes its own
// freshly planned job; NDJSON still requires the exact inspected plan digest.
#[cfg(all(windows, feature = "selected-windows-execution"))]
pub(super) fn execute_request(
    request: Request,
    cancel: &std::sync::atomic::AtomicBool,
    invalid_control: &std::sync::atomic::AtomicBool,
    confirm: &mut dyn FnMut(&sorter_core::job_plan::JobPlanV2, bool) -> Result<bool, Failure>,
    event: &mut dyn FnMut(&str, Value) -> Result<(), Failure>,
) -> Result<(), Failure> {
    use super::plan_files;
    use sha2::{Digest, Sha256};
    use sorter_core::{
        discovery::{ObjectIdentity, ReadStorage},
        job_plan::{AiProfile, JobMode, OperationKind},
    };
    use sorter_executor::{
        binding::{ExecutionScope, Resources, prepare_scope},
        results::{ItemStatus, JobStatus},
        windows_album::{
            AlbumScope, SelectedWindowsAlbumJob, prepare_album_scope, supported_album_plan,
            validate_album_execution,
        },
        windows_copy::WindowsPlanningStorage,
        windows_job::{
            CopiedMoveScope, SelectedWindowsJob, prepare_copied_move_scope,
            supported_copied_move_plan, supported_plan,
        },
        windows_repair::{
            RepairJobStatus, RepairScope, SelectedWindowsRepairJob, prepare_repair_scope,
            supported_repair_plan,
        },
    };
    use sorter_io::{
        desktop::{
            DesktopStorage,
            windows_mutation::{WindowsMutationStorage, WindowsSelectedRootGrant},
        },
        metadata::{MetadataReader, StorageMetadataProvider},
    };
    use std::sync::atomic::Ordering;
    if request.schema_version != 1 {
        return Err(Failure::new(
            2,
            "UNSUPPORTED_SCHEMA",
            "run-files requires schema_version 1",
        ));
    }
    plan_files::validate_path(&request.journal_directory)?;
    let planned = plan_files::execute_request_controlled(&request.preview, cancel)?;
    let plan = planned
        .plan
        .ok_or_else(|| Failure::new(1, "PLAN_MISSING", "No complete plan was produced"))?;
    if sorter_executor::no_op::is_clean_sort_noop(&plan) {
        if !confirm(&plan, planned.inference_executed)? || cancel.load(Ordering::Acquire) {
            event("cancelled", json!({"before_apply":true}))?;
            return Ok(());
        }
        let fresh = plan_files::execute_request_controlled(&request.preview, cancel)?;
        if !fresh.plan.as_ref().is_some_and(|fresh| {
            fresh.binding_sha256 == plan.binding_sha256
                && sorter_executor::no_op::is_clean_sort_noop(fresh)
        }) {
            return Err(Failure::new(
                1,
                "PLAN_CHANGED",
                "Files or resources changed; inspect a fresh plan",
            ));
        }
        if cancel.load(Ordering::Acquire) || invalid_control.load(Ordering::Acquire) {
            event("cancelled", json!({"before_apply":true}))?;
            return Err(Failure::new(
                1,
                "EXECUTION_INCOMPLETE",
                "No-op confirmation was cancelled",
            ));
        }
        // No mutation grant or journal is needed: the revalidated job has no
        // effects to perform. Never use this branch for an error/blocked plan.
        event(
            "result",
            json!({"job_id":format!("no-op-{}",plan.binding_sha256),
            "plan_id":plan.binding_sha256,"status":"complete","no_op":true,
            "source_items":0,"committed_destinations":0,"committed_operations":[],"effect_receipts":[],
            "moved_sources":0,"failed_operations":0,"blocked_operations":0,"recovery_operations":0,
            "cancelled_operations":0,"items":[],"journal_directory":null,"invalid_control":false,
            "legacy_summary":sorter_executor::legacy_summary::LegacySummary::default()}),
        )?;
        return Ok(());
    }
    let fail = |e: &dyn std::fmt::Display| Failure::new(1, "EXECUTION_REFUSED", e.to_string());
    let target = request.preview.target.as_ref();
    let repairing = plan.request.mode == JobMode::FixNames;
    let album = plan
        .operations
        .iter()
        .any(|operation| matches!(operation.kind, OperationKind::AlbumTransfer { .. }));
    let copied_move = !repairing && !album && supported_plan(&plan).is_err();
    let mut reads: Box<dyn ReadStorage + '_> = if repairing {
        supported_repair_plan(&plan).map_err(|e| fail(&e))?;
        Box::new(
            WindowsPlanningStorage::for_repair_preview(&request.preview.source)
                .map_err(|e| fail(&e))?,
        )
    } else {
        if album {
            supported_album_plan(&plan).map_err(|e| fail(&e))?;
        } else if copied_move {
            supported_copied_move_plan(&plan).map_err(|e| fail(&e))?;
        } else {
            supported_plan(&plan).map_err(|e| fail(&e))?;
        }
        Box::new(
            WindowsPlanningStorage::for_album_preview(
                &request.preview.source,
                target.ok_or_else(|| {
                    Failure::new(2, "TARGET_REQUIRED", "Sort execution requires a target")
                })?,
                sorter_io::desktop::WindowsDirectoryReadLimits::ALBUM_DEFAULT,
                cancel,
            )
            .map_err(|e| fail(&e))?,
        )
    };
    let digest = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let resource_hashes = plan
        .request
        .resource_ids
        .iter()
        .map(|id| {
            id.rsplit_once(":sha256:")
                .map(|(_, hash)| hash.to_owned())
                .ok_or_else(|| Failure::new(1, "RESOURCE_INVALID", "Resource digest is missing"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let models = plan
        .request
        .resource_ids
        .iter()
        .filter(|id| id.starts_with("original-"))
        .collect::<Vec<_>>();
    let resources = Resources {
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
            .then(|| digest(&serde_json::to_vec(&models).expect("string IDs"))),
    };
    enum Scope {
        Sort(ExecutionScope),
        CopiedMove(Box<CopiedMoveScope>, ObjectIdentity),
        Repair(RepairScope),
        Album(Box<AlbumScope>),
    }
    let scope = if repairing {
        Scope::Repair(
            prepare_repair_scope(&plan, reads.as_mut(), &resources, cancel)
                .map_err(|e| fail(&e))?,
        )
    } else if album {
        Scope::Album(Box::new(
            prepare_album_scope(&plan, reads.as_mut(), &resources, cancel).map_err(|e| fail(&e))?,
        ))
    } else if copied_move {
        // Preview only: every root is independently pinned, but no journal/file
        // is created. Auxiliary-stream evidence is retained through confirmation.
        let mut journal_reads = DesktopStorage::new(std::slice::from_ref(&request.journal_directory))
            .map_err(|e| fail(&e))?;
        let journal_ref = journal_reads.reference(&request.journal_directory).map_err(|e| fail(&e))?;
        let journal_identity = journal_reads.stat(&journal_ref).map_err(|e| fail(&e))?.identity
            .ok_or_else(|| Failure::new(1, "JOURNAL_UNAVAILABLE", "Journal identity missing"))?;
        let source_identity = reads.stat(&plan.request.source).map_err(|e| fail(&e))?.identity
            .ok_or_else(|| Failure::new(1, "ROOT_CHANGED", "Source identity missing"))?;
        let target_reference = plan.request.target.as_ref().ok_or_else(|| Failure::new(2, "TARGET_REQUIRED", "Sort requires target"))?;
        let target_identity = reads.stat(target_reference).map_err(|e| fail(&e))?.identity
            .ok_or_else(|| Failure::new(1, "ROOT_CHANGED", "Target identity missing"))?;
        let grant = |path, identity| WindowsSelectedRootGrant::new(path, identity).map_err(|e| fail(&e));
        let roots = WindowsMutationStorage::for_selected_roots(
            grant(request.preview.source.clone(), source_identity)?,
            grant(target.expect("copied target checked").clone(), target_identity)?,
            grant(request.journal_directory.clone(), journal_identity.clone())?,
        ).map_err(|e| fail(&e))?;
        Scope::CopiedMove(Box::new(prepare_copied_move_scope(
            &plan, reads.as_mut(), &roots, &resources, cancel,
        ).map_err(|e| fail(&e))?), journal_identity)
    } else {
        let scope =
            prepare_scope(&plan, reads.as_mut(), &resources, cancel).map_err(|e| fail(&e))?;
        if !plan.request.settings.copy_mode
            && !matches!((&scope.source_root,&scope.target_root),(ObjectIdentity::Windows {volume:a,..},ObjectIdentity::Windows {volume:b,..}) if a==b)
        {
            return Err(Failure::unsupported(
                "Cross-volume Move is unsupported; select Copy and inspect a fresh plan",
            ));
        }
        Scope::Sort(scope)
    };
    if !confirm(&plan, planned.inference_executed)? {
        event("cancelled", json!({"before_apply":true}))?;
        return Ok(());
    }
    if cancel.load(Ordering::Acquire) {
        event("cancelled", json!({"before_apply":true}))?;
        return Ok(());
    }
    // Re-read all aliases and models before minting mutation grants. The planner
    // independently revalidates exact metadata and content against retained scope.
    let fresh = plan_files::execute_request_controlled(&request.preview, cancel)?;
    if fresh.plan.as_ref().map(|p| &p.binding_sha256) != Some(&plan.binding_sha256) {
        return Err(Failure::new(
            1,
            "PLAN_CHANGED",
            "Files or resources changed; inspect a fresh plan",
        ));
    }
    let (mut recognition, _, _, _) = plan_files::recognition(&request.preview.ai_profile)?;
    // The explicit journal directory must already exist. Do not create any
    // path before independently pinning roots and checking their separation.
    let mut journal_reads = DesktopStorage::new(std::slice::from_ref(&request.journal_directory))
        .map_err(|e| fail(&e))?;
    let journal_ref = journal_reads
        .reference(&request.journal_directory)
        .map_err(|e| fail(&e))?;
    let journal_identity = journal_reads
        .stat(&journal_ref)
        .map_err(|e| fail(&e))?
        .identity
        .ok_or_else(|| Failure::new(1, "JOURNAL_UNAVAILABLE", "Journal identity missing"))?;
    let grant =
        |path, identity| WindowsSelectedRootGrant::new(path, identity).map_err(|e| fail(&e));
    let mut reads = plan_files::ReadAdapter(reads);
    let (scope, copied) = match scope {
        Scope::Album(scope) => {
            let validated = {
                let mut metadata = StorageMetadataProvider {
                    storage: &mut reads,
                    reader: MetadataReader,
                };
                validate_album_execution(
                    &plan,
                    &mut metadata,
                    recognition.as_mut(),
                    &resources,
                    &scope,
                    cancel,
                )
                .map_err(|e| fail(&e))?
            };
            // Relinquish every source read pin before exact whole-album authority.
            drop(reads);
            drop(journal_reads);
            let mut job = SelectedWindowsAlbumJob::prepare(
                grant(
                    request.preview.source.clone(),
                    scope.execution_scope().source_root.clone(),
                )?,
                grant(
                    target.expect("album target checked").clone(),
                    scope.execution_scope().target_root.clone(),
                )?,
                grant(request.journal_directory.clone(), journal_identity)?,
                validated,
                cancel,
            )
            .map_err(|e| fail(&e))?;
            return albums::execute(
                &mut job,
                &plan,
                &request.journal_directory,
                scope.execution_scope().inspected_sources.len(),
                cancel,
                invalid_control,
                event,
            );
        }
        Scope::Repair(scope) => {
            let mut metadata = StorageMetadataProvider {
                storage: &mut reads,
                reader: MetadataReader,
            };
            let roots = WindowsMutationStorage::for_selected_repair_roots(
                grant(request.preview.source.clone(), scope.source_root().clone())?,
                grant(request.journal_directory.clone(), journal_identity)?,
            )
            .map_err(|e| fail(&e))?;
            let mut job = SelectedWindowsRepairJob::prepare(
                roots,
                &plan,
                &mut metadata,
                recognition.as_mut(),
                &resources,
                &scope,
                cancel,
            )
            .map_err(|e| fail(&e))?;
            let native_id = job.job_id().to_owned();
            event(
                "started",
                json!({"job_id":native_id,"plan_id":plan.binding_sha256,"mode":"FixNames","journal_directory":request.journal_directory}),
            )?;
            let mut output_failed = false;
            let result = job.run(cancel, &mut |progress| {
                if event("progress", json!({"job_id":native_id,"operation":progress.operation,"total_files":progress.total_files,"phase":progress.phase})).is_err() {
                    output_failed = true;
                    cancel.store(true, Ordering::Release);
                }
            });
            let (recovery, recovery_error) = if job.journal_locations().is_empty() {
                (None, None)
            } else {
                match job.inspect_recovery(cancel) {
                    Ok(report) => (Some(report), None),
                    Err(error) => (None, Some(error.to_string())),
                }
            };
            match result {
                Ok(result) => {
                    let complete = result.status == RepairJobStatus::Complete;
                    event(
                        "result",
                        json!({"job_id":native_id,"mode":"FixNames","status":result.status,"repair":result,"recovery":recovery,"recovery_error":recovery_error,"journal_directory":request.journal_directory,"invalid_control":invalid_control.load(Ordering::Acquire)}),
                    )?;
                    if output_failed || invalid_control.load(Ordering::Acquire) || !complete {
                        return Err(Failure::new(
                            1,
                            "EXECUTION_INCOMPLETE",
                            "Inspect actual repair outcomes and retained originals before retrying",
                        ));
                    }
                    return Ok(());
                }
                Err(error) => {
                    event(
                        "result",
                        json!({"job_id":native_id,"mode":"FixNames","status":if job.journal_locations().is_empty() && cancel.load(Ordering::Acquire) {"cancelled"} else {"needs_recovery"},"error":error.to_string(),"recovery":recovery,"recovery_error":recovery_error,"journal_directory":request.journal_directory}),
                    )?;
                    return Err(Failure::new(
                        1,
                        "EXECUTION_INCOMPLETE",
                        "Inspect the repair journal; no automatic replay or backup deletion was attempted",
                    ));
                }
            }
        }
        Scope::Sort(scope) => (scope, None),
        Scope::CopiedMove(scope, expected_journal) => {
            if journal_identity != expected_journal {
                return Err(Failure::new(1, "JOURNAL_CHANGED", "Journal storage changed; inspect a fresh plan"));
            }
            (scope.execution_scope().clone(), Some(scope))
        }
    };
    let mut metadata = StorageMetadataProvider {
        storage: &mut reads,
        reader: MetadataReader,
    };
    let roots = WindowsMutationStorage::for_selected_roots(
        grant(request.preview.source.clone(), scope.source_root.clone())?,
        grant(
            target
                .ok_or_else(|| Failure::new(2, "TARGET_REQUIRED", "Sort requires target"))?
                .clone(),
            scope.target_root.clone(),
        )?,
        grant(request.journal_directory.clone(), journal_identity)?,
    )
    .map_err(|e| fail(&e))?;
    let mut job = if let Some(copied) = &copied {
        SelectedWindowsJob::prepare_copied_move(
            roots, &plan, &mut metadata, recognition.as_mut(), &resources, copied, cancel,
        )
    } else {
        SelectedWindowsJob::prepare(
            roots, &plan, &mut metadata, recognition.as_mut(), &resources, &scope,
        )
    }.map_err(|e| fail(&e))?;
    let recovery_scope = sorter_executor::binding::RecoveryScope {
        original_scope: scope,
        inspected_binding_sha256: job.binding_sha256().map_err(|e| fail(&e))?,
    };
    let native_id = job.job_id().to_owned();
    event(
        "started",
        json!({"job_id":native_id,"plan_id":plan.binding_sha256,"journal_directory":request.journal_directory}),
    )?;
    let mut output_failed = false;
    let result=job.run(cancel,&mut |progress| {
        if event("progress",json!({"job_id":native_id,"operation":progress.operation,"bytes_written":progress.bytes_written,"source_length":progress.source_length})).is_err() {output_failed=true;cancel.store(true,Ordering::Release);}
    });
    let (recovery, recovery_error) = if copied.is_none() || job.journal_locations().is_empty()
        || cancel.load(Ordering::Acquire)
    {
        (None, None)
    } else {
        match job.inspect_recovery(&recovery_scope) {
            Ok(report) => {
                use sorter_executor::recovery::{CopyMetadataRecoveryStatus, RecoveryStatus};
                let verified = report.directories.complete
                    && !report.directories.incomplete_tail
                    && report.directories.pending.is_empty()
                    && report.directories.rejected.is_empty()
                    && report.transfers.as_ref().is_some_and(|transfer| !transfer.append_blocked
                        && transfer.operations.iter().all(|operation| operation.status == RecoveryStatus::Committed
                            && matches!(operation.copy_metadata, CopyMetadataRecoveryStatus::NotRequested | CopyMetadataRecoveryStatus::Verified)));
                let transfer = report.transfers.map(|transfer| json!({
                    "append_blocked":transfer.append_blocked,
                    "operations":transfer.operations.iter().map(|operation| json!({
                        "operation":operation.operation,"status":format!("{:?}",operation.status),
                        "copy_metadata":format!("{:?}",operation.copy_metadata),
                    })).collect::<Vec<_>>(),
                }));
                (Some(json!({"directories":{"observed":report.directories.observed,
                    "pending":report.directories.pending,"rejected":report.directories.rejected,
                    "complete":report.directories.complete,"incomplete_tail":report.directories.incomplete_tail},
                    "transfers":transfer})), (!verified).then(|| "TRANSFER_RECONCILIATION_INCOMPLETE".to_owned()))
            }
            Err(error) => (None, Some(error.to_string())),
        }
    };
    match result {
        Ok(result) => {
            let legacy_summary = sorter_executor::legacy_summary::LegacySummary::from_receipts(
                &plan.operations,
                &result.committed_operations,
                &job.created_directory_receipts()
                    .iter()
                    .map(|(reference, _)| reference.clone())
                    .collect::<Vec<_>>(),
                0,
                &[],
            );
            let status = match result.status {
                JobStatus::Complete => "complete",
                JobStatus::Partial => "partial",
                JobStatus::Cancelled => "cancelled",
                JobStatus::NeedsRecovery => "needs_recovery",
            };
            let items=result.items.iter().map(|item|json!({"operation":item.operation,"status":match item.status {ItemStatus::Complete=>"complete",ItemStatus::SourceRetained=>"source_retained",ItemStatus::Failed=>"failed",ItemStatus::Cancelled=>"cancelled",ItemStatus::Blocked=>"blocked",ItemStatus::NeedsRecovery=>"needs_recovery"},"detail":item.detail})).collect::<Vec<_>>();
            event(
                "result",
                json!({"job_id":native_id,"status":status,"source_items":result.source_items,"committed_destinations":result.committed_destinations,"committed_operations":result.committed_operations,"effect_receipts":result.effect_receipts,"legacy_summary":legacy_summary,"recovery":recovery,"recovery_error":recovery_error,"moved_sources":result.moved_sources,"failed_operations":result.failed_operations,"blocked_operations":result.blocked_operations,"recovery_operations":result.recovery_operations,"cancelled_operations":result.cancelled_operations,"items":items,"journal_directory":request.journal_directory,"invalid_control":invalid_control.load(Ordering::Acquire)}),
            )?;
            if output_failed || invalid_control.load(Ordering::Acquire) || status != "complete" || recovery_error.is_some() {
                return Err(Failure::new(
                    1,
                    "EXECUTION_INCOMPLETE",
                    "Read the actual result and journal before retrying; completed changes were not rolled back",
                ));
            }
        }
        Err(error) => {
            event(
                "result",
                json!({"job_id":native_id,"status":if job.journal_locations().is_empty() && cancel.load(Ordering::Acquire) {"cancelled"} else {"needs_recovery"},"error":error.to_string(),"recovery":recovery,"recovery_error":recovery_error,"journal_directory":request.journal_directory}),
            )?;
            return Err(Failure::new(
                1,
                "EXECUTION_INCOMPLETE",
                "Inspect the journal; no automatic replay or rollback was attempted",
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_or_malformed_post_confirmation_commands_cannot_block_cancel() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        };
        for extra in [
            "{\"action\":\"apply\",\"plan_id\":\"duplicate\"}\n",
            "not-json\n",
        ] {
            let input = format!(
                "{{}}\n{{\"action\":\"apply\",\"plan_id\":\"inspected\"}}\n{}{{\"action\":\"cancel\"}}\n",
                extra.repeat(32)
            );
            let cancel = Arc::new(AtomicBool::new(false));
            let invalid = Arc::new(AtomicBool::new(false));
            let (tx, rx) = mpsc::sync_channel(1);
            let (finished_tx, finished_rx) = mpsc::channel();
            let c = cancel.clone();
            let i = invalid.clone();
            let worker = std::thread::spawn(move || {
                read_requests(std::io::Cursor::new(input), tx, &c, &i);
                finished_tx.send(()).unwrap();
            });
            assert!(
                rx.recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap()
                    .is_ok()
            );
            assert!(
                rx.recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap()
                    .is_ok()
            );
            finished_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .expect("reader blocked on commands nobody consumes");
            worker.join().unwrap();
            assert!(cancel.load(Ordering::Acquire));
            assert!(invalid.load(Ordering::Acquire));
            assert!(rx.try_recv().is_err());
        }
    }
    #[test]
    fn confirmation_is_strict_and_cannot_supply_a_replacement_plan() {
        assert!(
            matches!(serde_json::from_str::<Decision>(r#"{"action":"apply","plan_id":"abc"}"#),Ok(Decision::Apply {plan_id}) if plan_id=="abc")
        );
        assert!(
            serde_json::from_str::<Decision>(
                r#"{"action":"apply","plan_id":"abc","source":"elsewhere"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<Decision>(r#"{"action":"apply","plan_id":"a","plan_id":"b"}"#)
                .is_err()
        );
        assert!(serde_json::from_str::<Decision>(r#"{"action":"cancel"}"#).is_ok());
    }
}
