# Owned Windows journal file integration

This slice connects `PhysicalJournalV2` to a real, exclusive local-NTFS file on a
caller-owned `TempDir`. It does not connect Windows source/destination mutations
to the transfer driver. Production roots, CLI/UI apply, writable recovery/replay,
album namespace durability and Android writes remain disabled or unimplemented.
The original sorting/settings policy and its preview gates are unchanged.

## File-content durability model and evidence

The existing `JournalStore` contract requires exclusive private storage and
append/sync ordering. Here `sync` calls `FlushFileBuffers` on the same writable
handle and returns success only after Windows acknowledges it. This is the
ordinary OS-backed file-content durability model: Windows, local NTFS, drivers
and the device are assumed to honor their documented flush behavior. It is not
a hardware-perfect guarantee. No registry, privilege, volume-flush, device-cache
or security-setting changes are used.

Primary sources inspected during design:

- [Microsoft FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers): a writable file handle is required; failures are returned to the caller.
- [Microsoft file caching](https://learn.microsoft.com/en-us/windows/win32/fileio/file-caching): file metadata can be flushed with FlushFileBuffers.
- [Microsoft NtCreateFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile): root-relative names, exclusive share mode zero, and FILE_CREATE that fails if the name already exists.
- [SQLite upstream Windows VFS](https://github.com/sqlite/sqlite/blob/master/src/os_win.c): `winSync` checks `osFlushFileBuffers` and reports `SQLITE_IOERR_FSYNC` on failure.
- [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html): assumes the operating system honors flush, while explicitly describing broken flush/device implementations as outside that assumption.

That evidence supports ordinary file synchronization, not an assertion that
these CI fixtures test power failure. SQLite is a precedent, not proof of this
implementation. Native mutation `durable_flush` remains `Unknown`; album
`durable_directory_sync` is a separate guarantee covering created directories
and required ancestors. This journal bridge does not grant that guarantee.

## Ownership and order

Creation consumes no user path. `WindowsMutationStorage` already requires an
owned `TempDir`, local volume GUID, NTFS and pinned non-reparse ancestry. A new
journal is created relative to a validated parent handle using `FILE_CREATE`,
exclusive sharing and no reparse traversal. Regular-file and single-link checks
apply. The private file handle has no public raw-handle, truncate or writable
reopen API. Reading its bytes cannot reposition the next append: append always
seeks to the held file's end. Source leases and transfer primitives are unchanged.

The header is appended and flushed before a usable `PhysicalJournalV2` exists.
The existing protocol flushes each effect intent before allowing that effect,
then records its result. Failed append or sync poisons the protocol writer,
including failures reported after bytes were written or flushed. The raw file
also permanently poisons writes after an actual I/O/flush error. No error is a
license to retry the effect, truncate a tail or delete an orphan.

A location capability captures exact file identity plus held root/ancestor
handles at creation. Close/reopen inspection accepts only that capability,
requires the same owned storage, opens read-only and rechecks exact identity.
It cannot reconstruct authority from a filename or deserialize a writable
handle. `open_for_recovery` separately requires fresh host scope and prohibits
both journal append and journal creation with the recovered binding. Actual
cross-process restart needs a future independent root/location authorization
path; keeping an in-process location alive is not that implementation.

## Verification and limits

`windows_journal` contains six Windows-only fixture tests: header/event flush
and close/reopen with read-only reconciliation; existing-name/foreign-root
refusal; replaced-identity and ancestor-rename refusal; before/partial/after
write and before/after flush response faults; incomplete-tail preservation; and
unchanged unknown native mutation flush capability. The I/O unit test
`journal_flush_tests` provokes a real Windows FlushFileBuffers access error using
a test-only reduced-access handle and checks permanent poisoning.

Fault wrappers deliberately simulate loss of a returned response around real
native file I/O. They do not simulate kernel crashes or physical power loss.
Bytes observed after a failed sync are readback evidence, not proof of durability.
CI asserts exact nonzero native test counts so Linux skips cannot pass as native
acceptance. Native Windows success must be reported only for the exact tested
commit. Cross-compilation and Clippy alone do not establish runtime acceptance.
