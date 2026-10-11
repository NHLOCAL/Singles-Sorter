# Native read-only planning core and JSON CLI

This Rust workspace contains the contracts and pure single-song policy of the
Windows-first rebuild. It lives beside the unchanged released application. The
primary product target remains Windows; Android must eventually include the full
feature set and local AI. Linux contract tests establish neither platform's
final acceptance.

## Current scope

- `sorter-core`: public v1 DTOs, strict serde decoding, request validation,
  pure single-song matching/text/recognition policy, logical read-only planning,
  a source-deletion guard, a journal/replay evidence seam, and immutable fixture
  loaders used only by tests. Read-only v2 job planning now includes album/fix
  policy and an ordered virtual mutation timeline; see the
  [preview acceptance boundary](docs/phase2-preview-status.md).
- `sorter-cli`: `singles-sorter classify --json`, `plan --json`, and
  `capabilities --json`, plus `plan-files --json` for read-only real-file v2
  previews. Supplied-facts v1 classify/plan bytes and schemas are preserved.
  No-argument invocation shows help.
- `sorter-io`: ordered CSV resources, read-only identity-aware storage, and an
  owned read-only TagLib metadata bridge with typed field/error observations.
  See [metadata build and platform gates](docs/metadata-bridge.md). Frozen Linux
  media parity does not establish arbitrary-library or native Windows/Android
  acceptance. V2 previews expose these limitations and the frozen coverage
  report explicitly.
- No actual model inference, tag writes, transfers, UI,
  network service or telemetry is implemented in this slice.

The core's production dependencies are serde, serde_json and SHA256 (sha2),
with exact versions/checksums preserved in Cargo.lock. No Python runtime is required by this
workspace.

## Build and verify

Install Rust 1.99.0 from the official Rust distribution. Cargo dependencies
come from crates.io and their exact versions/checksums are in Cargo.lock.
Install CMake 3.24+ and a C++17 toolchain for the metadata bridge. Its two native
source dependencies are automatically fetched and verified at build time; see
[native dependency cache/offline instructions](crates/sorter-io/dependencies/README.md).
From this directory:

```sh
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --release -p sorter-cli
```

`rust-toolchain.toml` pins Rust 1.99.0, edition 2024, rustfmt and clippy.
For a command run from the repository root, select the toolchain explicitly
with `cargo +1.99.0 ... --manifest-path native/Cargo.toml`.

## Public v1 schema

All types are defined once in `crates/sorter-core/src/contract.rs` and exported
from the crate root as well as the `contract` module. Request structures and
tagged payloads reject unknown fields; future options cannot be silently
ignored. All required fields must be supplied except individual settings,
which use the released defaults when omitted from the settings object.

`validate_classify_request(&ClassifyRequest)` accepts schema_version 1 and
ruleset_id `singles-v14.0` only. It rejects empty source_id, filename, or a
configured recognition bundle_id with INVALID_INPUT. Source IDs and strings
are caller-supplied identities; validation does not open paths, normalize
Unicode, check file extensions or execute inference. Empty metadata and alias
strings are retained so later policy can reproduce released behavior.

### Input records

- Settings: copy_mode=false, main_folder_only=false, singles_folder=true,
  exist_only=false, abc_sort=false, duet_mode=false. An explicit false is kept.
- Alias: `{source, target}` strings. Array order and duplicate rows matter.
- SongFacts: `{source_id, filename, metadata}`.
- MetadataFacts: `{kind: "present", artist, title}` or
  `{kind: "read_error", code}`.
- PolicyContext: `{ruleset_id, settings, aliases, recognition}`.
- ClassifyRequest: `{schema_version, context, song}`.
- TargetSnapshot: `{root_id, existing_directories, existing_files}`. Directory
  and file lists contain ordered arrays of logical path components. They are
  supplied observations, not host filesystem checks.
- PlanRequest: `{schema_version, context, songs, target}`. The planner
  rejects duplicate source_id values and invalid logical snapshot shapes.

