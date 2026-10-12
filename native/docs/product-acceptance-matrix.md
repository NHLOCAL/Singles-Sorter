# Product acceptance matrix

Evidence categories are intentionally separate: released-policy parity,
read-only real-file behavior, physical execution, and actual product interaction.
A green test in one column does not establish the others. New candidate checks
below require final-head CI results; the presence of a test is not a passing run.
The latest actual Windows sorting-settings checkpoint is the 66-case / 64-vector
26e result, repeated at 2ec. The 2ec CLI consumer checkpoint below also passes
58 real staged/extracted subprocess cases. These supersede the older 041 gaps
only within their stated bounds; desktop and unrelated product acceptance remain
separate.

| Released behavior / flow | Planning and compatibility | Physical/product boundary |
| --- | --- | --- |
| Copy / Move toggle | Exact boolean and v14 virtual effects retained | Delivered selected Windows multi-file Copy / same-volume rename; new copied-Move host candidate binds ordinary NTFS main and named streams at preview for cross-volume singles and exact nonempty eligible duet cohorts. The one-eligible/zero-eligible follow-up passes the actual 2ec staged/extracted CLI cases; desktop consumer acceptance remains pending |
| Copy file attributes/timestamps | v14 uses `copy2` / `copytree` | Native Copy metadata preserves timestamps/readonly attributes; actual staged and extracted album-Copy fixtures passed at the desktop checkpoint below |
| Main folder only / full tree | Original traversal/order retained | All 64 six-boolean vectors have bounded actual original/native Windows comparison at 26e; broader UI/library coverage remains separate |
| Create singles folders | Original `סינגלים` destination component retained | Journaled directory creation; actual staged/extracted singles tests assert artist and `סינגלים` layout |
| Alphabetic sort | Original letter hierarchy retained | Bounded paired Windows matrix includes both states at 26e; full native UI combination coverage remains separate |
| Existing folders only | Original skip/target policy retained | Bounded paired Windows matrix includes both states and no-transfer routes at 26e; source-retaining all-skipped no-op is separately tested, not covered by these 66 cases |
| Duets, first artist | Original matching order retained | Ordinary single destination candidate supported |
| Duets, every artist | Original ordered destinations and deletion intention retained | Copy supported where backend preflight accepts; new Move candidate verifies every destination before retiring the source once. Actual 2ec CLI consumers verify one eligible original duet destination and zero eligible destinations retaining the source as an explicitly revalidated no-op. Failed required publication still retains the source. Ordinary same-volume singles in mixed jobs retain rename identity |
| Original alias list + personal CSV | Ordered rows, duplicates, malformed-row diagnostics preserved | UI edit/import/delete/save/export and persistence baseline covered; no silent normalization |
| Original NER + secondary classifier | Both exact original assets validated; no fallback | Actual staged/extracted Windows UI runs both original assets and original-AI selected Copy; unavailable/corrupt assets still fail closed |
| Disabled AI | Explicit choice; no inference | Never silently substituted for configured unavailable models |
| Albums | Original album recognition, metadata-error ordering and virtual transfer timeline | Actual staged/extracted Windows UI passed bounded album Copy, root-album and mixed same-volume Move with nested files/sidecars; cross-volume albums pass the actual 2ec staged/extracted CLI cases; desktop acceptance remains pending |
| Fix names / gibberish tags | Original planned rename/tag intentions retained | Original filename policy across 12 extensions; bounded MP3 ID3v2.3/v2.4, native FLAC and single-stream Ogg Vorbis/Opus five-field writes. Actual staged/extracted Windows gates passed exact retained originals/media, partial unsupported representations and stale-plan refusal; bounded MP4, existing ID3 in WAVE/AIFF/DSF, and terminal WavPack APEv2 also pass actual 2ec CLI consumers; their desktop consumers remain pending |
| Name collisions | Original reservation policy; exact plan revalidated | Identity-bound no-replace publication; changed destination stops execution |
| Preview cancellation / Back / reload | Host owner remains busy until native terminal state | Existing lifecycle tests; no stale plan resurrection |
| Apply review / repeat click | Host-selected paths and retained digest only | New UI/CLI candidate; cannot repeat consumed plan |
| Apply progress / cancellation | Actual bytes; predicted rows stay predictions | Native result distinguishes committed, blocked, cancelled, partial and recovery |
| Recovery | Hash-chained journal and identity-bound reconciliation | Source-reviewed read-only restart for singles/duets, Fix and file-Album is integrated; final native/package acceptance remains pending. Whole-Album restart is a separate source-reviewed candidate |
| Hebrew RTL compact layout / import editing | Shared UI and original labels preserved | The earlier [result/viewport checkpoint](ui-results-layout.md) passed actual staged/extracted Windows checks for action-only defaults, independent inactive rows, advanced developer settings, long-name width and stationary footer. The later user redesign is preserved and requires its own current runtime acceptance; the observed About icon target was corrected after 26e. Android evidence remains platform-specific |
| CLI supplied-facts v1 | Existing envelopes remain model-free and non-executable | No physical authorization granted |
| CLI real-file v2 JSON | Same request fields and original planning policy | Read-only; optional selected planning adapter strengthens capability observations |
| CLI NDJSON execution | One-process inspected-digest handshake | Same selected Windows backend; machine protocol remains explicit |
| CLI positional source/target and flags | Frozen v14 declarations/probes pinned, including all eight flags and default Move | The retained planner/executor runs without an interactive JSON prompt. Actual 2ec release/staged/extracted CLI acceptance covers 58 cases with the selected feature/notice profile, including newer formats and cross-volume Albums; desktop and restart consumer gates remain separate |
| CLI log level and file diagnostics | Original five levels, local timestamp, UTF-8 CWD/logs filename and matching stderr/file filtering | Pinned/exclusive sink with pre-create media/journal overlap refusal and honest log failure reporting. Ten positional and eight sink cases passed actual Windows native tests at f181; logging levels/files also passed the actual 041 staged/extracted CLI gates. Explicit dry-run and machine protocols create no file log |
| Android | Shared UI remains fail-closed without native apply eligibility | Separate Android workstream; Windows proof is not Android/provider/ARM64 proof |
| Portable Windows artifact | Explicit original-AI + selected-execution feature profile | EXE-bound staged/extracted Windows checks passed original AI, singles Copy/Move, bounded MP3/FLAC/Ogg repair, same-volume albums and the requested UI polish. This checkpoint does not activate the later copied-Move or process-restart candidates |

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

