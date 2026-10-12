//! Private staged metadata repair. Provider publication and original-plan authority stay in Java.
//! This module reuses the shared native stream transformer and never edits an input in place.
use super::*;
use serde::Deserialize;
use sorter_core::{fix_plan::FieldRepair, metadata::LegacyField};
use sorter_io::staged_repair::StagedRepairPlan;
use std::{fs::OpenOptions, io::{Seek, SeekFrom, Write}, path::PathBuf, sync::atomic::Ordering};

const MAX_REPAIR_REQUEST: usize = 1024 * 1024;
const MAX_REPAIR_FILE: u64 = 64 * 1024 * 1024;
const MAX_STAGED_FILE: u64 = 72 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairField { field: LegacyField, original: String, replacement: String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairRequest {
    schema: u8,
    source_file: String,
    output_file: String,
    filename: String,
    source_bytes: u64,
    source_sha256: String,
    repairs: Vec<RepairField>,
}
fn leaf(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value != "." && value != ".."
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'.')
}
fn parse_repair(input: &[u8]) -> Result<RepairRequest, String> {
    if input.len() > MAX_REPAIR_REQUEST { return Err("repair_request_limit".into()); }
    let request: RepairRequest = serde_json::from_slice(input).map_err(|_| "repair_request_invalid")?;
    if request.schema != 1 || !leaf(&request.source_file) || !leaf(&request.output_file)
        || request.source_file == request.output_file || request.filename.len() > 4096
        || request.source_bytes > MAX_REPAIR_FILE || request.source_sha256.len() != 64
        || !request.source_sha256.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        || request.repairs.len() > 5 { return Err("repair_input_bound".into()); }
    let mut previous = None;
    for repair in &request.repairs {
        let index = sorter_core::fix_plan::FIX_FIELDS.iter().position(|field| *field == repair.field).ok_or("repair_field")?;
        if previous.is_some_and(|last| index <= last) || repair.original.len() > 131072 || repair.replacement.len() > 131072 { return Err("repair_field_bound_or_order".into()); }
        previous = Some(index);
    }
    Ok(request)
}
struct BoundedOutput<'a> { file: &'a mut std::fs::File, remaining: u64 }
impl Write for BoundedOutput<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.remaining { return Err(std::io::Error::other("staged_file_limit")); }
        let count = self.file.write(bytes)?; self.remaining -= count as u64; Ok(count)
    }
    fn flush(&mut self) -> std::io::Result<()> { self.file.flush() }
}
fn stage_repair(id: i64, directory: &[u8], input: &[u8]) -> Result<Value, String> {
    let flag = {
        let state = JOBS.lock().map_err(|_| "poisoned")?;
        state.job.as_ref().filter(|job| job.id == id && job.running).ok_or("stale_or_running")?.repair_cancel.clone()
    };
    if flag.load(Ordering::Acquire) { return Err("cancelled".into()); }
    let request = parse_repair(input)?;
    if directory.len() > MAX_PATH { return Err("path_limit".into()); }
    let directory = std::str::from_utf8(directory).map_err(|_| "invalid_path")?;
    if directory.contains('\0') { return Err("invalid_path".into()); }
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() || !std::fs::symlink_metadata(&directory).map_err(|_| "snapshot_directory")?.file_type().is_dir() { return Err("snapshot_directory".into()); }
    let directory = directory.canonicalize().map_err(|_| "snapshot_directory")?;
    let mut options = OpenOptions::new();options.read(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt;options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK); }
    let mut source = options.open(directory.join(&request.source_file)).map_err(|_| "snapshot_open")?;
    let metadata = source.metadata().map_err(|_| "snapshot_stat")?;
    if !metadata.is_file() || metadata.len() != request.source_bytes { return Err("snapshot_shape".into()); }
    let repairs: Vec<_> = request.repairs.into_iter().map(|repair| FieldRepair { field: repair.field, original: repair.original, replacement: repair.replacement }).collect();
    let plan = StagedRepairPlan::inspect(&request.filename, &mut source, request.source_bytes, &repairs, &flag).map_err(|error| format!("{error:?}"))?;
    if plan.staged_length() > MAX_STAGED_FILE { return Err("staged_file_limit".into()); }
    if flag.load(Ordering::Acquire) { return Err("cancelled".into()); }
    source.seek(SeekFrom::Start(0)).map_err(|_| "snapshot_seek")?;
    let mut options = OpenOptions::new();options.write(true).create_new(true);
    #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt;options.mode(0o600).custom_flags(libc::O_NOFOLLOW); }
    let mut output = options.open(directory.join(&request.output_file)).map_err(|_| "staged_create")?;
    let evidence = plan.write_staged(&mut source, &mut BoundedOutput { file: &mut output, remaining: MAX_STAGED_FILE }, &request.source_sha256, &flag).map_err(|error| format!("{error:?}"))?;
    output.sync_all().map_err(|_| "staged_sync")?;
    if output.metadata().map_err(|_| "staged_stat")?.len() != evidence.staged_length || evidence.staged_length != plan.staged_length() { return Err("staged_length_mismatch".into()); }
    if flag.load(Ordering::Acquire) { return Err("cancelled".into()); }
    Ok(json!({"source_sha256":evidence.source_sha256,"staged_sha256":evidence.staged_sha256,"media_sha256":evidence.media_sha256,"media_length":evidence.media_length,"staged_length":evidence.staged_length,"changed_fields":evidence.changed_fields,"changes_bytes":plan.changes_bytes(),"output_file":request.output_file}))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalPlanning_stageRepair0(mut env: JNIEnv, _: JClass, id: jlong, directory: JByteArray, request: JByteArray) -> jbyteArray {
    let _cancellation = match claim(id) { Ok(token) => token, Err(error) => return output(&mut env, json!({"ok":false,"error":error})) };
    let _owner = OwnedRun(id);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let directory = bytes(&mut env, &directory, MAX_PATH)?;
        let request = bytes(&mut env, &request, MAX_REPAIR_REQUEST)?;
        stage_repair(id, &directory, &request)
    }));
    output(&mut env, match result { Ok(Ok(data)) => json!({"ok":true,"data":data}), Ok(Err(error)) => json!({"ok":false,"error":error}), Err(_) => json!({"ok":false,"error":"native_panic"}) })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Value { json!({"schema":1,"source_file":"source.bin","output_file":"staged.bin","filename":"owned.mp3","source_bytes":100,"source_sha256":"00".repeat(32),"repairs":[{"field":"Artist","original":"old","replacement":"new"}]}) }
    #[test]
    fn bounds_private_names_and_rejects_in_place_output() {
        assert!(parse_repair(&serde_json::to_vec(&request()).unwrap()).is_ok());
        for name in ["", ".", "..", "../source.bin", "/source.bin", "sub/source.bin", "a\\b", "a\0b"] {
            let mut value=request();value["source_file"]=json!(name);assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        }
        let mut value=request();value["output_file"]=value["source_file"].clone();assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value=request();value["source_bytes"]=json!(MAX_REPAIR_FILE+1);assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value=request();value["source_sha256"]=json!("GG".repeat(32));assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(parse_repair(&vec![b' ';MAX_REPAIR_REQUEST+1]).is_err());
    }
    #[test]
    fn only_original_five_fields_in_original_order() {
        let mut value=request();value["repairs"][0]["field"]=json!("TrackNumber");assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value=request();let duplicate=value["repairs"][0].clone();value["repairs"].as_array_mut().unwrap().push(duplicate);assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value=request();value["repairs"][0]["original"]=json!("x".repeat(131073));assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
        let mut value=request();value["unexpected"]=json!(true);assert!(parse_repair(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
