# Windows album directory inventory and empty-child cleanup

The product surface provides bounded read-only inventory and an auditable
`NoEffect(Unsupported)` gate for destructive empty-directory cleanup. The gate
performs no DELETE open or disposition in any authority domain. Ordinary
Copy/Move/Fix capabilities are unchanged.

Actual directory-disposition code and its injection checkpoints live only in
`windows_album_diagnostics.rs`, a `cfg(test)` module that additionally requires
owned synthetic-tree authority. They are OS-behavior experiments, not an admitted
production cleanup path. Selected source, target, repair and journal trees never
enter that diagnostic deletion code.

A review of the initial draft found a preservation gap: kernel emptiness protects
child entries, but does not exclude an ADS/EA added after the final userspace
metadata check. Repeated checks and a share-denying directory handle do not prove
metadata exclusion. The product therefore retains empty source directories and
must report incomplete/unsupported cleanup. No hidden backup root, path-delete
fallback or capability promotion is introduced.

## Inventory contract

`source_directory_members` returns lossless UTF-16 names, volume plus 128-bit
file identity, and entry kind from `FILE_ID_EXTD_DIR_INFO` records. It never
reopens child files. Consequently source file leases may all remain held with
read/write/delete sharing denied during inventory.

A normal directory pin has traverse/attribute access. Each inventory call opens
one listing handle relative to the held upstream directory, checks its full
identity against the pin, and enumerates directly through that handle with an
independent cursor. Both the directory and its pinned ancestry are rechecked for
reparse conversion. Missing identities and duplicate exact names fail closed.
Reparse children are reported as such without traversal; an executor must reject
unsupported planned members before mutation.

Each album-only scan admits at most 100,000 entries and 1,024 native 64-KiB
batches (64 MiB of returned buffer capacity). Exceeding either limit discards all
partial results. `source_directory_members_with_cancel` checks cancellation
between native calls and before returning; a synchronous kernel call is not
interrupted by that flag. The original method uses the same limits without an
external cancellation flag. General desktop read behavior is unchanged.

This is a bounded sequence of directory observations, not a membership exclusion
lease, directory oplock, or atomic snapshot. Concurrent additions/deletions can
occur between enumeration calls or after the final check. The album driver must
compare the exact planned remaining names, kinds and identities before every
effect and stop on a discrepancy. The final cleanup also relies on the kernel's
nonempty-directory rejection; an earlier empty listing is insufficient.

## Owned-fixture disposition diagnostic

The test-only `remove_empty_source_directory_with` accepts a source parent capability, one child
name, and the full expected identity. The caller releases every guard on the
child before this call while keeping the upstream parent pinned. The experimental sequence:

1. Checks cancellation, authority and name scope, then uses `NtCreateFile` with
   `FILE_OPEN`, `DELETE | FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE`,
   no sharing, `FILE_DIRECTORY_FILE`, `FILE_OPEN_REPARSE_POINT`, and
   `FILE_SYNCHRONOUS_IO_NONALERT`. It never uses `DELETE_ON_CLOSE` while opening.
2. Requires an ordinary non-reparse, case-insensitive directory with matching
   identity and actual name. Readonly and unknown representation flags are
   rejected. It refuses all directory `$DATA` streams and all extended attributes.
   It inventories children without opening them and requires an empty result.
3. Rechecks ancestry, representation and emptiness immediately before dispatch,
   then checks cancellation. Share denial does not purport to lock attributes,
   EAs or ACLs. These are checked observations, not hostile-concurrency exclusion.
4. Applies only `FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS`
   through the exact held directory handle. No readonly override, weaker-flag
   fallback, recursive removal, or pathname deletion exists. Native failure is
   returned as `NoEffect`; an unsupported OS/filesystem is not silently accepted.
5. Consumes that handle with exactly one `CloseHandle`. After accepted disposition,
   cancellation, missing acknowledgment, close uncertainty or verification error
   is `PendingUnknown`, carrying the original observation. It never retries close
   or reconstructs ownership from the possibly recycled numeric handle.
6. Reads the old child link under the still-pinned parent. Genuine NotFound or an
   observed different identity establishes removal of the original link. A
   foreign replacement is never mutated. Access/reparse errors and an unchanged
   identity are inconclusive and remain pending.

