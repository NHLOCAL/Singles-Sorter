# Android retained Album execution

## Scope and source authority

This source slice extends the existing Sort execution owner. It is not independent
sorting policy. `sorter_core::job_plan::album_projection::project_album_operations`
borrows the retained original AlbumTransfer operations, ordered direct children,
planned destination names, dependencies, predicted failures and original flags.
Java validates the projection against the retained plan and never extracts a
planned destination name from a StorageRef or provider document ID.

Sort captures a complete, bounded source listing **before** original planning.
The original `main_folder_only` setting remains unchanged: the original lazy
planner can inspect a direct child Album and its nested nonaudio content even
when recursive song discovery is disabled. `directory_children_complete` is
explicit in the private manifest. Missing listings are refused, never treated as
empty. Fix's root-only capture remains separate. After planning, Album subtree
metadata/membership is rechecked and every retained descendant file is hashed.

Those preview observations are not effect authority. Apply requires the fixed
system `com.android.externalstorage.documents` provider, selected persisted grants,
bidirectional disjoint-root checks, fresh source/target membership, exact names,
regular file handles and fresh byte hashes under the existing explicit
no-other-writer interval. Metadata/hash equality is not inode identity or CAS.

## Effects and stopping

- Original Single and Album operations execute in original operation-ID order.
- Original Album children execute in the borrowed child order. Descendant copying
  preserves captured names verbatim; it adds no sorting or filename policy.
- Copy creates every file under an owned `.bin` stage and then renames it to
  the exact frozen name, including nested members; provider MIME extension
  normalization cannot silently choose a new final name. Copy verifies the complete subtree, then
  publishes with provider rename. Exact planned names are required; auto-suffix
  results are retained as mismatches.
- Move uses provider move and, when the original child name changes, an owned
  staging directory plus rename and a final move. It never substitutes copy/delete.
- An original predicted failure remains a skipped child. An unsafe predicted
  success remains blocked, stops the dependent sequence, and never gets a success
  receipt. If no children are eligible, no destination directory is created.
- Original `remove_empty_source` is honored only after all captured direct children
  were covered and successfully moved, and a fresh complete listing proves the
  exact source directory is empty. Copy and skipped-child Albums keep the source.
- There is no rollback. Cancellation, uncertain returns or evidence mismatch stops
  the job with all recorded intent, return and observations retained.

Album directory admission also checks reverse ancestry against retained provider
namespaces. An Album cannot move or erase an ancestor of an original Fix backup.
Completed and interrupted Fix `original.backup` scopes remain reserved. Only a
fully reconstructed temporary-publication/cleanup certificate plus fresh complete
parent absence can release a temporary Album stage namespace.

Selecting an Album itself supports Copy and original operations that preserve the
selected root. If the original plan requires removing that selected picker root,
admission initially refuses with `ALBUM_SELECTED_ROOT_REMOVAL_UNAVAILABLE`. Only
then does the Android review show an explicitly labeled parent-folder picker with
its limited purpose. A successful selection invalidates the old preview and starts
a fresh original Sort preview with the same selected source, target and settings.
The returned parent token is never substituted for either original root. Cancelled
selection preserves the original preview, and no grant is inferred from an ID.

The supplemental private `album_root_parent` evidence binds the exact original
source, parent picker tree, parent metadata and complete bounded immediate-child
membership. It does not enter or alter the original Rust plan/projection, and it
does not enumerate sibling subtrees. Fixed system-provider persisted RW grants,
checked bidirectional ancestry and exact direct-child membership are required.
Parent evidence is capped at 256 KiB and 2,000 immediate children within the common
private retention budget. Before selected-root deletion, the current complete
source listing must be empty and the exact parent membership must still match.
After deletion, the full parent membership must equal the captured membership
minus that source only. A changed, missing, duplicate or foreign member stops.

This is not an atomic child-safe deletion primitive. The existing explicit
no-other-writer interval remains required. All source/destination hashes and
membership checks still apply. The original remove flag, successful child coverage
and nonzero eligible count remain prerequisites; zero eligible children never
cause destination creation or source deletion. No rollback is attempted.