RecognitionFacts is exactly one of:

```json
{"kind": "disabled"}
```

```json
{
  "kind": "configured",
  "bundle_id": "caller-supplied-bundle",
  "health": "failed",
  "classifier": {
    "exact requested text": {"kind": "unavailable"},
    "another exact text": {"kind": "error", "code": "MODEL_RUNTIME_FAILURE"}
  },
  "ner": {
    "exact requested text": {"kind": "artists", "value": ["second", "first", "second"]},
    "another exact text": {"kind": "error", "value": {"code": "MODEL_RUNTIME_FAILURE"}}
  }
}
```

RecognitionHealth scalar tags are `healthy` and `failed`. ClassifierFact kind
tags are `accept`, `reject`, `unavailable`, or `error` with a required code.
NerFact uses adjacent `kind` and `value` tags: artists has an ordered string
array, error has a code object, and unavailable is `{"kind":"unavailable"}`.
Map keys retain exact text, including whitespace and Unicode; there is no
automatic inference-key substitution. An absent fact is distinct from a
supplied unavailable/error fact. Disabled and failed-configured states are
both valid inputs and remain distinct after serialization. The policy uses exact requested-text keys and returns MISSING_INFERENCE_FACT
rather than manufacturing a failed observation.

RepairMode scalar tags are `hebrew`, `jibrish`, and `auto`. The fixture bridge
`RepairMode::from_legacy` maps only `heb` to Hebrew and `jib` to Jibrish; every
other legacy value maps to Auto. This fallback is not a permissive v1 JSON
enum decoder: unknown JSON enum tags still fail.

### Example classify request

```json
{
  "schema_version": 1,
  "context": {
    "ruleset_id": "singles-v14.0",
    "settings": {
      "copy_mode": false,
      "main_folder_only": false,
      "singles_folder": true,
      "exist_only": false,
      "abc_sort": false,
      "duet_mode": false
    },
    "aliases": [
      {"source": "אמן", "target": "אמן"},
      {"source": "אמן", "target": "אמן"}
    ],
    "recognition": {"kind": "disabled"}
  },
  "song": {
    "source_id": "song-1",
    "filename": "אמן - שיר.mp3",
    "metadata": {"kind": "present", "artist": "אמן", "title": "שיר"}
  }
}
```

### Response records

Decision and Plan require `provenance: "supplied_facts"`, represented in Rust
by `Provenance::ProvidedFacts`. This is a required field rather than a default
that could conceal missing provenance. No source file identity has been
verified. The CLI's decision and plan records retain this response provenance.

- Decision: `{provenance, artists, selected_by, trace, warnings,
  apply_blockers}`. artists is null or an ordered string array, including
  duplicates. selected_by is null or a DecisionSource scalar tag.
- DecisionSource: `filename_alias`, `metadata_artist_alias`,
  `metadata_artist_classifier`, `title_alias`, `filename_ner`, `title_ner`.
- DecisionEvent kind tags: alias_selected with source and zero-based index;
  classifier_requested with text; ner_requested with text;
  metadata_read_observed with failed. Event arrays retain original order.
- Diagnostic: `{code, message}`. Stable machine-readable codes are separate
  from explanatory messages; failure observations do not themselves authorize
  file changes.
- Plan: `{schema_version, provenance, plan_id, request_hash, ruleset_id, items,
  warnings, executable}`. Phase 1 planners must set executable=false. The
  planner computes and verifies content identities; the DTO alone grants no
  permission or physical apply capability.
- PlannedSong: `{operation_id, source_id, decision, destinations, copy_mode}`.
- DestinationIntent: `{destination_id, components, disposition}`. Disposition
  scalar tags are `required`, `skipped_existing_only`, and `blocked`.
- EngineError: `{code, message}`. It is an error record, not a successful
  supplied-facts decision or executable plan.

An illustrative Decision wire record, not a result of policy execution here:

