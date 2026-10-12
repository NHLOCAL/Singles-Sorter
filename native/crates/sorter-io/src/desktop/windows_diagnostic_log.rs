//! Dedicated legacy CLI diagnostic sink. No media or journal authority is exposed.
use super::*;

/// A newly created UTF-8 log beneath the process's verified current directory.
/// All ancestry remains pinned until drop. Existing log files are never adopted.
pub struct WindowsDiagnosticLog {
    _ancestry: Vec<File>,
    _directory: File,
    file: File,
    path: PathBuf,
    created_logs_directory: bool,
    poisoned: bool,
}

impl WindowsDiagnosticLog {
    /// `timestamp` is the caller's local `YYYYMMDD_HHMMSS`. The PID is obtained
    /// here, and neither a directory nor a filename can be supplied by callers.
    pub fn create_for_cwd(timestamp: &str) -> Result<Self, MutationError> {
        Self::create_at(&std::env::current_dir().map_err(io_error)?, timestamp)
    }

    fn create_at(cwd: &Path, timestamp: &str) -> Result<Self, MutationError> {
        let bytes = timestamp.as_bytes();
        if bytes.len() != 15
            || bytes[8] != b'_'
            || bytes.iter().enumerate().any(|(i, byte)| i != 8 && !byte.is_ascii_digit())
        {
            return Err(err(MutationErrorKind::InvalidName, "invalid diagnostic timestamp"));
        }
        let ancestry = chain(cwd).map_err(read_error)?;
        let parent = ancestry.last().ok_or_else(|| err(MutationErrorKind::Io, "missing cwd handle"))?;
        ensure_directory(parent)?;
        ensure_case_insensitive(parent)?;
        let logs: Vec<u16> = "logs".encode_utf16().collect();
        let (directory, created_logs_directory) = match open_directory_handle(parent, &logs, true) {
            Ok(file) => (file, true),
            Err(error) if error.kind == MutationErrorKind::StalePlan => {
                (open_directory_handle(parent, &logs, false)?, false)
            }
            Err(error) => return Err(error),
        };
        ensure_directory(&directory)?;
        let filename = format!("music_sorter_{timestamp}_{}.log", std::process::id());
        let file = open_relative_file(
            &directory,
            &filename.encode_utf16().collect::<Vec<_>>(),
            true,
            FILE_GENERIC_WRITE | FILE_READ_ATTRIBUTES,
        )?;
        ensure_regular(&file)?;
        let mut units = vec![0u16; 32768];
        let length = unsafe {
            GetFinalPathNameByHandleW(
                HANDLE(file.as_raw_handle()),
                &mut units,
                GETFINALPATHNAMEBYHANDLE_FLAGS(FILE_NAME_NORMALIZED.0 | VOLUME_NAME_DOS.0),
            )
        } as usize;
        if length == 0 {
            return Err(io_error(io::Error::last_os_error()));
        }
        if length >= units.len() {
            return Err(err(MutationErrorKind::Unsupported, "diagnostic path exceeds bound"));
        }
        units.truncate(length);
        Ok(Self {
            _ancestry: ancestry,
            _directory: directory,
            file,
            path: PathBuf::from(std::ffi::OsString::from_wide(&units)),
            created_logs_directory,
            poisoned: false,
        })
    }

