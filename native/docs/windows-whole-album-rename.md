# Whole-object Windows album Move

This separate native execution path moves an already-existing album directory as
one object. It is admitted only by a host that authorizes ordinary use with no
concurrent external album writers. Snapshot checks detect observed differences;
they are not a directory exclusion lock, oplock, transaction, or hostile-writer
preservation proof. Ordinary file leases and generic mutation capabilities remain
unchanged, and destructive empty-directory cleanup remains disabled.

The executor must also establish that the frozen v14 policy leaves every
relative descendant name unchanged and that the destination album is absent.
Existing-target merge/replacement, cross-volume copy-and-delete, changing child
names, reparse descendants, multiply-linked files, and tag editing are excluded.
The separate fallback may transfer known files while retaining source directories;
it must not turn that partial result into an all-complete album receipt.

## Container semantics

The complete original directory object is renamed. Child objects, named streams,
EAs and the original container metadata stay with that object. This deliberately
preserves the source container instead of reproducing v14's freshly-created
container metadata. Sorting names and policy counters remain the executor's
responsibility; this implementation does not claim metadata-identical v14
container behavior. The move may place preserved security metadata in a different
parent context. Post-move relative open/content verification must succeed before
a verified result is returned; access failure is pending, never rollback.

## Data and authority

`WindowsAlbumTreeEntry` is data only: a nonempty vector of lossless relative
names, full Windows identity, kind, and file digest including length. Directories
have no digest. The manifest must contain every intermediate directory and every
child, have no duplicate paths, use the root's volume, and stay within 100,000
entries, depth 128 and 16 MiB of UTF-16 path units. Its input order is immaterial;
member sets are compared in exact UTF-16 order, independently of policy ordering.

`WindowsAlbumDirectoryLease` is a non-cloneable one-shot capability. It exposes
only its actual source observation. It is not a `MutationStorage` source lease
and cannot be constructed from serialized planning evidence.

For an album child, `WindowsMutationStorage::acquire_album_directory` receives
an already-authorized source parent, exact leaf, expected full root identity and
manifest. It opens the directory once with eventual DELETE, LIST, READ_ATTRIBUTES
and SYNCHRONIZE access, no sharing, native WRITE_THROUGH, directory-only,
no-reparse and synchronous flags. Every planning pin and file lease in this album
must be released first. Other cohorts keep their ordinary leases.

For a selected source that is itself the album,
`for_selected_album_root_rename` consumes the three explicit selected root grants
before any generic source-root pins are acquired. It pins only the source's
ancestors, opens the exact granted root with eventual rename access, and validates
the granted identity, local volume GUID, NTFS and complete tree. It requires
source, target and journal roots to be pairwise disjoint. There is no release-and-
reopen transition through an unprotected source identity, and no common ancestor
is exposed as general source authority.

The returned `WindowsAlbumRootRenameSession` privately holds that source lease
and exposes the original selected source reference/identity plus a separate
`target_and_journal()` capability. That capability has no Source root. The
session's target/journal access remains usable after rename, including appending
and rereading the operation journal. Dropping the session never deletes a selected
path. Existing `for_selected_roots` and repair domains are unchanged.

## Full-tree verification and rename

1. Inventory each directory through its exact held handle, bounded by the
   existing album-only entry/native-batch limits and cancellation checks.
2. Compare every immediate name, kind and full identity with the manifest.
   Descendant opens are relative to that directory handle; no full-path reopen
   is used for authority or reading. Reparse conversion is refused.
3. Read each ordinary file's exact expected length and SHA-256 through a temporary
   read handle, denying competing file writes during that read. EOF is checked
   with a one-byte probe; a growing file cannot extend an unbounded hash loop.
   All descendant handles close before the native root rename.
4. Immediately before rename, recheck source/target authority, root identity/name,
   same-volume evidence, target-not-inside-source, and the complete relative tree.
   Cancel before dispatch is NoEffect.
5. Submit `NtSetInformationFile(FileRenameInformation)` with the exact root
   handle, held target-directory handle, one validated leaf, and
   `ReplaceIfExists = false`. Unsupported behavior/collision is not retried with
   weaker flags. A non-successful or inconsistent completion cannot be replayed.
