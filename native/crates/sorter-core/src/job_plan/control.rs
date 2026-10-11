//! Per-job, cooperative control. A cancelled token is never reset or reused.
use super::JobError;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbortKind {
    Cancelled,
    ResourceLimit,
    Allocation,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanningAbort {
    pub kind: AbortKind,
    pub detail: String,
}
impl PlanningAbort {
    pub fn cancelled() -> Self {
        Self {
            kind: AbortKind::Cancelled,
            detail: "Planning cancelled".into(),
        }
    }
}
impl From<PlanningAbort> for JobError {
    fn from(error: PlanningAbort) -> Self {
        Self {
            code: match error.kind {
                AbortKind::Cancelled => "PLANNING_CANCELLED",
                AbortKind::ResourceLimit => "PLANNING_RESOURCE_LIMIT",
                AbortKind::Allocation => "PLANNING_ALLOCATION",
            }
            .into(),
            detail: error.detail,
        }
    }
}
/// Ordinary released failures retain their original text and exception boundary.
/// Abort is control flow, never a negative recognition result or an item error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecognitionError {
    Model(String),
    Legacy(String),
    Abort(PlanningAbort),
}
impl From<String> for RecognitionError {
    fn from(value: String) -> Self {
        Self::Legacy(value)
    }
}
impl From<&str> for RecognitionError {
    fn from(value: &str) -> Self {
        Self::Legacy(value.into())
    }
}
impl From<PlanningAbort> for RecognitionError {
    fn from(value: PlanningAbort) -> Self {
        Self::Abort(value)
    }
}
impl std::fmt::Display for RecognitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Model(s) | Self::Legacy(s) => f.write_str(s),
            Self::Abort(e) => f.write_str(&e.detail),
        }
    }
}
impl std::error::Error for RecognitionError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanningBoundary {
    Start,
    Collection,
    Visit,
    File,
    Recognition,
    RecognitionAcceptance,
    DeferredPlanning,
    Binding,
    Result,
}
/// Hosts may bridge their own job token. Checks must not perform sorting work.
/// Providers doing long-running reads/inference must also check within that work.
pub trait PlanningControl {
    fn checkpoint(&self, boundary: PlanningBoundary) -> Result<(), PlanningAbort>;
}
pub struct UncontrolledPlanning;
impl PlanningControl for UncontrolledPlanning {
    fn checkpoint(&self, _: PlanningBoundary) -> Result<(), PlanningAbort> {
        Ok(())
    }
}
#[derive(Clone, Default, Debug)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
impl PlanningControl for CancellationToken {
    fn checkpoint(&self, _: PlanningBoundary) -> Result<(), PlanningAbort> {
        if self.is_cancelled() {
            Err(PlanningAbort::cancelled())
        } else {
            Ok(())
        }
    }
}
