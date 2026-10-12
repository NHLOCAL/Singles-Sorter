# Positional CLI compatibility

The source reference is the released v14 file at
[`78d001ae`](https://github.com/NHLOCAL/Singles-Sorter/blob/78d001aee5ce49628bdffebd34540fa83ba32c12/src/core/singles_sorter_v5.py),
not the subsequently modified Python tree. Its 41,353 bytes hash to
`ec4a98ce29c431b5d46b59f11a531eaf8fb4f3c8c16248372771de630086fffa`,
also pinned in the full-contract manifest. The fixture
`tests/fixtures/legacy-cli/frozen-v14-arguments.json` records the literal
`argparse.add_argument` declarations and standard-library parser probes extracted
by AST inspection, without executing the source module or its dependencies.

## Invocation and authority

`singles-sorter SOURCE TARGET -c` requests Copy. Omitting `-c` requests Move.
`singles-sorter SOURCE -f` requests filename and metadata repair without a target.
No interactive JSON confirmation is required: this invocation is the user's
execution request. Both positional and NDJSON frontends call the same retained
in-process plan, source/resource revalidation, selected-root grants and native
journaled driver. A file containing a saved plan never grants execution authority.
`--dry-run` / `--decision-only` explicitly request a read-only decision plan.
Unavailable physical execution fails rather than silently becoming a dry run.

The eight released flags, their long underscore spellings, boolean defaults,
short bundles, long-prefix abbreviations and uppercase logging choices are
preserved. New options need exact names so `--d` still means `--duet_mode`.
`--` terminates option interpretation. Relative paths are resolved against CWD,
without following symlinks to canonicalize selected roots; the native root walk
still rejects reparse ancestors and pins exact identities.

The primary alias order is selected explicitly with `--singer-list CSV`, otherwise
loaded from the original CWD `app/singer-list.csv` when present, otherwise from
the embedded frozen list. The native fallback keeps a self-contained package; a
selected empty list is not silently replaced. The legacy personal list
at `app/personal-singer-list.csv` is used when present; `--personal-aliases` selects
an explicit alternative. Primary rows precede personal rows exactly as in v14.
Both selected files use the existing bounded read adapter; fresh plan and resource
hashes are validated again before execution. The additive v2 `primary_aliases`
field exposes the same explicit resource selection to machine callers.
Native original-model assets are discovered beside the
executable at `models/original`, or selected with `--model-directory`.
`--no-ai` is explicit. A present invalid model bundle fails closed rather than
falling back. Arbitrary Python/pickle models from CWD are not loaded.

The default journal root is the already-existing `LOCALAPPDATA` directory;
`--journal-directory` may name another existing, disjoint directory. No parent
or child directory is created before selected-root validation. The actual location
is reported. Journals and original repair backups are retained without automatic
replay, cleanup or rollback.

Windows positional execution restores v14's UTF-8 file and stderr handlers.
The selected log level filters both; record format is
`YYYY-MM-DD HH:MM:SS - MusicSorter - LEVEL - message`, using local Windows time.
The log is `logs/music_sorter_YYYYMMDD_HHMMSS_PID.log` under CWD. A separate
identity-pinned diagnostic sink holds that parent and exclusively creates the
file. Existing files are never adopted or truncated, and write/sync uncertainty
is an explicit failure. The log capability grants no media or journal authority.
`--dry-run` remains an explicit read-only addition and creates no log; its
decisions go to stdout. Machine JSON/NDJSON protocols, help and invalid argument
handling do not create logs. Non-Windows builds currently expose read-only
preview and retain their stderr-only diagnostic format.

Before creating a log, pinned ancestry checks refuse intersections between
CWD/logs and the selected source, target or journal roots. Run from a separate
working directory in that case. No log is created inside a music tree and then
silently excluded from original album policy. Invalid/unverifiable selected
paths are reported on stderr before file logging begins. Log creation itself can
fail after leaving a new directory or empty file; the error does not claim a
clean no-effect result, and no automatic cleanup follows. The no-effect acceptance
exception names only the newly observed log and optional literal logs directory,
never an arbitrary subtree. Exit codes distinguish success
(0), processing/partial failure (1), argument error (2), unavailable capability
(3), and cooperative cancellation (130). V14 sometimes swallowed per-file errors
and returned 0; this frontend reports incomplete work with a nonzero exit.

On Windows, Ctrl-C/Break sets the same cooperative cancellation token used by the
native driver. The handler does no I/O and is removed at scope exit. Planning/model
work must reach its next completion boundary before cancellation reaches the
driver; process close/kill is not represented as successful cooperative cancellation.
The ABI follows [Microsoft's console handler documentation](https://learn.microsoft.com/windows/console/setconsolectrlhandler).

## Verification boundary

Parser fixtures and authored subprocess tests are required on final CI. Actual
Windows tests cover default batch Copy/Move with closed stdin, relative paths,
personal CSV, default and explicit existing journal roots, and rejection of an
overlapping journal. The two logging subprocess cases additionally cover all five
levels, exact UTF-8 stderr/file agreement, pre-create source/target/journal
overlap refusal and missing selected roots. Both staged and extracted consumer
EXEs repeat the logging/overlap checks. These are source tests until the exact
candidate passes.
The CLI package profile must enable selected execution and independently exercise
the extracted executable before it can be called an executable legacy replacement.
Album execution, cross-volume Move, duet source retirement and unsupported tag
formats remain the same backend boundaries as desktop; flags never bypass them.
