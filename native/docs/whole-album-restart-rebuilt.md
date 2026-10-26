# Whole-Album read-only restart inspection

This freshly reconstructed implementation is separate from File-Album restart.
It does not recover the lost, unpublished source objects. Each new source slice
and its native tests require a new review and actual Windows acceptance.

## Evidence and authority

A strict, bounded data-only descriptor replays the existing Whole journal and
retains the exact atomic album operations, main-file hashes, directory membership,
original native root/parent identities, changed-child expectations, and any genuine
deferred-single binding. It exposes no completion or filesystem authority.

The caller explicitly selects either a present source root or the original
containing parent of an absent selected source root, plus fresh target and journal
roots. Missing-root inspection requires the recorded native original leaf and
full identity. No path canonicalization can turn an absent path into authority.
The read owner verifies retained journal identity, length and content digest,
then rereads the exact bytes when consuming its fixed source/target ledgers.
It retains read-only handles only and cannot replay, rename, clean up or resume
execution.

Fresh native source and target spellings are checked against held identity-bound
roots. Whole destination directories and their ancestry are pinned through reads.
Every recorded album directory's immediate membership is checked before and after
media hashing. Other admitted target parents retain identity/ancestry pins without
claiming an unrecorded membership baseline. Present source-root membership uses its separate read-only root grant;
absent-root state cannot use that operation. Changed child names have two explicit
original-source location probes (original name and prepared name). Every expected
location and directory is also exposed as data for complete consumer coverage.

Deferred singles retain their physical journal and actual operation results.
Whole album descendants have inspection rows only, never invented file-transfer
receipts. The final report has a fixed row for each descriptor file; an omitted or
unverified row cannot become successful evidence. Returning `Ok` does not mean
that recorded terminal state, current directories, membership or content are clear.
Consumers must check all of them and preserve partial or pending uncertainty.

## Content scope

The historical Whole journal records main-stream hashes, full object identities
and membership, but no auxiliary-stream digest baseline. The report explicitly
names its scope `recorded_main_hash_identity_membership`. It cannot certify that
historical ADS content is unchanged. Reads preserve the data; access time may
advance as a consequence of filesystem reads.

All content reads use the actual cancellation token and observed byte progress.
Deferred physical verification runs first, preserving its metadata-before-content
ordering. A resume cursor rotates the whole-file read order only: every completed
pass freshly rereads every file, with no cached all-clear. Cancellation is distinct
from missing or changed data. No aggregate media-byte ceiling is imposed.

## Current limits and acceptance

The existing 4,096-operation, 4 MiB record and 64 MiB journal limits remain interim
admission bounds. This slice does not change the journal format or implement the
separately scoped large-library parser redesign.

The reconstructed suite requires 71 Windows album cases, 37 native read-owner
cases, 13 original-parent cases, and the current 14 cleanup cases. Local Windows
cross-target type checking and strict Clippy are compilation evidence only.
Actual writer-fed Windows tests and staged/extracted consumer acceptance remain
mandatory before a product completion claim.