The later inactive-filter/footer UI, album activation and bounded FLAC/Ogg repair
now have actual staged and extracted Windows proof in the desktop checkpoint
below. Local frontend/unit checks alone remain insufficient for native or visual
acceptance. New copied-Move and process-restart source must pass their own gates.


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
| Album handling | Single-keyword exclusions, track/album/artist inspection order, protected per-file errors, nested and mixed traversal timing | Actual staged/extracted Windows UI passed bounded album Copy, root Move and mixed Move; 26e later covers all 64 settings vectors in the authored paired sorting corpus. Cross-volume Album CLI consumers passed at 2ec; desktop consumers and broader UI combinations remain separate work |
| Duet effects | Original artist order and duplication; first/all-artist toggle | New copied-Move candidate composes complete multi-destination cohorts with the existing original-order runner; no early source deletion. A nonempty exact original eligible cohort is supported by the reviewed admission; zero eligible targets retain the source through a separately validated no-op. Cross-volume Album CLI consumer acceptance passed at 2ec; desktop consumer acceptance remains pending |
| Failures and cancellation | Planning metadata failures are not silently converted into success; native actual effects remain explicit | Read-only repair recovery now passes both ordinary and short Windows path spellings; process-kill/restart UI remains separate |
| File lists and settings | Bundled/personal ordering, duplicate rows, CSV editing/import/export and every six-boolean setting | The 26e paired engine corpus covers all 64 boolean vectors; expanded actual UI combinations and CSV edit/import/export/persistence retain separate product gates |
| User interface | Hebrew RTL, compact display, immediate help dismissal, richer About, default-minimal results and explicit developer mode | Actual staged/extracted Windows evidence covers advanced-Settings developer placement, independent inactive filtering, long-name bounds and stable footer; Android/device and broader settings combinations remain separate |

### All twelve released extensions

