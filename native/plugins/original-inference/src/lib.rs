//! Read-only Android recognition seam; product planner is explicitly feature-gated.
#[cfg(feature = "product")]
mod product;
use jni::{
    JNIEnv,
    objects::{JByteArray, JClass},
    sys::{jbyteArray, jlong},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sorter_core::job_plan::RecognitionProvider;
use sorter_recognition::{BUNDLE_ID, JobCancellation, Limits, OriginalRecognition};
use std::{path::Path, sync::Mutex};

const MAX_REQUEST: usize = 65_536;
const MAX_PATH: usize = 4_096;
const MAX_TEXT: usize = 4_096;
const MAX_TEXTS: usize = 32;
struct Job {
    id: i64,
    running: bool,
    phase: i32,
    cancel: JobCancellation,
}
struct Registry {
    next: i64,
    job: Option<Job>,
}
static JOBS: Mutex<Registry> = Mutex::new(Registry { next: 1, job: None });

fn begin() -> i64 {
    let Ok(mut state) = JOBS.lock() else { return 0 };
    if state.job.is_some() {
        return 0;
    }
    let id = state.next;
    let Some(next) = id.checked_add(1) else {
        return 0;
    };
    state.next = next;
    state.job = Some(Job {
        id,
        running: false,
        phase: 0,
        cancel: JobCancellation::new(),
    });
    id
}
fn cancel(id: i64) {
    if let Ok(state) = JOBS.lock()
        && let Some(job) = &state.job
        && job.id == id
    {
        job.cancel.cancel();
    }
}
fn claim(id: i64) -> Result<JobCancellation, &'static str> {
    let mut state = JOBS.lock().map_err(|_| "poisoned")?;
    let job = state
        .job
        .as_mut()
        .filter(|j| j.id == id && !j.running)
        .ok_or("stale_or_running")?;
    job.running = true;
    Ok(job.cancel.clone())
}
struct OwnedRun(i64);
impl Drop for OwnedRun {
    fn drop(&mut self) {
        if let Ok(mut state) = JOBS.lock()
            && state.job.as_ref().is_some_and(|j| j.id == self.0)
        {
            state.job = None;
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    texts: Vec<String>,
}
fn parse(bytes: &[u8]) -> Result<Request, &'static str> {
    if bytes.len() > MAX_REQUEST {
        return Err("request_limit");
    }
    let request: Request = serde_json::from_slice(bytes).map_err(|_| "invalid_request")?;
    if request.texts.is_empty()
        || request.texts.len() > MAX_TEXTS
        || request.texts.iter().any(|t| t.len() > MAX_TEXT)
    {
        return Err("input_limit");
    }
    Ok(request)
}
fn phase(id: i64, phase: i32) {
    if let Ok(mut state) = JOBS.lock()
        && let Some(job) = state.job.as_mut().filter(|j| j.id == id)
    {
        job.phase = phase;
    }
}
fn execute(
    id: i64,
    path: &[u8],
    request: &[u8],
    cancellation: JobCancellation,
) -> Result<Value, String> {
    if cancellation.is_cancelled() {
        return Err("cancelled".into());
    }
    if path.len() > MAX_PATH {
        return Err("path_limit".into());
    }
    let path = std::str::from_utf8(path).map_err(|_| "invalid_path")?;
    if path.contains('\0') {
        return Err("invalid_path".into());
    }
    let request = parse(request)?;
    let limits = Limits {
        max_inference_calls: 1024,
        ..Limits::default()
    };
    phase(id, 2);
    let mut model =
        OriginalRecognition::load(Path::new(path), BUNDLE_ID, limits, cancellation.clone())
            .map_err(|e| format!("{e:?}"))?;
    let mut results = Vec::with_capacity(request.texts.len());
    for text in request.texts {
        if cancellation.is_cancelled() {
            return Err("cancelled".into());
        }
        phase(id, 3);
        let artists = model.ner(&text).map_err(|e| format!("{e:?}"))?;
        results.push(json!({"text": text, "artists": artists}));
    }
    if cancellation.is_cancelled() {
        return Err("cancelled".into());
    }
    Ok(
        json!({"bundle": BUNDLE_ID, "resources": model.resource_ids(), "runtime": model.adapter_versions(), "results": results, "diagnostics": model.diagnostics(), "calls": model.inference_calls()}),
    )
}
fn bytes(
    env: &mut JNIEnv<'_>,
    array: &JByteArray<'_>,
    max: usize,
) -> Result<Vec<u8>, &'static str> {
    let size = env.get_array_length(array).map_err(|_| "jni_length")?;
    if size < 0 || size as usize > max {
        return Err("input_limit");
    }
    env.convert_byte_array(array).map_err(|_| "jni_bytes")
}
fn output(env: &mut JNIEnv<'_>, value: Value) -> jbyteArray {
    // Never call JNI with a pending exception. Let the original Java exception propagate.
    if env.exception_check().unwrap_or(true) {
        return std::ptr::null_mut();
    }
    let Ok(data) = serde_json::to_vec(&value) else {
        return std::ptr::null_mut();
    };
    env.byte_array_from_slice(&data)
        .map(|a| a.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalInference_begin0(
    _: JNIEnv,
    _: JClass,
) -> jlong {
    begin()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalInference_cancel0(
    _: JNIEnv,
    _: JClass,
    id: jlong,
) {
    cancel(id);
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalInference_phase0(
    _: JNIEnv,
    _: JClass,
    id: jlong,
) -> jni::sys::jint {
    JOBS.lock()
        .ok()
        .and_then(|s| s.job.as_ref().filter(|j| j.id == id).map(|j| j.phase))
        .unwrap_or(-1)
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalInference_abandon0(
    _: JNIEnv,
    _: JClass,
    id: jlong,
) {
    if let Ok(mut state) = JOBS.lock()
        && state.job.as_ref().is_some_and(|j| j.id == id && !j.running)
    {
        state.job = None;
    }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_nhlocal_singlesorter_inference_OriginalInference_run0(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
    path: JByteArray,
    request: JByteArray,
) -> jbyteArray {
    let cancellation = match claim(id) {
        Ok(token) => token,
        Err(error) => return output(&mut env, json!({"ok": false, "job": id, "error": error})),
    };
    // Retain ownership through JNI output allocation, not only model execution.
    let _owned = OwnedRun(id);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path = bytes(&mut env, &path, MAX_PATH)?;
        let request = bytes(&mut env, &request, MAX_REQUEST)?;
        execute(id, &path, &request, cancellation.clone())
    }));
    let value = match result {
        _ if cancellation.is_cancelled() => json!({"ok": false, "job": id, "error": "cancelled"}),
        Ok(Ok(value)) => json!({"ok": true, "job": id, "data": value}),
        Ok(Err(error)) => json!({"ok": false, "job": id, "error": error}),
        Err(_) => json!({"ok": false, "job": id, "error": "native_panic"}),
    };
    output(&mut env, value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_requests() {
        assert!(parse(r#"{"texts":["Hebrew שלום"]}"#.as_bytes()).is_ok());
        assert!(parse(br#"{"texts":[]}"#).is_err());
        assert!(parse(br#"{"texts":["a"],"other":0}"#).is_err());
        assert!(parse(&vec![b' '; MAX_REQUEST + 1]).is_err());
        assert!(
            parse(&serde_json::to_vec(&json!({"texts": ["a".repeat(MAX_TEXT+1)]})).unwrap())
                .is_err()
        );
    }
    #[test]
    fn ownership_and_cancellation() {
        let id = begin();
        assert!(id > 0);
        assert_eq!(begin(), 0);
        assert!(claim(id + 1).is_err());
        cancel(id + 1);
        let token = claim(id).unwrap();
        assert!(!token.is_cancelled());
        assert!(claim(id).is_err());
        cancel(id);
        assert!(token.is_cancelled());
        assert!(
            execute(id, b"/does-not-exist", br#"{"texts":["test"]}"#, token)
                .unwrap_err()
                .contains("cancelled")
        );
        drop(OwnedRun(id));
        let second = begin();
        assert!(second > id);
        cancel(id);
        assert!(!claim(second).unwrap().is_cancelled());
        drop(OwnedRun(second));
    }
}
