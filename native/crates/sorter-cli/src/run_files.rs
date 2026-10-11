//! Interactive one-process execution. A JSON plan from disk is never authority.
use super::{Failure, MAX_INPUT_BYTES};
#[cfg(any(test, all(windows,feature="selected-windows-execution")))]
use serde::Deserialize;
use serde_json::{Value, json};
use std::{io::{self, Write}, process::ExitCode};

#[cfg(any(test, all(windows,feature="selected-windows-execution")))]
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Decision { Apply { plan_id: String }, Cancel }
fn event(kind: &str, data: Value) -> Result<(), Failure> {
    let mut output = io::stdout().lock();
    let bytes = serde_json::to_vec(&json!({"schema_version":1,"event":kind,"data":data}))
        .map_err(|e| Failure::new(1,"OUTPUT_ERROR",e.to_string()))?;
    output.write_all(&bytes).and_then(|()|output.write_all(b"\n")).and_then(|()|output.flush())
        .map_err(|e| Failure::new(1,"OUTPUT_ERROR",e.to_string()))
}
pub(super) fn run() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let _ = event("error",json!({"code":failure.error.code,"message":failure.error.message}));
            ExitCode::from(failure.exit)
        }
    }
}
#[cfg(not(all(windows,feature="selected-windows-execution")))]
fn execute() -> Result<(), Failure> {
    // Do not consume any user paths in a build that cannot execute them.
    let _ = MAX_INPUT_BYTES;
    Err(Failure::unsupported("run-files requires Windows and the selected-windows-execution feature; no files were changed"))
}
#[cfg(all(windows,feature="selected-windows-execution"))]
fn execute() -> Result<(), Failure> {
    use super::plan_files;
    use sha2::{Digest,Sha256};
    use sorter_core::{discovery::{ReadStorage,ObjectIdentity},job_plan::AiProfile};
    use sorter_executor::{binding::{Resources,prepare_scope},windows_copy::WindowsPlanningStorage,windows_job::{SelectedWindowsJob,supported_plan},results::{JobStatus,ItemStatus}};
    use sorter_io::{metadata::{MetadataReader,StorageMetadataProvider},desktop::{DesktopStorage,windows_mutation::{WindowsSelectedRootGrant,WindowsMutationStorage}}};
    use std::{io::{BufRead,Read},path::PathBuf,sync::{Arc,atomic::{AtomicBool,Ordering},mpsc}};
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request { schema_version:u16, preview:plan_files::Request, journal_directory:PathBuf }
    let cancel=Arc::new(AtomicBool::new(false));
    let reader_cancel=cancel.clone();
    let (tx,rx)=mpsc::sync_channel(8);
    std::thread::Builder::new().name("execution-stdin".into()).spawn(move || {
        let stdin=io::stdin();let mut input=stdin.lock();let mut first=true;
        loop {
            let mut bytes=Vec::new();
            let read=input.by_ref().take((MAX_INPUT_BYTES+1) as u64).read_until(b'\n',&mut bytes);
            match read {
                Ok(0)=>break,
                Ok(_) if bytes.len()<=MAX_INPUT_BYTES=>{
                    if !first && matches!(serde_json::from_slice::<Decision>(&bytes),Ok(Decision::Cancel)) {reader_cancel.store(true,Ordering::Release);}
                    first=false;if tx.send(Ok(bytes)).is_err(){break;}
                }
                _=>{reader_cancel.store(true,Ordering::Release);let _=tx.send(Err(Failure::new(2,"INVALID_INPUT","Each NDJSON request must fit within 16 MiB")));break;}
            }
        }
    }).map_err(|e|Failure::new(1,"WORKER_FAILED",e.to_string()))?;
    let request_bytes=rx.recv().map_err(|_|Failure::new(2,"INVALID_INPUT","Expected a run-files request line"))??;
    let request:Request=serde_json::from_slice(&request_bytes).map_err(|e|Failure::new(2,"INVALID_INPUT",e.to_string()))?;
    if request.schema_version!=1 {return Err(Failure::new(2,"UNSUPPORTED_SCHEMA","run-files requires schema_version 1"));}
    plan_files::validate_path(&request.journal_directory)?;
    let planned=plan_files::execute_request(&request.preview)?;
    let plan=planned.plan.ok_or_else(||Failure::new(1,"PLAN_MISSING","No complete plan was produced"))?;
    let fail=|e:&dyn std::fmt::Display|Failure::new(1,"EXECUTION_REFUSED",e.to_string());
    supported_plan(&plan).map_err(|e|fail(&e))?;
    let target=request.preview.target.as_ref().ok_or_else(||Failure::new(2,"TARGET_REQUIRED","Execution requires a target"))?;
    let mut reads=WindowsPlanningStorage::for_preview(&request.preview.source,target).map_err(|e|fail(&e))?;
    let digest=|bytes:&[u8]|format!("{:x}",Sha256::digest(bytes));
    let resource_hashes=plan.request.resource_ids.iter().map(|id|id.rsplit_once(":sha256:").map(|(_,hash)|hash.to_owned()).ok_or_else(||Failure::new(1,"RESOURCE_INVALID","Resource digest is missing"))).collect::<Result<Vec<_>,_>>()?;
    let models=plan.request.resource_ids.iter().filter(|id|id.starts_with("original-")).collect::<Vec<_>>();
    let resources=Resources {resource_ids:plan.request.resource_ids.clone(),adapter_versions:plan.request.adapter_versions.clone(),ai_profile:plan.request.ai_profile.clone(),resource_hashes,adapter_hashes:plan.request.adapter_versions.iter().map(|s|digest(s.as_bytes())).collect(),model_hash:(!matches!(plan.request.ai_profile,AiProfile::Disabled)).then(||digest(&serde_json::to_vec(&models).expect("string IDs")))};
    let scope=prepare_scope(&plan,&mut reads,&resources,&cancel).map_err(|e|fail(&e))?;
    if !plan.request.settings.copy_mode && !matches!((&scope.source_root,&scope.target_root),(ObjectIdentity::Windows {volume:a,..},ObjectIdentity::Windows {volume:b,..}) if a==b) {return Err(Failure::unsupported("Cross-volume Move is unsupported; select Copy and inspect a fresh plan"));}
    event("planned",json!({"plan":plan,"inference_executed":planned.inference_executed,"apply_eligible":true,"source":request.preview.source,"target":target,"journal_directory":request.journal_directory,"confirmation":{"action":"apply","plan_id":plan.binding_sha256}}))?;
    let decision=rx.recv().map_err(|_|Failure::new(2,"CONFIRMATION_REQUIRED","stdin ended before explicit confirmation; no files changed"))??;
    match serde_json::from_slice::<Decision>(&decision).map_err(|e|Failure::new(2,"INVALID_CONFIRMATION",e.to_string()))? {
        Decision::Cancel=>{event("cancelled",json!({"before_apply":true}))?;return Ok(());}
        Decision::Apply {plan_id} if plan_id==plan.binding_sha256=>{}
        Decision::Apply {..}=>return Err(Failure::new(2,"PLAN_CHANGED","Confirmation must identify the exact retained inspected plan")),
    }
    if cancel.load(Ordering::Acquire) {event("cancelled",json!({"before_apply":true}))?;return Ok(());}
    // Re-read all aliases and models before minting mutation grants. The planner
    // independently revalidates exact metadata and content against retained scope.
    let fresh=plan_files::execute_request(&request.preview)?;
    if fresh.plan.as_ref().map(|p|&p.binding_sha256)!=Some(&plan.binding_sha256) {return Err(Failure::new(1,"PLAN_CHANGED","Files or resources changed; inspect a fresh plan"));}
    let (mut recognition,_,_,_)=plan_files::recognition(&request.preview.ai_profile)?;
    std::fs::create_dir_all(&request.journal_directory).map_err(|e|fail(&e))?;
    let mut journal_reads=DesktopStorage::new(std::slice::from_ref(&request.journal_directory)).map_err(|e|fail(&e))?;
    let journal_ref=journal_reads.reference(&request.journal_directory).map_err(|e|fail(&e))?;
    let journal_identity=journal_reads.stat(&journal_ref).map_err(|e|fail(&e))?.identity.ok_or_else(||Failure::new(1,"JOURNAL_UNAVAILABLE","Journal identity missing"))?;
    let grant=|path,identity|WindowsSelectedRootGrant::new(path,identity).map_err(|e|fail(&e));
    let roots=WindowsMutationStorage::for_selected_roots(grant(request.preview.source.clone(),scope.source_root.clone())?,grant(target.clone(),scope.target_root.clone())?,grant(request.journal_directory.clone(),journal_identity)?).map_err(|e|fail(&e))?;
    let mut metadata=StorageMetadataProvider {storage:&mut reads,reader:MetadataReader};
    let mut job=SelectedWindowsJob::prepare(roots,&plan,&mut metadata,recognition.as_mut(),&resources,&scope).map_err(|e|fail(&e))?;
    let native_id=job.job_id().to_owned();
    event("started",json!({"job_id":native_id,"plan_id":plan.binding_sha256,"journal_directory":request.journal_directory}))?;
    let mut output_failed=false;
    let result=job.run(&cancel,&mut |progress| {
        if event("progress",json!({"job_id":native_id,"operation":progress.operation,"bytes_written":progress.bytes_written,"source_length":progress.source_length})).is_err() {output_failed=true;cancel.store(true,Ordering::Release);}
    });
    match result {
        Ok(result)=>{
            let status=match result.status {JobStatus::Complete=>"complete",JobStatus::Partial=>"partial",JobStatus::Cancelled=>"cancelled",JobStatus::NeedsRecovery=>"needs_recovery"};
            let items=result.items.iter().map(|item|json!({"operation":item.operation,"status":match item.status {ItemStatus::Complete=>"complete",ItemStatus::SourceRetained=>"source_retained",ItemStatus::Failed=>"failed",ItemStatus::Cancelled=>"cancelled",ItemStatus::Blocked=>"blocked",ItemStatus::NeedsRecovery=>"needs_recovery"},"detail":item.detail})).collect::<Vec<_>>();
            event("result",json!({"job_id":native_id,"status":status,"source_items":result.source_items,"committed_destinations":result.committed_destinations,"moved_sources":result.moved_sources,"failed_operations":result.failed_operations,"blocked_operations":result.blocked_operations,"recovery_operations":result.recovery_operations,"cancelled_operations":result.cancelled_operations,"items":items,"journal_directory":request.journal_directory}))?;
            if output_failed || status!="complete" {return Err(Failure::new(1,"EXECUTION_INCOMPLETE","Read the actual result and journal before retrying; completed changes were not rolled back"));}
        }
        Err(error)=>{
            event("result",json!({"job_id":native_id,"status":if job.journal_locations().is_empty() && cancel.load(Ordering::Acquire) {"cancelled"} else {"needs_recovery"},"error":error.to_string(),"journal_directory":request.journal_directory}))?;
            return Err(Failure::new(1,"EXECUTION_INCOMPLETE","Inspect the journal; no automatic replay or rollback was attempted"));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn confirmation_is_strict_and_cannot_supply_a_replacement_plan() {
        assert!(matches!(serde_json::from_str::<Decision>(r#"{"action":"apply","plan_id":"abc"}"#),Ok(Decision::Apply {plan_id}) if plan_id=="abc"));
        assert!(serde_json::from_str::<Decision>(r#"{"action":"apply","plan_id":"abc","source":"elsewhere"}"#).is_err());
        assert!(serde_json::from_str::<Decision>(r#"{"action":"apply","plan_id":"a","plan_id":"b"}"#).is_err());
        assert!(serde_json::from_str::<Decision>(r#"{"action":"cancel"}"#).is_ok());
    }
}
