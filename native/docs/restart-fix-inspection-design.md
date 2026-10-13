# Read-only FixNames restart inspection

This staged family extension starts from the source-sealed streaming Copy/rename
checkpoint 92230a1ca9561da1007a0f49dc0f011680540945. The unchanged checkpoint still
shows Fix and Album as unsupported. This document activates neither family.

## Data and authority

A new public data-only Fix journal descriptor contains the recorded job ID,
source reference/full Windows identity and operation display references/names.
It never contains RepairScope, SelectedRepairJob, writable journal handles or
live root grants. Bounded hash-chain/framing and header validation run with the
existing InspectionControl; the descriptor is explicitly not a claim that
physical phase replay or current filesystem verification succeeded.

Listing always labels a valid descriptor as requiring inspection. The full
existing strict Fix phase parser is deferred until the user freshly selects the
source folder, including exact original root reference and full identity. This
preserves a necessary distinction: some valid native observations carry the
normalized root spelling from a held Windows handle. That alias must remain
serde-skipped and must be freshly observed from SelectedRepair roots. Discovery
cannot invent it from serialized paths or declare a plausible string alias safe.

The host obtains the existing selected repair root and disjoint application
journal grant, then calls only the existing read-only recovery seam. The public
recovery descriptor never returns a mutation session. Journals, retained original
backups and staged files remain untouched. Album and Android are unchanged.

## Controlled reads and parser bounds

Preserve the existing recover_repair_journal(roots, bytes, cancel) API. Add an
optional controlled route for restart inspection. Check both the existing cancel
flag and the optional control before and after each root/stat/open/read probe,
at each strict journal record, operation, candidate and history loop. Cancelled
must escape distinctly rather than becoming Missing, Changed or Unverified.

Use the streaming single-job limits for this new descriptor/controlled route:
64 MiB journal, 4 MiB record, 65,536 records and 4,096 operations. Reject excess
before expensive phase replay. Existing uncontrolled callers retain their
current limit and behavior. Verification streams 64 KiB chunks with no normal
aggregate-media cap, records checked cumulative actual bytes including repeated
current/backup/candidate reads and an EOF/growth probe, and rechecks full held
identity/size/mtime evidence before accepting a file.

A retained cancelled/failed run token is only an observation-order hint. Re-read
all operations, current files, backups and relevant interrupted candidates each
new run. Preserve original operation IDs/output ordering and all existing Fix
phase, replacement-v2 length, source-retention and intended-name checks. Observed
results describe an inspection interval rather than an atomic library snapshot.

## Shared host and renderer contract

Reuse the streaming start/snapshot/cancel worker and its single lifecycle owner.
Make the target-root token optional only for the repair family; the host rejects
an omitted target for a single-transfer family and rejects an extra target for
Fix. Resume identity binds journal token, fresh source token and the optional
target token. Recorded source text never substitutes for a grant.

The repair row shows source-only selection, the repair family label and the
recorded operation count. Results show current, original/backup and staged or
interrupted-candidate paths with their verified/missing/changed/unverified state.
A complete result requires a strictly parsed finished journal without a torn
record, every operation recorded Complete, and every required evidence item
verified. Partial, stopped, unsupported metadata and failed jobs remain visible
and unresolved even if an individual retained file was verified. Users get the
concrete next step to keep journals and backups and inspect unresolved paths.

## Verification

Add actual Windows fixtures for completed MP3/FLAC/Ogg repair where supported,
partial rename/publish/backup intent and retained originals, root replacement,
header/chain corruption, existing root-spelling alignment, metadata/content
change, more than 64 MiB cumulative evidence, exact chunk cancellation and fresh
reinspection. Compare full owned source/journal snapshots, excluding atime.
Add host source-only token/resume/lifecycle tests and renderer tests for missing
or extra role selection, interrupted operation evidence and cancellation.
Update packaged restart acceptance to inspect a real prior Fix job instead of
asserting permanent unsupported status; Album remains explicitly unsupported
until its separate reviewed authority and parser adapters are implemented.

Rust compilation, pinned formatting and Windows/native package execution remain
external gates. Portable tests validate contracts, not Windows runtime behavior.

## Official API basis

Microsoft documents opened versus normalized names for a held handle in
[GetFinalPathNameByHandleW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew).
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
documents reparse-point and share-mode behavior. This design retains the existing
held, reparse-rejecting adapters; it does not infer authority from those names.

## Implemented family checkpoint

The controlled backend descriptor/reader, typed retained host family, optional
source-only target contract and evidence UI are implemented in follow-on commits.
Fix descriptors always require live inspection. An ordinary failed inspection
says only that it changed no files and that the recorded source/backups need
checking; it does not assert already-missing originals survived. Raw backend
error details are not included in that ordinary failure message.

The renderer acceptance mock now follows start/snapshot/cancel rather than the
removed synchronous IPC. The earlier streaming-only 92230a1 checkpoint required
this fixture follow-up before its renderer CI gate could pass. The coherent Fix
checkpoint includes that correction. The browser route remains unavailable
locally; fixture syntax and unit/build checks are distinct from rendering proof.

Native source gates include completed MP3/FLAC/Ogg replacement and backup
verification, partial staging/backup, chunk cancellation, fresh reinspection,
changed content/root and70MiB media verification. Native host source tests require
source-only admission, reject wrong roots/extra targets and detect changed backup
bytes. Package restart checks now inspect completed and partial real Fix jobs and
a deliberately modified owned backup, using that mutation as the new no-write
baseline. Album remains visible/unsupported pending its own reviewed adapter.

No Rust compiler, pinned formatter, native Windows test or browser was run
locally. Those gates remain mandatory before runtime or package success claims.
