# Product acceptance matrix

Evidence categories are intentionally separate: released-policy parity,
read-only real-file behavior, physical execution, and actual product interaction.
A green test in one column does not establish the others. New candidate checks
below require final-head CI results; the presence of a test is not a passing run.

| Released behavior / flow | Planning and compatibility | Physical/product boundary |
| --- | --- | --- |
| Copy / Move toggle | Exact boolean and v14 virtual effects retained | Delivered selected Windows multi-file Copy / same-volume rename; cross-volume Move blocked |
| Copy file attributes/timestamps | v14 uses `copy2` / `copytree` | Current transfer proof covers content and names; full copied metadata preservation remains an explicit parity gap |
| Main folder only / full tree | Original traversal/order retained | Selected plans validated whole; actual multi-file Windows gate required |
| Create singles folders | Original `סינגלים` destination component retained | Journaled directory creation candidate; native UI test asserts physical layout |
| Alphabetic sort | Original letter hierarchy retained | Backend directories supported; requires expanded native UI acceptance matrix |
| Existing folders only | Original skip/target policy retained | No fabricated missing artist destinations; explicit feature-specific runtime coverage still required |
| Duets, first artist | Original matching order retained | Ordinary single destination candidate supported |
| Duets, every artist | Original ordered destinations and deletion intention retained | Copy supported where backend preflight accepts; Move retirement cohort not enabled in first candidate |
| Original alias list + personal CSV | Ordered rows, duplicates, malformed-row diagnostics preserved | UI edit/import/delete/save/export and persistence baseline covered; no silent normalization |
| Original NER + secondary classifier | Both exact original assets validated; no fallback | CLI and Windows UI original-AI real-file preview baseline exists; new apply + AI composition requires final Windows gate |
| Disabled AI | Explicit choice; no inference | Never silently substituted for configured unavailable models |
| Albums | Original album recognition, metadata-error ordering and virtual transfer timeline | Whole album physical execution unavailable in first selected-root candidate |
| Fix names / gibberish tags | Original planned rename/tag intentions retained | New source-only repair candidate: original filename policy across 12 extensions; bounded MP3 ID3v2.3/v2.4 five-field writes, retained original backup, explicit partial unsupported tag outcomes. Final Windows host/CLI gates pending |
| Name collisions | Original reservation policy; exact plan revalidated | Identity-bound no-replace publication; changed destination stops execution |
| Preview cancellation / Back / reload | Host owner remains busy until native terminal state | Existing lifecycle tests; no stale plan resurrection |
| Apply review / repeat click | Host-selected paths and retained digest only | New UI/CLI candidate; cannot repeat consumed plan |
| Apply progress / cancellation | Actual bytes; predicted rows stay predictions | Native result distinguishes committed, blocked, cancelled, partial and recovery |
| Recovery | Hash-chained journal and identity-bound reconciliation | Read-only retained-job inspection; process-restart product UI/import not implemented |
| Hebrew RTL compact layout / import editing | Shared UI and original labels preserved | Latest [result/viewport candidate](ui-results-layout.md): action-only default, optional inactive rows, developer settings, bounded width and stationary footer. Portable tests pass; final Windows native receipts and Android device evidence still required |
| CLI supplied-facts v1 | Existing envelopes remain model-free and non-executable | No physical authorization granted |
| CLI real-file v2 JSON | Same request fields and original planning policy | Read-only; optional selected planning adapter strengthens capability observations |
| CLI NDJSON execution | One-process inspected-digest handshake | Same selected Windows backend; machine protocol remains explicit |
| CLI positional source/target and flags | Frozen v14 declarations/probes pinned, including all eight flags and default Move | Separate frontend candidate reuses retained planner/executor without an interactive JSON prompt; final native and extracted CLI package checks pending |
| Android | Shared UI remains fail-closed without native apply eligibility | Separate Android workstream; Windows proof is not Android/provider/ARM64 proof |
| Portable Windows artifact | Explicit original-AI + selected-execution feature profile | EXE-bound real Copy/Move/artist-folder plus source-only MP3 repair checks required on staged and extracted ZIP; package README and provenance identify supported mutations |

## Current verification record

The delivered d917 baseline passed its exact Windows native safety, CLI, and
staged/extracted original-AI Copy/Move gates. Its ZIP SHA-256 is
`bf90d51cc5e98bcd16102f898c497cc9083297bfbbf88d742dba543b4d0b02f5`;
that evidence does not cover later UI polish or repair changes.

