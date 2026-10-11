//! Synthetic-only physical journal/recovery contract. No native mutation adapter,
//! CLI apply, or Windows durability claim is provided by this crate.
pub mod binding;
pub mod physical_journal;
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
