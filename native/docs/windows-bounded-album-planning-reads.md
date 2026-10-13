# Explicit bounded album planning enumeration

The album collector's whole-tree cap cannot protect an earlier unbounded
`ReadStorage::children` allocation. Directory capability probing also lists
children before the collector sees an entry. This increment adds an opt-in
bounded route at the native listing boundary and uses it for both paths.

## Native API

`DesktopStorage::children_bounded(reference, WindowsDirectoryReadLimits, cancel)`
checks cancellation before/after native batches and before/after exact child
observation. Native calls are synchronous; this cannot interrupt a stalled
kernel/filter call already in progress. One aligned 64-KiB native batch is parsed
at a time. Entry count and cumulative complete child-reference UTF-16 units are
checked before child observation/result growth. The batch limit also stops a
concurrently growing or otherwise nonterminating enumeration. Limits/cancellation
return errors and discard the partial vector, never a truncated successful list.

`WindowsDirectoryReadLimits::ALBUM_DEFAULT` and its `Default` implementation use
100,000 entries, 16 Mi UTF-16 path units, and 1,024 native calls per enumeration.
The count includes the call establishing EOF. Callers may lower these bounds;
zero entries/path units admits only empty results, zero native calls is invalid,
and callers cannot silently raise bounds beyond the supported default domain.
These are per-enumeration limits. The collector retains its separate aggregate
whole-tree count, path-length and depth accounting.

The bounded route uses the existing exact observation helper, full native IDs,
relative child opens, reparse representation and default capabilities. Names are
lossless in references, unchanged in display behavior, and returned in native
order. No sorting or policy-name normalization is introduced. Ordinary
`ReadStorage::children` remains uncapped and retains its previous behavior.
Its observation conversion is factored into the same helper without changing
its whole-directory enumeration or observation order.

## Planning adapter

`WindowsPlanningStorage::for_album_preview(source, target, limits, &cancel)` and
`::for_album(owned, source_reference, target_reference, limits, &cancel)` return
`WindowsAlbumPlanningStorage<'_>`. This dedicated wrapper borrows cancellation;
legacy planning/Fix/single-file adapter lifetimes and constructors are unchanged.
The owned constructor retains the existing checked native root pins, and neither
constructor grants new mutation authority.

Both wrapper `children` and directory `establish` capability probes use the
native bounded route. In particular, probing a root with an enormous directory,
or establishing a small parent entry whose own children are enormous, cannot
call the old unbounded directory routine. File read/seek probes and before/after
observation checks are unchanged, with additional cancellation checks. A failed
or over-budget probe cannot promote capabilities. References and reads remain
restricted to the same checked roots.

The host/executor must explicitly select these constructors for product album
preview/revalidation. Generic caller-provided mocks remain generic ReadStorage
inputs and do not grant product mutation authority. This increment intentionally
does not edit the album executor/collector or assert that the host has already
switched its constructor.

## Owned native verification

Eight new sorter-io cases cover unchanged order/identity/capability/reference
observations (including unpaired UTF-16), entry/path/batch limits, cancellation
inside batch/record processing, native child identity replacement, unsupported
limit expansion, and a junction observation without traversal. Five executor
cases cover exact legacy-equivalent capabilities/order, a root directory probe,
a nested directory probe, borrowed cancellation, and checked owned-root scope.

All 13 cases are authored, not compiled or run: the authoring environment has no
Rust/Cargo or Windows runtime, and no tool installation is retried. Required
Windows commands (from `native`) include:

```text
cargo +1.99.0 test --locked -p sorter-io --lib album_bounded -- --list
cargo +1.99.0 test --locked -p sorter-io --lib album_bounded -- --test-threads=1
cargo +1.99.0 test --locked -p sorter-executor --features experimental-windows-copy --test windows_album_reads -- --test-threads=1
cargo +1.99.0 clippy --locked -p sorter-executor --features selected-windows-execution --all-targets -- -D warnings
```

No Cargo features/dependencies or mutation capabilities change. Source review
and native runtime are distinct gates; the bounds are not an exclusion lease or
an atomic directory snapshot.