```json
{
  "provenance": "supplied_facts",
  "artists": ["אמן", "אמן"],
  "selected_by": "filename_alias",
  "trace": [
    {"kind": "alias_selected", "source": "filename_alias", "index": 0},
    {"kind": "metadata_read_observed", "failed": false}
  ],
  "warnings": [],
  "apply_blockers": []
}
```

ErrorCode scalar values are INVALID_INPUT, UNSUPPORTED_SCHEMA,
UNSUPPORTED_RULESET, UNSUPPORTED_CAPABILITY, MISSING_INFERENCE_FACT,
INFERENCE_ERROR, INPUT_TOO_LARGE, UNSUPPORTED_TARGET_SHAPE, STALE_PLAN,
JOURNAL_CORRUPT, and INVALID_JOURNAL_TRANSITION. Unknown error enum values fail
decoding rather than becoming generic successes.

## Immutable compatibility evidence

`tests/fixtures/manifest.json` records source identities, generator hashes,
the recorded oracle runtime, byte counts and limitations. No throwaway probe
implementation was copied. Both corpus files are byte-for-byte data imports,
including their existing expected outputs and formatting:

| File | Evidence | SHA256 |
| --- | --- | --- |
| release-contract.jsonl | 574 release-source cases | d7ce4a2e543a2a4782036c5e55d396a1eb9fd7e4601104ad9d97b485ba095536 |
| ordered-aliases.json | 1,196 ordered aliases; 2,409 cases | dbffac487b7c4ccc787725b8b2e869f5a5b5bb62e5dfe95f12a223bcb4d5ec61 |

The source is v14.0, commit 78d001aee5ce49628bdffebd34540fa83ba32c12.
The shared oracle records CPython 3.12.14 / Unicode 15.0.0 on Linux. The alias
artifact embeds its source/CSV hashes but does not embed a runtime version;
the manifest keeps this provenance limit explicit. These identities do not
recover the historical Windows binary's complete runtime.

The test-only loaders in `crates/sorter-core/tests/common/mod.rs` verify each
SHA256 before decoding and assert 574 cases, 1,196 aliases and 2,409 alias
cases. Expectations must never be regenerated with the new Rust core. Policy
tests consume all 486 applicable shared rule cases (298 matching,
38 cleaning, 27 sanitation, 46 artist validation, 20 encoding repair, and 57
routing cases) plus all 2,409 ordered-alias cases. The logical planner also consumes all 88 frozen target-component cases,
without changing expectations. Independent direct tests exercise every one of
the 64 settings combinations; main_folder_only is retained and hash-bound but
does not imply directory traversal in this supplied-facts stage.

## Pure single-song policy

The crate root and `policy` module export `exact_match`, `clean`, `sanitize`,
`check_artist`, `fix_jibrish`, and `classify`. All input is supplied facts.
These functions never read audio, paths, tags, CSV, or model files and never
perform physical operations. The six settings and public DTO wire shapes are
unchanged.

`classify` preserves filename alias -> metadata artist alias/classifier ->
title alias -> filename NER -> title NER precedence. It observes supplied
metadata after filename lookup even when the filename wins. Alias selection
retains the original zero-based index and verbatim target. NER facts represent
ordered, already-filtered legacy artist outputs, including raw invalid strings
and duplicates; this slice does not run entity recognition or its candidate
classifier. No duet setting truncates the decision artist array.

Configured inference requires an explicit fact for each exact requested text.
Missing facts return MISSING_INFERENCE_FACT. Classifier Error/Unavailable
preserves legacy acceptance with CLASSIFIER_FAIL_OPEN evidence and an apply
blocker. NER Unavailable preserves the legacy empty result and title fallback,
with warning/blocker; NER invocation Error returns INFERENCE_ERROR. Failed
bundle health warns, and adds an apply blocker on paths that require configured
inference. An unused failed bundle does not block a deterministic alias result.
Deliberately Disabled recognition neither requests facts nor accepts an
arbitrary plausible metadata artist. Metadata errors warn without discarding a
filename match. A filename/title that sanitation turns into no string returns
INVALID_INPUT when the release would need to analyze that value.

