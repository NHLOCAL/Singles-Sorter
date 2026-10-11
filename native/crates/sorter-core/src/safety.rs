//! Mechanical source-deletion guard for a future apply adapter. No file operation.

/// A move may remove its source only after a nonzero set of required allowed
/// destinations has been completely verified, with no failure or cancellation.
/// These counts are supplied facts, not verification or authority to delete.
/// Copy mode always retains the source. Skipped/blocked intents are not successes.
pub fn source_delete_allowed(
    copy_mode: bool,
    required_destinations: usize,
    verified_destinations: usize,
    failed_or_cancelled: bool,
) -> bool {
    !copy_mode
        && required_destinations > 0
        && verified_destinations == required_destinations
        && !failed_or_cancelled
}
