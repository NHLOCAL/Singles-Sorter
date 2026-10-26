> Historical staged checkpoint/design. The current composed capability and limits are documented in `current-restart-backend.md` and `controlled-selected-inspection.md`; Copy, selected Move (including copied routes) and Fix now have separate reviewed read-only paths. Actual composed Windows gates remain pending.

# Read-only desktop restart inspection candidate

The selected-Windows desktop exposes “פעולות קודמות” in the ordinary Help menu.
It discovers persisted application journals and explains the next safe action.
This is a source candidate until the exact combined commit passes the native
Windows gates. Portable tests and mocked renderer checks are not Windows proof.

## Supported boundary

- Selected single-song Copy and same-volume rename Move are inspectable after
  process exit. Historical completion is labeled as a journal claim, not proof
  that today's files still match.
- A user selects a record, then selects source and target again with native
  folder pickers. Recorded display paths are never reusable authorization.
  Fresh roles, exact lossless references and complete Windows identities must
  match before reconciliation. Preview root tokens remain separate.
- Inspection compares current source/destination identities and bytes, including
  the current Copy metadata receipts and metadata verification. Every recorded
  created directory is reopened at the same path and its full identity is checked;
  matching handles pin its ancestry through file inspection. A replaced directory,
  incomplete job or uncertain evidence is not shown as consistent.
- Album and FixNames journal filenames remain visible with explicit unsupported
  restart-inspection guidance. Orphan transfer journals also remain visible.
  They acquire no recovery record authority and cannot be inspected or resumed.
- No journal import, resume, replay, rollback, deletion, cleanup, source retirement
  or Android restart-recovery entry point is added. Journals and original repair
  backups remain untouched.

## Bounds and authority

The discovery scan considers at most 4,096 directory entries and retains at most
256 records, newest generated timestamps first. This ordering is display only.
History pages contain at most ten records, with a 128 MiB read budget. The shared
selected-journal API retains its 64 MiB hard ceiling; this restart entry point
uses a tighter 4 MiB per-journal limit, 2,048 records per journal, and 128 transfer
operations. Live restart inspection makes at most 256 directory/file/metadata
observations and consumes at most 64 MiB of music content in total. Actual bytes
are debited for every repeated source/destination read and the bounded one-byte
EOF/growth probe; initial file lengths do not replace cumulative accounting. Exceeding a live
budget produces needs_attention with an explicit limit notice; omitted reads are
never interpreted as absence or completion. Parser-bound excess is rejected
without granting an inspectable record. Journal bytes are
read under identity-checked, reparse-rejecting handles. A retained record captures
file identity, size, modification evidence and SHA-256; any change requires a
fresh listing. The directory journal's complete chain is checked before its
recorded paths are projected. Unsupported families are listed without interpreting
their content. Per-operation rendering is paginated at 50 rows.

The parser returns data-only RecordedSelectedJob/RecordedSelectedSummary values.
Executable bindings are not reconstructed or exposed. Its read-only internal
scope can be used only after exact current root rebinding; current physical
journal and Copy metadata validation remain in place.

Only list_recovery, select_recovery_root and inspect_recovery are added to the
existing command manifest, handlers and explicit application permission. No broad
filesystem, shell or opener capability is introduced.

## Acceptance

- TypeScript controller tests: separate fresh selections, unsupported families,
  picker cancellation, stale/failed refresh, wrong-job replies, pending duplicate
  prevention and bounded operation pages.
- Mocked renderer acceptance: ordinary-state guidance, unsupported families,
  native-picker cancellation simulation, close/Escape while pending, focus return,
  operation pagination and narrow-window layout.
- Windows-only executor cases: completed owned Copy/Move reopened using fresh
  roots; a partial owned Copy stopped after one committed destination; changed
  root identity; replaced artist directory with the same copied files; changed
  destination content or Copy metadata; large-file observation limits;
  corrupt/oversize journals, parser record limits and an incomplete tail. Partial reconciliation preserves source files,
  the already committed destination and the exact original journal bytes.
- Windows host cases: new Host discovery/rebinding, exact grants, oversize and
  changed journal rejection, unsupported/orphan records, and token separation.
- Actual Windows desktop acceptance restarts the app after owned Copy/Move and
  Album/Fix lanes, uses native pickers, rejects wrong roots, reconciles both
  supported modes, verifies externally changed content, and verifies that
  unsupported families are visible. Full-tree snapshots include root/directories,
  kind, identity, permissions, public attributes, size, mtime and file SHA-256
  (excluding access time), so empty-directory or metadata writes cannot pass.
  The deliberately modified disposable destination becomes the new baseline only
  for its subsequent inspection; journal snapshots remain unchanged.
- Both staged and extracted desktop package gates require those actual restart
  checks. Rust compilation, formatter, Clippy and actual Windows runtime gates
  must pass on the final source before validation is claimed.

Official host/security references used for integration:

- https://v2.tauri.app/security/capabilities/
- https://v2.tauri.app/security/permissions/
- https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew
