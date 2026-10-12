# Product acceptance matrix

Evidence categories are intentionally separate: released-policy parity,
read-only real-file behavior, physical execution, and actual product interaction.
A green test in one column does not establish the others. New candidate checks
below require final-head CI results; the presence of a test is not a passing run.

| Released behavior / flow | Planning and compatibility | Physical/product boundary |
| --- | --- | --- |
| Copy / Move toggle | Exact boolean and v14 virtual effects retained | Delivered selected Windows multi-file Copy / same-volume rename; cross-volume Move blocked |
| Copy file attributes/timestamps | v14 uses `copy2` / `copytree` | New native Copy metadata contract and album host candidate preserve timestamps/readonly attributes; final-head native and staged/extracted metadata checks remain required |
| Main folder only / full tree | Original traversal/order retained | Selected plans validated whole; actual multi-file Windows gate required |
| Create singles folders | Original `סינגלים` destination component retained | Journaled directory creation candidate; native UI test asserts physical layout |
| Alphabetic sort | Original letter hierarchy retained | Backend directories supported; requires expanded native UI acceptance matrix |
| Existing folders only | Original skip/target policy retained | No fabricated missing artist destinations; explicit feature-specific runtime coverage still required |
| Duets, first artist | Original matching order retained | Ordinary single destination candidate supported |
| Duets, every artist | Original ordered destinations and deletion intention retained | Copy supported where backend preflight accepts; Move retirement cohort not enabled in first candidate |
| Original alias list + personal CSV | Ordered rows, duplicates, malformed-row diagnostics preserved | UI edit/import/delete/save/export and persistence baseline covered; no silent normalization |
| Original NER + secondary classifier | Both exact original assets validated; no fallback | CLI and Windows UI original-AI real-file preview baseline exists; new apply + AI composition requires final Windows gate |
| Disabled AI | Explicit choice; no inference | Never silently substituted for configured unavailable models |
| Albums | Original album recognition, metadata-error ordering and virtual transfer timeline | Separate candidate composes bounded original album plans, nested files/sidecars, root-album and mixed same-volume Move; actual new host/CLI/package gates pending |
| Fix names / gibberish tags | Original planned rename/tag intentions retained | New source-only repair candidate: original filename policy across 12 extensions; bounded MP3 ID3v2.3/v2.4 five-field writes, retained original backup, explicit partial unsupported tag outcomes. MP3 physical repair, exact retained originals and media preservation passed actual Windows CLI and staged/extracted UI at 690194e; later format additions remain separate |
| Name collisions | Original reservation policy; exact plan revalidated | Identity-bound no-replace publication; changed destination stops execution |
| Preview cancellation / Back / reload | Host owner remains busy until native terminal state | Existing lifecycle tests; no stale plan resurrection |
| Apply review / repeat click | Host-selected paths and retained digest only | New UI/CLI candidate; cannot repeat consumed plan |
| Apply progress / cancellation | Actual bytes; predicted rows stay predictions | Native result distinguishes committed, blocked, cancelled, partial and recovery |
| Recovery | Hash-chained journal and identity-bound reconciliation | Read-only retained-job inspection; process-restart product UI/import not implemented |
| Hebrew RTL compact layout / import editing | Shared UI and original labels preserved | Latest [result/viewport candidate](ui-results-layout.md): action-only default, optional inactive rows, developer settings, bounded width and stationary footer. Portable tests pass; final Windows native receipts and Android device evidence still required |
| CLI supplied-facts v1 | Existing envelopes remain model-free and non-executable | No physical authorization granted |
| CLI real-file v2 JSON | Same request fields and original planning policy | Read-only; optional selected planning adapter strengthens capability observations |
| CLI NDJSON execution | One-process inspected-digest handshake | Same selected Windows backend; machine protocol remains explicit |
| CLI positional source/target and flags | Frozen v14 declarations/probes pinned, including all eight flags and default Move | Separate frontend candidate reuses retained planner/executor without an interactive JSON prompt; all five original compatibility subprocess cases passed at b293/690; selected-feature standalone package activation awaits measured notice closure and actual extracted checks |
| Android | Shared UI remains fail-closed without native apply eligibility | Separate Android workstream; Windows proof is not Android/provider/ARM64 proof |
| Portable Windows artifact | Explicit original-AI + selected-execution feature profile | EXE-bound real Copy/Move/artist-folder plus source-only MP3 repair checks required on staged and extracted ZIP; package README and provenance identify supported mutations |

