# Android local-provider execution contract (implementation boundary)

This contract is not an assertion that the current read-only preview executes files.
The Android provider adapter consumes original `JobPlanV2` operations; it must never
infer operations from display rows or recreate sorting/album/FixNames decisions.

## Authority and retained input

- Only the system `com.android.externalstorage.documents` provider is initially
  eligible. Source and target are native-owned picker scopes, with explicit write
  grants checked before effects. WebView requests contain only retained job/plan IDs.
- Preserve the complete original plan privately, the opaque-token mapping, resource
  identities, settings and all source-content digests needed by its operations.
  A displayed plan digest alone does not bind bytes. Metadata snapshots are not
  current-source authority; filename-only recognition must not skip content binding.
- Bound files, total bytes, operations and journal size. Exceeding a bound fails
  before mutation. Revalidate grants, ancestry, names, fresh source content and
  destination expectations before the corresponding effect.
- Provider support assumes no other writer changes the selected files/directories
  during execution. It does not provide Windows leases, compare-and-swap, atomic
  batches or reliable identity from a document ID alone. Returned URIs may change.

## Effects and recovery

One process-owned worker retains the job until cleanup ends, including across
Activity destruction. Cancellation prevents the next effect; an in-flight provider
call is recorded before the worker reports a terminal status.

Each run owns a private UUID journal. Persist intent before a provider call and its
verified outcome after it. An interrupted call is uncertain until reconciled;
never retry a destructive operation merely because the prior result is missing.
Keep provider-generated names/URIs and independently verified hashes in receipts.
No UI-provided URI/path becomes authority.

- Copy: create an unpredictable owned staging child, stream bounded bytes, flush,
  reopen and hash, then publish only under the original planned destination name.
  A provider-adjusted collision name is an effect mismatch, not successful sorting.
- Move: use tested same-provider direct move/rename where possible, preserving the
  original operation's semantics. Verify returned destination ancestry/name/content.
  Cross-provider copy/delete is not silently substituted. If separately supported,
  its partial states and final fresh source-delete check require their own receipts.
- Duets/albums: retain original ordering, dependencies, destination selection and
  cleanup decisions. Stop and replan on effect mismatch. A partially completed
  batch stays partial; counters derive from observed effects.
- FixNames: use the shared native staged metadata transformer. Keep the original
  in a recorded, provider-visible recovery location before publishing a replacement;
  never perform in-place tag writes. Initial transformer format limits remain
  explicit. A rename-only operation must not imply metadata repair succeeded.
- Cleanup only affects proven-owned staging objects after fresh verification.
  Uncertain or externally changed objects remain for explicit reconciliation.

## Acceptance evidence

The separate marker-authorized provider probe establishes actual create, rename,
move, collision, descriptor and changed-content behavior on owned fixtures. It is
not sufficient by itself to accept product execution or recovery. Product tests
must exercise original-plan execution, cancellation, recreation/process death,
collisions, changed sources, partial failures and journal reconciliation through
the actual shared UI. ARM64 and 16 KiB runtime claims require those actual devices;
ELF alignment and cross-compilation alone do not establish them.
