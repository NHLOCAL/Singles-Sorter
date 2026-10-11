# Held Windows publication synchronization and the next Move contract

## Implemented boundary

`WindowsMutationStorage::sync_published_file` adds the missing post-publication
file-buffer synchronization primitive, on an already-held publication beneath
an owned local-NTFS `TempDir`. It checks the originating storage/root, native
identity/name, full digest and actual length before dispatch, checks cancellation,
requests `FlushFileBuffers` on that same handle, and verifies again. The opaque
acknowledgment carries the actual observation; it cannot be deserialized or
constructed from a pathname. Ancestor and publication leases stay held.

A failure after dispatch poisons further sync attempts on that guard and returns
`UnknownOutcome` with the observed identity. Inspection remains available. A
cancel before dispatch performs no flush; cancellation after a successful call
cannot undo the already acknowledged call. No cleanup, reopen-by-name mutation,
source retirement, automatic retry or replay is performed. Readback after an
error is evidence of current content, not evidence of a successful flush.

The owned Windows Copy driver now invokes this primitive after the synced
`DestinationPublished` record and before `DestinationVerified`/`Committed`. Its
existing publication intent is journaled before namespace effects. A dedicated
owned-Windows adapter hook defaults to an explicit unsupported error, never a
no-op; synthetic and other domains do not call it. The journal format is unchanged.
Post-publication cancellation or sync failure yields pending reconciliation and
retains the source. A lost acknowledgment never advances to a committed phase. `durable_flush` remains `Unknown`, owned bindings remain Copy-only, and
production roots, CLI/UI apply, albums, directories and tag writes stay disabled.
The primitive preserves native UTF-16, including unpaired units; it does not
change the existing string-based planner's rejection of unpaired source leaves.

## Evidence, not hardware certification

Microsoft's [file caching documentation](https://learn.microsoft.com/en-us/windows/win32/fileio/file-caching)
says file-system metadata is cached and should be flushed or opened write-through.
[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers)
requires a writable handle and reports request success/failure. Rust's
[Windows `fsync` implementation](https://github.com/rust-lang/rust/blob/main/library/std/src/sys/fs/windows.rs)
and SQLite's [Windows VFS `winSync`](https://github.com/sqlite/sqlite/blob/master/src/os_win.c)
use checked file-buffer flushes. These support an ordinary OS-backed file-sync
model assuming Windows, local NTFS, drivers and storage honor their flush
contracts. Neither this primitive nor native CI establishes power-cut resilience.

Six Windows-native primitive tests cover verified multi-buffer Hebrew/supplementary names,
retained source and ancestor/publication guards, cancellation before and after
native flush, foreign ownership and incorrect digest rejection, actual Windows
access-denied flush on the existing read+DELETE rename lease, before/after real
flush response loss with permanent poisoning/read-only inspection, and lossless
unpaired UTF-16. A ninth owned-Copy integration case exercises pre/post-sync
response loss and cancellation, forbids later verified/committed/retirement phases,
and checks read-only recovery and replay rejection. The existing journal-fault
matrix still covers every Copy phase around the added sync. CI checks both exact
nonzero counts. These are real native I/O and
injected response-fault tests, not kernel-crash or power-failure tests. Linux skips
and Windows-target Clippy are not native runtime acceptance.

## Practical next Move path

The current atomic Move branch already performs same-held-source, same-volume,
no-replacement rename with intent journaling before the effect and identity/hash
verification afterward. However, the source opens with read+DELETE access, without
write-through, and there is no pre/post-rename flush. Flipping `durable_flush`
would therefore be incorrect even under ordinary application assumptions.

A narrow next candidate is a separately authorized owned-fixture Move binding
restricted to one file and positively identical local NTFS volumes, with existing
non-reparse parents. Acquire a write-capable, exclusive source handle with native
`FILE_WRITE_THROUGH`; validate the same identity/name/content and pinned ancestry;
flush that same handle before the journaled rename, then flush and reverify the
same renamed handle before durable completion is recorded. Failure/response loss
after rename must remain pending reconciliation, never claim the original name
still exists or retry rename automatically. Cancellation before rename keeps the
source; after dispatch, report the actual observed phase.

This is a practical OS/storage-assumption candidate, not an impossible demand for
perfect hardware: Microsoft's [CreateFileW caching discussion](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
explicitly includes NTFS rename metadata in write-through behavior.
[Native create options](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-zwcreatefile)
provide `FILE_WRITE_THROUGH`; [native rename](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
provides handle-relative targets, same-volume restriction and nonreplacement.
[PostgreSQL durable_rename](https://doxygen.postgresql.org/fd_8c.html)
flushes the renamed file and attempts the parent-directory flush, tolerating
unsupported directory synchronization on Windows. That is application precedent,
not a proof that our currently missing steps already exist.

The candidate still needs a reviewed exact contract mapping from native
write-through open to rename metadata, an explicitly scoped capability rather
than promotion of unrelated storage promises, a writable source lease, pre/post
sync ordering in the existing journal state machine, and native response-loss,
collision, cancellation and read-only recovery fixtures. Cross-process recovery
still needs independent root reauthorization; journal bytes are not authority.

## Copy-and-retire is a separate gate

For copy-and-retire, every required distinct destination must remain held,
verified and synchronized before issuing a journaled same-source-lease
retirement. Cross-volume rename cannot be emulated as atomic; this slice does
not create cross-volume authority or mount points.

Current `FileDispositionInfo` acceptance yields `DispositionRequested`, not
proof of completed removal. Microsoft [NtSetInformationFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile)
documents closing as the only subsequent file operation after disposition;
"set disposition then flush the same handle" is therefore not a documented
completion recipe. [POSIX extended disposition](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_disposition_information_ex)
can establish a visible-link removal contract on close, but is not by itself a
post-close namespace synchronization acknowledgment. Looking up a now-missing
path does not authorize deletion of a replacement or certify durable retirement.
That distinct missing completion/barrier contract must not be papered over by
converting `DispositionRequested` into `Retired`. A same-volume rename-only Move
slice need not pretend that copy-and-retire is already solved.

The separately gated candidate has since been implemented as the
[owned same-volume rename-only domain](windows-owned-rename.md), retaining these
OS/storage assumptions and leaving copy-and-retire unsupported. Its native
acceptance is an independent exact-tree CI gate.
