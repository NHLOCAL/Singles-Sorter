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