    /// Actual normalized path obtained from the newly created file handle.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn created_logs_directory(&self) -> bool {
        self.created_logs_directory
    }

    pub fn append_utf8(&mut self, text: &str) -> Result<(), MutationError> {
        self.check_usable()?;
        if let Err(error) = self.file.write_all(text.as_bytes()) {
            self.poisoned = true;
            return Err(io_error(error));
        }
        Ok(())
    }

    pub fn sync(&mut self) -> Result<(), MutationError> {
        self.check_usable()?;
        if let Err(error) = self.file.sync_all() {
            self.poisoned = true;
            return Err(io_error(error));
        }
        Ok(())
    }

    fn check_usable(&self) -> Result<(), MutationError> {
        if self.poisoned {
            return Err(err(MutationErrorKind::UnknownOutcome, "diagnostic sink failed; do not retry"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod diagnostic_log_tests {
    use super::*;
    mod reparse_fixture {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/support/windows_reparse.rs"));
    }

    #[test]
    fn new_log_is_utf8_and_reports_only_its_created_objects() {
        let root = tempfile::tempdir().unwrap();
        let mut log = WindowsDiagnosticLog::create_at(root.path(), "20261012_020000").unwrap();
        assert!(log.created_logs_directory());
        let path = log.path().to_path_buf();
        log.append_utf8("מידע: שיר\n").unwrap();
        log.sync().unwrap();
        drop(log);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "מידע: שיר\n");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn existing_directory_is_retained_and_filename_collision_never_truncates() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("logs")).unwrap();
        let mut log = WindowsDiagnosticLog::create_at(root.path(), "20261012_020001").unwrap();
        assert!(!log.created_logs_directory());
        log.append_utf8("retain").unwrap();
        log.sync().unwrap();
        let path = log.path().to_path_buf();
        drop(log);
        assert!(WindowsDiagnosticLog::create_at(root.path(), "20261012_020001").is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"retain");
    }

    #[test]
    fn invalid_timestamp_and_non_directory_logs_have_no_file_effects() {
        let root = tempfile::tempdir().unwrap();
        for timestamp in ["../escape", "20261012_02000x", "202610120020000", "20261012_020000/child"] {
            assert!(WindowsDiagnosticLog::create_at(root.path(), timestamp).is_err());
            assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
        }
        std::fs::write(root.path().join("logs"), b"foreign").unwrap();
        assert!(WindowsDiagnosticLog::create_at(root.path(), "20261012_020002").is_err());
        assert_eq!(std::fs::read(root.path().join("logs")).unwrap(), b"foreign");
    }

    #[test]
    fn held_ancestry_prevents_retargeting_and_poison_refuses_later_writes() {
        let root = tempfile::tempdir().unwrap();
        let cwd = root.path().join("cwd");
        std::fs::create_dir(&cwd).unwrap();
        let mut log = WindowsDiagnosticLog::create_at(&cwd, "20261012_020003").unwrap();
        assert!(std::fs::rename(&cwd, root.path().join("moved")).is_err());
        assert!(std::fs::rename(cwd.join("logs"), cwd.join("elsewhere")).is_err());
        log.poisoned = true;
        assert!(log.append_utf8("must not write").is_err());
        assert!(log.sync().is_err());
        let path = log.path().to_path_buf();
        drop(log);
        assert!(std::fs::read(path).unwrap().is_empty());
    }

    #[test]
    fn junction_logs_and_junction_ancestry_never_write_outside() {
        let root = tempfile::tempdir().unwrap();
        let outside = root.path().join("outside");
        let cwd = root.path().join("cwd");
        std::fs::create_dir(&outside).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        std::fs::write(outside.join("sentinel"), b"unchanged").unwrap();
        std::fs::create_dir(cwd.join("logs")).unwrap();
        reparse_fixture::set_junction(&cwd.join("logs"), &outside);
        assert!(WindowsDiagnosticLog::create_at(&cwd, "20261012_020004").is_err());
        let alias = root.path().join("alias");
        std::fs::create_dir(&alias).unwrap();
        reparse_fixture::set_junction(&alias, &outside);
        assert!(WindowsDiagnosticLog::create_at(&alias, "20261012_020004").is_err());
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 1);
        assert_eq!(std::fs::read(outside.join("sentinel")).unwrap(), b"unchanged");
    }

    #[test]
    fn real_write_failure_poison_is_retained() {
        let root = tempfile::tempdir().unwrap();
        let mut log = WindowsDiagnosticLog::create_at(root.path(), "20261012_020005").unwrap();
        let readonly = root.path().join("readonly-fixture");
        std::fs::write(&readonly, b"sentinel").unwrap();
        // Test-only replacement of the private handle forces a real failed write.
        log.file = File::open(&readonly).unwrap();
        assert!(log.append_utf8("rejected").is_err());
        assert!(log.poisoned);
        assert!(log.append_utf8("still rejected").is_err());
        assert!(log.sync().is_err());
        assert_eq!(std::fs::read(readonly).unwrap(), b"sentinel");
    }

}