Future apply eligibility must inspect `Decision.apply_blockers`; producing a
decision is neither permission to apply it nor proof that a file/model exists.
Actual apply remains unsupported.

### Frozen Unicode predicates

`crates/sorter-core/src/unicode15.rs` contains generated CPython 3.12 / Unicode
15 word/space ranges. Rust's current Unicode predicates and regex classes are
not used. `unicode/unicode15-provenance.json` records source references, runtime,
generator/table hashes and exhaustive codepoint-membership hashes. The tests
verify those memberships over all 1,114,112 codepoints (surrogates are false),
as well as independent Kawi, combining-mark, numeric and whitespace anchors.

To reproduce the development-time tables with CPython 3.12 / Unicode 15:

```sh
python3.12 unicode/generate_unicode15.py
cargo +1.99.0 fmt --all -- --check
cargo +1.99.0 test --locked -p sorter-core --test unicode_tables
```

The generator refuses a different Python minor version or Unicode database.
It never changes the immutable release/alias corpus or its expected values.
The product does not require Python.


## Read-only logical planning and safety seam

`plan(&PlanRequest) -> Result<Plan, EngineError>` builds one ordered PlannedSong
for each supplied single-song record. It invokes the existing classifier,
retains the full raw decision artist list, and chooses its first destination
artist unless duet_mode is enabled. Duet destinations retain order and
duplicates. All successful plans require provenance=supplied_facts and
executable=false, including plans without blockers. No source or target has
been opened, inspected, or physically verified.

The four single-song layouts preserve verbatim artist components and an
optional first Unicode codepoint/alphabetic component, followed by the literal
סינגלים when enabled. The output name sanitizes the original filename, never
the cleaned recognition name. Supplied existing files and directories both
occupy candidate filenames. Logical reservations are ordered across songs and
duet destinations, using `_1`, `_2`, ... before the Python 3.12 final suffix.
No additional truncation or filename cleanup is introduced after suffixing.

exist_only checks the exact complete directory component array, including
סינגלים, in existing_directories. Its missing-directory intents remain
skipped_existing_only and do not reserve. Recognition apply blockers are
path-dependent, retained from classify, and mark allowed safe intents blocked.
Safe inference-blocked intents still receive collision suffixing/reservations
in the complete logical projected sequence, preserving legacy target evidence.
Unsafe intents never reserve. A later executor must not drop blocked operations
and silently reinterpret the remaining targets; selecting a subset requires a
fresh plan. Disabled optional recognition is not itself an apply blocker.

Obvious `..` or absolute-root escapes retain raw intended components and artists,
with blocked disposition and UNSAFE_DESTINATION diagnostics. Relative nested
slash/backslash names, drive-relative names, provider URI shapes, empty/dot/NUL
components, and malformed snapshot component arrays return
UNSUPPORTED_TARGET_SHAPE rather than substituting a destination. This is a
Phase 1 capability boundary, not a new artist sanitation rule. root_id and
source_id remain opaque supplied identities, including path/URI-like strings.
Later Windows/SAF adapters must resolve supported safe nested target semantics
and enforce physical boundaries, aliases, permissions, case collisions,
reserved names, symlinks and provider identity. Logical comparisons use exact
supplied strings and do not infer any of those platform properties.

`request_hash` is lowercase hexadecimal SHA256 over compact serde_json bytes
of the complete schema-versioned PlanRequest DTO, using declared struct field
order, BTreeMap key order and original array order. All songs, metadata/error
facts, aliases, six settings, recognition bundle IDs/health/facts, target root
and ordered snapshot observations are bound. `plan_id` hashes the complete
canonical Plan DTO content with exactly its own plan_id field omitted. The
private serialization struct retains the remaining Plan field order. Song and
destination IDs additionally bind schema, request hash and sequence indexes.

