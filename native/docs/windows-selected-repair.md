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
* Native FLAC repairs use the same released five-field policy and exact raw
  comma-space projection. Duplicate repaired comments collapse in policy order,
  so the replacement can be shorter than the original. Vendor, untouched
  comments (including their casing/order), pictures, standard metadata blocks
  and encoded audio remain byte-identical. Product dispatch rejects opaque
  APPLICATION/reserved blocks when their offsets would move; equal-length repair
  preserves private bytes without promising their semantics. Native reader
  projection mismatches and the stream writer's representation limits refuse
  repair before staging.
* Files with no changed tag values are counted as processed but do not receive a
  physical tag write or backup. `unchanged_files` means unchanged tags; names may
  still have changed. Rename and repair counters can overlap.
* Other formats and unsupported MP3/FLAC representations report an explicit partial
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
then receives a committed journal record.

Version 2 first runs the exact transformer into a discard sink to bind the
replacement SHA256, complete replacement length and unchanged-media SHA256 in the
flushed staging intent. The real staged write must produce identical evidence,
and held-file readback independently checks its hash/length. Original source and
backup evidence always retain the original length/hash. Publication intent,
replacement observations and lost-response candidates use only the replacement
length/hash. A partial unverified temp can be no longer than the planned result.
Recovery checks fresh file and stream-bound sizes as well as identities/hashes.

Version 1 journals remain read-only compatible, without rewriting their bytes or
pretending they contain pre-staging replacement hashes. Their replacement size
must still equal the original size. Missing, unknown or inconsistent version-2
staging evidence is rejected. Rehashed journal edits cannot turn mismatching
physical identities, sizes or bytes into verified recovery evidence. The chain
is an integrity/consistency check, not authentication or execution authority. This uses the ordinary Windows/NTFS
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

The original thirteen Windows driver cases are retained unchanged. Twelve FLAC
product/recovery cases cover shrink/equal-size repair, exact retained backups,
unchanged tags, unsupported opaque shifts, cancellation, lost backup/publication
responses, no-overwrite collision, foreign identical-byte replacement, version-1
compatibility and rehashed original/replacement length/hash substitutions. The
staged writer and native preservation suites remain mandatory. These require actual Windows CI and a selected
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

### Validation state for FLAC activation

This source change does not claim a local Windows execution result. The authoring
environment has no Rust toolchain, and prior installation denials were respected.
The four independent Python/FFmpeg stream fixture checks pass, but do not execute
Rust or the Windows backend. Exact normal-CI gates must pass for the frozen source,
including formatter/lints and the native FLAC metadata roundtrip. The current
policy supports equal-size and smaller output; no policy-valid growth fixture is
claimed. The primitive Windows metadata/ACL preservation implementation is not
changed by this activation.
