# Ordered direct-child album renames (native adapter)

This increment extends the separately reviewed whole-album rename adapter. It
implements only the exact, journalled child-name preparation needed before a
whole-container Move. It does not activate the executor, delete directories,
merge existing destinations, or claim metadata-identical v14 container creation.

## Authority and admission

`WindowsAlbumChildRename { source, destination, identity }` binds every direct
child, including unchanged leaves, in the caller's frozen planner order. The
native adapter checks complete exact-name/full-identity coverage, preserves that
order verbatim, and cannot independently certify the caller's v14 policy order.
The executor owns that proof and must journal each changed-child intent before
calling the native cursor. Empty/skipped destinations, unsafe names, omitted or
duplicate sources, and foreign identities fail acquisition without effects.

Acquisition verifies the same bounded full relative tree as whole-album rename.
Destination admission checks all changed leaves before the first effect:

- Every destination must be a valid lossless single-component Windows name.
- Native ordinal case-insensitive comparisons conservatively reject ambiguity
  between planned destinations, even on a case-sensitive directory. A fallible
  O(n log n) merge sort works on indices; the stored planner order is unchanged.
- Relative, noncreating opens reject every currently occupied destination,
  including aliases of existing children. Swaps, cycles, and case-only source
  aliases are not reinterpreted as a rename strategy.

Ordinal admission does **not** prove the absence of all NTFS short-name aliases,
future aliases or concurrent external additions. The kernel's no-replace result
remains authoritative. A later native failure stops the scope; previously
observed child changes remain in the original container and must be reported as
partial. No names are normalized and no replacement or rename replay is allowed.

## Same-parent native operation

The original album handle keeps the existing strict sharing mode (zero), with
its eventual DELETE rights acquired upfront. Every changed child is opened
relative to that handle, identity/name/kind checked, with DELETE + generic read,
write-through and zero sharing. File data is checked again through that child
handle. Descendant verification handles are closed before the directory rename.

Microsoft documents `FILE_RENAME_INFORMATION` with a simple leaf and NULL
`RootDirectory` as changing only the file's name within its current directory.
The operation uses exactly that form, with `ReplaceIfExists = false`. The child
handle denies DELETE sharing and its parent is pinned; no arbitrary target path
or mutable album-parent capability is exposed. Actual NTFS tests must establish
that this form works with the strict parent share mode. The code does not
silently relax sharing if a filesystem/filter rejects it.

After dispatch, the child handle is consumed by one `CloseHandle` attempt,
including when a post-rename response is lost. It is never reconstructed or
closed again after uncertainty. Full tree verification then reopens descendants
relative to the retained album handle. Successful verification updates the full
manifest prefix (including every descendant of a renamed directory) and advances
the original planner cursor. The returned observation includes the original
planner index, exact rename, actual child identity/location, and complete actual
tree observations. Nested objects, streams, EAs and security metadata remain in
the same objects; child access/read failures are reported honestly.

Cancellation before dispatch is `NoEffect`. Every attempted native rename stops
the scope until verified success. Lost/native uncertain completion, failed
post-verification, post-dispatch cancellation or a lost close response is
`PendingUnknown`; the stored manifest remains the last *verified* manifest and
is not evidence of the uncertain child's present location. The root stays held,
inspection stays bound to the original root, and neither child replay nor
container publication is available through a stopped scope.

## Public integration

Child albums use `acquire_album_directory_for_child_renames`, followed by
`rename_next_album_child`. The scope exposes the full ordered binding, next
changed index/item, last verified tree, `is_stopped`, and `is_complete`.
`inspect_original_album_child_rename_location` reports only the bound root.

A selected-root album uses the dedicated upfront
`for_selected_album_root_child_renames` constructor. Its session provides the
same cursor, target/journal access, original selected-source reference/identity
and original-location inspector. Source ancestors remain private; the session
cannot supply a general Source root or an ancestor grant.

`into_album_directory_lease` / `into_root_rename_session` consume a complete
scope without releasing/reacquiring its root handle. An incomplete or stopped
conversion returns the complete scope boxed in `Err`, preserving its pin. The
existing whole-container native operation re-verifies the now-current manifest
before publication. The executor must persist changed-child observed receipts
and the updated manifest before beginning that distinct container intent.

