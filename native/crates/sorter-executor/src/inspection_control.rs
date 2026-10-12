//! Read-only inspection lifecycle and measured progress. This carries no storage
//! references, writable bindings, resume authority, or cached content evidence.
use crate::{ExecutionError as E, Result};
use serde::Serialize;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize)]
pub struct InspectionProgress {
    pub stage: &'static str,
    pub operation: Option<usize>,
    pub total_operations: usize,
    pub completed_operations: usize,
    pub records_checked: u64,
    pub observations: u64,
    pub bytes_read: u64,
    pub elapsed_ms: u64,
}
impl Default for InspectionProgress {
    fn default() -> Self {
        Self { stage: "queued", operation: None, total_operations: 0,
            completed_operations: 0, records_checked: 0, observations: 0,
            bytes_read: 0, elapsed_ms: 0 }
    }
}
struct State {
    progress: InspectionProgress,
    started: Instant,
    emitted: Instant,
}
enum CancellationFlag<'a> {
    Owned(Arc<AtomicBool>),
    Borrowed(&'a AtomicBool),
}
impl CancellationFlag<'_> {
    fn get(&self) -> &AtomicBool {
        match self { Self::Owned(flag) => flag, Self::Borrowed(flag) => flag }
    }
}
pub struct InspectionControl<'a> {
    cancel: CancellationFlag<'a>,
    state: Mutex<State>,
    publish_every: Duration,
    observer: Box<dyn Fn(InspectionProgress) + Send + Sync>,
}
impl<'a> InspectionControl<'a> {
    pub fn new(cancel: Arc<AtomicBool>, observer: impl Fn(InspectionProgress) + Send + Sync + 'static) -> Self {
        Self::with_progress_interval(cancel, Duration::from_millis(100), observer)
    }
    /// Reporting cadence only; every read/probe still checks cancellation.
    pub fn with_progress_interval(cancel: Arc<AtomicBool>, publish_every: Duration, observer: impl Fn(InspectionProgress) + Send + Sync + 'static) -> Self {
        Self::with_flag(CancellationFlag::Owned(cancel), publish_every, observer)
    }
    /// Borrow the CLI/host's actual flag for this synchronous inspection. Never
    /// mint an independent token or poll/reset another owner's cancellation.
    pub fn borrowed(cancel: &'a AtomicBool, observer: impl Fn(InspectionProgress) + Send + Sync + 'static) -> Self {
        Self::with_flag(CancellationFlag::Borrowed(cancel), Duration::from_millis(100), observer)
    }
    fn with_flag(cancel: CancellationFlag<'a>, publish_every: Duration, observer: impl Fn(InspectionProgress) + Send + Sync + 'static) -> Self {
        let now = Instant::now();
        Self { cancel, state: Mutex::new(State { progress: InspectionProgress::default(), started: now, emitted: now }),
            publish_every, observer: Box::new(observer) }
    }
    /// Native read adapters share the original owner's cooperative flag.
    #[cfg(all(windows, feature = "experimental-windows-copy"))]
    pub(crate) fn cancellation_flag(&self) -> &AtomicBool { self.cancel.get() }
    pub fn checkpoint(&self) -> Result<()> {
        if self.cancel.get().load(Ordering::Acquire) { Err(E::Cancelled) } else { Ok(()) }
    }
    pub fn snapshot(&self) -> Result<InspectionProgress> {
        let mut state = self.state.lock().map_err(|_| E::Io)?;
        state.progress.elapsed_ms = u64::try_from(state.started.elapsed().as_millis()).map_err(|_| E::Unsupported)?;
        Ok(state.progress.clone())
    }
    fn update(&self, force: bool, change: impl FnOnce(&mut InspectionProgress) -> Result<()>) -> Result<()> {
        self.update_inner(force, false, change)
    }
    fn update_inner(&self, force: bool, consumed: bool, change: impl FnOnce(&mut InspectionProgress) -> Result<()>) -> Result<()> {
        if !consumed { self.checkpoint()?; }
        let next = {
            let mut state = self.state.lock().map_err(|_| E::Io)?;
            change(&mut state.progress)?;
            state.progress.elapsed_ms = u64::try_from(state.started.elapsed().as_millis()).map_err(|_| E::Unsupported)?;
            if force || state.emitted.elapsed() >= self.publish_every {
                state.emitted = Instant::now();
                Some(state.progress.clone())
            } else { None }
        };
        if let Some(next) = next { (self.observer)(next); }
        self.checkpoint()
    }
    pub fn stage(&self, stage: &'static str) -> Result<()> {
        self.update(true, |progress| { progress.stage = stage; Ok(()) })
    }
    pub fn operations(&self, total: usize) -> Result<()> {
        self.update(true, |progress| { progress.total_operations = total; Ok(()) })
    }
    pub fn begin_operation(&self, operation: usize) -> Result<()> {
        self.update(true, |progress| { progress.stage = "files"; progress.operation = Some(operation); Ok(()) })
    }
    pub fn complete_operation(&self) -> Result<()> {
        self.update(true, |progress| {
            let completed = progress.completed_operations.checked_add(1).ok_or(E::Unsupported)?;
            if completed > progress.total_operations { return Err(E::InvalidBinding); }
            progress.completed_operations = completed;
            Ok(())
        })
    }
    pub fn record(&self) -> Result<()> {
        self.update(false, |progress| { progress.records_checked = progress.records_checked.checked_add(1).ok_or(E::Unsupported)?; Ok(()) })
    }
    pub fn observation(&self) -> Result<()> {
        self.update(false, |progress| { progress.observations = progress.observations.checked_add(1).ok_or(E::Unsupported)?; Ok(()) })
    }
    pub fn bytes(&self, count: usize) -> Result<()> {
        self.update_inner(false, true, |progress| { progress.bytes_read = progress.bytes_read.checked_add(u64::try_from(count).map_err(|_| E::Unsupported)?).ok_or(E::Unsupported)?; Ok(()) })
    }
}
pub(crate) fn checkpoint(control: Option<&InspectionControl>) -> Result<()> {
    control.map_or(Ok(()), InspectionControl::checkpoint)
}
/// A synchronous native call cannot be forcibly preempted here. Cancellation is
/// tested both before and after, including before interpreting an error/absence.
#[cfg(any(test, all(windows, feature = "experimental-windows-copy")))]
pub(crate) fn probe<T>(control: Option<&InspectionControl>, read: impl FnOnce() -> Result<T>) -> Result<T> {
    checkpoint(control)?;
    if let Some(control) = control { control.observation()?; }
    let result = read();
    checkpoint(control)?;
    result
}