Discovery and filename repair retain all twelve suffixes. The read bridge's
frozen-fixture report is not a blanket arbitrary-file or platform compatibility
claim. For metadata open failures, original rename-before-metadata ordering is
preserved; a filename change does not imply a successful tag repair.

| Extension | Frozen reference metadata-open fixture | Current physical tag repair boundary |
| --- | --- | --- |
| `.mp3` | Supported ID3 fixture classes | Bounded unflagged ID3v2.3/v2.4 with existing padding; five original fields; original backup retained. Unsupported flags/representations/insufficient padding stop tag publication explicitly |
| `.m4a`, `.alac` | MP4 AAC/ALAC fixtures | Exact-extent UTF-8 five-field repair candidate; main media/chunk offsets/fast-start and opaque ordinary items retained, exact original backup. Duplicate outer text fields are combined in original order, including a later-only changed value; unsupported structures refuse tag writes. Actual 2ec staged/extracted CLI cases passed; desktop consumer acceptance pending |
| `.wav` | WAVE ID3 fixtures | Candidate: existing unflagged ID3v2.3/v2.4 in ordinary PCM/float RIFF; exact file/chunk/media extents and INFO bytes retained. Actual 2ec staged/extracted CLI cases passed; desktop consumer acceptance pending |
| `.aiff` | AIFF fixtures | Candidate: existing unflagged ID3v2.3/v2.4 in ordinary AIFF, exact COMM/SSND and unknown chunks retained; AIFC and insufficient padding refuse tags. Actual 2ec staged/extracted CLI cases passed; desktop consumer acceptance pending |
| `.flac` | FLAC fixtures | Bounded native FLAC Vorbis comments, including shifted audio after shorter metadata; pictures/vendor/unknown pairs/audio and original backup retained. Dedicated staged/extracted Windows cases passed; opaque-offset representations explicitly remain partial |
| `.wv` | WavPack fixtures | Candidate: existing terminal APEv2 UTF-8 five-field patches, original NUL scalar, exact block/tag layout and unknown binary/artwork retained; read-only/binary/external repaired items and ambiguous layouts refuse. Actual 2ec staged/extracted CLI cases passed; desktop consumer acceptance pending |
| `.ogg` | Ogg Vorbis fixtures | Bounded single-logical-stream Vorbis writes preserve encoded packets/timing/vendor/artwork/unknown pairs. Dedicated staged/extracted Windows cases passed; chained/multiplexed/other-codec layouts refused |
| `.dsf` | DSF fixtures | Candidate: existing terminal ID3v2.3/v2.4 in ordinary DSF, exact metadata pointer/header/media extents retained. Actual 2ec staged/extracted CLI cases passed; desktop consumer acceptance pending |
| `.opus` | Ogg Opus fixtures | Bounded ordinary single-logical-stream Opus writes; dedicated staged/extracted Windows byte, timing and retained-backup cases passed, with the same representation limits as Vorbis |
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

## Paired actual-Windows audit

The diagnostic lane compares the exact frozen v14 Python implementation against
fresh selected EXE executions in separate authored trees. It records 32 duet-disabled settings
combinations plus four original-model duet cases, raw paths/bytes, skip decisions,
operation order and original counters. It runs after mandatory package gates and
retains `OPEN` or `MISMATCH` rather than turning incomplete coverage into a pass.
The 36 authored cases cover 34 distinct six-boolean vectors: all 32 duet-disabled
vectors and two duet-enabled vectors. Thirty duet-enabled vectors remain
unpaired, especially partial eligible destinations and nested traversal. The
source helpers and Linux original-method fixture characterization are not
actual-Windows evidence; the first full Windows paired run remains pending.

The selected CLI graph has been measured and reviewed: 46 resolved nodes,
37 unchanged registry packages and nine local packages, including the local MIT
executor. The desktop selected graph remains the reviewed 267-node graph.
Neither completed graph review nor authored physical tests imply package runtime
acceptance. File logging is implemented; the paired reader
accepts its exact timestamp/level envelope without substituting predictions for
missing execution-derived counters.

## Copied-Move activation checkpoint

