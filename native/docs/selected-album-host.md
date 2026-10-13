# Selected Windows album host composition

This is a source candidate until exact native and product gates pass. It uses the
unchanged v14 planner and the reviewed Windows album driver; no UI or CLI code
chooses a new destination, performs its own file transfer or decodes authority.

## Ownership and limits

- Sort preview and metadata reads use `for_album_preview` with
  `WindowsDirectoryReadLimits::ALBUM_DEFAULT` and the current cancellation token.
- AlbumScope retains full membership and source digests. At Apply, original
  resources and the exact plan are revalidated into an opaque in-process value.
- Every source planning/metadata adapter leaves scope before native prepare.
  This matters when the selected source directory itself is moved. Windows
  sharing options last for the life of each open handle; a conflicting read
  handle cannot be bypassed by a later rename grant. See Microsoft's
  [CreateFileW documentation](https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/fileapi/nf-fileapi-createfilew.md).
- The host then supplies fresh independently observed source, target and existing
  journal roots. There is no recursive journal-directory creation or automatic
  retry/replay. Copy/Move bounds and original planner order remain enforced by
  the backend. Cross-volume Move remains unsupported.
- Album confirmation explicitly asks the user to stop other applications/sync
  that could change either selected tree until the operation completes. CLI
  planned output and positional warning carry the same condition. Membership
  checks are not presented as an exclusion lock against every possible writer.

## Display evidence

Physical file totals combine complete whole-directory descendant receipts with
its disjoint deferred-single transfer report. Copy already includes all files in
its transfer report. The retained unique source evidence supplies source_items.
Album count and original immediate-file artist counters stay separate; sidecars
are not silently discarded, and recursive file counts do not replace v14 totals.
Child renames can be partial effects without moving any file to its final target.

Read-only reconciliation retains root/directory evidence and pending original
operation IDs. Expected absence of a successfully moved source is not a failure.
Changed/unverified roots, pending phases, copy metadata mismatches and incomplete
reconciliation raise the ordinary final-check warning without rewriting actual
mutation status or counters. Technical details remain in the developer/CLI view.

## Required acceptance

- Native host: real TagLib preview to retained root-album Copy/Move, one-shot use,
  exact files and successful read-only reconciliation.
- Positional CLI: closed-stdin child Copy, root Move and mixed album/single Move;
  original pre-alias artist/direct-file counters and recursive physical counts.
- Staged and extracted desktop: nested files/sidecars, Copy readonly/mtime
  preservation, root Move, mixed Move, visible confirmation scope, review dismiss,
  changed album membership rejected without effects, and simple terminal rows.
- Portable tests validate authored fixture frames and require every receipt in
  the package gate. Those tests alone establish no Windows execution result.