pub(crate) fn any<T>(items: impl IntoIterator<Item = T>, control: Option<&InspectionControl>, mut predicate: impl FnMut(T) -> bool) -> Result<bool> {
    for item in items {
        checkpoint(control)?;
        if predicate(item) { return Ok(true); }
    }
    Ok(false)
}

pub(crate) fn find<T>(items: impl IntoIterator<Item = T>, control: Option<&InspectionControl>, mut predicate: impl FnMut(&T) -> bool) -> Result<Option<T>> {
    for item in items {
        checkpoint(control)?;
        if predicate(&item) { return Ok(Some(item)); }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_control_uses_the_original_owner_flag() {
        let cancel = AtomicBool::new(false);
        let control = InspectionControl::borrowed(&cancel, |_| {});
        control.checkpoint().unwrap();
        cancel.store(true, Ordering::Release);
        assert_eq!(control.checkpoint(), Err(E::Cancelled));
        assert!(cancel.load(Ordering::Acquire));
    }
    #[test]
    fn cancellation_is_distinct_and_does_not_run_next_probe() {
        let cancel = Arc::new(AtomicBool::new(false));
        let control = InspectionControl::new(cancel.clone(), |_| {});
        cancel.store(true, Ordering::Release);
        assert_eq!(probe(Some(&control), || panic!("cancelled probe ran")), Err::<(), _>(E::Cancelled));
        assert_eq!(control.snapshot().unwrap().bytes_read, 0);
        // If a synchronous read returns bytes while cancellation was requested,
        // record those actual bytes before propagating cancellation.
        assert_eq!(control.bytes(42), Err(E::Cancelled));
        assert_eq!(control.snapshot().unwrap().bytes_read, 42);
    }
    #[test]
    fn cancellation_after_probe_overrides_absence_or_error() {
        let cancel = Arc::new(AtomicBool::new(false));
        let control = InspectionControl::new(cancel.clone(), |_| {});
        let result = probe(Some(&control), || { cancel.store(true, Ordering::Release); Ok(None::<usize>) });
        assert_eq!(result, Err(E::Cancelled));
    }
    #[test]
    fn progress_counts_actual_repeated_bytes_and_checks_overflow() {
        let control = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
        control.bytes(65536).unwrap(); control.bytes(65536).unwrap(); control.bytes(1).unwrap();
        assert_eq!(control.snapshot().unwrap().bytes_read, 131073);
        control.state.lock().unwrap().progress.bytes_read = u64::MAX;
        assert_eq!(control.bytes(1), Err(E::Unsupported));
    }
    #[test]
    fn operation_progress_preserves_ids_and_checked_counts() {
        let control = InspectionControl::new(Arc::new(AtomicBool::new(false)), |_| {});
        control.operations(2).unwrap(); control.begin_operation(1).unwrap(); control.complete_operation().unwrap();
        control.begin_operation(0).unwrap(); control.complete_operation().unwrap();
        let progress = control.snapshot().unwrap();
        assert_eq!(progress.operation, Some(0)); assert_eq!(progress.completed_operations, 2);
        assert_eq!(control.complete_operation(), Err(E::InvalidBinding));
    }
}

/// A controlled parser is bounded before expensive strict replay. These are
/// recorded-input ceilings, not cumulative media-content limits.
pub(crate) fn check_recorded_input(bytes: &[u8], control: Option<&InspectionControl>) -> Result<()> {
    if control.is_none() { return Ok(()); }
    if bytes.len() > 64 * 1024 * 1024 { return Err(E::Unsupported); }
    let mut records = 0usize;
    let mut length = 0usize;
    for chunk in bytes.chunks(65536) {
        checkpoint(control)?;
        for byte in chunk {
            length = length.checked_add(1).ok_or(E::Unsupported)?;
            if length > 4 * 1024 * 1024 { return Err(E::Unsupported); }
            if *byte == b'\n' {
                records = records.checked_add(1).ok_or(E::Unsupported)?;
                if records > 65536 { return Err(E::Unsupported); }
                length = 0;
            }
        }
    }
    checkpoint(control)
}
