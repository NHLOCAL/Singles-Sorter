# Owned Windows single-file Copy integration

`sorter-executor/experimental-windows-copy` connects the existing verified
transfer driver to real Windows identities and held handles, exclusively beneath
an owned `TempDir`. This is a development fixture capability, not a user-library
or shipping apply command. The original routing/settings planner is unchanged.
No CLI/UI apply, Move, source deletion, duet execution, recursive execution,
album/directory creation, tag writes or writable recovery is enabled.

## Authority and domain boundaries

The ordinary `validate_execution` remains synthetic-only. `validate_owned_copy`
requires the concrete `WindowsMutationStorage` capability which consumes the
owned tree. Before metadata/planning/content reads, it checks root identity and
containment, rejects mixed reference domains, overlapping roots (including
mixed-case NTFS aliases, checked through held ancestor identities), non-Copy modes,
recursive/duet settings and more than one operation. Replanning then uses the
owned-Copy read adapter restricted to those checked roots. It establishes directory
listing by successful native enumeration, and file read/seek by checked actual
read and seek calls, with identity/size/timestamp consistency checks. Unsupported
operations on each entry kind are explicitly false. It never strips the planner's
`UNKNOWN_CAPABILITY` blocker; the ordinary `DesktopStorage` preview continues to
report unestablished capabilities honestly. Both initial fixture planning and
fresh revalidation use these observed capabilities. All destination parents
must already exist. Every ordinary plan/resource/adapter/hash/source check still
applies; no blocker is rewritten to claim shipping apply support.

The private binding records an explicit owned-Windows Copy domain separately
from `synthetic_only`. No serialized header, mode flag or digest can construct a
live execution binding. Mixed domains and unsupported shapes fail closed.
Execution independently resolves source and target parents beneath the consumed
tree and checks exact root ancestry, so a valid binding cannot be used with
another owned tree. Real IDs are never relabeled synthetic.

Native names use UTF-16. The source leaf comes from its lossless Windows
reference, not the display-only name. This slice rejects unpaired-surrogate
source leaves because the unchanged planner's string-based destination policy
cannot represent them losslessly. Valid Hebrew and supplementary Unicode names
are preserved. The native name validator rejects unsafe/reserved names without
inventing suffixes or alternate normalization.

## Copy sequence and durability

The source's identity, parent, size, timestamp (the existing read adapter's
encoding) and digest must match on the same held source lease. That lease denies
read/write/delete sharing; ancestor handles remain held. The driver streams into
an exclusively created owned temporary file, handles bounded short writes, checks
length/hash/EOF, calls checked `FlushFileBuffers` on that same temp handle, and
verifies its identity/name/content before no-replace publication. The published
handle remains held through verification and the final journal acknowledgment.
The source is retained on every outcome.

Journal intent is appended and synced before each effect. An append/sync error,
including a lost response after real I/O, blocks success and automatic replay.
Known partial temp writes are retained for inspection; no path-based cleanup or
adoption occurs. Cancellation stops subsequent effects while preserving evidence
of effects already issued. UnknownOutcome errors require reconciliation.

Copy requires exclusive creation, no-replace publication and content verification,
plus successful file-content sync and journal sync. It does **not** require the
stronger mutation `durable_flush` capability that gates destructive Move.
That capability remains `Unknown`. Successful Copy establishes verified content
and journal completion under ordinary local NTFS/OS/device flush assumptions;
it does not certify destination-name survival after power loss. Keeping the
original source is important to this distinction. No volume flush, admin privilege,
registry setting or device-cache change is used.

Primary references:

- [Microsoft FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) specifies checked file-buffer flush behavior and its writable-handle requirement.
- [Microsoft buffered I/O flushing](https://learn.microsoft.com/en-us/windows/win32/fileio/flushing-system-buffered-i-o-data-to-disk) describes forcing buffered file contents to disk.
- [Rust upstream Windows filesystem implementation](https://github.com/rust-lang/rust/blob/main/library/std/src/sys/fs/windows.rs) implements `fsync` with checked `FlushFileBuffers`; this is precedent for ordinary file sync, not proof of power-cut resilience.
- [Microsoft FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information) documents relative target directories and nonreplacement behavior.
- [Owned journal assumptions](windows-owned-journal.md) covers the separately reviewed journal implementation and SQLite precedent.

## Read-only close/reopen

`open_owned_copy_recovery` requires fresh original host scope, exact inspected
binding digest, and the retained owned-tree capability. It verifies source and
target root identities. `WindowsCopyRecovery` exposes only the bound source and
destination, reads identity/size/timestamp/hash evidence, and treats only a real
NotFound response as absence. It cannot observe arbitrary caller-supplied paths.
Recovered bindings cannot create or append journals or run transfers. A found
same-name file never establishes ownership without the expected identity/hash.

These fixtures close/reopen while retaining in-process root/journal-location
capabilities. Actual cross-process root reauthorization is future work; the
journal bytes alone do not grant it. Put fixture journals in their own sibling
subtree, outside planned source/target roots, so creation does not contaminate
planning enumeration or root timestamps.

## Verification boundary

Eight native Windows tests cover exact multi-buffer Unicode copies and source
lease protection; final-name and foreign-temp collisions; source changes and
foreign owned roots; cancellation before/during writes with retained orphan data;
short writes and lost write/flush/publish responses; partial/lost journal responses
at every Copy phase; mixed domains/disabled modes; and rehashed domain forgery,
constructor containment and read-only recovery restrictions. All I/O uses
generated owned files. Fault wrappers simulate response loss around actual I/O;
they are not kernel-crash or power-cut tests.

CI requires exactly eight nonzero Windows fixtures and reruns existing executor
regressions with the feature enabled. Linux contract tests and Windows-target
Clippy are useful gates but do not establish native runtime acceptance. Report
actual Windows results only for the exact tested commit.