UI file/copy totals use verified descendant file counts. Direct-child publication
counts and directory counts are separate diagnostics. An empty no-destination
Album is never reported as a completed Album.

## Receipt and recovery contract

Album-containing Sort journals declare `album_protocol: 1` and retain the exact
private `album_projection`. Existing pure-Single and Fix journals retain their
prior parser paths. Album events are explicit:

- `album-start`, optional existing `album-parent-observed`
- `operation-N/album-step-S` intent/returned/observed or terminal effect-mismatch
- ordered `album-child-start`, `album-child-destination`, `album-child-verified`
  or explicit `album-child-skipped`
- compact `album-verified` counters with `children_count`

Each effect declares its action, role, child, exact references and captured node
where relevant. The reconstruction state machine checks the original operation,
child, destination, mode, tree binding, owned staging shape and required cleanup
sequence. Partial prefixes are readable but confer no historical acknowledgment
or retry authority. The final receipt does not repeat an unbounded child array.
Live results retain ordered per-child evidence, including published destinations
when subsequent stage cleanup failed.

Current recovery inspection remains read-only. It can inspect recorded Album
roots and matching named descendants within the selected grants, compare current
file hashes, and report incomplete/different listings. A matching current file or
name never changes an unacknowledged historical outcome. Explicit retention
acceptance still requires a fresh matching inspection and creates a new receipt;
it never deletes a stage, retries a move or rewrites original history.

Selected-root removal uses the explicit `source-album-root` cleanup role and an
exact-parent-membership-after observation. Interrupted root removal requires a
fresh explicitly labeled read-only parent picker for the exact journal. Inspection
keeps the original source authority, resolves only recorded candidates through the
selected parent and displays source-present, absent, reused-name or changed-identity
uncertainty. It never traverses unrelated sibling subtrees. Retention consent uses
a separate parent-review policy, rechecks the same explicit choice, and binds it to
the exact original history; old Single/Fix acceptance is unchanged. An uncertain
root cleanup does not inflate the UI's count of retained source directories.

## Bounds

Capture retains at most 2,000 references, depth 128, 64 MiB per file and 512 MiB of
snapshot bytes. Original plan/projection/references/bindings share an 8 MiB private
retention ceiling. Mixed Album jobs are admitted below the existing 10,000-event,
8 MiB state and 64 KiB event ceilings using conservative event/URI estimates.
Parsing, tree walking, hashing and provider enumeration check cancellation. A
provider exceeding any observed bound is refused without blind cleanup.

## Validation and remaining acceptance gates

The strict Java gate runs `AlbumSnapshotEvidenceTest` (20 host checks).
`AlbumPlanJsonTest` includes selected-root, exact-unit UI, and retained-plan JSON checks. `AlbumJournalJsonTest` exercises
all valid receipt prefixes and malformed direct/nested copy, staged and unchanged
Move, skipped/empty Album, and mixed Single/Album histories. Both JSON mains must
be executed on the owned Android runtime; SDK compilation alone is not proof.
`AlbumParentJsonTest` adds selected-root authority, both Move receipt families and
all prefixes, parent-list changes, cancellation, explicit review binding and
uncertain-retention tests. The portable gate separately checks the parent-review
warning and diagnostic strings. The UI adds bounded Android-only transport and
picker ownership/fresh-preview tests. Its mocked Android browser renderer gate
must run in an environment permitted to launch Chromium; a blocked browser launch
does not constitute visual or device evidence.

The actual product-feature Rust gate includes original-planner main-only Album
and nested nonaudio coverage, unknown-versus-empty listing refusal, and the shared
projection's original frozen-plan tests. No model, SDK, native build output or
Python runtime is included in source.

Before product acceptance, the sole device owner must test an exact native/UI/APK
source binding: direct/nested copy; direct/renamed Move; original main-only mode;
provider suffix collisions; stale membership/content; revoked/overlapping grants;
partial/skipped and no-eligible children; every owned staging interruption;
relaunch inspection/explicit retention; repeated jobs; and Album ancestors of
both completed and interrupted Fix backup namespaces. Independent source review
and actual provider proof remain required; this document is not a device claim.