This remains an explicitly quiescent ordinary-use contract. Full-tree snapshots
are not exclusion leases and this increment makes no global durability or
power-loss promise. Generic file leases, ordinary root roles, Copy/Fix behavior
and the unconditional product directory-deletion gate remain unchanged.

## Feature and verification evidence

The only Cargo change is adding the existing `windows` dependency's
`Win32_Globalization` feature for `CompareStringOrdinal`. Version/lockfile and
other features are unchanged. Packaging must measure/review the feature's
notice closure before activation; source review alone does not establish it.

Seventeen new owned Windows tests cover file/directory child renames followed by
whole-container publication; original planner ordering and unchanged leaves;
actual selected-root conversion with usable journals; incomplete/uncertain root
pins; malformed coverage/identity/empty names; present/future ordinal aliases;
late membership/content/identity changes; pre/post cancellation; lost rename
and close responses with exactly one close; late native collision; foreign old
leaf; streams/EAs/nested readonly; and strict sharing. These are authored tests,
not local compile or runtime acceptance: Rust/Cargo and Windows are unavailable
in the authoring environment, and installation remains disallowed.

Run from `native` on the owned NTFS fixture runner:

```text
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib album_rename::child_rename::tests -- --list
cargo +1.99.0 test --locked -p sorter-io --features experimental-windows-mutation --lib album_rename::child_rename::tests -- --test-threads=1
cargo +1.99.0 clippy --locked -p sorter-io --features experimental-windows-mutation --all-targets -- -D warnings
```

Retain the separate whole-album suite (15 cases) and directory diagnostic suite
(20 cases), plus executor/product admission, journalling and recovery tests.

### Primary API references

- [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information): simple-name NULL-parent operation, relative target form, no-replace semantics, and open-descendant limitations.
- [CompareStringOrdinal](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-comparestringordinal): explicit-length UTF-16 comparison using the operating system uppercase table, not a volume alias guarantee.
- [windows-rs 0.62.2 binding](https://microsoft.github.io/windows-docs-rs/doc/windows/Win32/Globalization/fn.CompareStringOrdinal.html): exact existing dependency API signature.

## Read-only stopped-scope evidence increment

`verify_album_child_rename_tree` and the selected-root session's
`verify_current_tree` recheck the last **verified** manifest through the same
held album root, even when mutation has stopped. An uncertain dispatched child
rename can make that old manifest fail verification; these methods never infer
success or advance the cursor.

`inspect_album_child_rename_location(scope, planner_index, at_destination)` and
session `inspect_child_location` probe only the original binding's source or
destination leaf relative to the retained root. They return actual child
observations (including foreign replacement identities) or None only for native
NotFound. Invalid indices, reparses and access failures remain errors. Parent
identity is the bound album root's identity; child evidence must never be
recorded as album-root evidence. No new parent capability or caller-provided
path escapes, and inspection cannot unpoison the scope or authorize replay.

Four additional owned native cases cover verified/current tree observations,
stopped uncertain and foreign-child evidence, actual selected-root inspection,
and rejection of a bound-leaf junction without following it. The ordered scope
suite now contains 21 authored cases, still requiring actual compile/runtime.

## Complete-cursor publication without scope replacement

`rename_completed_album_children(&mut scope, target, name, cancel)` and the
selected-root child session's same-named method forward only a complete cursor
to the existing whole-directory operation. Incomplete/stopped scopes return
`NoEffect(StalePlan)` before forwarding. No lease conversion, root reopen or
scope replacement is necessary, including an all-unchanged child order.

The existing one-shot directory operation moves the held root handle into its
publication guard (or consumes it on uncertainty); the original scope keeps its
private source ancestry and original-location inspector. `is_complete` still
means the child cursor completed, not that a container publication occurred.
An uncertain forwarded container operation stops the scope. A second publication
attempt cannot replay a consumed root handle. Full-tree publication verification
continues through the publication guard; current-source-tree verification cannot
pretend that a consumed source handle remains at its original location.

Four additional owned cases cover complete changed/unchanged child publication,
incomplete/stopped refusal, selected-root forwarding with usable journals, and a
lost whole-container response with original-location inspection. There are now
25 authored ordered-scope cases, still pending Windows compile/runtime.