`verify_plan_binding(&Plan, &PlanRequest)` regenerates the complete expected
plan and compares all content, including identities, provenance, diagnostics,
decisions, ordered intents and executable=false. A stored matching request_hash
is insufficient. Altered content or a request that cannot regenerate the plan
returns STALE_PLAN; the verifier does not silently replan. None of these IDs or
successful checks grant authorization or establish actual file/model identity.

`source_delete_allowed(copy_mode, required_destinations, verified_destinations,
failed_or_cancelled)` returns true only for a move, a nonzero required count,
exact complete verified count, and no failure/cancellation. It is a pure
mechanical guard over supplied counts, not deletion or physical verification.

Tests use disposable source/target sentinel trees and compare before/after
manifests containing every relative directory, file byte and SHA256. Both copy
and move logical planning, binding verification and tamper rejection leave
those trees byte-for-byte unchanged. An API type assertion and compile-fail
doctest prove the planner accepts only DTO facts, with no file-opening adapter.
Actual apply, albums, CSV parsing, traversal, tag decoding and model inference
remain unsupported; Linux tests are not Windows or Android acceptance.

## Journal and read-only replay evidence

`JournalBinding::from_plan(&Plan)` captures the inspected plan ID, ordered
operation IDs, immutable source IDs, plan-derived copy/move mode and ordered
Required destination IDs. Blocked and SkippedExistingOnly intents are never
verification successes. Duplicate operation/destination IDs and empty identities
are rejected. Conversion is not authentication: verify a stored plan with
`verify_plan_binding` against its original request when that check is needed.
Every Phase 1 plan remains non-executable.

`Journal::open(&Path, &JournalBinding)` creates only the explicitly selected
journal, or validates its complete existing JSONL evidence. The first record
is a strict version-1 header containing the entire immutable binding. Reopen
and `replay` compare that full typed binding, including source/mode/destination
association and array order. Subsequent strict `JournalEvent` records contain
sequence, plan_id, operation_id, destination_id and a snake_case state.
There are no event-supplied verification counts.

Sequences are global, contiguous, and start at one. Each operation requires
Prepared first. DestinationVerified requires a listed Required ID, counted
exactly once for its bound operation. Committed requires every Required ID and
a nonempty set. SourceDeleted requires Committed, move mode and the existing
source-deletion guard. Failed/Cancelled may follow Prepared, verification or
Committed, and are terminal, as is SourceDeleted. A repeated prepare/commit,
unknown/wrong ID, cross-operation verification or invalid transition returns
INVALID_JOURNAL_TRANSITION before writing. These states are recorded evidence;
they never authorize or perform a physical transfer or deletion.

Every accepted append uses one newline-terminated JSON record, flush and
sync_all before its in-memory state advances. The header is also synced. A
write/sync error poisons that open handle without truncation or retrying an
uncertain record. Cooperative standard-library file locks exclude a second
writer and read-only replay while the writer handle is alive; drop releases
the lock. Out-of-protocol length changes also poison the writer. Locks are not
a tamper-proof boundary against unrelated programs or hostile filesystems.

`replay(&Path, &JournalBinding)` returns `ReplayState { events, incomplete_tail }`
under a read-only shared lock. It validates every complete earlier record and
never synthesizes recovery/copy/delete instructions. Malformed complete JSONL
records or incomplete/missing headers return JOURNAL_CORRUPT. A final event
without a newline, even otherwise valid JSON, sets incomplete_tail=true and is
never promoted. Its bytes stay intact; reopen blocks all further append until
a separately designed recovery decision. No repair/truncate API is provided.

Only disposable test journals have been exercised. Linux flush/sync/lock tests
do not establish Windows crash durability, durable parent-directory creation,
storage-controller guarantees or Android SAF atomicity. Actual media I/O,
apply, discovery, tag decoding and model inference remain unsupported.

## Agent-friendly JSON CLI

The supplied-facts v1 commands remain:

```sh
singles-sorter classify --json < classify-request.json
singles-sorter plan --json < plan-request.json
singles-sorter capabilities --json
```

