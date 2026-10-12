# Opt-in selected Copy metadata finalization

This native adapter adds an explicit held-handle metadata step for selected Copy
and created album directories. It is not product activation or a general
capability promotion. Original source files/directories are preserved. Existing
Fix, ordinary unfinalized temp and whole-object Move flows retain their prior
behavior; no security descriptor is rewritten.

## Python baseline and precise contract

Frozen Python 3.10 `copy2` copies bytes, then `copystat` applies access/write times
and mode. On Windows `chmod` changes readonly only. It does not copy creation
time, the complete Windows attributes, owner/ACLs, NTFS streams or EAs. This is
not Python 3.12's different CopyFile2 implementation.

The native contract deliberately uses the actual metadata observed on the held
source handle after source-content reads. Direct `copy2(Path)` obtains a fresh
stat at copystat time, whereas `copytree` can reuse a `DirEntry` stat cached by
its earlier copyfile check. NTFS also defers last-access updates. Therefore the
receipt does not promise equality with a later closed-handle source stat or
Python's cached atime. Python 3.10 copytree also suppresses some Windows
postorder copystat errors; the native executor must report observed failure
instead of claiming a successful metadata operation.

`WindowsCopyMetadataEvidence` records source/destination full native IDs,
positive access/write FILETIME ticks, readonly, and the destination's retained
creation time and non-readonly attributes. The native adapter creates this
receipt; data supplied in a journal cannot grant metadata-write authority.
Zero/negative special FILETIME encodings are unsupported before effects rather
than treated as a successful timestamp copy.

## Files

After bytes have been written, synchronized and verified, call
`finalize_selected_copy_metadata(source, temp, cancel)`. It returns
`MutationOutcome<WindowsCopyMetadataEvidence>` and independently checks source,
temp identity, roles and content. It accepts only the ordinary selected Copy
source/temp domain, not repair staging or the write-through Move lease.

The optional private temp state is installed poisoned before the first flush
or metadata operation. Finalization flushes content, suppresses this held
handle's automatic access/write timestamp changes, sets explicit access/write
times, applies only the source readonly bit over the destination's retained
attributes, reasserts suppression, flushes, and verifies the exact handle.
Creation time, inherited security and other attributes are not intentionally
changed. Timestamp suppression's actual behavior after prior writes and final
handle close remains an explicit native-test gate, not an assumed guarantee.

Any metadata-bound temp refuses further content writes. A finalized or poisoned
temp cannot be finalized again. Before-dispatch cancellation/failure is
`NoEffect`; after dispatch uncertainty/cancellation is `PendingUnknown` with the
retained temp's observation. Source objects are never retired by this API.

The existing temp publication method dispatches the stricter copy-bound branch
only when this optional state exists. It verifies content plus metadata, uses
an exact synchronous same-volume no-replace rename, transfers the bound state
to the publication, and verifies again. Unexpected completion consumes the
possibly renamed handle rather than replaying. Generic publication inspection,
content verification and post-publication sync all check metadata for bound
copies. Their behavior for unbound ordinary/Fix objects is unchanged.

## Exclusive-created directories

`create_selected_copy_directory` performs native relative `FILE_CREATE` with
WRITE_ATTRIBUTES acquired upfront and returns `WindowsCreatedCopyDirectory`.
It cannot adopt a preexisting directory or a generic parent capability. Existing
files/directories are preserved on collision. Uncertain creation keeps the
created namespace entry and reports `PendingUnknown` without replay.

The token exposes its target parent capability for creating descendants.
`finalize_selected_copy_directory_metadata(source_parent, created, cancel)`
applies the same observed timestamps/readonly **postorder**, after descendant
namespace/content work. It never flushes a directory while inventing a global
namespace-durability acknowledgment. `verify_selected_copy_directory_metadata`
rechecks identity/location, creation time, attributes and requested timestamps.
The executor must stop namespace writes through previously cloned destination
parents after finalization; those clones are not revoked. Later edits that
change recorded metadata are verification failures, not evidence of completion.
An uncertain directory metadata step poisons the token and disables replay.

## Verification status and required evidence

Seventeen owned native cases cover actual selected-root Copy; readonly and
writable sources; exact held-handle and post-close times; destination creation,
non-readonly attributes and ACL preservation; write/finalization replay denial;
foreign temp and digest mismatch; all metadata cancellation/response-loss
boundaries; pre/post-publication tampering; no-replace collision; unbound temp
behavior; postorder exclusive-created directories; existing-directory refusal;
directory uncertainty/create loss; foreign tokens and source/target role checks.

The authoring environment has no Rust/Cargo or Windows runtime. Tests are
**authored, not compiled or passed**. No installation is retried. Independent
source review, actual NTFS runtime/Clippy, executor intent/evidence integration
and recovery validation remain required before activation. There is no Cargo
feature or dependency change in this increment.

```text
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib copy_metadata::tests -- --list
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib copy_metadata::tests -- --test-threads=1
cargo +1.99.0 clippy --locked -p sorter-io --features experimental-windows-mutation --all-targets -- -D warnings
```

### Primary sources

- [CPython 3.10 shutil source](https://github.com/python/cpython/blob/v3.10.0/Lib/shutil.py): copy2/copystat, DirEntry caching and copytree postorder error behavior.
- [Python 3.10 os.chmod](https://docs.python.org/3.10/library/os.html#os.chmod): Windows readonly-only mode behavior.
- [SetFileTime](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfiletime): held-handle WRITE_ATTRIBUTES, null/zero and all-ones semantics, deferred NTFS atime.
- [FILE_BASIC_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_basic_info): FILETIME fields, metadata change time, and zero attributes meaning no change.
- [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information): no-replace exact-handle rename.

## Read-only restart metadata inspection increment

`inspect_selected_copy_metadata(target_parent, leaf, expected_identity,
expected_kind)` returns `Result<Option<WindowsCopyMetadataObservation>, _>`.
It reads only the exact bound File/Directory leaf through a live target parent
capability, checking role/ancestry before and after. A serialized reference or
identity cannot reconstruct that capability. The file publication guard must
first be released because its share-denying handle intentionally prevents a
second open; failure to open is an error, not missing metadata success.

The result carries the actual object observation and kind, creation/access/write
FILETIME ticks, readonly and non-readonly attributes. It does not invent a source
identity. None is reserved for native NotFound. Foreign identity, wrong kind or
actual name mismatch returns a stale error containing actual object evidence;
reparses, access failures and unsupported representations remain errors. No
mutation, adoption, cleanup, replay or receipt-to-authority conversion is offered.

The executor must compare every stored metadata field and independently verify
file bytes. This is a read-only observation, not metadata exclusion: another
actor may edit attributes after the snapshot. Missing/error/mismatch must remain
unverified rather than treated as completed recovery evidence.

Six added owned cases cover file and directory receipt comparisons, held-file
sharing refusal, missing/foreign identities, junction refusal, exact spelling/stream
rejection and foreign/source parent rejection. The Copy metadata suite now has
23 authored native cases;
compile/runtime acceptance remains pending.
