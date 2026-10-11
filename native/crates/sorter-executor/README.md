# Experimental execution contracts

This crate provides binding, append-only physical journals, read-only recovery,
and bounded streaming transfer contracts. The transfer driver and outcome
aggregation are tested against owned in-memory fault-injecting storage.
`sorter-io` separately provides an experimental Windows NTFS primitive adapter,
available only behind `experimental-windows-mutation` and an owned `TempDir`
constructor. Its native fixture suite must pass on Windows; Linux compilation
or skipped tests are not platform acceptance.

There is no public CLI apply command, production-root mutation constructor,
shipping execution flag, or automatic recovery replay. Existing v1/v2 previews
and v14 routing are unchanged. Album and fix/tag effects remain unsupported.

## Trust and authorization

`validate_execution` verifies the read-only v2 plan, reruns the existing policy
against current read storage and metadata, checks resources/adapters/model and
ordered operation/dependency identities, and streams source content hashes against
host-inspected evidence. Source reads must carry identity/size/modified evidence.
Configured inference errors are recorded by a wrapper and block binding, even
when released policy would have failed open. AI disabled is a distinct profile.

The host independently supplies `ExecutionScope`: exact inspected plan, roots,
source observations/content digests, resources and permitted transfer modes.
Neither a plan hash nor deserialized journal grants authority. The journal header
contains the complete immutable binding and scope evidence. Resource byte hashing
and granting consent remain host responsibilities; test hashes are synthetic.

The original plan's physical-apply-unavailable and historical-duet-delete gates
remain intact. The synthetic contract may exercise their guarded state machine;
it does not promote those plans to native executable documents. Every other
blocker rejects binding. Duet destination IDs derive from full bound destinations;
repeated artist names with suffixed distinct destinations stay distinct.

## Durability and state

`JournalStore` requires exclusive ownership, append and durable sync. This task
implements the protocol and fault-injecting in-memory store. The optional
`experimental-windows-journal` bridge now supports a newly created exclusive
owned-TempDir local-NTFS journal file with checked FlushFileBuffers. Its ordinary
OS-backed file-content durability assumptions and native evidence limits are
documented in `native/docs/windows-owned-journal.md`. It grants no production
root, transfer or album-namespace capability. Events are hash-chained, sequenced and bound to job/plan/operation/source/
destination/digest. Intent is synced before effects; results follow effects.
Allocation is preceded by TempAllocationPrepared, writing by BytesWritePrepared,
publication by DestinationPublishPrepared, rename by AtomicRenamePrepared, and
retirement by SourceRetirementPrepared. Every new intent rechecks current
prerequisites; results of already-issued effects can still be recorded after a
predecessor is invalidated. Failed writes or
syncs poison the writer even when the outcome could already be durable.

Opening validates the header, full event chain and legal typed transitions.
Incomplete trailing bytes are preserved exactly and block append; there is no
truncate or repair method. Header/middle corruption rejects the whole journal.
The chain detects damage/order changes; it is not a MAC or authentication token.

Recovery only inspects storage and reports per-destination completion, explicit
resume candidates, proven effects awaiting journal reconciliation, or ambiguity.
It never retries, appends, renames, deletes or cleans up orphan temps. Publication
can be adopted only with the exact owned temp identity recorded before publish
and bound digest; rename requires the original source identity at the destination
and absence at source. Absence after retirement intent alone is ambiguous.
Committed reports describe destination completion, not a completed move count.
A missing source needs a durable retirement result and all distinct destinations.
Destination commitment is separate from dependency completion: a move dependency
needs its source retirement or verified atomic-rename completion. Recovery also
requires observed source disappearance. Remaining copies within the same exact
planned duet cohort may proceed before retirement; the binding preserves the
complete ordered DuetDeleteIntention cohort and each per-transfer copy effect.
Identity equality alone never grants this exception, and cohort failure or
cancellation blocks new effects. Later source/traversal dependencies still wait.

After restart, `open_for_recovery` accepts a new host `RecoveryScope` identifying
the complete inspected binding plus original scope. Its reconstructed authority
is read-only and cannot create/append a journal. This works when the source has
already moved; re-running pre-job planning would incorrectly require its return.
Explicit tail repair, effect-adoption records and mutating resume remain gated
future work. Recovery errors/access failures are ambiguity, not proven absence.

## Verification and remaining gates

Run:

    cargo test --manifest-path native/Cargo.toml -p sorter-executor --test journal_faults
    cargo test --manifest-path native/Cargo.toml -p sorter-core -p sorter-executor
    cargo clippy --manifest-path native/Cargo.toml -p sorter-executor --all-targets -- -D warnings

The test runner operates only on owned in-memory synthetic storage. It injects
failure before/after every append/sync/storage effect and readback, with both
volatile and durable journal outcomes, for explicit Copy-mode singles/duets, Move-mode singles/duets and atomic
rename. Copy-mode runs assert source retention and no retirement effects. Tests cover scope/plan/source/destination/resource/model mismatch,
rehashed invalid sequences, corrupt header/middle, every incomplete final-record
cut, cancellation/dependency state, exact identity/content adoption and no replay.
Initial missing-contract tests were red; focused cancellation/dependency tests
also reproduced unsafe historical-commit counting before the current-state fix.

The bounded transfer driver verifies streamed byte count and digest before
publication, retains pending outcomes, and reports cancellation truthfully.
Its fault tests do not establish native crash or power-loss durability.
Windows runtime acceptance for each exact change, production journal/root
authorization, real Android providers, production-root binding, album/tag effects, and public apply
remain separate gates. Linux tests do not certify Windows or Android execution.

### Independent review correction

The four independent review repros were copied into the owned test suite and
reproduced red before correction. Regression coverage now distinguishes destination
commit from complete move dependencies, stops new effects after invalidation,
preserves in-flight allocation/write/publication/rename/retirement observations,
and separates remaining duet copies from later traversal dependencies. No core
routing changes or physical mutation capabilities were introduced.