`classify` consumes one `ClassifyRequest`; `plan` consumes one `PlanRequest`,
with an ordered list of supplied single-song records and a logical target
snapshot. Neither command opens its source/target IDs, audio, tags, CSV, or
model bundle. `capabilities` does not read stdin. Its original fields describe
v1; the additional `real_file_planning` section describes v2. The executable
works from an unrelated current directory. It embeds the frozen bundled CSV
and metadata coverage, and never searches CWD or legacy preferences for them.

Requests and machine output are UTF-8. Stdin must contain exactly one JSON
value, followed only by optional whitespace; NDJSON is not supported. Strict
DTO decoding rejects unknown fields, missing required fields, duplicate DTO
fields, unknown enum tags, invalid types, and non-UTF-8. Recognition facts are
supplied observations, not instructions to run a model. A configured missing
fact remains `MISSING_INFERENCE_FACT`, rather than an invented failed model
observation or a fallback to disabled recognition.

The complete classify/plan stdin request, including whitespace, is limited to
16 MiB (16,777,216 bytes). Exactly that size is accepted when otherwise valid.
The reader checks one extra byte and rejects a larger input with
`INPUT_TOO_LARGE` before any prefix is parsed or processed; it does not drain
unbounded input. This request limit is not an output-size guarantee.

Every v1 machine response has exactly four envelope fields:

```json
{
  "schema_version": 1,
  "status": "ok",
  "result": {
    "command": "classify",
    "inference_executed": false,
    "executable": false,
    "decision": {
      "provenance": "supplied_facts",
      "artists": null,
      "selected_by": null,
      "trace": [],
      "warnings": [],
      "apply_blockers": []
    }
  },
  "error": null
}
```

The decision shown above illustrates the wire shape, not an executed fixture.
Successful results contain exactly one of `decision`, `plan`, or
`capabilities`, besides `command` and the two safety facts. A plan retains its
own `executable=false` and `provenance=supplied_facts` fields and original
request/plan identities. Successful supplied-failure decisions preserve their
ordered warnings and apply blockers; they still have status `ok` because a
decision was produced, never because physical execution became safe.

Failures use `status="error"`, `error={code,message}`, and a result containing
only `command`, `inference_executed=false`, and `executable=false`. They do not
return a partial decision or plan. Schema version 1 describes the response
even when the input requests an unsupported schema. Output is one compact
JSON envelope plus a newline, with no colors, prompts, banners, or logs on
stdout. A human-readable copy of an error goes to stderr. Messages are
explanatory; automation should inspect the stable error code and exit code.

Phase 1 exit codes:

- `0`: valid successful decision, logical plan, capabilities, or help.
- `1`: processing/internal failure. A caller-supplied NER failure uses
  `INFERENCE_ERROR`; an actual stdin/serialization/stdout transport error uses
  `INTERNAL_ERROR`. Code 1 alone does not mean an internal software bug.
- `2`: invalid request or CLI arguments. Includes `INVALID_INPUT`,
  `UNSUPPORTED_SCHEMA`, `UNSUPPORTED_RULESET`, `MISSING_INFERENCE_FACT`, and
  `INPUT_TOO_LARGE`.
- `3`: unsupported operation/capability, including `UNSUPPORTED_CAPABILITY`
  and the core's `UNSUPPORTED_TARGET_SHAPE` boundary.

If stdout itself fails, delivery of a complete JSON envelope cannot be
guaranteed; the process exits 1 and reports the transport error on stderr.
No-argument invocation, `--help`, `-h`, or a supported command followed only
by `--help`/`-h` shows human-readable help without reading stdin. A supported
JSON command requires exactly one `--json`: unknown, repeated or missing
arguments return `INVALID_INPUT`. An unsupported command returns
`UNSUPPORTED_CAPABILITY`; `apply` always returns exit 3 without decoding its
stdin or changing source/target files. The CLI never exposes the journal
writer or any apply executor.