The current repair candidate adds 12 backend product cases, 3 journal boundary
cases, 20 native preservation cases, 10 staged tag transformer cases, real
source-only CLI subprocess cases,
and actual staged/extracted UI cases. These are required checks, not claimed
passes until the final-head Windows runs finish. Local frontend checks currently
pass 107 tests/typecheck/build; independent Python ID3 fixture/parser and package
gate tests also pass. Local Rust installation and Chromium rendering remain
blocked; neither limitation is counted as a passing native or visual test.


## Frozen v14 completeness checklist

The reference is commit `78d001aee5ce49628bdffebd34540fa83ba32c12`, version
14.0. Original defaults are Move, recursive traversal, inner `סינגלים` folder,
first artist only, alphabetic sorting off and existing-only off. The complete CLI
flag inventory is `-c/-a/-e/-n/-m/-d/-f/-l`, with the exact underscore long names
recorded in [the CLI compatibility contract](legacy-cli.md). A frontend flag being
parsed does not imply that every resulting operation is physically supported.

| Decision/effect family | Preserved reference behavior | Outstanding physical or product proof |
| --- | --- | --- |
| Classification | Ordered filename aliases, metadata artist/title routes, Hebrew repairs, exact-name check, original NER/classifier fallback | Broader arbitrary-library representations; platform-specific inference acceptance remains separately recorded |
| Filename cleanup | Original underscore/hyphen/substrings/whitespace and Windows-character sanitization, collision suffix order | Actual standalone positional Fix acceptance remains required |
| Album handling | Single-keyword exclusions, track/album/artist inspection order, protected per-file errors, nested and mixed traversal timing | Album Copy/Move directory effects and cleanup are not enabled by the singles-only backend |
| Duet effects | Original artist order and duplication; first/all-artist toggle | Multi-destination Move source-retirement cohort remains separate; no early source deletion |
| Failures and cancellation | Planning metadata failures are not silently converted into success; native actual effects remain explicit | Read-only repair recovery must reconcile all valid selected-path spellings; process-kill/restart UI remains separate |
| File lists and settings | Bundled/personal ordering, duplicate rows, CSV editing/import/export and every six-boolean setting | Expanded actual UI combination tests for alphabetic/existing-only/main-only/duet settings remain required |
| User interface | Hebrew RTL, compact display, immediate help dismissal, richer About, default-minimal results and explicit developer mode | Actual Windows polish must pass the corrected closed/open/reclosed About layout gate |

### All twelve released extensions

Discovery and filename repair retain all twelve suffixes. The read bridge's
frozen-fixture report is not a blanket arbitrary-file or platform compatibility
claim. For metadata open failures, original rename-before-metadata ordering is
preserved; a filename change does not imply a successful tag repair.

| Extension | Frozen reference metadata-open fixture | Current physical tag repair boundary |
| --- | --- | --- |
| `.mp3` | Supported ID3 fixture classes | Bounded unflagged ID3v2.3/v2.4 with existing padding; five original fields; original backup retained. Unsupported flags/representations/insufficient padding stop tag publication explicitly |
| `.m4a`, `.alac` | MP4 fixtures | Tag writes unavailable; filename repair can still complete |
| `.wav` | WAVE ID3 fixtures | Tag writes unavailable; filename repair can still complete |
| `.aiff` | AIFF fixtures | Tag writes unavailable; filename repair can still complete |
| `.flac` | FLAC fixtures | Staged writer under separate development; not enabled by the MP3 candidate |
| `.wv` | WavPack fixtures | Tag writes unavailable; filename repair can still complete |
| `.ogg` | Ogg Vorbis fixtures | Tag writes unavailable; filename repair can still complete |
| `.dsf` | DSF fixtures | Tag writes unavailable; filename repair can still complete |
| `.opus` | Ogg Opus fixtures | Tag writes unavailable; filename repair can still complete |
| `.wma` | Released metadata open rejects ASF | Preserve metadata-error outcome after any supported filename repair |
| `.aac` | Released ADTS fixture rejects tags | Preserve metadata-error outcome after any supported filename repair |

The initial f86 Windows run passed all 10 staged MP3 writer, 20 preservation,
3 journal-boundary and 12 native repair product cases. The CLI's actual repair
passed renamed/repaired counts, unchanged audio and exact retained-original bytes,
but its read-only recovery report was absent, so that CLI test failed. The strict
assertion and error diagnostics remain; this is not complete CLI acceptance.
The current follow-up must pass fresh native, host and staged/extracted gates.