## Current verification record

The delivered d917 baseline passed its exact Windows native safety, CLI, and
staged/extracted original-AI Copy/Move gates. Its ZIP SHA-256 is
`bf90d51cc5e98bcd16102f898c497cc9083297bfbbf88d742dba543b4d0b02f5`;
that evidence does not cover later UI polish or repair changes.

The 690194e Windows checkpoint passed the existing 13 native repair product,
4 journal-boundary, 20 preservation, 10 staged MP3 transformer, 6 NDJSON CLI
and 5 positional CLI cases. Its product run also passed real staged/extracted
MP3 repair. The native job's overall formatter failure was recorded separately;
physical/runtime passes are not described as an all-green final build.

The later inactive-filter/footer UI and album activation require new final-head
results. Local frontend/unit checks and portable fixture tests are useful source
evidence, not native or visual acceptance. Local Rust installation and Chromium
rendering remain blocked; actual Windows CI supplies those checks.


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
| Filename cleanup | Original underscore/hyphen/substrings/whitespace and Windows-character sanitization, collision suffix order | Positional source-only MP3 Fix passed the actual Windows subprocess gate; standalone consumer package checks remain required |
| Album handling | Single-keyword exclusions, track/album/artist inspection order, protected per-file errors, nested and mixed traversal timing | Album host/CLI candidate now composes the separately reviewed driver; new actual product gates must pass before package acceptance |
| Duet effects | Original artist order and duplication; first/all-artist toggle | Multi-destination Move source-retirement cohort remains separate; no early source deletion |
| Failures and cancellation | Planning metadata failures are not silently converted into success; native actual effects remain explicit | Read-only repair recovery now passes both ordinary and short Windows path spellings; process-kill/restart UI remains separate |
| File lists and settings | Bundled/personal ordering, duplicate rows, CSV editing/import/export and every six-boolean setting | Expanded actual UI combination tests for alphabetic/existing-only/main-only/duet settings remain required |
| User interface | Hebrew RTL, compact display, immediate help dismissal, richer About, default-minimal results and explicit developer mode | Latest actual UI gate checks advanced-Settings developer placement, independent inactive filtering, long-name bounds and stable footer; fresh Windows evidence is required |

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

The initial f86 run found a real read-only recovery failure after physical repair:
short Windows ancestor spelling differed from the held object's long spelling.
The identity-bound correction subsequently passed actual CLI reconciliation at
9776 and 690. The diagnostic assertion was preserved; neither the earlier
physical effect nor that earlier failed reconciliation was relabeled.

## Observed Windows repair checkpoint and next album scope

The 690194e product run (38190107795) passed feature-off, staged and extracted
Windows UI lanes, including both MP3 tag versions, partial unsupported tags,
stale-plan rejection and exact retained backup/audio bytes. Its internal ZIP
SHA-256 is `4aedd96d9516bbb19ccd5220719a4684e1859d08c567303f4d6c6f9b8164ea03`.
This is not evidence for the subsequently requested inactive/filter/footer UI,
which has its own fresh run, nor for the next album candidate.

Album host activation adds a real metadata root-album Copy/Move host case,
a positional Copy/root-Move/mixed-Move subprocess case and mandatory staged and
extracted UI fixtures. Whole-directory receipts count recursive physical files
separately from the original album/artist counters. The confirmation states the
no-concurrent-writer condition. Bounds and unsupported plans stay explicit;
no automatic replay, journal deletion or guessed cleanup is introduced.