The new host/CLI candidate retains a data-only `CopiedMoveScope` during the
read-only preview, including each copied source's named-stream inventory, lengths
and hashes. Its explicit application journal root is identity-bound at that same
point. Fresh Apply revalidates the exact retained plan, resources, roots and all
stream evidence before effects. It does not collect a newly trusted inventory at
confirmation. Ordinary same-volume single transfers keep write-through rename;
only cross-volume singles and actual multi-target duet sources use copied Move.
The domain is ordinary local NTFS with no concurrent source/target writers.
That initial activation excluded cross-volume albums and partially eligible
duet cohorts; the separately reviewed follow-ups below extend those boundaries.

Mandatory new staged and extracted UI/CLI checks use two actual existing CI
volumes, real original-model duets, mixed ordinary/duet jobs, named streams,
metadata, exact source retirement, and reconciliation. UI acceptance additionally
checks no-effect review, changed auxiliary content with restored primary mtime,
changed target, and cooperative cancellation. These are authored gates, not
runtime acceptance. The paired v14 Move-all case now requires actual execution;
refusal is a mismatch rather than the earlier explicitly open capability gap.
Raw cross-volume receipts remain `file_published` plus `source_retired` and are
never relabeled as physical rename events.

At f18100d, actual Windows native gates passed six summary tests, ten positional
CLI cases, seven NDJSON cases, eight logging sink cases, 25 album cases and all
37 MP3/FLAC/Ogg repair product cases. The overall native job failed a separate
known test lint. Its real staged renderer passed the latest help/About, advanced
developer setting, independent inactive rows, long-name width and stable-footer
checks, selected Copy/Move and MP3/FLAC/Ogg checks, then stopped on the completed
root-album descendant display assertion. That exact projection defect is fixed
and now passed the fresh staged and extracted gates recorded below; no
paired-Windows result is inferred.


## Verified Windows desktop consumer checkpoint

The [actual Windows product run 38201794764](https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38201794764)
passed feature-off, staged original-AI and consumer-extracted original-AI lanes.
The package itself records source commit
`b066edff53a51d02be451feeb418c83769853920` and source tree
`05b8df40aabbe6180091a193668c3cecd819f406`. That commit is the verified PR
synthetic merge of base `3f767c58a9c597444d4d540e6a5147bca881cbda` and
public branch head `d96ebd2c3ea77fb130c31b2ea53b4e2bdb5fc2f2`; these
are the build provenance, not a claim about subsequent source commits. Both physical lanes bind the same
EXE and manifest, with four cases each for selected single transfers, MP3 repair,
FLAC repair, Ogg repair and albums. The completed root/mixed album screens show
only the relevant operation rows; consumed descendants are optional, while real
partial/unsupported outcomes remain visible.

Independent archive verification measured:

- ZIP: 16,493,475 bytes; SHA-256
  `786797198f695d9c24a361daf1257d6b5690480b9a6468f07acd10330b4cb238`.
- Unpacked regular-file bytes: 47,527,676 across 565 files; all 564 manifest
  entries verified (the manifest does not hash itself). This is logical file
  size, not allocated disk blocks or a measurement of the original Python app.
- EXE: 16,174,080 bytes; SHA-256
  `1360979fabecc90009fb45681d760a8ab94d2fa7f3f344625492a69f3dc7e3f2`.
- Manifest SHA-256:
  `209a1944811d1ea8f0d1023f76e3b8e6c2edf4885501c0c3543a349520125c0b`.

These results establish this authored Windows product boundary, not all v14
settings combinations, arbitrary metadata representations, copied cross-volume
Move, process-restart recovery or Android. No speedup claim follows from the
package size or passing functional gates. Paired runtime/memory measurements
remain pending.

## Copied-album integration checkpoint (2026-10-12)

Cross-volume ordinary local-NTFS albums now have an authored composition under independent review for main/ADS copying, per-album metadata and exact empty
source-directory cleanup, including a selected source-root album. This is not yet
native/runtime or host acceptance. Quiescent external writers remain an explicit
scope assumption. An interim 4,096 physical-operation limit is checked at readiness
and preparation; exact serialized journal headers/records have matching read/write
budgets. Scalable indexed parsing remains required for full library-size coverage.
Existing larger Copy/rename admission and legacy reader behavior are preserved.
Partially eligible duet cohorts, remaining tag families and fresh Album restart
inspection remain separate open requirements.


