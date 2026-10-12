//! Identity-bound experimental journal/recovery and transfer contracts.
//! Native Copy and same-volume rename are owned Windows fixtures only; no CLI/UI apply.
pub mod binding;
pub mod physical_journal;
pub mod inspection_control;
pub mod recovery;
use serde::Serialize;
use sha2::{Digest, Sha256};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    InvalidBinding,
    Stale,
    Unsupported,
    Scope,
    CorruptJournal,
    IncompleteTail,
    InvalidTransition,
    Io,
    NeedsRecovery,
    Cancelled,
}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ExecutionError {}
pub type Result<T> = std::result::Result<T, ExecutionError>;
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn hash<T: Serialize>(v: &T) -> Result<String> {
    Ok(digest(
        &serde_json::to_vec(v).map_err(|_| ExecutionError::InvalidBinding)?,
    ))
}

pub mod results;
pub mod transfer;

pub mod album;

#[cfg(all(windows, feature = "experimental-windows-journal"))]
pub mod windows_journal;

#[cfg(all(windows, feature = "experimental-windows-copy"))]
pub mod windows_copy;

#[cfg(all(windows, feature = "experimental-windows-rename"))]
pub mod windows_rename;

#[cfg(all(windows, feature = "selected-windows-execution"))]
pub mod windows_job;

#[cfg(all(windows, feature = "selected-windows-execution"))]
pub mod windows_repair;

#[cfg(all(windows, feature = "selected-windows-execution"))]
pub mod windows_album;

pub mod legacy_summary;

pub mod no_op;