6. Once rename succeeds, consume the source lease before any fallible callback,
   observation or cancellation. Observe the same directory handle at its new
   name, repeat all relative tree/identity/content/access checks, and return
   actual root plus child observations in `WindowsAlbumPublication`.

Publication keeps the renamed root and target ancestry pinned until the executor
finishes its dedicated album journal phases. This is one directory syscall, not
fabricated per-file rename receipts. After dispatch, cancellation, response loss,
changed membership/content, unusable descendant access or failed observation is
`PendingUnknown`. No guessed-path removal, rollback, reverse rename, source
retirement, overwrite, directory merge, or automatic replay exists. An unrelated
object later created at the old source name is untouched.

## Bound read-only source-location inspection

`inspect_original_album_location` for child leases and
`WindowsAlbumRootRenameSession::inspect_original_location` query only the
original bound leaf under its retained parent. While the lease is live, the
exact held root supplies the observation. After consumption, a relative
attribute-only open supplies current evidence. Only actual NotFound returns
`None`; permission/sharing errors are not absence and reparses are refused.
A foreign replacement returns its actual identity without mutation. Source-root
inspection never exposes an ancestor as a general grant or requires constructing
a desktop-storage root at the source path that has just disappeared. These
observations permit read-only reconciliation, not resume or replay.

`selected_target_observed_reference()` returns canonical spelling from the held,
identity-checked selected target root, with ancestry checked before and after.
The executor may record it as a verified alias for comparing returned descendant
references (including original 8.3 TEMP spelling). It is not an arbitrary-path
canonicalizer or new grant. Recovery continues through the originally bound
selected target reference.

## Windows API evidence and durability limits

Microsoft's [`FILE_RENAME_INFORMATION` contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)
documents same-volume rename, no replacement when requested, relative target
handles and the restriction on open descendant files for directory rename. This
is why planning handles must close instead of weakening ordinary source leases.

The source is opened with native
[`FILE_WRITE_THROUGH` and synchronous I/O](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-zwcreatefile).
Microsoft documents that a Win32 write-through request causes
[NTFS metadata changes including rename to be flushed](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/ns-fileapi-createfile2_extended_parameters).
Applying that metadata behavior to the corresponding native source option is an
explicit engineering inference under ordinary Windows/NTFS/driver/device
assumptions. No directory `FlushFileBuffers` acknowledgment is fabricated.
`WindowsAlbumPublication` proves the observed live root/tree result; it does not
promote `durable_flush`, prove power-cut recovery, certify hardware caches, or
establish an atomic transaction spanning the application journal and namespace.

## Owned native tests and verification status

Fifteen Windows-only cases use fresh owned temporary trees, including actual
SelectedRootGrant and selected-source-root sessions:

- Unordered manifest, nested children, exact root/file identity, content, directory
  and file ADS, and directory/file EAs retained by one rename
- Actual selected source root moved while target/journal access remains usable;
  held publication survives journal completion and later verification
- Existing file/directory destination collision without merging or replacement
- Late pre-dispatch membership/content/identity mismatch with no rename
- Last-instant external child addition retained by the whole-object move and
  reported pending when the post-snapshot differs
- Cancellation before and after dispatch, lost response, consumed-lease replay
  refusal and intact data at the moved location
- Original child pins/file leases remain strict; unrelated cohort lease stays held
- Reparse descendants and a foreign root are refused without outside traversal
- Malformed/incomplete/duplicate manifests and wrong digests
- Foreign replacement at the old source name remains untouched
- Post-move inaccessible child produces PendingUnknown with all data retained
- Selected-root overlap/acquisition failure and authority drop preserve paths
- Bound original-location inspection before/after child and selected-root moves,
  genuine absence, foreign file/directory replacement and reparse refusal

```
cargo +1.99.0 clippy --locked -p sorter-io --features experimental-windows-mutation --all-targets -- -D warnings
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib album_rename::tests -- --list
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib album_rename::tests -- --test-threads=1
```

The native list must contain 15 nonzero cases. These are source-level authored
tests, not passed runtime evidence in the current environment: Rust/Cargo and
Windows execution are unavailable here. `git diff --check` and independent source
review do not replace compile/Clippy/native CI on the exact candidate tree.
