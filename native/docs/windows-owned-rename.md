# Owned Windows single-file, same-volume rename

`sorter-executor/experimental-windows-rename` is a separate development-fixture
execution domain. It performs one `copy_mode = false` single-file operation on
an owned local NTFS `TempDir`, using the existing original sorting policy,
verified journal driver and read-only recovery. It does not authorize production
or user-selected roots, UI/CLI apply, copied-source retirement, cross-volume
Move, recursive/duet work, directory or album creation, or tag writes.

## Concrete authority and supported shape

`WindowsRenameStorage` consumes the existing owned-tree capability. It is a
rename-only adapter: temporary creation, copy publication and separate source
retirement explicitly return Unsupported. Its generic `durable_flush` remains
Unknown, as does the original mutation adapter's value. The executor accepts the
narrow write-through rename contract only for a separately validated private
`owned_windows_rename` binding. The ordinary adapter cannot run this domain, and
the rename adapter cannot run Copy or synthetic Move by promoting global flags.

Binding validation uses the same pre-read containment checks, non-overlapping
identity-bound roots, fresh original-policy replan, source content evidence and
native UTF-16 reference checks as owned Copy. It requires Move-only scope,
`copy_mode = false`, one ordinary file, `main_folder_only`, no duet, pre-existing
parents and positive same-volume identity evidence. The source root and target
root are inside the same owned local-NTFS tree with pinned non-reparse ancestry.
Native names preserve UTF-16; unchanged string-based policy refuses unpaired
source leaves instead of silently replacing them. The native primitive can test
such names independently without changing that planner boundary.

The generic executor additionally requires atomic eligibility before proceeding.
If it is unavailable, execution stops; no copy-and-delete fallback exists.
Journal validation rejects temp/copy/retirement phases in the rename-only domain,
including rehashed recovery input. Domain flags cannot be combined or used to
construct a live binding from serialized bytes.

## Successful sequence

1. Revalidate the known source on an exclusive held read/write/DELETE handle,
   opened with native `FILE_WRITE_THROUGH` and the existing no-reparse rules.
   Its private lease mode prevents substituting an ordinary read-only Copy lease.
2. Sync `AtomicRenamePrepared` in the owned journal before dispatching the native
   mutation adapter. A journal append/flush error prevents rename.
3. Check cancellation, exact identity/name/parent, size, timestamp, full digest
   and same-volume destination evidence. Check `FlushFileBuffers` on that same
   source handle, then revalidate before rename.
4. Call handle-relative `NtSetInformationFile(FileRenameInformation)` with
   `ReplaceIfExists = false`. The original lease is consumed on success. This
   same-volume operation intrinsically moves the original link; there is no
   later separate deletion.
5. Observe the destination through that same handle, check a post-rename
   `FlushFileBuffers`, and reverify original identity, destination name/parent,
   full content, length and timestamp. Keep publication and ancestor guards held.
6. Sync `AtomicRenameObserved`, then verify and sync `DestinationVerified` and
   `Committed` through the existing journal driver. Cancellation at intermediate
   acknowledgments stops further success phases and requires reconciliation.

## Failure, response loss and recovery

A failed preflush, rejected collision or cancellation before rename leaves the
source name unchanged. A successful native rename followed by observation,
flush or verification failure is `PendingUnknown`, with actual available
namespace observations. Cancellation after dispatch has the same pending
classification; it cannot claim rollback or `NoEffect`. The source lease is
already consumed, and there is no replay or compensating delete/rename.

A missing returned response after real native success also requires recovery.
The journal can contain only the synced intent even though the original is now
at the destination. Read-only recovery inspects the bound source and destination,
compares the exact expected source identity and digest, and reports evidence;
it never resumes, deletes a collision, adopts an orphan or appends to the journal.
A replacement at the old source name is a different object, never authority to
remove it. Only genuine NotFound is absence; access errors are not absence.

Close/reopen tests retain the owned-root/journal-location capability in process.
Actual cross-process restart still requires a future independent reauthorization
of those roots. Journal bytes and path strings alone cannot grant it.

## Ordinary local-NTFS durability contract

This is an OS-backed application contract, not a promise of perfect storage:
Windows, NTFS, device drivers and storage are assumed to honor successful flush
and write-through requests. Relevant existing ancestor namespaces are assumed to
have been established durably before the operation. No new directory durability,
volume flush, administrative privilege or device-cache configuration is claimed.

Microsoft's [CreateFileW caching behavior](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew#caching-behavior)
explicitly includes NTFS rename metadata in write-through behavior. Native
[`FILE_WRITE_THROUGH`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-zwcreatefile)
is the corresponding native file-object create option; it is not a Win32
attribute bit. Applying that documented NTFS behavior to this native option is
an explicit engineering inference, supported by pre/post checked file flushes.
Microsoft documents [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)
on writable handles and [native no-replace same-volume rename](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
with relative directory handles. No single page specifies this application's
entire multi-call sequence or certifies any particular storage hardware.

Established applications use comparable OS-level assumptions:
[PostgreSQL durable rename](https://raw.githubusercontent.com/postgres/postgres/master/src/backend/storage/file/fd.c)
flushes before/after rename while tolerating unsupported directory synchronization
on Windows; [Rust Windows fsync](https://raw.githubusercontent.com/rust-lang/rust/master/library/std/src/sys/fs/windows.rs)
and [SQLite winSync](https://raw.githubusercontent.com/sqlite/sqlite/master/src/os_win.c)
check `FlushFileBuffers`. Their replacement/path-reopen or deletion semantics
are not imported into this identity-bound no-overwrite implementation.

Copy-and-retire remains unsupported for this domain. Legacy `FileDispositionInfo`
is only a deletion request; Microsoft documents only close afterward. Neither
successful disposition nor a pathname disappearance proves the stronger durable
retirement contract. See [the separate assessment](windows-published-sync.md).

## Verification and limits

Native tests operate only on generated owned fixtures. Twelve primitive tests cover
write-through mode evidence, exact cross-directory identity/content, Unicode,
collision refusal, lease/root restrictions, cancellation and failure/response
loss around real preflush, rename and postflush. Seven driver tests cover atomic
journal phases, source/destination evidence, no fallback, every journal failure
boundary, domain forgery and read-only recovery/replay refusal. Cross-volume
rejection is checked from positive volume IDs; the fixture suite does not claim
physical testing on two storage volumes.

Fault injection surrounds actual native calls; it is not kernel-crash, VM
power-cut or physical power-loss testing. CI counts nonzero Windows cases and
must pass on the exact reviewed tree. Linux tests and Windows-target Clippy do
not establish native execution acceptance. Successful native CI demonstrates
live ordering, sharing, identity and returned-response behavior under the stated
contract, not the behavior of every filesystem filter or hardware cache.
