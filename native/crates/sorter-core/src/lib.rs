//! Native Singles Sorter policy: v1 supplied facts and v2 read-only job planning.

pub mod contract;

pub use contract::*;

pub mod policy;
pub use policy::{check_artist, classify, clean, exact_match, fix_jibrish, sanitize};
pub mod unicode15;

pub mod planning;
pub use planning::{plan, verify_plan_binding};
pub mod safety;
pub use safety::source_delete_allowed;

pub mod journal;
pub use journal::{
    Journal, JournalBinding, JournalEvent, OperationBinding, OperationState, ReplayState, replay,
};

pub mod discovery;
pub mod metadata;

pub mod album;
pub mod fix_plan;
pub mod job_plan;
