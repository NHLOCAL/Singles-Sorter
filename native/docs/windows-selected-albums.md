# Selected Windows album execution

This implementation consumes `singles-v14.0` planner operations. It does not
identify albums, choose artists, sanitize names, or resolve collisions itself.
The frozen reference is `78d001aee5ce49628bdffebd34540fa83ba32c12`,
`src/core/singles_sorter_v5.py`, blob
`0b67726e72e527556f47a8eeffeb44778c62809b`.

An existing album destination is skipped by that planner, never merged. Nested
names remain unchanged; only direct album children receive the original name
sanitization. Successful album receipts count every direct destination file,
including non-audio sidecars, under the original analyzed artist before alias
mapping. They do not increment `songs_sorted`. The destination still uses the
mapped artist. Later refactored Python files with different counters are not the
reference. Partial completion is reported as partial rather than substituting
predicted totals for observed effects.

## Operation domains

Copy expands the planned recursive content into held-source, verified per-file
Copy operations. It snapshots complete directory membership, including skipped
children, and compares member sets independently of enumeration order. Transfer
order remains the planner's order. Known sources are leased before effects;
observed membership changes stop further publication. New directories are
created exclusively, with intent recorded and synced first. Actual output and
per-file recovery keep the original album operation ID.

Same-volume Move uses a distinct whole-directory native operation after any
ordered direct-child name sanitization from the unchanged planner. Every changed
child has an exact synced intent and observed identity receipt; unchanged children
retain their position in the original order. Known destination collisions stop
before sanitization. A later collision or uncertainty retains the source tree and
reports the actual partial child-name changes. Full relative membership, identities and file
content are checked before and after a no-replace rename of the same held
object. It preserves all descendants and container streams, EAs and security
metadata. There are no invented per-file rename journal receipts. Whole-tree
checks assume ordinary use without concurrent external album writers; they are
observations, not a universal membership lock. Post-dispatch uncertainty is
reported for read-only inspection without replay or automatic cleanup.

A selected source-root album uses a dedicated upfront root lease. Its parent
ancestry remains private; the source root never becomes a general writable
parent capability. Target and journal grants remain usable after the old source
path disappears. Recovery probes only the original bound leaf and the planned
destination. All files are retained after an uncertain outcome.

Retaining the album container's original ACL/timestamps/streams is an explicit
preservation difference from v14's fresh destination container. Child paths,
selection, ordering and counters stay frozen. Post-move file reads verify that
known children remain accessible. No claim is made that container metadata is
identical to v14, or that file/directory write-through implies hardware-perfect
power-loss guarantees.

## Current integration checkpoint

This source checkpoint is not accepted until its exact strict Windows CI suites
pass. Mixed album/single Move orchestration and ordered direct-child sanitization
are implemented. Initial preview and revalidation use the explicitly capped album
read adapter (100,000 entries, 16 Mi UTF-16 path units and 1,024 native calls per
enumeration); aggregate collector limits remain separate. This is not a single
global scan-work budget. The older empty-
directory deletion path remains unconditionally unsupported in product code;
only owned diagnostic tests can exercise disposition. A per-file fallback that
retains a source container must report partial completion, never album-complete.

The frozen Windows installer and CLI workflows configure Python 3.10. Their
exact packaged patch level has not been independently established. That family's
`copy2` uses `copyfile` plus `copystat`, transferring atime/mtime and Windows
readonly mode; it is not the later Python 3.12 CopyFile2 path. Timestamp/readonly
parity is required separately from byte verification. Existing source data and
streams remain intact in Copy mode.

Sources:
- https://github.com/NHLOCAL/Singles-Sorter/blob/78d001aee5ce49628bdffebd34540fa83ba32c12/.github/workflows/publish-installer.yml
- https://github.com/NHLOCAL/Singles-Sorter/blob/78d001aee5ce49628bdffebd34540fa83ba32c12/.github/workflows/publish-cli.yml
- https://raw.githubusercontent.com/python/cpython/v3.10.11/Lib/shutil.py
- `windows-whole-album-rename.md` for the bounded native Windows contract.

## Copy metadata and receipts

New selected Windows Copy bindings explicitly require metadata version 1. Their
physical journal records file-metadata intent and the actual held-handle receipt
after verified bytes and before publication. Unsupported or uncertain metadata
cannot silently become successful byte-only Copy. Existing journals omit the new
fields and remain byte-compatible; they never acquire a metadata-preservation
claim. A fixed old journal fixture and unknown/inconsistent-marker regressions
cover this distinction.

For copytree children, exclusively created directories receive source access/write
times and readonly attributes in postorder, after all child writes. The album
container itself is freshly created, as in v14, and does not receive the source
container's copystat. A private post-commit callback completes these required
metadata steps and the album receipt before starting later album/single transfers.
Later destination overlap with a finalized child tree is refused in preparation.
Earlier acknowledged album counts survive later cancellation or failure.

Recovery rechecks exact target identities and metadata. It probes metadata before
its own content reads can update access time, then independently checks bytes.
The metadata result is a point-in-time observation, not an exclusion guarantee.
Directory receipts, pending metadata and actual recheck statuses are exposed
separately from file transfer evidence. No recovery action replays mutations.
