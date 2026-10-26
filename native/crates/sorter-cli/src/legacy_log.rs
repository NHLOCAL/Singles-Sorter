//! Released Windows positional logging, separate from media execution authority.
use super::{Failure, legacy_args::LogLevel};
#[cfg(all(windows, feature = "selected-windows-execution"))]
use sorter_io::desktop::windows_mutation::WindowsDiagnosticLog;
#[cfg(all(windows, feature = "selected-windows-execution"))]
use std::cell::RefCell;
use std::io::{self, Write};

pub(super) struct Logger {
    threshold: LogLevel,
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    file: Option<RefCell<WindowsDiagnosticLog>>,
}
impl Logger {
    pub(super) fn new(
        threshold: LogLevel,
        dry_run: bool,
        exclusions: &[std::path::PathBuf],
    ) -> Result<Self, Failure> {
        #[cfg(all(windows, feature = "selected-windows-execution"))]
        let file = if dry_run {
            None
        } else {
            let stamp = local_time();
            Some(RefCell::new(WindowsDiagnosticLog::create_for_cwd(&stamp.filename(), exclusions)
                .map_err(|error| Failure::new(1, "LOG_CREATE_FAILED", format!("{error}. Use existing selected roots and run from a working directory whose logs child is separate from source, target and journal. A creation failure may leave a new diagnostic directory or file; inspect it before retrying.")))?))
        };
        #[cfg(not(all(windows, feature = "selected-windows-execution")))]
        let _ = (dry_run, exclusions);
        Ok(Self {
            threshold,
            #[cfg(all(windows, feature = "selected-windows-execution"))]
            file,
        })
    }
    pub(super) fn emit(&self, level: LogLevel, message: &str) -> Result<(), Failure> {
        if level < self.threshold {
            return Ok(());
        }
        #[cfg(windows)]
        let record = format_record(&local_time(), level, message);
        // Non-Windows builds expose read-only preview only. Keep their existing
        // diagnostic format rather than inventing a local-time conversion.
        #[cfg(not(windows))]
        let record = format!("{} - MusicSorter - {}\n", level.name(), message);
        #[cfg(all(windows, feature = "selected-windows-execution"))]
        if let Some(file) = &self.file {
            file.borrow_mut()
                .append_utf8(&record)
                .map_err(|error| Failure::new(1, "LOG_WRITE_FAILED", error.to_string()))?;
        }
        io::stderr()
            .lock()
            .write_all(record.as_bytes())
            .map_err(|error| Failure::new(1, "OUTPUT_ERROR", error.to_string()))
    }
    pub(super) fn announce(&self) -> Result<(), Failure> {
        #[cfg(all(windows, feature = "selected-windows-execution"))]
        if let Some(file) = &self.file {
            let path = file.borrow().path().to_owned();
            self.emit(LogLevel::Info, &format!("Log file: {}", path.display()))?;
        }
        Ok(())
    }
    pub(super) fn finish(&self) -> Result<(), Failure> {
        #[cfg(all(windows, feature = "selected-windows-execution"))]
        if let Some(file) = &self.file {
            file.borrow_mut()
                .sync()
                .map_err(|error| Failure::new(1, "LOG_SYNC_FAILED", error.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(any(windows, test))]
#[derive(Clone, Copy)]
struct LocalTime {
    year: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
}
#[cfg(any(all(windows, feature = "selected-windows-execution"), test))]
impl LocalTime {
    fn filename(&self) -> String {
        format!(
            "{:04}{:02}{:02}_{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}
#[cfg(any(windows, test))]
fn format_record(time: &LocalTime, level: LogLevel, message: &str) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} - MusicSorter - {} - {}\n",
        time.year,
        time.month,
        time.day,
        time.hour,
        time.minute,
        time.second,
        level.name(),
        message
    )
}
#[cfg(windows)]
fn local_time() -> LocalTime {
    // Documented Kernel32 output-only SYSTEMTIME ABI, eight consecutive WORDs.
    // https://learn.microsoft.com/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlocaltime
    #[repr(C)]
    #[derive(Default)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(time: *mut SystemTime);
    }
    let mut value = SystemTime::default();
    // The stack object is writable, correctly aligned and exactly the documented
    // structure size. GetLocalTime retains no pointer and has no failure return.
    unsafe { GetLocalTime(&mut value) };
    LocalTime {
        year: value.year,
        month: value.month,
        day: value.day,
        hour: value.hour,
        minute: value.minute,
        second: value.second,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn released_filename_and_utf8_record_format_match() {
        let time = LocalTime {
            year: 2026,
            month: 2,
            day: 3,
            hour: 4,
            minute: 5,
            second: 6,
        };
        assert_eq!(time.filename(), "20260203_040506");
        assert_eq!(
            format_record(&time, LogLevel::Warning, "תיקייה לא זמינה"),
            "2026-02-03 04:05:06 - MusicSorter - WARNING - תיקייה לא זמינה\n"
        );
    }
}
