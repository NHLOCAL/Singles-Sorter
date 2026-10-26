# Scoped ordinary NTFS album cleanup

The native primitive and copied-album product composition are authored and under
source review. Windows compilation, thirteen native cleanup cases, thirty-eight
album product cases and three directory/manifest boundary cases remain mandatory.

The explicit selected album Move constructor opens the exact selected source
root with DELETE access before any journal or destination effect. Existing
selected-root constructors gain no cleanup authority. Root consumption requires
all descendant/parent aliases to have been released; an outstanding Arc refuses
without attempting a close/reopen workaround. Its private parent ancestry stays
held in a one-shot cleanup object, which can inspect only the original leaf after
removal. It cannot authorize sibling/ancestor mutation.

Only empty ordinary local NTFS directories with no named data streams or EAs
are eligible. Readonly and unsupported attributes refuse. Representation and
bounded child inventory are rechecked at dispatch; Windows also enforces directory
emptiness. No recursive deletion, readonly override, replay, or automatic cleanup
fallback exists. As with copied file Move, this is an explicit ordinary-use domain
requiring quiescent external writers: a userspace ADS/EA check cannot exclude a
new stream inserted after that check. This is not a global filesystem guarantee.

A checked close consumes the native handle once. Every failure or cancellation
after disposition dispatch is uncertain, with exact original object evidence.
The original-name probe preserves foreign replacement entries. Live absence is
not a promise of deletion persistence after power loss; recovery must tolerate a
reappeared original and never delete it from historical journal state.

The authored executor composition binds preview directory identity and
representation, journals intent before cleanup, verifies every main/ADS destination
and required metadata before source retirement, and acknowledges cleanup before
claiming a completed album. Any failure leaves a partial/recovery report with
actual retained directories. The native primitive alone establishes none of those
product-level ordering requirements.

### Per-album copied-Move completion

The private selected-album transfer callback releases a consumed source lease only
following its synced `SourceRetired` event. Generic jobs and ordinary Copy keep
existing full-job guard retention. Each copied album records `AlbumTransfersObserved`
with its original operation ID and the exact ordered physical `Committed` and
`SourceRetired` events. Recovery validates those against the retained binding and
physical journal, including destination identities. Metadata completion and empty
source-directory cleanup follow that acknowledgment before the next album starts.
`AlbumCompleted` is credited only after those acknowledgments, so later failure or
cancellation cannot erase an earlier completed album. Whole-object rename remains
its separate sequential route. Prior Copy/rename manifests retain their prior
phase rules; the new acknowledgment is accepted only in the copied-Move domain.

Live file-path album reconciliation uses the shared inspection control for journal,
main-stream and auxiliary-stream reads. Created target directories remain identity-
pinned through content verification. A consumed source root is checked only through
its retained original-parent capability; this does not authorize restart recovery.

The first copied-album scope admits at most 4,096 physical file operations, checked
both at preview and preparation before effects. This matches the reviewed shared
controlled journal parser. Larger Copy/rename jobs retain their previous admission
and legacy media-read behavior; the explicit controlled API refuses that size
rather than silently using uncancellable media reads. Removing this interim copied-
album ceiling requires a separately reviewed indexed parser and scale tests.

Copied-album Header schema 2 records the complete original native root observation
under the upfront held lease, including exact native UTF-16 leaf/reference and full
parent/root identities. This supports a future separately reviewed fresh-parent
read-only reopening without guessing an absent short alias. Existing Copy/rename
schema-1 headers remain unchanged. Preparation checks exact physical and manifest
header envelopes/newlines, the manifest record-count bound and a maximum-width
physical destination ID projection for each per-album transfer acknowledgment.
Controlled manifest writes share the reader's per-line/total/record limits.

The selected-root short-leaf regression requires an actual differing NTFS alias
from [GetShortPathNameW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getshortpathnamew),
which may otherwise return an unchanged long path. It never treats an unavailable
alias as a successful test. Root cleanup replay compares the recorded native leaf;
descendant leaf comparisons remain exact and unchanged.
