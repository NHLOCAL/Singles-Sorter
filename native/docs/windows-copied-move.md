# Scoped copied Move completion

This source checkpoint adds preview-bound NTFS stream evidence and held known-stream leases. It does not activate cross-volume Move or duet retirement. The native auxiliary staging/publication and checked retirement primitives are authored. The ordinary local-NTFS copied-Move product driver, preview scope, journal phases and recovery composition are being implemented separately.

The operation domain assumes no concurrent external writer changing the selected files during execution. Named streams have independent sharing: a lease on the unnamed stream does not universally exclude new named streams. Known streams deny write sharing; fresh full membership/content and representation checks detect observed changes before retirement. An extra stream observed at that boundary retains the source. This is not a claim of exclusion against a writer racing after the check.

Evidence contains the exact held source identity, ordinary non-readonly representation, and every data-stream UTF-16 name, length and SHA256, including the unnamed stream. Inspection and Apply acquisition compare full evidence. NTFS EAs, nonordinary representations, hard links, reparses, readonly sources, malformed inventories and more than 256 streams refuse. Stream enumeration is bounded to 1 MiB; hashing uses 64 KiB buffers and main/named-stream cancellation checkpoints. Auxiliary destination streams now have a separately staged, flushed, read-back-verified path with retained handles and copied metadata before no-replace publication. The native retirement primitive requires the configured number of distinct, fully verified, synced destination guards, then rechecks source streams and consumes the source close once. It verifies live namespace removal, not power-loss persistence of deletion. Its count and same-source evidence do not independently bind the planner destination list: the product driver must derive the cohort from the retained original plan and verify every exact destination reference/identity before journaling retirement intent. Product activation remains unavailable.

Microsoft documents [per-stream sharing and whole-file delete conflicts](https://learn.microsoft.com/en-us/windows/win32/fileio/file-streams), [stream enumeration](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryinformationfile) and [FILE_STREAM_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_stream_information). Native cases deliberately permit either documented observed late creation followed by refusal or an OS sharing refusal; neither outcome promotes a universal stream-lock guarantee.

Sixteen native cases are authored and mandatory in CI. Local Rust/Windows execution is unavailable; source review and actual CI acceptance are separate.

## Preview-bound job composition (source candidate, runtime pending)

`windows_job::CopiedMoveScope` retains the read-only preview's exact source/root
identities, ordinary representation and complete stream names, sizes and hashes.
`prepare_copied_move_scope` uses already granted selected roots; journal storage
must be an existing explicitly bound application directory. No serialized scope
can mint these grants. `SelectedWindowsJob::prepare_copied_move` reruns the
original planner and acquires every source's stream leases before creating any
journal or artist directory. It retains original single/duet destination order.
The ordinary same-volume rename constructor is unchanged.

The distinct `SelectedWindowsCopiedMove` binding has required auxiliary-stream
and Copy metadata phases. Each destination is staged, stream-verified, metadata
finalized, published without replacement and flushed before its committed
receipt. Retirement takes exactly the retained original plan cohort; generic
byte-only adapters cannot satisfy its hooks. Failure or cancellation retains
sources until the whole cohort is verified. No backup-transfer substitution is
used. Binding headers above 8 MiB refuse before effects; the new domain stops
before either journal exceeds the reader's 64 MiB limit and preserves its prefix.

Read-only reconciliation verifies every current destination identity, unnamed and
named stream hash, complete stream inventory and metadata. Metadata observations
are captured for the whole cohort before content reads. A missing/changed stream
or bad metadata at any required destination invalidates the retired-source
cohort. A source link present after a retirement receipt is a recovery condition,
including a foreign replacement which is never deleted. Reconciliation supplies
no automatic retry, resume, cleanup or deletion authority.

The candidate adds two native read-only/cancellable-verification tests (18 primitive cases total),
five journal phase/marker cases and ten actual Windows job cases. The latter
include real duet routing, stale auxiliary preview, cancellation during the second
destination, no-overwrite collision, source reappearance, changed destination ADS,
one-shot execution and a strict distinct-volume case. That case uses owned TEMP
and checkout subdirectories, asserts their volume identities differ and never
substitutes same-volume evidence. All are authored and unrun locally. Product
host/CLI selection, independent composition review and actual Windows acceptance
remain required before activating this path for users.

Admission is checked against the freshly held root volume identities: same-volume
non-duet operations remain on the existing exact-object rename path. This copied
domain admits same-volume genuine duet cohorts and distinct-volume file Moves.
A mixed same-volume plan containing both routes currently refuses as a whole;
operations are neither dropped nor split into reordered jobs. A per-operation
route composition is still required for that case. Native full-stream publication
verification now uses the actual execution cancellation flag and preserves cursor
positions at each bounded hash chunk; metadata/namespace inspection does not hide
an uncancellable full-file hash. Existing generic adapters are unchanged.