Within this owned-only diagnostic, `Applied` means verified live namespace removal.
It is not a product source-preservation result: a compatible metadata-only EA
write can occur after the final check. The public cleanup method never dispatches
this sequence, even for an owned tree.

Observed live removal does not establish directory
flush durability, power-cut survival, perfect device behavior, or permission to
retry the deletion. The caller must persist intent before dispatch and preserve
uncertain outcomes for read-only reconciliation.

The stream guard deliberately uses the documented handle-based
[`GetFileInformationByHandleEx(FileStreamInfo)` no-stream error](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex).
Only `ERROR_HANDLE_EOF` establishes no returned streams; returned records, buffer
errors, unsupported calls and permission failures do not.
[Directories have no unnamed `$DATA` stream](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-findfirststreamw),
but may contain named streams even when their child listing is empty. This is
why the draft assumption that native stream queries must return success with
zero bytes was removed. The implementation does not call pathname-based stream
search APIs.

Microsoft documents the
[synchronous native open options](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-zwcreatefile),
[POSIX disposition's close boundary](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_disposition_information_ex),
and the
[kernel nonempty and readonly checks](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fsa/4c26c8cb-5a8f-4339-a372-b315c4974131).
Those API contracts justify the sequence; they do not promise atomicity for its
multiple metadata observations.

## Unsupported selected-root terminal cleanup

A selected source root is still pinned in `Root.directory` and shared by live
`Arc` capabilities. Its ancestor is an identity pin, not authorization to mutate
arbitrary children there. The root was opened without DELETE access, and its
no-share-delete handle prevents independently acquiring DELETE access. Dropping
pins and reopening the path would introduce an unprotected identity gap.

Therefore this API removes only source descendants. It never turns a selected
root's upstream pin into a general source-parent grant or silently releases all
root guards to delete the root. If original album semantics require removing the
selected source root itself, that terminal step remains unsupported. A driver
must report the retained selected root as incomplete/unsupported cleanup rather
than claim the entire album operation is complete. Supporting that terminal
step requires a separately reviewed consuming root capability and lifecycle.

## Native test matrix and current evidence

Twenty `album_directory_guard_tests` run only on fresh owned `TempDir` fixtures:

- Inventory with all source file leases held, lossless Unicode/unpaired UTF-16,
  full identities, child kinds, and independent repeated cursors
- Late child add/remove, same-name replacement identity, nested/root enumeration,
  multi-buffer inventory and blocked parent replacement
- Real owned junctions: report child kind without traversal and reject converted
  pinned directories/parents
- Ordinary empty cleanup and direct no-stream API behavior
- Nonempty preservation and a child added after the last userspace empty check,
  forcing the native kernel emptiness guard
- Held child-guard refusal and successful cleanup after its last guard is dropped
- Existing named streams and EAs, late metadata mutation either excluded by OS
  sharing or rejected by the repeated guard, and readonly preservation
- Foreign identity and reparse refusal; replacement after close remains intact
- Cancellation before dispatch versus after disposition/close
- Lost disposition/close/observation response with exactly one close and no replay
- Selected source/target/repair role and root-boundary enforcement
- Inventory entry/batch caps and pre-scan cancellation with no partial return
- Public cleanup always remains gated, including a completely ordinary empty tree
- Strict compatible-share late-EA insertion at `BeforeDisposition`, followed by
  an explicit diagnostic abort before deletion; this is not skipped on failure
- Compatible-share late-ADS insertion probe at the same boundary, preserving it
  if the OS permits insertion and requiring a sharing violation otherwise

Private injection checkpoints surround real native calls. A lost close-response
fixture performs the real close before dropping its acknowledgment; it does not
pass an invalid or recycled handle to Windows. Tests do not simulate an actual
kernel crash or storage power loss.

Required Windows checks on the exact candidate tree:

```
cargo +1.99.0 clippy --locked -p sorter-io --features experimental-windows-mutation --all-targets -- -D warnings
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib album_directory_guard_tests -- --list
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib album_directory_guard_tests -- --test-threads=1
```

The list gate must count 20 nonzero native cases. These checks have not run in the
current Linux editing environment: Rust/Cargo and a Windows runtime are absent.
Whitespace/source review is not compiler or native execution evidence. Existing
selected MP3 Fix validation retains priority; no unrelated path is weakened to
make this candidate pass.
