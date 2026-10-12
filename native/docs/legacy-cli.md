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

Original aliases remain embedded in their frozen order. The legacy personal list
at `app/personal-singer-list.csv` is used when present; `--personal-aliases` selects
an explicit alternative. Native original-model assets are discovered beside the
executable at `models/original`, or selected with `--model-directory`.
`--no-ai` is explicit. A present invalid model bundle fails closed rather than
falling back. Arbitrary Python/pickle models from CWD are not loaded.

The default journal root is the already-existing `LOCALAPPDATA` directory;
`--journal-directory` may name another existing, disjoint directory. No parent
or child directory is created before selected-root validation. The actual location
is reported. Journals and original repair backups are retained without automatic
replay, cleanup or rollback.

Log levels filter stderr diagnostics. `--dry-run` writes its requested decisions
to stdout. Unlike v14's implicit `logs/` directory, no log file is created by
default; shell redirection can capture the output. Exit codes distinguish success
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
overlapping journal. These are source tests until the exact candidate passes.
The CLI package profile must enable selected execution and independently exercise
the extracted executable before it can be called an executable legacy replacement.
Album execution, cross-volume Move, duet source retirement and unsupported tag
formats remain the same backend boundaries as desktop; flags never bypass them.
