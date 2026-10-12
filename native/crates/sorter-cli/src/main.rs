//! Versioned supplied-facts v1 and read-only real-file v2 CLI.
mod legacy;
mod legacy_args;
mod legacy_log;
mod plan_files;
mod run_files;

use std::io::{self, Read, Write};
use std::process::ExitCode;

use serde::Serialize;
use sorter_core::{ClassifyRequest, Decision, EngineError, ErrorCode, Plan, PlanRequest};

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const HELP: &str = "singles-sorter: supplied-facts v1 decisions and read-only real-file v2 previews

Usage:
  singles-sorter classify --json       Read one v1 ClassifyRequest from UTF-8 stdin
  singles-sorter plan --json           Read one v1 PlanRequest from UTF-8 stdin
  singles-sorter plan-files --json     Read one v2 real-file preview request
  singles-sorter run-files --ndjson    Inspect, explicitly confirm, and execute one Windows job
  singles-sorter capabilities --json   List supported and upcoming capabilities
  singles-sorter --help                Show this help without reading stdin

JSON commands emit one versioned envelope to stdout; diagnostics go to stderr.
The stdin request limit is 16 MiB (16,777,216 bytes), including whitespace.
V1 executes no inference. V1 decisions and plan-files plans are non-executable.
V2 original inference requires the original-ai build and an explicit verified bundle.
Physical apply requires the selected-windows-execution Windows build and a live
retained plan. run-files uses an explicit inspect/apply handshake; positional
invocations authorize their own fresh plan. plan-files remains read-only.
V2 previews use read-only traversal, CSV aliases and fixture-scoped metadata.
Linux is a read-only test backend. Windows candidate gates and Android acceptance are separate.
run-files NDJSON is a separate single-job protocol. Positional source/target commands
use the retained planner/executor directly; --dry-run is an explicit read-only choice.
Exit codes: 0 success; 1 processing/internal failure; 2 invalid request;
3 unsupported capability. This slice is not the complete Windows/Android app.
";

#[derive(Serialize)]
struct Envelope {
    schema_version: u16,
    status: &'static str,
    result: ResultPayload,
    error: Option<ProtocolError>,
}

#[derive(Serialize)]
struct ResultPayload {
    command: String,
    inference_executed: bool,
    executable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    decision: Option<Decision>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<Plan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capabilities: Option<Capabilities>,
}

impl ResultPayload {
    fn empty(command: &str) -> Self {
        Self {
            command: command.into(),
            inference_executed: false,
            executable: false,
            decision: None,
            plan: None,
            capabilities: None,
        }
    }
}

#[derive(Serialize)]
struct ProtocolError {
    code: String,
    message: String,
}

struct Failure {
    exit: u8,
    error: ProtocolError,
    inference_executed: bool,
}

impl Failure {
    fn new(exit: u8, code: &str, message: impl Into<String>) -> Self {
        Self {
            exit,
            inference_executed: false,
            error: ProtocolError {
                code: code.into(),
                message: message.into(),
            },
        }
    }

    fn unsupported(message: &str) -> Self {
        Self::new(3, "UNSUPPORTED_CAPABILITY", message)
    }
}

impl From<EngineError> for Failure {
    fn from(error: EngineError) -> Self {
        let exit = match error.code {
            ErrorCode::InvalidInput
            | ErrorCode::UnsupportedSchema
            | ErrorCode::UnsupportedRuleset
            | ErrorCode::MissingInferenceFact
            | ErrorCode::InputTooLarge => 2,
            ErrorCode::UnsupportedCapability | ErrorCode::UnsupportedTargetShape => 3,
            ErrorCode::InferenceError
            | ErrorCode::StalePlan
            | ErrorCode::JournalCorrupt
            | ErrorCode::InvalidJournalTransition => 1,
        };
        Self::new(exit, &error.code.to_string(), error.message)
    }
}