Standalone `albums`, `scan`, `parse-csv`, `decode-tags`, `infer`, and physical
apply commands remain unsupported. Traversal, CSV parsing, metadata decoding
and album/fix policy are available only through the read-only v2 `plan-files`
command below. Unsupported commands cannot fall back to single-song handling. An album-shaped
object supplied to a classify/plan DTO is rejected as an invalid request.
`--ndjson` and the recognized standalone legacy flags are explicitly upcoming
capabilities; legacy positional/flag execution, status/cancel/resume, UI and
physical execution are not implemented. Combining unsupported flags with
extra unknown/repeated arguments remains invalid CLI syntax.

### Runnable request fixtures

From the repository root after the release build:

```sh
native/target/release/singles-sorter classify --json \
  < native/tests/fixtures/protocol/classify-alias.json
native/target/release/singles-sorter classify --json \
  < native/tests/fixtures/protocol/classify-fail-open.json
native/target/release/singles-sorter plan --json \
  < native/tests/fixtures/protocol/plan-collisions.json
native/target/release/singles-sorter capabilities --json
```

- `classify-alias.json`: Hebrew filenames and ordered aliases. The first
  alias selects `אמן ראשון`; supplied metadata does not replace the winner.
- `classify-fail-open.json`: an explicitly supplied unavailable classifier
  with failed bundle health. Legacy acceptance selects `אמן חדש`, preserving
  `CLASSIFIER_FAIL_OPEN` evidence and the recognition apply blocker. No model
  is loaded or run.
- `plan-collisions.json`: two single-song records and one supplied existing
  file produce ordered `_1` and `_2` collision intents. No existing file or
  directory is physically checked, created, copied, or deleted.

These small request examples were authored for the protocol tests. They are
separate from the immutable legacy oracle fixtures and never regenerate those
fixtures' expected values. `crates/sorter-cli/tests/protocol.rs` runs real
subprocesses, including malformed/oversized requests, explicit and missing
inference facts, unsupported operations, transport errors, unrelated CWD, and
disposable source/target manifests comparing all paths, bytes, and SHA256s.
Linux test/release-build success is evidence for this slice only; it does not
prove Windows runtime/packaging, Android product support, native NER, or the
complete dry-run application.

## Real-file v2 preview CLI

`singles-sorter plan-files --json` reads one strict UTF-8 JSON object, up to the
same 16 MiB stdin bound. It calls the actual read-only storage and metadata
adapters and virtual job planner. It never writes selected source/target trees,
preferences, tags, or legacy data. OS-managed access times can change.

```json
{
  "schema_version": 2,
  "mode": "Sort",
  "source": "/absolute/owned-fixture/source",
  "target": "/absolute/owned-fixture/target",
  "settings": {
    "copy_mode": false,
    "main_folder_only": false,
    "singles_folder": true,
    "exist_only": false,
    "abc_sort": false,
    "duet_mode": false
  },
  "ai_profile": "Disabled",
  "personal_aliases": null
}
```

These are illustrative paths, not a request to test a user's music library.
On Windows use absolute Windows paths with JSON-escaped backslashes. The CLI
input is a UTF-8 path subset; observations retain lossless adapter references
(UTF-16 on Windows, explicit test-only opaque IDs on Linux). It does not accept
Android URIs or infer paths from provider IDs. Relative paths and parent-dot
components are rejected rather than resolved from CWD.

`mode` is `Sort` or `FixNames`. Sort requires an existing target directory;
FixNames accepts omitted/null target. `settings` is required; `{}` selects the
six released defaults. `ai_profile` must be explicit. `Disabled` intentionally
runs without AI. `{"Configured":{"bundle_id":"my-bundle"}}` fails with exit 3,
`RECOGNITION_UNAVAILABLE`, and a structured blocker; it cannot silently become
Disabled. Caller-supplied `available` flags and inference facts are rejected.
No native inference/model loading is implemented.

