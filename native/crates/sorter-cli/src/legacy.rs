//! Human CLI over the existing real-file planner and selected-root executor.
use super::{
    Failure,
    legacy_args::{self, LogLevel, Options, Parsed},
    plan_files,
};
use sorter_core::job_plan::{JobMode, JobPlanV2, OperationKind};
use std::{
    io::{self, Write},
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

pub(super) const HELP: &str = "Usage: singles-sorter source_dir [target_dir] [options]

Released v14 options (defaults unchanged):
  -c, --copy_mode        Copy instead of move
  -a, --abc_sort         Sort artist folders alphabetically
  -e, --exist_only       Transfer to existing artist folders only
  -n, --no_singles_dir   Do not create the inner singles folder
  -m, --main_dir_only    Process only the main folder
  -d, --duet_mode        Use every detected artist for duets
  -f, --fix_names        Repair filenames and the five released metadata fields
  -l, --log_level LEVEL  DEBUG, INFO, WARNING, ERROR or CRITICAL (default INFO)
  -h, --help             Show help

Additional explicit controls:
  --dry-run             Plan and print decisions without changing files
  --decision-only       Same read-only decision plan as --dry-run
  --no-ai               Explicitly disable recognition
  --model-directory DIR Use a verified original native model bundle
  --singer-list CSV     Use this primary list instead of app/singer-list.csv or embedded aliases
  --personal-aliases CSV Use this personal list instead of app/personal-singer-list.csv
  --journal-directory DIR  Existing, separate journal root (default LOCALAPPDATA)

A positional invocation executes its freshly validated plan without a JSON prompt.
Sort needs source and target; Fix needs source only. Relative paths use the current
working directory. Models default to models/original beside this executable when
present; an invalid existing bundle is an error, never a silent fallback.
Unsupported plans are refused before mutation. Completed partial work is reported;
retained repair backups and journals are never automatically deleted or replayed.
Exit codes: 0 complete; 1 processing/partial failure; 2 invalid arguments;
3 unavailable capability; 130 cooperative Ctrl-C cancellation.
Diagnostics go to stderr; --dry-run decisions go to stdout. No log file is created
implicitly. The supplied-facts and real-file machine protocols remain separate:
classify --json; plan --json; plan-files --json; run-files --ndjson;
capabilities --json. The supplied-facts commands never perform physical changes;
unsupported capabilities fail closed.
";

fn log(level: LogLevel, threshold: LogLevel, message: &str) -> Result<(), Failure> {
    if level < threshold {
        return Ok(());
    }
    writeln!(
        io::stderr().lock(),
        "{} - MusicSorter - {}",
        level.name(),
        message
    )
    .map_err(|e| Failure::new(1, "OUTPUT_ERROR", e.to_string()))
}
fn absolute(path: &Path) -> Result<PathBuf, Failure> {
    if path.as_os_str().is_empty() || path.to_str().is_none_or(|s| s.contains('\0')) {
        return Err(Failure::new(
            2,
            "INVALID_INPUT",
            "Paths must be nonempty valid UTF-8 without NUL",
        ));
    }
    #[cfg(windows)]
    if path.has_root() != path.is_absolute()
        || (!path.is_absolute() && matches!(path.components().next(), Some(Component::Prefix(_))))
    {
        return Err(Failure::new(
            2,
            "INVALID_INPUT",
            "Use an absolute drive path or a path relative to the current directory, not a drive-relative/root-relative path",
        ));
    }
    let joined = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| Failure::new(1, "CURRENT_DIRECTORY", e.to_string()))?
            .join(path)
    };
    // Resolve dot components lexically; never canonicalize through a link. The
    // existing native read/grant walk rejects reparse ancestors of this path.
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => {
                normalized.push(component.as_os_str());
                if matches!(component, Component::Normal(_)) {
                    reject_link_prefix(&normalized)?;
                }
            }
        }
    }
    plan_files::validate_path(&normalized)?;
    Ok(normalized)
}
fn reject_link_prefix(path: &Path) -> Result<(), Failure> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(Failure::new(1, "PATH_INSPECTION", error.to_string())),
    };
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = false;
    if metadata.file_type().is_symlink() || reparse {
        return Err(Failure::new(
            2,
            "REPARSE_PATH",
            "Selected path contains a link or reparse component",
        ));
    }
    // This compatibility precheck prevents dot normalization from erasing an
    // explicitly selected link. Only the later held native root chain grants
    // mutation authority; this path-based precheck is never a substitute.
    Ok(())
}
fn optional_existing(path: &Path) -> Result<Option<PathBuf>, Failure> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => absolute(path).map(Some),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Failure::new(1, "RESOURCE_READ_FAILED", e.to_string())),
    }
}
fn request(options: &Options) -> Result<plan_files::Request, Failure> {
    let source = absolute(&options.source)?;
    let target = if options.fix_names {
        None
    } else {
        Some(absolute(options.target.as_deref().ok_or_else(|| {
            Failure::new(
                2,
                "TARGET_REQUIRED",
                "Sort requires target_dir; use -f to repair the source without sorting",
            )
        })?)?)
    };
    let primary_aliases = match &options.primary_aliases {
        Some(path) => Some(absolute(path)?),
        None => optional_existing(Path::new("app/singer-list.csv"))?,
    };
    let personal_aliases = match &options.personal_aliases {
        Some(path) => Some(absolute(path)?),
        None => optional_existing(Path::new("app/personal-singer-list.csv"))?,
    };
    let models = if options.no_ai {
        None
    } else if let Some(path) = &options.model_directory {
        Some(absolute(path)?)
    } else {
        let exe = std::env::current_exe()
            .map_err(|e| Failure::new(1, "APPLICATION_PATH", e.to_string()))?;
        optional_existing(
            &exe.parent()
                .ok_or_else(|| {
                    Failure::new(1, "APPLICATION_PATH", "Executable directory unavailable")
                })?
                .join("models/original"),
        )?
    };
    let ai_profile = match models {
        Some(directory) => plan_files::RequestedAi::Configured {
            bundle_id: "original-v14-ner-3.7.5".into(),
            model_directory: Some(directory),
            limits: None,
        },
        None => plan_files::RequestedAi::Disabled,
    };
    Ok(plan_files::Request {
        schema_version: 2,
        mode: if options.fix_names {
            JobMode::FixNames
        } else {
            JobMode::Sort
        },
        source,
        target,
        settings: options.settings.clone(),
        ai_profile,
        primary_aliases,
        personal_aliases,
    })
}
fn describe(plan: &JobPlanV2, output: &mut impl Write) -> Result<(), Failure> {
    let mut write = |text: String| {
        writeln!(output, "{text}").map_err(|e| Failure::new(1, "OUTPUT_ERROR", e.to_string()))
    };
    write(format!(
        "Plan {}: {:?}; {} operations (predictions only)",
        plan.binding_sha256,
        plan.request.mode,
        plan.operations.len()
    ))?;
    for operation in &plan.operations {
        match &operation.kind {
            OperationKind::SingleTransfer {
                source,
                artist,
                transfer,
            } => write(format!(
                "{}: {} {:?} -> {:?} / {} [{}]",
                operation.id,
                if transfer.copy { "COPY" } else { "MOVE" },
                source.reference,
                transfer.destination.root,
                transfer.destination.components.join("/"),
                artist
            ))?,
            OperationKind::AlbumTransfer {
                source,
                destination,
                children,
                ..
            } => write(format!(
                "{}: ALBUM {:?} -> {:?} / {} ({} files)",
                operation.id,
                source.reference,
                destination.root,
                destination.components.join("/"),
                children.len()
            ))?,
            OperationKind::FixFile(fix) => write(format!("{}: FIX {:?}", operation.id, fix))?,
        }
    }
    for warning in &plan.warnings {
        write(format!("WARNING: {warning}"))?;
    }
    for blocker in &plan.blockers {
        write(format!("BLOCKED: {blocker}"))?;
    }
    Ok(())
}
fn execute(options: &Options) -> Result<(), Failure> {
    if !options.dry_run && !cfg!(all(windows, feature = "selected-windows-execution")) {
        return Err(Failure::unsupported(
            "This build cannot execute positional jobs; use --dry-run for an explicit read-only plan or the Windows selected-execution build",
        ));
    }
    let request = request(options)?;
    if options.dry_run && options.journal_directory.is_some() {
        log(
            LogLevel::Debug,
            options.log_level,
            "Dry run does not open or create journals",
        )?;
    }
    log(
        LogLevel::Info,
        options.log_level,
        if options.fix_names {
            "Starting filename and metadata repair"
        } else {
            "Starting directory scan"
        },
    )?;
    if matches!(request.ai_profile, plan_files::RequestedAi::Disabled) {
        log(
            LogLevel::Info,
            options.log_level,
            "AI disabled; using the released alias and metadata rules",
        )?;
    }
    if options.dry_run {
        let planned = plan_files::execute_request(&request)?;
        let plan = planned
            .plan
            .ok_or_else(|| Failure::new(1, "PLAN_MISSING", "No complete plan was produced"))?;
        describe(&plan, &mut io::stdout().lock())?;
        log(
            LogLevel::Info,
            options.log_level,
            "Dry run complete; no files changed",
        )?;
        return Ok(());
    }
    #[cfg(all(windows, feature = "selected-windows-execution"))]
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        let journal_directory = match &options.journal_directory {
            Some(path) => absolute(path)?,
            None => absolute(&PathBuf::from(
                std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
                    Failure::new(
                        2,
                        "JOURNAL_REQUIRED",
                        "LOCALAPPDATA is unavailable; provide an existing --journal-directory",
                    )
                })?,
            ))?,
        };
        // No mkdir: all journal paths must already exist, pass the same pinned
        // root observations and be disjoint from the selected source/target.
        log(
            LogLevel::Info,
            options.log_level,
            &format!("Journal directory: {}", journal_directory.display()),
        )?;
        let _handler = console_cancel::Registration::install()?;
        let result = super::run_files::execute_request(
            super::run_files::Request {
                schema_version: 1,
                preview: request,
                journal_directory,
            },
            &console_cancel::CANCEL,
            &AtomicBool::new(false),
            &mut |plan, _| {
                if plan.operations.iter().any(|operation| {
                    matches!(
                        operation.kind,
                        sorter_core::job_plan::OperationKind::AlbumTransfer { .. }
                    )
                }) {
                    log(
                        LogLevel::Warning,
                        options.log_level,
                        super::run_files::ALBUM_CONDITION,
                    )?;
                }
                if options.log_level == LogLevel::Debug {
                    describe(plan, &mut io::stderr().lock())?;
                }
                Ok(!console_cancel::CANCEL.load(Ordering::Acquire))
            },
            &mut |kind, data| {
                log(
                    if kind == "progress" {
                        LogLevel::Debug
                    } else {
                        LogLevel::Info
                    },
                    options.log_level,
                    &format!("{kind}: {data}"),
                )
            },
        );
        if console_cancel::CANCEL.load(Ordering::Acquire) {
            return Err(Failure::new(
                130,
                "CANCELLED",
                "Cancelled; inspect reported outcomes and retained journals before retrying",
            ));
        }
        result
    }
    #[cfg(not(all(windows, feature = "selected-windows-execution")))]
    Err(Failure::unsupported(
        "Physical execution is unavailable on this platform",
    ))
}
pub(super) fn run(args: &[String]) -> ExitCode {
    let options = match legacy_args::parse(args) {
        Ok(Parsed::Help) => {
            return match io::stdout().lock().write_all(HELP.as_bytes()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::from(1),
            };
        }
        Ok(Parsed::Run(options)) => options,
        Err(message) => {
            let _ = writeln!(
                io::stderr().lock(),
                "singles-sorter: {message}\nUse --help for usage."
            );
            return ExitCode::from(2);
        }
    };
    match execute(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            let _ = log(
                LogLevel::Error,
                options.log_level,
                &format!("{}: {}", failure.error.code, failure.error.message),
            );
            ExitCode::from(failure.exit)
        }
    }
}

