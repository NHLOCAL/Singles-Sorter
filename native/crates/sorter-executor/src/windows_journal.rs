//! Owned local-NTFS file journal; not a production-root authorization API.
//!
//! Sync follows the ordinary OS-backed file durability model: checked
//! FlushFileBuffers, assuming Windows, NTFS and the device honor that request.
//! It does not certify hardware against power loss or sync album namespaces.
use crate::{ExecutionError, Result, physical_journal::JournalStore};
use sorter_io::desktop::windows_mutation::{WindowsJournalFile, WindowsJournalLocation};

/// Consumes a newly, exclusively created file with held root/ancestor guards.
/// No writable reopen exists. Recovery reads require its identity-bound location.
/// This bridge cannot authorize a real transfer: bindings remain synthetic-only
/// and native mutation durable_flush remains Unknown.
pub struct WindowsJournalStore {
    file: WindowsJournalFile,
}
impl WindowsJournalStore {
    pub fn new(file: WindowsJournalFile) -> Self {
        Self { file }
    }
    pub fn location(&self) -> WindowsJournalLocation {
        self.file.location()
    }
}
impl JournalStore for WindowsJournalStore {
    fn read_all(&mut self) -> Result<Vec<u8>> {
        self.file.read_all().map_err(|_| ExecutionError::Io)
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        self.file.append(bytes).map_err(|_| ExecutionError::Io)
    }
    fn sync(&mut self) -> Result<()> {
        self.file
            .flush_file_buffers()
            .map(|_| ())
            .map_err(|_| ExecutionError::Io)
    }
}
