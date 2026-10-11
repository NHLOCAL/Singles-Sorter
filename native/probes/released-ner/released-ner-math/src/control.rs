//! Per-request cooperative cancellation and finite resource budgets.
//! These bounds do not guarantee recovery from process-wide allocator exhaustion.
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InferenceError {
    Cancelled,
    ResourceLimit {
        resource: &'static str,
        limit: usize,
    },
    Allocation {
        resource: &'static str,
    },
    Model(String),
}
impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("inference cancelled"),
            Self::ResourceLimit { resource, limit } => {
                write!(f, "{resource} exceeds limit {limit}")
            }
            Self::Allocation { resource } => write!(f, "allocation failed for {resource}"),
            Self::Model(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for InferenceError {}

/// A token cannot be reset; use a fresh token for a new cancellation scope.
#[derive(Debug, Default)]
pub struct CancellationToken(AtomicBool);
impl CancellationToken {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Callers may lower these defaults. Values above HARD_MAX are rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub input_bytes: usize,
    pub input_chars: usize,
    pub tokens: usize,
    pub regex_match: usize,
    pub regex_depth: usize,
    pub regex_heap_kib: usize,
    pub regex_calls: usize,
    pub regex_callouts: usize,
    pub work_units: usize,
    /// Maximum cumulative bytes requested for numeric tensors in one request.
    pub tensor_bytes: usize,
}
impl Limits {
    pub const HARD_MAX: Self = Self {
        input_bytes: 16_384,
        input_chars: 4_096,
        tokens: 1_024,
        regex_match: 1_000_000,
        regex_depth: 512,
        regex_heap_kib: 1_024,
        regex_calls: 32_768,
        regex_callouts: 4_000_000,
        work_units: 2_000_000_000,
        tensor_bytes: 32 * 1024 * 1024,
    };
    pub fn validate(&self) -> Result<(), InferenceError> {
        macro_rules! check { ($($field:ident),*) => { $(
            if self.$field > Self::HARD_MAX.$field {
                return Err(InferenceError::ResourceLimit { resource: stringify!($field), limit: Self::HARD_MAX.$field });
            }
        )* }; }
        check!(
            input_bytes,
            input_chars,
            tokens,
            regex_match,
            regex_depth,
            regex_heap_kib,
            regex_calls,
            regex_callouts,
            work_units,
            tensor_bytes
        );
        Ok(())
    }
}
impl Default for Limits {
    fn default() -> Self {
        Self::HARD_MAX
    }
}

/// Mutable budgets are private to one request; only the token is shared.
pub struct RequestControl<'a> {
    #[cfg(test)]
    cancel_after: Cell<Option<usize>>,
    limits: &'a Limits,
    cancellation: &'a CancellationToken,
    work_used: Cell<usize>,
    tensor_used: Cell<usize>,
    regex_calls_used: Cell<usize>,
    regex_callouts_used: Cell<usize>,
}
impl<'a> RequestControl<'a> {
    pub fn new(
        limits: &'a Limits,
        cancellation: &'a CancellationToken,
    ) -> Result<Self, InferenceError> {
        if cancellation.is_cancelled() {
            return Err(InferenceError::Cancelled);
        }
        limits.validate()?;
        let control = Self {
            #[cfg(test)]
            cancel_after: Cell::new(None),
            limits,
            cancellation,
            work_used: Cell::new(0),
            tensor_used: Cell::new(0),
            regex_calls_used: Cell::new(0),
            regex_callouts_used: Cell::new(0),
        };
        control.checkpoint()?;
        Ok(control)
    }
    pub fn limits(&self) -> &Limits {
        self.limits
    }
    #[cfg(test)]
    pub(crate) fn cancel_after_checkpoints(&self, count: usize) {
        self.cancel_after.set(Some(count));
    }
    pub fn checkpoint(&self) -> Result<(), InferenceError> {
        #[cfg(test)]
        if let Some(left) = self.cancel_after.get() {
            if left == 0 {
                self.cancellation.cancel();
            } else {
                self.cancel_after.set(Some(left - 1));
            }
        }
        if self.cancellation.is_cancelled() {
            Err(InferenceError::Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn resource(
        &self,
        resource: &'static str,
        actual: usize,
        limit: usize,
    ) -> Result<(), InferenceError> {
        self.checkpoint()?;
        if actual > limit {
            Err(InferenceError::ResourceLimit { resource, limit })
        } else {
            Ok(())
        }
    }
    fn add(
        used: &Cell<usize>,
        units: usize,
        resource: &'static str,
        limit: usize,
    ) -> Result<(), InferenceError> {
        let next = used
            .get()
            .checked_add(units)
            .filter(|&n| n <= limit)
            .ok_or(InferenceError::ResourceLimit { resource, limit })?;
        used.set(next);
        Ok(())
    }
    pub fn charge(&self, units: usize) -> Result<(), InferenceError> {
        self.checkpoint()?;
        Self::add(&self.work_used, units, "work_units", self.limits.work_units)
    }
    pub fn ensure_work(&self, units: usize) -> Result<(), InferenceError> {
        self.checkpoint()?;
        if units > self.limits.work_units.saturating_sub(self.work_used.get()) {
            return Err(InferenceError::ResourceLimit {
                resource: "work_units",
                limit: self.limits.work_units,
            });
        }
        Ok(())
    }
    pub fn tensor_bytes(&self, bytes: usize) -> Result<(), InferenceError> {
        self.checkpoint()?;
        Self::add(
            &self.tensor_used,
            bytes,
            "tensor_bytes",
            self.limits.tensor_bytes,
        )
    }
    pub fn ensure_tensor_bytes(&self, bytes: usize) -> Result<(), InferenceError> {
        self.checkpoint()?;
        if bytes
            > self
                .limits
                .tensor_bytes
                .saturating_sub(self.tensor_used.get())
        {
            return Err(InferenceError::ResourceLimit {
                resource: "tensor_bytes",
                limit: self.limits.tensor_bytes,
            });
        }
        Ok(())
    }
    pub fn charge_regex_call(&self) -> Result<(), InferenceError> {
        self.checkpoint()?;
        Self::add(
            &self.regex_calls_used,
            1,
            "regex_calls",
            self.limits.regex_calls,
        )
    }
    pub fn charge_regex_callout(&self) -> Result<(), InferenceError> {
        self.checkpoint()?;
        Self::add(
            &self.regex_callouts_used,
            1,
            "regex_callouts",
            self.limits.regex_callouts,
        )
    }
    pub fn work_used(&self) -> usize {
        self.work_used.get()
    }
    pub fn tensor_bytes_used(&self) -> usize {
        self.tensor_used.get()
    }
}
