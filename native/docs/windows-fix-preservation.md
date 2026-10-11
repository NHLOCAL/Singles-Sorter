# Windows FixNames ordinary-file preservation boundary

The repair adapter stages new tag bytes only after checking the exact held source
and the empty, exclusively created staging object. It does not change ACLs,
privileges, file sharing, or user-selected root permissions to make a repair work.
The source, staging file, retained original, and publication are all identity-bound
native handles under explicitly authorized, pinned NTFS parents.

## Required call order

1. `acquire_source(parent, expected)` obtains the existing write-through source
   lease. It remains suitable for same-object filename-only cleanup.
2. Apply the planned filename-first `rename_original`, if needed. Its result
   rebinds the same held file and carries any existing preservation snapshot.
3. Only when actual tag fields will change, call
   `prepare_metadata_preservation(&mut source)`. Unsupported representation
   errors here do not undo or hide an already completed filename rename.
4. `create_staged(&source, correlation, cancel)` derives the source parent. It
   revalidates the source and creates an empty no-replace temp there. Before
   returning a writable guard it compares source/staged owner, group, exact DACL
   bytes and relevant descriptor control flags, including DACL protection and
   inheritance. A mismatch never yields a writable guard. A rejected created
   object is retained empty and returned as `PendingUnknown` with actual identity,
   name, and size, not misreported as a side-effect-free rejection.
5. Stream the repaired content, flush it, verify its digest and requested fields.
   Each write rechecks the staged descriptor/representation and the original
   through a duplicate of the same held source handle. `observe_staged` obtains
   fresh identity/name/size evidence after a partial write or cancellation.
6. `finalize_staged(&source, &mut temp)` checks source/temp binding, flushes content,
   restores the supported attributes and creation/access timestamps, sets the
   changed content's last-write time, suppresses later automatic access/write
   timestamp updates from these handles, flushes again and verifies the result.
   It prohibits subsequent content writes. Dispatched metadata/flush failures
   poison the staging guard and retain the object for inspection.
7. Persist staged verification evidence before `preserve_original`. Immediately
   before its no-replace native rename and again afterward, that method rechecks
   source identity, security, attributes, creation/last-write times and streams.
   It retains the entire original object at its same-parent backup name.
8. `publish_staged(&mut temp, &backup, parent, name, cancel)` requires the matching
   retained-original capability and finalized staging state. It checks both held
   objects, performs no-replace publication, then checks them again. All failures
   after a backup was made, including cancellation/collision/preflight rejection,
   report `PendingUnknown` with actual retained-object evidence. There is no
   replacement of an occupant, source unlink, rollback guess, or backup cleanup.
9. `sync_published` checks ordinary preservation and the repaired digest around
   the existing same-handle file-buffer flush. `WindowsPublished::observation`
   exposes the native result even when a later verification fails. Persist the
   verified/committed journal result and retain backup/publication guards for the
   caller's required transaction lifetime.

A successful file-buffer flush is not a directory/namespace barrier and is not a
power-loss guarantee. A crash can leave the original name temporarily absent
between backup and publication. Recovery remains inspection-only, never replay.

## Accepted and rejected metadata

The initial staging domain supports ordinary single-link local NTFS files with:

- equal owner SID, group SID, exact DACL bytes (including ACE order/flags), and
  owner/group/DACL defaulted and DACL inheritance/protection control bits;
- ordinary `ARCHIVE`, `HIDDEN`, `SYSTEM`, and `NOT_CONTENT_INDEXED` attributes;
- positive creation and access timestamps that can be represented without Win32
  setter sentinel meanings; and
- exactly the unnamed `::$DATA` stream.

The DACL comparison deliberately favors false rejections over silently widening
access. In particular, a restrictive/protected original is not staged under a
wider directory-inherited ACL. It is not an effective-access equivalence solver.

Readonly, EFS/encrypted, sparse, compressed, offline/recall/integrity and unknown
attribute representations fail closed before audio bytes are written. The temp
is checked too, because its parent can impose compression/encryption. Reparse
and multi-link objects remain rejected by the common native adapter. Named
streams, including `Zone.Identifier`, are explicitly unsupported for tag repair.
Held-handle `FileStreamInfo` enumeration is bounded to 1 MiB and fails closed on
query failure or malformed/ambiguous output. Filename-only same-object renames
can still retain those streams because they do not create a replacement object.

Creation time and the access time captured when preservation is prepared are
restored on the visible repaired file. Source reading/preview can itself affect
last-access time; this is not an assertion about an earlier pre-preview value.
For changed tag content, the visible file's mtime is explicitly the wall-clock
completion time of staging, matching the intent of a content modification rather
than copying the old mtime. The retained original keeps its old last-write time.
NTFS change time and new file identity are not copied.

The supported security comparison is owner/group/DACL, not a claim to preserve
privileged auditing SACLs, arbitrary resource-security policy, NTFS object IDs,
extended attributes, or every metadata feature. No SACL-read privilege is enabled.
These additional representations require separate native acceptance and support;
this implementation must not be described as universal file-security cloning.

## Concurrent-change boundary

A share-denying **main-stream** handle does not establish a global lock over all
named streams or security descriptors. The repeated held-handle checks catch
observed changes before staging, before each write, during finalization, at backup,
and before/after publication/sync. Windows does not provide an atomic transaction
covering this comparison plus independent ACL/ADS changes. Another authorized
actor may change security/streams immediately after a check, including after the
last check. Such changes are outside the exclusion guarantee. No claim is made
that a main-stream lease prevents them. The whole-object original backup remains
available; detected changes halt completion and require inspection.

## Journal bounds and verification status

Both selected-root domains read an identity-checked held journal through a 64 MiB
bound, enforced on initial native length and on the stream read. This also applies
to the selected journal writer's readback. Synthetic-only journal behavior is
unchanged. Serialized journal/root data is evidence, never mutation consent.

Owned-TempDir native tests cover inherited-security success; restrictive/protected
ACL mismatch with an empty retained temp; source, temp and backup ACL changes;
ADS rejection and native late-stream change attempts; attribute policy including
EFS/sparse/compression rejection; actual creation/access/mtime restoration and
reopen checks; source/backup identity binding; cancellation; collision; retained
original bytes; and selected-journal size/identity checks. Attribute-bit rejection
is not a claim of EFS conversion testing. ACL edits affect only generated test
files. No real user library fixture is opened or modified.

At implementation time the available executor had no usable Rust/Windows
execution toolchain. These new tests were written but not run there; native
Windows CI is required before claiming acceptance. No toolchain-install denial
was retried or bypassed.

## API references

- [GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo)
- [Security descriptor control flags](https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-control)
- [FILE_STREAM_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_stream_info)
- [SetFileTime](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfiletime)
- [windows 0.62.2 GetSecurityInfo binding](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Security/Authorization/fn.GetSecurityInfo.html)
