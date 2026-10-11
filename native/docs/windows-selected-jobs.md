# Selected Windows jobs

This is the product host boundary for local NTFS sorting. The host supplies three
explicitly selected root grants: source, target and application journal storage.
Each grant contains the path and the full identity observed by the host. Grants
are not deserializable. All three roots and their ancestors are pinned separately;
no common-ancestor permission is inferred. Existing directories are never owned
by TempDir or removed on drop. Same/nested roots, reparse points, identity changes,
non-local volumes and non-NTFS roots are rejected.

## Host sequence

1. Use `WindowsPlanningStorage::for_preview` for read-only planning. It establishes
   read/list/seek evidence through actual operations rather than inventing access.
2. Immediately capture `prepare_scope` and retain it with the displayed plan.
   It streams source digests with cancellation and verifies handle bindings and
   before/after observations. Never regenerate consent by hashing changed sources
   at apply time.
3. Reconstruct metadata/recognition from the same verified resources. Consume the
   live root grants, then call `SelectedWindowsJob::prepare`. Exact original policy
   is rerun and the original source content digests must still match.
4. Run once on a worker with the shared cancellation flag. Read the complete
   `JobResult`; attempted operations are not completed operations. A second run of
   the same owner is rejected, including after cancellation or response loss.
5. Retain the job ID, binding hash, original scope and each journal location's
   name/full identity. The application journal root is independent of music roots.
   Recovery uses a fresh host root grant and identity-checked read-only journal
   access. `inspect_recovery` never mutates, resumes, cleans up or retires sources.

## Supported milestone

Multiple single-file Copy operations and same-volume single-file rename Move,
including required artist/singles directory creation and recursive source paths.
The immutable original plan defines ordering, destinations, name collision policy,
copy mode and dependencies. Copy retains sources. Rename retains the exact known
source handle across no-replace publication with checked pre/post file flushes.

Missing destination directories use no-replace handle-relative creation. A separate
hash-chained directory journal synchronizes intent before dispatch and observed
identity afterwards. Returned directory guards remain held through all transfers.
Existing target parents are also retained. Unknown creation response or journal
failure stops before later transfers; created directories are never automatically
removed. Setup cancellation may therefore leave an empty, journaled directory.
Directory creation itself is ordinary NTFS namespace behavior, not a claim that a
directory-handle flush or independent power-loss-proof namespace barrier exists.

Transfers use the existing physical journal and checked publication sync. Selected
and fixture binding domains are disjoint. Generic durable-flush capability remains
Unknown: the narrower rename contract does not license copy-and-delete fallback.

## Remaining operation domains

Cross-volume Move, duet Move requiring copied-source retirement, album operations
and tag repair are explicitly unsupported by this milestone. They cannot silently
fall back to ordinary deletion, copies or metadata rewriting. The host must expose
the reason before apply. These are subsequent completion stages, not a declaration
that the requested complete application is finished.

## Durability and recovery

Assumptions are the documented ordinary local Windows/NTFS/file-buffer contract,
including an OS, driver and device that honor successful flush requests. No claim
is made of hardware-perfect power-loss survival. Existing selected root namespaces
are assumed durable before the job. Journals use exclusive creation and checked
file flushes; errors are not retried. A pending directory intent is evidence of an
uncertain effect, not evidence of absence or authorization to recreate it.

The directory recovery report is journal evidence; transfer recovery additionally
re-observes source/destination identities and contents under the selected roots.
No automatic replay or cleanup is provided. Partial/incomplete journals are kept.

## Verification status

Native Windows tests cover role-separated roots, overlap/stale identity rejection,
cleanup ownership, no-replace directory creation and lost response after creation.
Selected-job tests cover multiple files, Hebrew paths, artist creation, Copy/Move,
stale preview data, cancellation, duplicate-run refusal, collision preservation and
read-only recovery. Tests mutate only generated fixture trees even though they
exercise the product selected-root constructor. Local Cargo/rustfmt were unavailable
in the restored environment; actual passing CI is required before release acceptance.