`personal_aliases` is an optional absolute path to a read-only UTF-8 CSV,
bounded to 16 MiB. The binary embeds the exact frozen 1,196-row bundled CSV;
personal rows follow it without trimming, BOM removal or reordering. Alias reads
use the same identity-aware storage adapter and refuse symlink/reparse traversal. Missing,
unreadable and invalid-codec resources fail explicitly. Malformed row widths
are retained with structured source/index/record/width diagnostics and an apply
blocker, preserving the core's lazy legacy lookup boundary.

The v2 envelope retains `schema_version`, `status`, `result`, `error`. `result`
contains `command`, `inference_executed=false`, `executable=false`, `plan`,
`resources`, `capabilities`, and `blockers`. Success has `status="ok"`,
`error=null`; zero exit means a preview was produced, never apply eligibility.
Failures have null `plan`/`resources`, a stable error record, explicit blocker
and capability/coverage report. Recognized plan-files syntax failures use the
v2 envelope too. Exit codes are 0 success, 1 storage/resource-read/processing
failure, 2 invalid input/resource/schema/root shape, 3 unavailable capability.

Metadata reads require identity, size and modification evidence from the actual
opened read handle to match the supplied observation before parsing. Changed or
unverified bindings fail closed, without substituting metadata from another
object. This is not proof against every in-place content mutation.

Plans bind settings, roots, original object observations, logical metadata read
points, exact resource hashes, metadata coverage hash, adapter versions and the
explicit disabled-model identity. Their `binding_sha256` detects alteration of
the observed plan; it does not prove unchanged audio content. No whole-audio
content digest or execution permission is claimed. Effects and legacy counters
are predictions. Raw unsafe intentions and metadata failures remain visible.

Capabilities include the complete frozen metadata fixture-class coverage,
bridge build availability, and these remaining gates: Linux is a test backend;
actual Windows runtime validation, Android provider integration, native
inference and physical apply are pending. Metadata parity is scoped to the
owned synthetic corpus, not arbitrary files or shipping format acceptance.
The embedded coverage's `enable_native_compatibility=false` is intentional.

The owned synthetic audio is documented in
[media provenance](tests/fixtures/media/README.md). CLI integration tests copy
that committed audio into disposable trees, run from an unrelated directory,
and compare names, bytes, sizes and modification times for source, target and
preference sentinels. See the [preview acceptance boundary](docs/phase2-preview-status.md).

### Phase 2 read-only adapter capability boundary

`sorter-io` exposes read-only observations independently of the unchanged v1
CLI. Windows native file inputs are bounded and seekable. **Windows streaming
scratch fallback is unsupported**: `ScratchSpace::spool` returns typed
`UnsupportedCapability` before any input read, budget charge or file creation.
Selected-root identity and no-share-delete handles alone do not prevent
in-place junction changes, so path-based temporary-file creation is disabled
on Windows pending a validated handle-relative/no-reparse implementation.
This restriction does not disable ordinary Windows file reads.

`streaming_spool_capability()` explicitly reports `Unsupported` on Windows and
other unimplemented platforms, and `LinuxTestOnly` on Linux. The Linux spooler
is a bounded disposable-fixture test backend, not a production concurrent-
mutation sandbox. Android provider input remains unimplemented. No physical
apply capability is enabled. Native Windows runtime acceptance requires an
actual Windows test run; cross-target checking is insufficient.

Windows storage traversal now opens only the drive/share anchor by absolute
name. All descendant opens use one validated UTF-16 component relative to an
owned parent handle via `NtCreateFile`, with `OBJ_DONT_REPARSE` and
`FILE_OPEN_REPARSE_POINT`; reparse errors never trigger a permissive retry.
Read/list access is reacquired against the same parent and checked against the
original full object ID. Directory enumeration uses the directory handle and
bounded `FILE_ID_EXTD_DIR_INFO` records, retaining order and verifying opened
children against their enumerated 128-bit IDs. Static reparse entries remain
visible as blocked listing observations. Native Windows adversarial junction
tests are required before treating this implementation as runtime-validated.