## Cross-volume album host candidate

The current source composes bounded cross-volume album Move on ordinary local
NTFS with quiescent writers. Main data and named-stream evidence are captured at
preview using verified roots, then rechecked after the same original plan is
confirmed. Metadata/planning readers are dropped before acquiring the driver's
exact root cleanup authority. The journal identity is retained across confirmation.
No source-retirement evidence is inferred from intent or final counts.

Actual `completed_album_operations` distinguish earlier finished albums from a
later cancelled cleanup. Read-only reconciliation retains the manifest status,
terminal record, per-file/stream verification, and exact acknowledged removed
source directory roles. Only the validated original reference plus full identity
permits an expected current absence; target absence or a reappeared source warns.
Cancellation during post-effect verification keeps physical outcomes and credits
while showing an unresolved final check.

The mandatory staged/extracted UI lane adds real two-volume root and mixed albums
with sidecars, nested directories and named streams, exact full destination sets,
original and physical counts, and unchanged source/target/journal snapshots on
review, stale stream and stale membership refusals. The positional CLI uses
closed stdin and requires an identity bijection between each file's acknowledged
publication, completed item and later retirement. A host fixture cancels from
observed verification bytes, checks no inspection effects and Busy release.
Portable fixture checks are source evidence only. These new native and actual
consumer gates have not yet run; the accepted d96 package above does not contain
this cross-volume album activation. Album restart remains a separately reviewed
read-only workstream.

## MP4 AAC/ALAC consumer candidate

The [exact-extent MP4 transformer](staged-mp4-repair.md) is wired into the
existing source-only repair driver, with no new transport or mutation backend.
The next staged/extracted Windows desktop and positional CLI gates require real
AAC `.m4a`, ALAC `.alac`, fast-start multiple-value collapse, duplicate outer-field consolidation and
unknown structural-atom refusal. The desktop additionally checks stale-preview no effects, observed
renamed filenames, source-only review, unchanged-file filtering and exact retained
originals. Source/journal no-effect snapshots exclude only the already documented
live WebView storage contents; no media or journal exception is introduced.

