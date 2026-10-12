# Original AI and selected Windows CLI release gate

This gate builds the actual `singles-sorter` executable with the explicit
`original-ai` feature and invokes its real `plan-files --json` transport. Windows
also enables `selected-windows-execution` and separately exercises the original
positional frontend against owned files. It also
builds the default-off executable and verifies that a configured profile cannot
enable an uncompiled feature.

The workflow `.github/workflows/original-ai-cli.yml` runs on Linux and Windows.
It uses the workspace's application release profile, not the isolated NER
probe's release profile. PCRE2, TagLib and zlib are static libraries; Windows
uses the dynamic MSVC CRT consistently. Read-only oracle proof and actual
Windows execution proof are distinct required gates. Neither establishes an
Android result or a clean-machine static-CRT claim.

## Independent original oracles

- Both models come only from public baseline
  `78d001aee5ce49628bdffebd34540fa83ba32c12`, with original hashes checked before
  loading. The classifier exporter checks the pinned pickle blob and SHA-256.
- The NER exporter generates the existing fixed synthetic platform suite. The
  recognition integration test checks exact original tokens, transitions and
  spans through workspace release dependencies, including the close-margin case.
- The classifier's existing `verify_original.py` checks all exported parameters
  and 1,519 synthetic inputs with exact analyzed features, sparse TF-IDF, scores,
  classes and class-zero acceptance. Probability/margin tolerance is `1e-14`.
- `check_cli.py` materializes unchanged v14 routing and AI Python source from
  that pinned Git commit. It explicitly supplies authenticated original model
  objects and sets the original availability flag. No prediction or routing
  method is replaced. Wrappers record calls without altering their results.
- Each CLI fixture is a newly generated, silent PCM WAV carrying authored ID3
  metadata. Original `music-tag` verifies fixture metadata and reads the files
  during the original routing oracle; the CLI reads those same bytes via TagLib.

Fixed real-file cases cover aliases, metadata classifier acceptance/rejection,
filename NER, title NER fallback, rejection of a NER `SINGER` span by the
classifier, duet ordering, and a Unicode title. Each is repeated with AI disabled.
Separate cases check configured FixNames without inference, default-off behavior,
wrong/missing/relative configuration, every required bundle part missing or
corrupt, authenticated resource/runtime binding, binding changes when effective
limits change, forged provenance/availability rejection, model input limits, zero
call budget, and exhaustion after one successful prediction. Every fatal response must have no partial plan.

Source, target, preferences and unrelated CWD are fingerprinted before and after
every subprocess (names, types, permissions, size, content hashes and mtimes).
Read access time is intentionally excluded. Executables are copied to a Hebrew
path. The bundle is also checked for modification. Every successful plan remains
non-executable.

`inference_executed` records that a model invocation began; a bounded invocation
can fail before arithmetic. The gate distinguishes those failures from a zero
call budget or unavailable bundle, where no invocation starts. It does not claim
CLI Ctrl-C cooperative cancellation; shared library cancellation is tested in the
recognition integration tests.

## Reproduce

Use Python 3.12.10, the hash-locked NER and classifier oracle requirements, and
`metadata-requirements.txt`. `music-tag` publishes only source; the workflow
builds that authenticated distribution with the NER-pinned setuptools backend
and no isolated/unpinned build dependency download. All other dependencies use
hash-pinned official wheels.

After exporting both models into one job-local bundle and building the release
CLI as shown in the workflow:

```sh
python native/tests/original_ai_cli/check_cli.py \
  --binary native/target/release/singles-sorter \
  --disabled-binary native/target/original-ai/default-cli \
  --bundle /absolute/job-local/bundle \
  --baseline /absolute/pinned-baseline-checkout \
  --report /absolute/job-local/cli-report.json
```

Use `.exe` on Windows. `--disabled-binary` is optional for a focused local check;
CI always supplies it. `python -m unittest discover -s native/tests/original_ai_cli`
checks the harness's own rejection and mutation-detection behavior without model
assets.

No model assets, original pickle, private or sealed corpus, generated numeric
arrays, audio recordings, or test outputs are committed or uploaded by this
workflow. Python is an oracle/build dependency, not a CLI runtime dependency.

## Actual selected Windows execution

`check_execution.py` runs only on the owned Windows CI runner, with closed stdin
and only System32 on PATH. It checks actual positional Copy, same-volume Move,
automatically configured original-AI Copy, MP3 ID3v2.3/v2.4 five-field repair,
unsupported-tag partial outcomes, exact retained backups/media/unknown frames,
read-only dry-run, overlap rejection and original help/argument behavior.

The same checks run on the release EXE, staged package and consumer-extracted ZIP.
Reports bind the EXE hash and, for packaged trials, the complete package manifest.
The packager refuses to archive without both its read-only PowerShell smoke and
its distinct actual execution report. Package notice features must exactly match
the selected CLI feature profile. Its measured 574dcc7 graph contains the same
37 registry packages and notices as before, plus the local MIT executor. The
exact graph and unchanged Cargo.lock are now pinned; actual runtime gates remain
mandatory.

Fixtures are created under a fresh owned temporary directory and retained until
runner disposal for diagnosis. The report is written only after all byte/layout
checks pass. Optimized Python is refused because the independent MP3 parser uses
assertions. The runtime application has no Python dependency. Positional Ctrl-C
handler behavior still needs its own actual console-signal acceptance case.

## Paired actual-Windows v14 diagnostics (not a parity approval)

`check_windows_v14.py` is an additive diagnostic lane after all mandatory
release proof, package and artifact steps. It refuses non-Windows/non-CI execution. Its 36 cases use two fresh owned
trees each, never the user's music or the reference checkout:

- All 32 combinations of Copy/Move, main-only/recursive, abc, singles directory,
  and exist-only. The bit order in each case ID is recorded in `SETTING_NAMES`.
- Real authored silent WAVs at root, direct-child and deeper levels, an unknown
  song, an unsupported sidecar, an existing artist destination, a missing artist
  destination, a processable three-track album with a sidecar, and a protected
  album with three distinct artists and no 70% main artist.
- Four real-original-model duet cases: Copy/Move × first/all artists. The duet
  must actually invoke original NER and the original classifier and return at
  least two artists. Unsupported Move-all remains explicitly OPEN, not passing
  parity. A crash or unrelated execution failure is not treated as unsupported.

The oracle reuses the authenticated model/source loader in `check_cli.py`. Its
existing two-result interface and 44-route checks are unchanged; the new optional
`include_sorter` exposes the same loaded original object to the paired harness.
The sorter source must match frozen commit
`78d001aee5ce49628bdffebd34540fa83ba32c12`, original blob
`0b67726e72e527556f47a8eeffeb44778c62809b`, 41,353 bytes, SHA-256
`ec4a98ce29c431b5d46b59f11a531eaf8fb4f3c8c16248372771de630086fffa`.
It never uses the later modified checkout's `src/core/singles_sorter_v5.py`.
Both case CWDs receive exactly the immutable released primary alias CSV, with
hashes recorded. No original policy or counter method is reimplemented. The
original runs unchanged class methods with real music-tag metadata; wrappers
record their actual returns and filesystem calls. All original writes and
mutation source/destination paths are confined to that case with a BaseException
safety abort that its ordinary Exception handlers cannot swallow. No injected
metadata, synthetic artist result or model replacement is used in these cases.

Each native case first records a real `plan-files` response without mutation,
then runs the selected executable's actual positional frontend with closed stdin
and only System32 on PATH. Evidence contains raw stdout/stderr, exit code,
source/target snapshots with exact paths, file hashes and mtimes, original actual
summary/decisions/operation order/model calls, native preview decisions and
predicted counters, native final result, raw journal bytes and ordered records,
and a mapping of committed receipts to independently observed destination bytes.
Physical committed/moved counts remain separate from original summary counters.

A native final `legacy_summary` must match the original return exactly:
`songs_sorted`, `artist_folders_created`, `albums_processed`, and ordered
`top_artists` pairs. Missing actual summary or actual decision evidence is OPEN;
matching predictions never fills that gap. A single committed journal supplies
transfer order for older results. New `effect_receipts` supply the actual cross-
journal namespace order: each file publish/rename must match exactly one committed
identity, and each whole-directory rename must match the returned completed root
and the retained source identity. A whole rename stays one directory event. It is
never expanded to mimic the original per-child calls. When exact final
names/bytes/counters and complete receipt coverage match, only a whole rename
replacing the contiguous direct-child Move calls may be classified as
`OBSERVED_DIFFERENCE` in physical operation granularity, without failing the
overall sorting-result comparison. The surrounding ordinary transfer order must
still match. All other order/path/counter differences remain MISMATCH. Source
retirement and child-name receipts remain in the raw stream, outside the narrowly
named original copy/move-call comparison. Missing ordered evidence remains OPEN.
All traces remain in encounter order, including ties and skips.

Only each exact owned root prefix is stripped for comparison, accepting the
known local-drive `\\?\` prefix returned by `GetFinalPathNameByHandleW` for that
exact owned root. Raw UTF-16 references remain in the evidence. No names, case,
Unicode, separators, artists, counters or order are normalized. Original tuples
serialize as JSON arrays without changing values/order. Authored file mtimes use
one fixed epoch and are compared exactly. Raw directory mtimes are retained but
excluded from paired equality because the independent jobs run at different
wall times; access time is not sampled. Runtime/job IDs, OS identities and plan
hashes are recorded where available but are not expected equal across trees.
These choices are repeated in the machine-readable report.

The two historical unsafe cases remain narrowly identified from the hash-bound
immutable `historical-unsafe.jsonl`: deleting an untransferred duet after all
exist-only destinations were skipped, and incrementing legacy success counters
after a failed Move. This lane does not rerun those synthetic historical tests
or claim Windows proof for them. They are never blanket exclusions for other
mismatches and never requirements to reproduce data loss or false physical
success.

Every attempted case writes a separate JSON record, including errors; subsequent
cases continue. The initial OPEN/incomplete report is persisted before each
case, and completed phases are checkpointed. An interrupted diagnostic remains
explicitly incomplete; it cannot starve the preceding mandatory proof/package
steps or change their already-uploaded artifacts. The summary binds each evidence file's SHA-256 and records all
36 outcomes. Nonzero exit codes distinguish OPEN (2) and mismatch/harness error
(1) from complete MATCH (0). The workflow deliberately marks this diagnostic
step `continue-on-error`, limits it to eight minutes, and always uploads its evidence separately, including
when other gates fail. A green enclosing workflow therefore does NOT establish
v14 parity. No preview package is relabeled as parity-approved by this lane.
No authored media, model pickle or original model assets are uploaded; only
owned-fixture diagnostics are retained for seven days.

Reproduce on the owned Windows CI runner after the pinned export/build steps:

```powershell
python tests/original_ai_cli/check_windows_v14.py --binary target/release/singles-sorter.exe --bundle target/original-ai/bundle --baseline "$env:GITHUB_WORKSPACE/original-baseline" --report target/original-ai/windows-v14-paired/report.json
```

Portable `test_windows_v14.py` checks exhaustive coverage, mutation confinement,
exact comparisons, narrow timestamp handling, counter separation and authored
fixture equivalence. These harness tests, including any Linux original-method
smoke, are not substitutes for the Windows paired report.