#[cfg(all(windows, feature = "selected-windows-execution"))]
mod console_cancel {
    use super::Failure;
    use std::sync::atomic::{AtomicBool, Ordering};
    pub(super) static CANCEL: AtomicBool = AtomicBool::new(false);
    // Documented Kernel32 ABI: BOOL WINAPI HandlerRoutine(DWORD).
    // https://learn.microsoft.com/windows/console/setconsolectrlhandler
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }
    unsafe extern "system" fn handler(event: u32) -> i32 {
        if matches!(event, 0 | 1) {
            // CTRL_C_EVENT / CTRL_BREAK_EVENT only.
            CANCEL.store(true, Ordering::Release);
            1
        } else {
            0
        }
    }
    pub(super) struct Registration;
    impl Registration {
        pub(super) fn install() -> Result<Self, Failure> {
            CANCEL.store(false, Ordering::Release);
            // The static callback only stores an atomic; it owns no borrowed
            // data and performs no allocation, logging, filesystem or cleanup.
            if unsafe { SetConsoleCtrlHandler(Some(handler), 1) } == 0 {
                return Err(Failure::new(
                    1,
                    "CANCEL_HANDLER",
                    std::io::Error::last_os_error().to_string(),
                ));
            }
            Ok(Self)
        }
    }
    impl Drop for Registration {
        fn drop(&mut self) {
            // Remove exactly our handler; never install the NULL ignore flag.
            unsafe {
                SetConsoleCtrlHandler(Some(handler), 0);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn dot_resolution_cannot_hide_a_selected_link() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("real")).unwrap();
        std::os::unix::fs::symlink(root.path().join("real"), root.path().join("link")).unwrap();
        assert!(absolute(&root.path().join("link/../real")).is_err());
    }
    #[test]
    fn relative_and_dot_paths_are_lexical_not_canonicalized() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(
            absolute(Path::new(".")).unwrap_or_else(|failure| panic!("{}", failure.error.message)),
            cwd
        );
        assert_eq!(
            absolute(Path::new("missing/../song"))
                .unwrap_or_else(|failure| panic!("{}", failure.error.message)),
            cwd.join("song")
        );
        assert!(absolute(Path::new("")).is_err());
    }
}
