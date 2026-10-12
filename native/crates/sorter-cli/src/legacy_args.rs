//! Positional/flag compatibility with the pinned v14 argparse declarations.
//! Parsing never opens paths, loads resources, or authorizes a serialized plan.
use sorter_core::Settings;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}
impl LogLevel {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "DEBUG" => Ok(Self::Debug),
            "INFO" => Ok(Self::Info),
            "WARNING" => Ok(Self::Warning),
            "ERROR" => Ok(Self::Error),
            "CRITICAL" => Ok(Self::Critical),
            _ => Err(format!(
                "invalid log level {value:?}; choose DEBUG, INFO, WARNING, ERROR or CRITICAL"
            )),
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Critical => "CRITICAL",
        }
    }
}
#[derive(Debug)]
pub(super) struct Options {
    pub source: PathBuf,
    pub target: Option<PathBuf>,
    pub settings: Settings,
    pub fix_names: bool,
    pub log_level: LogLevel,
    pub dry_run: bool,
    pub no_ai: bool,
    pub model_directory: Option<PathBuf>,
    pub personal_aliases: Option<PathBuf>,
    pub journal_directory: Option<PathBuf>,
}
#[derive(Debug)]
pub(super) enum Parsed {
    Help,
    Run(Options),
}
const RELEASED: [(&str, char); 8] = [
    ("copy_mode", 'c'),
    ("abc_sort", 'a'),
    ("exist_only", 'e'),
    ("no_singles_dir", 'n'),
    ("main_dir_only", 'm'),
    ("duet_mode", 'd'),
    ("fix_names", 'f'),
    ("log_level", 'l'),
];
fn boolean(options: &mut Options, name: char) -> Result<(), String> {
    match name {
        'c' => options.settings.copy_mode = true,
        'a' => options.settings.abc_sort = true,
        'e' => options.settings.exist_only = true,
        'n' => options.settings.singles_folder = false,
        'm' => options.settings.main_folder_only = true,
        'd' => options.settings.duet_mode = true,
        'f' => options.fix_names = true,
        _ => return Err(format!("unrecognized option -{name}")),
    }
    Ok(())
}
fn value(
    args: &[String],
    index: &mut usize,
    attached: Option<&str>,
    name: &str,
) -> Result<String, String> {
    if let Some(value) = attached {
        return Ok(value.to_owned());
    }
    *index += 1;
    args.get(*index)
        .filter(|v| !v.starts_with('-'))
        .cloned()
        .ok_or_else(|| format!("option {name} requires a value"))
}
pub(super) fn parse(args: &[String]) -> Result<Parsed, String> {
    let mut options = Options {
        source: PathBuf::new(),
        target: None,
        settings: Settings::default(),
        fix_names: false,
        log_level: LogLevel::Info,
        dry_run: false,
        no_ai: false,
        model_directory: None,
        personal_aliases: None,
        journal_directory: None,
    };
    let mut positional = Vec::new();
    let mut literal = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if literal || !arg.starts_with('-') || arg == "-" {
            positional.push(PathBuf::from(arg));
        } else if arg == "--" {
            literal = true;
        } else if matches!(arg.as_str(), "-h" | "--help") {
            return Ok(Parsed::Help);
        } else if let Some(long) = arg.strip_prefix("--") {
            let (name, attached) = long
                .split_once('=')
                .map_or((long, None), |(k, v)| (k, Some(v)));
            if !name.is_empty() && "help".starts_with(name) && attached.is_none() {
                return Ok(Parsed::Help);
            }
            // Released abbreviations retain their original meaning. New options
            // require their complete spelling and cannot make --d ambiguous.
            let released = RELEASED
                .iter()
                .filter(|(candidate, _)| candidate.starts_with(name))
                .collect::<Vec<_>>();
            if !name.is_empty() && released.len() == 1 {
                let short = released[0].1;
                if short == 'l' {
                    options.log_level = LogLevel::parse(&value(args, &mut index, attached, arg)?)?;
                } else if attached.is_some() {
                    return Err(format!("option --{name} does not take a value"));
                } else {
                    boolean(&mut options, short)?;
                }
            } else {
                match name {
                    "dry-run" | "decision-only" | "no-ai" if attached.is_none() => {
                        if name == "no-ai" {
                            options.no_ai = true;
                        } else {
                            options.dry_run = true;
                        }
                    }
                    "model-directory" => {
                        options.model_directory =
                            Some(value(args, &mut index, attached, arg)?.into())
                    }
                    "personal-aliases" => {
                        options.personal_aliases =
                            Some(value(args, &mut index, attached, arg)?.into())
                    }
                    "journal-directory" => {
                        options.journal_directory =
                            Some(value(args, &mut index, attached, arg)?.into())
                    }
                    _ => return Err(format!("unrecognized option {arg}")),
                }
            }
        } else {
            for (offset, name) in arg[1..].char_indices() {
                if name == 'h' {
                    return Ok(Parsed::Help);
                }
                if name == 'l' {
                    let rest = &arg[1 + offset + name.len_utf8()..];
                    let rest = rest.strip_prefix('=').unwrap_or(rest);
                    options.log_level = LogLevel::parse(&value(
                        args,
                        &mut index,
                        (!rest.is_empty()).then_some(rest),
                        "-l",
                    )?)?;
                    break;
                }
                boolean(&mut options, name)?;
            }
        }
        index += 1;
    }
    if positional.is_empty() {
        return Err("source_dir is required".into());
    }
    if positional.len() > 2 {
        return Err("expected source_dir and optional target_dir".into());
    }
    options.source = positional.remove(0);
    options.target = positional.pop();
    if options.no_ai && options.model_directory.is_some() {
        return Err("--no-ai and --model-directory cannot be combined".into());
    }
    Ok(Parsed::Run(options))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parsed(args: &[&str]) -> Options {
        match parse(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>()).unwrap() {
            Parsed::Run(options) => options,
            Parsed::Help => panic!("unexpected help"),
        }
    }
    #[test]
    fn every_recorded_frozen_parser_probe_matches() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/legacy-cli/frozen-v14-arguments.json"
        ))
        .unwrap();
        assert_eq!(
            fixture["source_commit"],
            "78d001aee5ce49628bdffebd34540fa83ba32c12"
        );
        assert_eq!(
            fixture["source_sha256"],
            "ec4a98ce29c431b5d46b59f11a531eaf8fb4f3c8c16248372771de630086fffa"
        );
        for probe in fixture["probes"].as_array().unwrap() {
            let args = probe["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap())
                .collect::<Vec<_>>();
            let actual = parsed(&args);
            assert_eq!(
                serde_json::json!({
                    "source_dir": actual.source, "target_dir": actual.target,
                    "copy_mode": actual.settings.copy_mode, "abc_sort": actual.settings.abc_sort,
                    "exist_only": actual.settings.exist_only, "singles_folder": actual.settings.singles_folder,
                    "main_folder_only": actual.settings.main_folder_only, "duet_mode": actual.settings.duet_mode,
                    "fix_names": actual.fix_names, "log_level": actual.log_level.name(),
                }),
                probe["parsed"],
                "{args:?}"
            );
        }
    }
    #[test]
    fn released_defaults_and_bundles_match_frozen_v14() {
        let defaults = parsed(&["source", "target"]);
        assert!(defaults.settings.singles_folder);
        assert!(
            !defaults.settings.copy_mode
                && !defaults.settings.abc_sort
                && !defaults.settings.exist_only
                && !defaults.settings.main_folder_only
                && !defaults.settings.duet_mode
                && !defaults.fix_names
        );
        assert_eq!(defaults.log_level, LogLevel::Info);
        let bundled = parsed(&["source", "target", "-caenmd", "-lDEBUG"]);
        assert!(
            bundled.settings.copy_mode
                && bundled.settings.abc_sort
                && bundled.settings.exist_only
                && bundled.settings.main_folder_only
                && bundled.settings.duet_mode
        );
        assert!(!bundled.settings.singles_folder);
        assert_eq!(bundled.log_level, LogLevel::Debug);
        assert_eq!(parsed(&["source", "-l=DEBUG"]).log_level, LogLevel::Debug);
    }
    #[test]
    fn fix_source_only_long_abbreviations_and_literal_paths() {
        let fixed = parsed(&["source", "--fix_n", "--log_level=WARNING"]);
        assert!(fixed.fix_names && fixed.target.is_none());
        assert_eq!(fixed.log_level, LogLevel::Warning);
        let literal = parsed(&["--", "-source", "target"]);
        assert_eq!(literal.source, PathBuf::from("-source"));
        assert!(parsed(&["source", "target", "--d"]).settings.duet_mode);
        assert!(parsed(&["source", "--decision-only"]).dry_run);
    }
    #[test]
    fn parser_errors_do_not_authorize_or_default_to_execution() {
        for args in [
            vec![],
            vec!["source", "target", "extra"],
            vec!["source", "--copy_mode=true"],
            vec!["source", "-l"],
            vec!["source", "-l", "debug"],
            vec!["source", "--unknown"],
            vec!["source", "--no-ai", "--model-directory", "models"],
        ] {
            assert!(
                parse(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>()).is_err(),
                "{args:?}"
            );
        }
        assert!(matches!(parse(&["--help".into()]), Ok(Parsed::Help)));
    }
}
