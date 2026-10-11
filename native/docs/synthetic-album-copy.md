# Synthetic owned-directory and flat album Copy boundary

This is an executor capability test seam, **not physical apply**. No Windows-root,
Android, CLI or UI mutation entry point implements or activates it. All accepted
references/identities are explicitly synthetic. Neither TagLib writes nor recursive
album moves are included. Existing single-transfer behavior remains separate.

## Exact bounded scope

`validate_album_copy` reruns the unchanged v14 job planner and verifies its complete
plan/resource/scope evidence. It accepts exactly one `AlbumTransfer` in Copy mode,
with a nonempty, complete, ordered set of flat regular-file children, including
sidecars. Mixed jobs, multiple albums, nested directories, links/reparse entries,
collisions and unsupported names fail closed. These are executor capability gaps,
not additional sorting policy, changed planner output, or new legacy counters.
The original baseline is `78d001aee5ce49628bdffebd34540fa83ba32c12`; its
`handle_album_transfer` rules remain represented by the unchanged core planner.
This seam verifies byte-copy effects; it does not claim complete `shutil.copy2`
metadata fidelity or native filesystem durability acceptance.

The binding includes original source/target roots, the album directory, exact
ordered child observations and full digests, and every required destination
prefix. Existing artist/ABC parents are identity-bound. Every absent prefix must
be created exclusively; an existing intended-new album directory is never
adopted. Names are rejected conservatively rather than changed or suffixed.
Platform-specific equivalence and name support must also be checked by the adapter.

Before the first effect, `AlbumStorage::acquire_album` must validate the complete
binding and hold real source-membership/content/ancestry guarantees throughout
the job. All existing parent guards are acquired before creation. Rechecking
without a stable lease is explicitly insufficient. An adapter unable to implement
these promises must reject the request. There is no fallback to name-only lookup.

## Effects, evidence and stopping

Each newly owned directory has separate durable intent, observed result and
successful directory-sync events. Creation operates on one exact leaf below a
held parent; implicit recursive mkdir and cleanup-on-drop are forbidden.
Observed identities must be synthetic, distinct from bound roots/sources/parents,
and match exact parent/name evidence before any dependent effect is allowed.

The album journal chains the existing verified streaming transfer journal as
individual embedded records. Both levels validate provenance and transitions.
A private context gate prevents running extracted album child bindings through
the ordinary single-transfer API. File transfers must resolve back to the exact
held, newly owned album-parent identity. All parent/source/owned-directory guards
and the original published-child guards remain held through final result
commitment, including the parent Complete append and sync. Nested temporary-file
identities cannot alias the source album or any existing/new directory. A child failure blocks later child
effects; completed and partial objects remain in place. No source is deleted.
The synthetic result's child counts are evidence counts, not legacy album counters.

Cancellation is checked at directory and file boundaries. Effects already
returned by storage are retained even when cancellation arrives during dispatch.
Any append/sync uncertainty poisons the live journal and prevents further effects.
A sync error can follow actual persistence; it never produces live success.

## Restart and reconciliation

Opening always produces read-only authority. It requires a fresh host grant for
the exact full album binding digest and original scope. Parent records and nested
transfer records are checked together. Truncated tails are retained byte-for-byte,
block append, and prevent completion claims. No open API replays, appends, deletes,
cleans up, adopts a directory, or automatically resumes an incomplete operation.

A directory found by name after a lost creation result is ambiguous. Even an
identity-matching created event without the corresponding durable sync event does
not count as a committed directory. Source membership/content, bound directories,
and each child's committed identity/digest must all remain valid for a complete
album report. A stopped run remains incomplete even when some children succeeded.
After a failed live sync, a later read may contain the entire durable Complete
record; reconciliation may then report that exact evidence, while remaining read-only.

## Platform research behind the contract

- [Microsoft CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw)
  creates only the final component and reports existing directories as an error.
  Its success does not itself furnish the stronger held-identity ownership contract.
- [Microsoft CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
  describes directory handles and handle-sharing restrictions. This seam does not
  claim that ordinary path opening alone provides its stable membership lease.
- [Rust create_dir_all](https://doc.rust-lang.org/std/fs/fn.create_dir_all.html)
  is non-atomic, retains successfully created prefixes on failure, and can accept
  concurrent creation by others. It cannot implement exclusive job-owned creation
  by itself and is deliberately not used here.

## Verification

`cargo test --locked -p sorter-executor` includes synthetic fault sweeps at every
storage call and every journal append/sync boundary. Tests cover directory intent
and result loss, sync failure, incomplete nested tails, wrong/foreign identity,
source-set replacement/reordering after binding, cancellation, partial child
success, parent-binding bypass attempts and read-only restart. Existing journal,
transfer, original-policy and workspace regressions remain required. These Linux
synthetic tests are not native Windows/Android acceptance.