An independent authored atom oracle requires byte-for-byte exact repaired output,
unchanged original file/moov/media extents, opaque items/artwork and chunk tables.
Actual FFmpeg decoding of both tiny owned files must produce identical nonempty
PCM; FFprobe verifies repaired tag values. Decoder commands use bounded subprocess
timeouts. These follow the official [FFmpeg stream-selection documentation](https://ffmpeg.org/ffmpeg.html#Stream-selection)
and [FFprobe JSON/tag output documentation](https://ffmpeg.org/ffprobe.html).
Portable fixtures do not execute the Rust writer. Native/runtime and exact
consumer package acceptance remain pending. No universal MP4, fragmented,
encrypted, UTF-16 or private-offset representation support is claimed.

The native reader now preserves the ordered duplicate-outer-field projection,
including a first value already in Hebrew followed by a later repairable value.
The physical writer consolidates that repair into one canonical text item with
exact padding. Separate real-native-reader fixtures cover that later-only case
and intact metadata inside a movie containing a refused private structural atom.
Both belong to the pending exact-head runtime gates, not prior package evidence.


## Existing-ID3 WAVE/AIFF/DSF consumer candidate

The [bounded container writer](staged-id3-container-repair.md) reuses the same
retained-original source-only Fix authority and the five frozen fields. It does
not infer a supported tag representation from the filename extension. Missing
tags, unsupported flags/chunks, ambiguous duplicates and insufficient existing
padding remain explicit boundaries; a filename repair can still complete with
an honestly partial tag outcome.

The real staged/extracted desktop gate now requires four successful container
trials (WAVE v2.3/v2.4, AIFF v2.4, DSF v2.3), one insufficient-padding partial,
and one retained-plan stale-source refusal. The independent authored oracle
checks exact original backup, full byte layout outside the existing tag, all
five repaired fields, retained opaque/artwork frames, WAVE INFO bytes that must
not replace ID3 fields, and actual FFmpeg-decoded PCM equality. Preview/review
and stale trials bind the complete media tree and application journal snapshot;
ordinary completed names and the independent inactive filter are exercised.
The positional CLI package requires the same five physical positive/partial
trials, native counters and fresh reconciliation on both staging and extraction.

Six portable helper tests pass, including all three formats under both ID3
versions and actual local FFmpeg decoding of the independent expected bytes.
They do not run the Rust writer. Ten stream/native cases, six container product
cases (60 full repair cases with the current MP4 candidate), and actual packaged
Windows consumer gates remain required before claiming these formats accepted.

Primary format references: [Mutagen WAVE](https://mutagen.readthedocs.io/en/latest/api/wave.html),
[AIFF](https://mutagen.readthedocs.io/en/latest/api/aiff.html) and
[DSF](https://mutagen.readthedocs.io/en/latest/api/dsf.html) explicitly use ID3 tags.
Historical packaged versions remain separately unverified; the transformer’s
pinned reader evidence and narrower admission are documented in its own scope.


## Intentionally skipped duet destinations

The one-eligible follow-up retains the original ordered duet cohort instead of
requiring two successful targets when `exist_only` intentionally omitted the
other destination. It still requires every admitted destination to complete
before source retirement. Failed required writes do not become successful skips.

When every destination was skipped, the old blocked `DuetDeleteIntention` is
retained only as historical decision evidence. The frontend admits a zero-effect
completion only for a fresh matching Sort plan with no operations, zero predicted
credits, no read/recognition errors, Move + duet + existing-only settings, and a
one-to-one exact match between empty blocked intentions and present duet source
observations. It creates no mutation grant, destination or journal. The ordinary
UI explicitly states that no changes were made and the original files remain;
its optional inactive list retains the original skipped row. This is the stated
safe departure from the frozen all-skipped deletion, not a fabricated successful
move or a suppression of the historical evidence.

Actual staged/extracted UI and positional CLI gates now require a configured
original-model one-eligible transfer and a zero-eligible retained-source case,
including exact source identity, main/named-stream bytes, unchanged target and
journal trees, actual zero/nonzero counters, and original summary credits. New
Rust admission tests and these Windows consumer cases remain unrun at this
source checkpoint. The 36 actually executed paired cases stay unchanged; broader
duet settings vectors and decision/skip trace parity are a separate active lane.


## WavPack APEv2 consumer candidate

The [terminal APEv2 writer](staged-wavpack-repair.md) uses the same retained-original
Fix authority. Its native scalar preserves embedded NUL separators, matching the
pinned original APE getter rather than applying the Xiph/MP4 comma-space rule.
It changes only equal-length UTF-8 target value bytes; header/footer flags, keys,
item order, encoded WavPack blocks, binary artwork and unknown fields stay exact.

Staged/extracted UI gates require footer-only, header-plus-footer, NUL-value and
read-only-item partial cases plus stale-source refusal. The first three require
an exact original backup, complete independent byte/field/layout comparison and
actual FFmpeg PCM equality. The unsupported trial must retain all original tag
bytes after filename repair without a fabricated tag backup or success. UI
preview/review/stale checks include the application journal snapshot, observed
filename and optional inactive rows. Positional CLI package gates require the
same four physical cases with actual repair counters and fresh reconciliation.

Five portable helpers, including independent local FFmpeg decoding, pass. These
are authored reference bytes, not execution of the Rust writer. Ten native/
stream and five Windows product cases (65 full repair cases), plus the actual
Windows staged/extracted consumers, remain mandatory. No general APE, ID3v1
mirror or tag-at-start support is claimed.

## Historical Windows CLI consumer checkpoint: 2026-10-12, public 041f7039

[Windows CLI run 38209034534](https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38209034534)
passed the release, staged-package and consumer-extracted execution lanes.
The package provenance records the PR build commit
`b690ae6b7b8833bfdd326b9c77acfb008db7ffe9` and tree
`24ed2f060b9e11c57f8253be94f0b0f93755bf1a`, corresponding to the public
`041f7039` source checkpoint. It is not the newer cross-volume Album/MP4/
container-ID3/WavPack/partial-eligibility candidate.

| Measured artifact | Exact value |
| --- | --- |
| Inner ZIP | `singles-sorter-original-ai-windows-x64-selected-cli.zip`, 12,258,071 bytes |
| ZIP SHA-256 | `47ba53ea1c78eca63694489c43f1ef0a9482afce4d266289527fc568295dc388` |
| Inventory | 144 files and one explicit empty directory; all 143 non-manifest files independently rehashed |
| Unpacked logical file bytes | 34,456,300; not allocated filesystem space |
| `singles-sorter.exe` | 5,733,888 bytes; SHA-256 `84c72f45b85d9574cc7053b5fd05cc508b9a5c6700f5a646f36ba694e45c7b71` |
| Manifest SHA-256 | `1ff066ee540369be7790c32082882cc81480814b595141db11dfd5ec059a7225` |

Both `windows-preview-stage-execution.json` and
`windows-preview-execution.json` contain 38 actual subprocess cases, bind those
exact EXE/manifest hashes, and require closed stdin and removal of developer
tools from the child PATH. The exact source's package, notice-inventory and
execution gates were independently rerun against the downloaded bytes. Physical
coverage includes single Copy/Move, actual cross-volume Move, genuine original-
model duet and mixed Move with named streams, album Copy/root/mixed Move,
MP3/FLAC/Ogg repairs with retained originals and honest unsupported partials,
primary CSV precedence, logging levels/files, and no-effect dry-run/overlap
refusals. This is practical positional CLI acceptance, not only JSON protocol
unit coverage. The read-only smoke receipt separately reports its own narrower
scope; it does not claim to have exercised mutation or Ctrl-C.

The same run produced the first 36 actual frozen-v14 paired reports. Independent
audit found matching final trees, observed original summary counters, committed
byte coverage and physical counters in all 36 cases. Actual operation order is
exact in 28; eight retain the explicitly allowed atomic whole-album grouping
difference, with no invented child effects. Overall paired status remains
**OPEN** because execution-derived decision/skip trace is still missing.
There are 34 distinct boolean setting vectors, not 36: all 32 duet-disabled
vectors and two duet-enabled vectors. The other 30 duet-enabled vectors remain
a separate authored follow-up. These results do not establish full v14 feature
or settings regression parity.

Optional fresh-process/package telemetry had path-boundary failures and does
not establish a speed or memory advantage. The package sizes above were measured
directly from the authenticated ZIP. Keep functional evidence separate from
pending comparable timing/peak-memory measurements and from the independently
accepted earlier desktop checkpoint.


## Actual paired Windows sorting checkpoint: 2026-10-12, public 26e812a9

[Windows CLI run 38282703780](https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38282703780)
executed all **66 paired cases covering all 64 distinct six-boolean settings
vectors** against the exact frozen v14 baseline. Its PR build source was
`3f86423a5d6748e329962ce6fd3a9c00b6c651f4`; the observed native EXE SHA-256 was
`5c47ec5b3eb927b3fed9fe1414a6588570cb949ae81eeb29bc823d0965a15055`.
The authenticated paired-evidence ZIP SHA-256 is
`517dc45eb81e85b96083111f4a182b9eceb84d26676cb4682a24c7e71db4e542`.

Independent reconstruction from raw reports verifies:

- Exact final media trees, original summary counters, committed byte coverage,
  physical counts and accepted decision/route evidence agree in all 66 cases.
- 380 fresh-preflight decisions and 234 routes include 56 routes with no admitted
  transfer. These are fresh Apply-preflight decisions plus observed effects;
  they do not claim equal interleaved decision scheduling.
- 242 destinations, 103 removed sources and all 140 native journal chains are
  accounted for. 38 copied source-retirement receipts match full identities.
- 58 cases have exact effect order. Eight retain the documented atomic whole-
  album grouping difference after equal final trees/counters/byte coverage;
  no child transfer receipts are invented.
- 34 cases execute the configured original models, with 234 recorded original
  model calls. The remainder retain the specified disabled-AI path.

This closes the historical 041 missing decision-trace and remaining 30-vector
coverage gap for the authored corpus. It is not full v14 product acceptance:
CSV editing/export, settings persistence, arbitrary media representations,
interruption/fault boundaries, final package interaction and Android provider
behavior require their own evidence. These cases contain no
`fresh_noop_preflight` receipt; the safe source-retaining all-skipped duet case
remains a separate acceptance requirement, rather than preserving the unsafe
historical deletion.

The same revision did not produce a newly accepted package: the cross-volume
Album consumer stopped at an authored artist-folder expectation, the narrow
About dialog stopped at a 36px icon target, and the native job reached its time
limit while later Album cases were running. Their source-reviewed corrections
are in the subsequent 2ec04c8 batch; passing that rerun is still required. The
failed counter assertion did not retain its actual value, so the correction is
based on an independently reproduced frozen-oracle expectation, not a guessed
native result.

Optional cold-original telemetry failed because a SciPy import attempted to
create a temporary file outside the owned trial and confinement refused it.
Package-size telemetry had no package to inspect because
the mandatory consumer gate had stopped. Functional reports were already saved.
No comparable speed or memory advantage, nor a new package size, follows from
these failed optional measurements. The exact earlier 041 package sizes above
remain the measured consumer checkpoint.


## Actual Windows CLI consumer checkpoint: 2026-10-12, public 2ec04c8

[Windows CLI run 38286628344](https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38286628344)
passed release, staged-package and ZIP-extracted consumer execution. Its package
provenance binds PR build source `e8e67f92008194589d23bdb23a3511926e2fad63`,
tree `60137d5efac2dea0d52983a7d1ddb8ab6b88cd12`, matching the public 2ec04c8
source tree. This package does not contain the later Whole-Album restart host.

| Measured artifact | Exact value |
| --- | --- |
| Inner CLI ZIP | 12,381,484 bytes |
| ZIP SHA-256 | `ad9959b7fe38241c1dff81256fd16c13228c3f82f9dd7f0c4a44346d1b936bdb` |
| Inventory | 144 files and one explicit empty directory; all 143 manifest entries rehashed |
| Unpacked logical file bytes | 34,793,504, excluding filesystem allocation overhead |
| `singles-sorter.exe` | 6,069,760 bytes; SHA-256 `3d48be3f71a2d1f8a52b5dce644147c55e6e4dd5971f92de59d9135819e0a381` |
| Manifest SHA-256 | `c18368f72f577ac04ccf4b580046df7c84c5590d41ca7dc1435690ac87312586` |

Both actual execution receipts have **58 subprocess cases and 56 required
checks**, with closed stdin and developer tools removed from the child PATH.
Both bind the same EXE and package manifest. Independent inspection rechecked
all retained case records, matching UTF-8 diagnostic log hashes to stderr,
original backups or unsupported-container byte preservation, physical counts,
source cleanup identities, and strict reconciliation status. Packaged notice
inventory and actual PE imports also match their reviewed contracts. This audit
reads authenticated runtime evidence; it does not rerun the Windows EXE on Linux.

New bounded consumer proof includes:

- Root cross-volume Album Move: 5 destinations and 5 moved files; original
  summary 0 songs, 0 artist folders, 1 album, top artist credit 4.
- Mixed cross-volume Album Move: 6 destinations and 6 moved files; original
  summary 1 song, 0 artist folders, 1 album, top artist credit 5. Both cases have
  verified named streams, three exact source-directory removals and no recovery
  error. This supplies the actual value missing from the prior failed assertion.
- One eligible original-model duet destination commits and retires one source.
  Zero eligible destinations return the explicit fresh-preflight no-op with
  zero counters and preserve the source, rather than performing the unsafe
  historical all-skipped deletion.
- Bounded MP4 AAC/ALAC (including duplicate outer-field consolidation), existing
  ID3 in WAVE/AIFF/DSF, and terminal UTF-8 WavPack APEv2 repairs now pass both
  consumer lanes, alongside the earlier MP3/FLAC/Ogg cases. Unsupported
  representations remain explicit partial results with unchanged tag bytes.

The separate read-only PowerShell smoke has 12 checks. It does not claim physical
mutation or graceful Ctrl-C proof. The paired 66-case / 64-vector corpus also
passes again at this revision; its timing observations retain a separate audit.

This is CLI consumer acceptance. The same revision's desktop run passed native
layout/About/help and real Sort/Fix preview, then stopped because the settings
harness tried to click the correctly hidden Copy control while Fix mode remained
selected. A real Sort-tab navigation correction is reviewed for the next run.
Full current desktop interaction, file/Whole restart consumers and Android
provider acceptance remain distinct requirements.
