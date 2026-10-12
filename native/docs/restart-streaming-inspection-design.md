> Historical staged checkpoint/design. The current composed capability and limits are documented in `current-restart-backend.md` and `controlled-selected-inspection.md`; Copy, selected Move (including copied routes) and Fix now have separate reviewed read-only paths. Actual composed Windows gates remain pending.

# Cancellable streaming restart inspection

Design candidate extending the sealed read-only checkpoint
81668179378e514cda94f9817c6849b0876f20c1. No new budgets or backend behavior are
activated by this document. The prior checkpoint remains unchanged.

## Outcome

Ordinary users can inspect prior Windows jobs without an aggregate music-byte
limit stopping after a handful of songs. The work remains read-only, streams
bounded chunks, reports measured progress and elapsed time, and responds to
cooperative cancellation. Current-file verification remains distinct from
journal history. Album and FixNames must gain their own reviewed inspection
adapters rather than remain permanently unsupported.

## Authority and evidence

Only retained opaque record/root/run tokens reach IPC. Journal data never becomes
an executable plan, append handle, cleanup permission or source-retirement grant.
Every new run reopens and validates the exact recorded journal files, roles,
lossless root references and full identities. Created-directory identities stay
pinned while dependent files are inspected. Existing strict phase validation,
Copy metadata checks and full file identity/content comparisons remain mandatory.

A stopped run is unresolved. A cursor and previously completed operation IDs are
ordering hints only, never current verification evidence. A user-requested new
run may visit the first unfinished operation first, then wrap in deterministic
journal order. Every operation must be read and verified again within that new
run before an overall consistent result is possible. No cached hash is trusted
after reopening. Process restart requires fresh native root choices and starts
from the persisted journal, not from deserialized authority or cached results.
This is resumption of read-only inspection, never resumption of mutations.

## Read-only control and bounds

Introduce an optional read-only control with cancellation checkpoints and progress
callbacks. Existing callers keep their existing constructors and behavior.
Controlled restart calls check before and after each filesystem/metadata/parent
probe, before every journal record and operation, and around every bounded data
read. Cancellation propagates as Cancelled, never absence, mismatch or success.
Synchronous native calls themselves are not forcibly preempted; control is checked
at their boundaries and at each 64 KiB streaming chunk.

Music verification has no normal aggregate-byte refusal threshold. Memory remains
bounded: one fixed-size content buffer, bounded journal inputs, bounded operation
records, bounded retained snapshots, and no collection of whole music files.
Actual bytes consumed, including repeat observations and EOF/growth probes, use
checked cumulative arithmetic. Identity, size and modification evidence are
rechecked after EOF. Oversized parser inputs or unsupported records remain visible
and unresolved, with explicit reasons.

Proposed configurable-in-code parser ceilings for source review: 64 MiB per
journal, 4 MiB per JSON record, 65,536 records per journal and 4,096 operations.
These are ceilings, not runtime proof or a claim of arbitrary-library support.
They are increased only together with cancellation in the relevant parser loops.
Media progress shows actual bytes, operations checked and elapsed time; it does
not infer completion percentages from predicted transfer bytes.

## Native host and UI ownership

Use the existing single BusyGuard and lifecycle generation. A native inspection
run owns its cancellation token, an opaque ID, bounded progress, terminal state
and result. Starting returns the ID; snapshot and cancellation commands do not
claim Busy or block behind the worker. Every terminal path releases ownership
once. The existing close/reload/exit callback also cancels this inspection.

The UI polls token-bound progress and offers Cancel and Close. Cancellation while
start acknowledgement is pending is retained and sent as soon as ownership is
known. Stale run replies cannot overwrite a newer selection or report. Closing
requests cancellation; the host continues to own Busy until its actual terminal
release. Reload waits for release using the existing startup Busy retry, without
restoring old file authority. Cancelled/incomplete results remain unresolved.

## Families

Single Copy/same-volume Move uses the current strict physical journal and read-only
WindowsCopyRecovery seams. FixNames has an existing source-only fresh-root
recover_repair_journal entry point; discovery needs a validated data-only header
projection and the same progress/control semantics. Album inspection needs a
separately reviewed journal descriptor and fresh-root reconciliation adapter for
both flat transfer manifests and whole-directory Move evidence. Root/child
renames, retained backups and replacement lengths require their own existing
invariants. No single-transfer parser substitution is allowed.

Each family is activated only after its exact adapter is source-reviewed and its
Windows owned-fixture tests are present. Until then its visible row remains
explicitly unresolved/unsupported rather than disappearing or implying recovery.

## Implementation and verification order

1. Review this session/evidence/control design with backend owner and reviewer.
2. Add controlled parser, operation and streaming-read seams, preserving default
   callers and strict phase/metadata validation. Test cancellation and actual-byte
   progress, including EOF/growth behavior, before changing the small interim cap.
3. Add token-bound host start/snapshot/cancel commands and lifecycle ownership;
   add UI progress, pending-start cancellation, close/reload and stale-reply tests.
4. Activate larger reviewed parser ceilings and uncapped-total streaming media
   inspection. Test ordinary multi-file jobs above 64 MiB, cancellation at every
   stage and deterministic reinspection after stopped/reopened sessions.
5. Integrate reviewed data-only FixNames and Album discovery/reconciliation in
   separate commits, with exact root/backup/manifest evidence tests.
6. Require final Rust compile, pinned formatter, Clippy, actual Windows restart
   and staged/extracted package gates. Compare full owned source/target/journal
   snapshots, excluding atime, around every inspection. Portable tests do not
   establish native runtime behavior.

## Review requirements accepted

Cancellation enters strict record replay and history loops, not only pre-scans,
and Cancelled escapes storage-error handling distinctly. Resume may reorder
observations only; operation IDs, output ordering and dependencies retain their
original semantics. Results describe observations across an inspection interval,
not an atomic library snapshot after early file handles have closed. Progress
counters are checked and terminal host snapshots become immutable only after
cleanup; ownership is released once, including late-start/cancel/close races.

Whole-root Album Move cannot reselect a vanished original source. Its pending
contract requires a separately reviewed read-only grant to the existing parent,
plus the exact recorded missing leaf, fresh destination and journal grants, and
proof of the expected absence. An absent path is never silently canonicalized or
inferred into authority. This remains staged behind the single-file adapter.

## Implemented streaming slice and evidence limits

The optional backend control, token-bound host worker and renderer polling are
implemented in the follow-on commits. The original 8166817 checkpoint remains
unchanged. Ordinary controlled media verification now streams with a 64 KiB
buffer and has no aggregate-byte cutoff. Controlled journal parsing remains
limited to 64 MiB per file, 4 MiB per record, 65,536 records, 4,096 operations and
8,192 retained created-directory identities. Discovery remains bounded to 256
visible records, ten per page and 128 MiB actual journal bytes per page. Exhausted
parser bounds are explicit unresolved rows/errors, never successful recovery.

Native source tests include an 80 MiB cumulative owned Copy observation, an exact
first-chunk stop, and an actual host lifecycle cancellation triggered by bytes
from a live file read followed by fresh-host root selection and full reinspection.
An exact Windows CI test-name gate prevents the host case from compiling out.
Desktop package acceptance also requires measured progress and closing/reopening
the journal view with fresh roots. These native cases have not run locally.
Portable renderer tests/build validate UI contracts only. Rust compilation, pinned
formatting, native Windows and staged/extracted package evidence remain required.