#[derive(Serialize)]
struct Capabilities {
    input_provenance: &'static str,
    scope: &'static str,
    schema_versions: [u16; 1],
    ruleset_ids: [&'static str; 1],
    commands: [&'static str; 3],
    stdin_request_limit_bytes: usize,
    native_inference: bool,
    metadata_decoding: bool,
    csv_parsing: bool,
    directory_traversal: bool,
    album_execution: bool,
    physical_apply: bool,
    upcoming: [&'static str; 1],
    real_file_planning: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_execution: Option<serde_json::Value>,
}

impl Capabilities {
    fn phase_one() -> Self {
        Self {
            input_provenance: "supplied_facts",
            scope: "supplied_single_song_facts",
            schema_versions: [sorter_core::SCHEMA_VERSION],
            ruleset_ids: [sorter_core::RULESET_ID],
            commands: ["classify", "plan", "capabilities"],
            stdin_request_limit_bytes: MAX_INPUT_BYTES,
            native_inference: false,
            metadata_decoding: false,
            csv_parsing: false,
            directory_traversal: false,
            album_execution: false,
            physical_apply: false,
            upcoming: ["ndjson_streaming"],
            real_file_planning: plan_files::capabilities(),
            selected_execution: cfg!(all(windows,feature="selected-windows-execution")).then(||serde_json::json!({"command":"run-files --ndjson","scope":"single-process inspected-plan confirmation","modes":["Copy","same-volume Move","bounded copied Move","FixNames"],"copied_move_scope":"ordinary quiescent NTFS singles across volumes or nonempty eligible original duet cohorts; main and named streams retained from preview; ordinary same-volume singles remain rename","album_execution":"bounded Copy, same-volume whole-directory Move and quiescent NTFS cross-volume file/stream Move; exact empty-source cleanup; original planner order; nested files and sidecars","tag_write_scope":"bounded unflagged MP3 ID3v2.3/v2.4 , native FLAC, single-stream Ogg Vorbis/Opus comments and exact-extent UTF-8 MP4 AAC/ALAC and existing ID3 in ordinary WAVE/AIFF/DSF and terminal UTF-8 WavPack APEv2; five released fields; retained original backup","automatic_backup_cleanup":false,"automatic_replay":false,"process_restart_recovery_ui":false})),
        }
    }
}

enum Invocation {
    Help,
    Json(&'static str),
    RunFiles,
}

fn dispatch(args: &[String]) -> Result<Invocation, Failure> {
    let Some(command) = args.first() else {
        return Ok(Invocation::Help);
    };
    if args.len() == 1 && matches!(command.as_str(), "--help" | "-h") {
        return Ok(Invocation::Help);
    }
    if command == "run-files" {
        return if args.len() == 2 && args[1] == "--ndjson" {
            Ok(Invocation::RunFiles)
        } else if args.len() == 2 && matches!(args[1].as_str(), "--help" | "-h") {
            Ok(Invocation::Help)
        } else {
            Err(Failure::new(
                2,
                "INVALID_INPUT",
                "Expected run-files --ndjson; inspect the returned plan before sending its exact apply confirmation",
            ))
        };
    }
    let command = match command.as_str() {
        "classify" => "classify",
        "plan" => "plan",
        "plan-files" => "plan-files",
        "capabilities" => "capabilities",
        _ if command.starts_with('-') => {
            return Err(Failure::new(
                2,
                "INVALID_INPUT",
                "Expected a command before its arguments; this top-level flag is invalid.",
            ));
        }
        _ => {
            return Err(Failure::unsupported(
                "This operation is unsupported; use classify --json, plan --json, or capabilities --json. No files were changed.",
            ));
        }
    };
    if args.len() == 2 && matches!(args[1].as_str(), "--help" | "-h") {
        return Ok(Invocation::Help);
    }
    // Recognized upcoming syntax is a capability boundary, never a fallback to
    // a legacy executor. Unknown or repeated arguments remain invalid requests.
    if args.len() == 2
        && matches!(
            args[1].as_str(),
            "--ndjson"
                | "--copy_mode"
                | "--abc_sort"
                | "--exist_only"
                | "--no_singles_dir"
                | "--main_dir_only"
                | "--duet_mode"
                | "--fix_names"
                | "--log_level"
                | "--dry-run"
        )
    {
        return Err(Failure::unsupported(
            "NDJSON and legacy flags are upcoming capabilities, unsupported in this supplied-facts slice.",
        ));
    }
    if args.len() != 2 || args[1] != "--json" {
        return Err(Failure::new(
            2,
            "INVALID_INPUT",
            "Expected exactly one --json argument after the command; unknown, duplicate, or missing arguments are invalid.",
        ));
    }
    Ok(Invocation::Json(command))
}

fn read_request() -> Result<Vec<u8>, Failure> {
    let mut bytes = Vec::new();
    // One additional byte distinguishes an oversized request from one exactly
    // at the bound. Never decode a truncated prefix or wait to drain huge input.
    io::stdin()
        .lock()
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            Failure::new(1, "INTERNAL_ERROR", format!("Cannot read stdin: {error}"))
        })?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(Failure::new(
            2,
            "INPUT_TOO_LARGE",
            format!("The complete stdin request must not exceed {MAX_INPUT_BYTES} bytes"),
        ));
    }
    Ok(bytes)
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Failure> {
    // Decode directly into the reviewed strict DTO. An intermediate JSON Value
    // would silently collapse duplicate object fields before DTO validation.
    serde_json::from_slice(bytes).map_err(|error| {
        Failure::new(
            2,
            "INVALID_INPUT",
            format!("Expected one strict v1 UTF-8 JSON request: {error}"),
        )
    })
}

fn execute(command: &str) -> Result<ResultPayload, Failure> {
    let mut result = ResultPayload::empty(command);
    match command {
        "classify" => {
            let request: ClassifyRequest = decode(&read_request()?)?;
            result.decision = Some(sorter_core::classify(&request)?);
        }
        "plan" => {
            let request: PlanRequest = decode(&read_request()?)?;
            result.plan = Some(sorter_core::plan(&request)?);
        }
        "capabilities" => result.capabilities = Some(Capabilities::phase_one()),
        _ => return Err(Failure::unsupported("This operation is unsupported")),
    }
    Ok(result)
}

fn emit(command: &str, response: Result<ResultPayload, Failure>) -> ExitCode {
    let (exit, envelope) = match response {
        Ok(result) => (
            0,
            Envelope {
                schema_version: sorter_core::SCHEMA_VERSION,
                status: "ok",
                result,
                error: None,
            },
        ),
        Err(failure) => {
            let _ = writeln!(
                io::stderr().lock(),
                "{}: {}",
                failure.error.code,
                failure.error.message
            );
            (
                failure.exit,
                Envelope {
                    schema_version: sorter_core::SCHEMA_VERSION,
                    status: "error",
                    result: ResultPayload::empty(command),
                    error: Some(failure.error),
                },
            )
        }
    };
    // Serialize the entire envelope before any stdout write. A transport write
    // failure cannot guarantee delivery, so report it on stderr and exit 1.
    let bytes = match serde_json::to_vec(&envelope) {
        Ok(mut bytes) => {
            bytes.push(b'\n');
            bytes
        }
        Err(error) => {
            let _ = writeln!(
                io::stderr().lock(),
                "INTERNAL_ERROR: Cannot encode response: {error}"
            );
            return ExitCode::from(1);
        }
    };
    let mut stdout = io::stdout().lock();
    if let Err(error) = stdout.write_all(&bytes).and_then(|()| stdout.flush()) {
        let _ = writeln!(
            io::stderr().lock(),
            "INTERNAL_ERROR: Cannot write stdout: {error}"
        );
        return ExitCode::from(1);
    }
    ExitCode::from(exit)
}

fn uses_legacy_frontend(args: &[String]) -> bool {
    let Some(first) = args.first() else {
        return false;
    };
    if args
        .iter()
        .take_while(|arg| arg.as_str() != "--")
        .any(|arg| matches!(arg.as_str(), "--json" | "--ndjson"))
    {
        return false;
    }
    if matches!(
        first.as_str(),
        "classify" | "plan" | "plan-files" | "run-files" | "capabilities"
    ) {
        // Explicit positional target or source-only Fix distinguishes a directory
        // with this literal name from a bare machine command/help diagnostic.
        args.get(1)
            .is_some_and(|argument| !argument.starts_with('-'))
            || matches!(legacy_args::parse(args), Ok(legacy_args::Parsed::Run(options))
                if options.target.is_some() || options.fix_names)
    } else {
        true
    }
}

fn main() -> ExitCode {
    let args: Result<Vec<String>, _> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.into_string())
        .collect();
    let args = match args {
        Ok(args) => args,
        Err(_) => {
            return emit(
                "unknown",
                Err(Failure::new(
                    2,
                    "INVALID_INPUT",
                    "Arguments must be valid UTF-8",
                )),
            );
        }
    };
    if uses_legacy_frontend(&args) {
        return legacy::run(&args);
    }
    match dispatch(&args) {
        Ok(Invocation::Help) => {
            let mut stdout = io::stdout().lock();
            if let Err(error) = stdout
                .write_all(HELP.as_bytes())
                .and_then(|()| stdout.flush())
            {
                let _ = writeln!(
                    io::stderr().lock(),
                    "INTERNAL_ERROR: Cannot write help: {error}"
                );
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Ok(Invocation::RunFiles) => run_files::run(),
        Ok(Invocation::Json("plan-files")) => plan_files::run(),
        Ok(Invocation::Json(command)) => emit(command, execute(command)),
        Err(failure) if args.first().is_some_and(|arg| arg == "plan-files") => {
            plan_files::emit(Err(failure))
        }
        Err(failure) => emit(args.first().map_or("unknown", String::as_str), Err(failure)),
    }
}

#[cfg(test)]
mod frontend_selection_tests {
    use super::*;
    #[test]
    fn literal_command_named_directories_do_not_replace_explicit_machine_protocols() {
        let choose = |args: &[&str]| {
            uses_legacy_frontend(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
        };
        for name in [
            "classify",
            "plan",
            "plan-files",
            "run-files",
            "capabilities",
        ] {
            assert!(choose(&[name, "target", "-c"]));
            assert!(choose(&[name, "target", "--help"]));
            assert!(choose(&[name, "-f"]));
            assert!(choose(&[name, "--", "--json"]));
            assert!(!choose(&[name]));
            assert!(!choose(&[name, "--help"]));
            assert!(!choose(&[name, "--json"]));
            assert!(!choose(&[name, "--ndjson"]));
        }
        assert!(!choose(&["classify", "--copy_mode"]));
        assert!(!choose(&["unknown", "--json"]));
        assert!(choose(&["-1", "target"]));
    }
}
