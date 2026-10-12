# Selected Windows FixNames

The selected-root executor now has a separate one-shot `SelectedWindowsRepairJob`.
Its source and existing application journal directory are independent host grants.
The preview retains root identity, per-file content digests, policy plan and resource
bindings. Apply rechecks the complete plan and those bytes. Serialized records
are observations; they cannot deserialize into an execution grant.

## Current physical coverage

* Filename cleanup follows the released FixNames policy for all twelve released
  audio extensions. The planned no-replace rename happens before metadata work,
  and the exact original handle is rebound to the returned name.
* Existing ID3v2.3/v2.4 MP3 text repairs cover artist, album artist, title, album
  and genre in original order. The staged writer's documented flag, encoding,
  frame and padding limitations apply. Audio, unknown frames and artwork remain
  byte-identical. A changed file retains its original object as a visible backup
  beside it, with its exact identity and byte digest recorded in the job journal.
* Files with no changed tag values are counted as processed but do not receive a
  physical tag write or backup. `unchanged_files` means unchanged tags; names may
  still have changed. Rename and repair counters can overlap.
* Other formats and unsupported MP3 representations report an explicit partial
  filename-only outcome. Metadata-open failures retain the filename outcome,
  matching the released ordering, without writing partial in-memory repairs.

The native preservation layer validates ordinary local NTFS metadata and access
controls before any audio enters staging. Its refusal domains and timestamp
semantics are specified in `windows-fix-preservation.md`. Backups consume disk
space and remain indefinitely until a separately authorized recovery/cleanup
operation exists. The driver never deletes them or offers automatic replay.

## Ordering and interruption

Every rename, staging allocation, backup rename and publication has a flushed
journal intent first. Staging is content-verified, parsed by the native metadata
reader, metadata-finalized and flushed before the original becomes a backup.
The replacement publishes without overwrite, is flushed again on its held handle,
then receives a committed journal record. This uses the ordinary Windows/NTFS
flush contract; it does not claim hardware-perfect power-loss protection.

Cancellation before backup leaves the original at its current name. Cancellation
or uncertainty after backup reports recovery required and retains both objects.
A failed journal acknowledgment poisons the journal; no later effects follow.
The exact in-memory observations are reported even for partial results. Progress
never substitutes for a verified outcome.

Read-only reconciliation validates the hash chain and phase protocol, matches
freshly granted source roots, then checks recorded object identities and content.
A truncated final record is reported. It never resumes or deletes files. Journal
reads are bounded at 64 MiB. Fresh restart discovery/UI is a separate integration.

## Acceptance

The source adds twelve owned Windows driver cases and retains the staged writer and
native preservation suites. These require actual Windows CI and a selected
application build with native metadata. Source review or portable parsing alone
is not runtime acceptance. Host and CLI must show filename/tag counts separately,
partial unsupported outcomes and retained backup bytes. Packaging must refresh
and review its measured dependency/feature notice graph.

### Identity-bound selected-root spelling during recovery

Recovery keeps the original selected path and its no-reparse ancestor pins. It
also reads the normalized DOS path from that same held, identity-checked root
handle. That live observation may align an ancestor spelling such as Windows
TEMP's `RUNNER~1` with its long spelling. The alias is recovery-local and omitted
from journal serialization; a journal cannot supply or change it. Only the root
prefix is mapped. Relative child names, parent/file identities, sizes, phase
bindings, and content digests still match exactly. Other roots, volumes, device
aliases, traversal, and short-name aliases below the selected root are not
normalized. Recovery remains read-only and never creates execution authority.
