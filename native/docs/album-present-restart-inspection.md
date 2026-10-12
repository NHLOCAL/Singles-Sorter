# Present-source Album inspection ownership

The present-source constructor consumes freshly selected source, target and
journal read grants. It compares the recorded source reference, identity, parent
and directory kind, then obtains the native root alias only from those held
handles. The three roots must remain disjoint. This creates no mutation session.

Present and absent source states are distinct and immutable. The existing absent
constructor still requires the recorded exact original leaf to be absent under
the independently selected containing parent. A present owner cannot use the
absent binding method; an absent owner cannot acquire a source ledger.

Both states first read the fixed journal set. Binding consumes that initial
state, rereads every retained journal and compares its exact bytes. The executor
must derive the supplied ledgers from strict manifest/physical replay. Present
source and target ledgers use separate private root owners, index spaces and
ancestry maps; the public source methods cannot reinterpret a target reference.
No arbitrary path, sibling lookup or raw handle getter is exposed.

Every metadata/content/ADS read keeps the existing cancellation and actual-byte
observation boundary. Source entry probing may prove an exact recorded child
absent when a bound ancestor is missing, after revalidating the retained prefix.
A foreign or reparse ancestor, moved root, read error or cancellation is never
absence. File digests still require the exact bound leaf and full identity.

The native slice authors nine new owned cases, bringing the existing owner gate
to 33. These cover separate same-index source/target reads, root binding and
overlap refusal, cross-role/state refusal, changed journal bytes, chunk
cancellation, foreign equal-content files, missing versus foreign parents, and
root relocation or exact native sharing refusal. All native execution remains
pending. Executor replay/reconciliation and host activation are separate work.

## Strict executor façade

`recover_recorded_album_file_job` consumes fresh explicit Present source or
Missing parent selection, target/journal grants, and retained journal identity,
length and SHA-256 records. It rechecks the first held reads against those digests,
reparses the data-only descriptor, and requires exact equality with the selected
record before strict shared manifest and physical-journal replay. No historical
scope is promoted into execution authority; the internal binding is recovery-only.

Only that replay supplies the separate fixed source and target ledgers. Every
journal is reread during the consuming ledger handoff. All known target directory
identities stay pinned through content verification. Directory metadata is probed
before media reads, and the existing whole-cohort file-metadata prepass remains
inside controlled physical reconciliation. Full copied-Move streams use the same
existing comparison helper. Missing original roots are checked before and after
inspection through the exact native parent/leaf grant.

The typed result retains the full manifest terminal/pending evidence and explicit
fresh source state. Its per-file rows pair original source/destination references
and original plan IDs with distinct physical operation IDs and physical recovery
statuses. The older aggregate report retains its original-plan ID remapping;
consumers must not join those repeated album IDs as if they were unique files.
`Ok` alone is never an all-clear predicate. An absent transfer journal leaves each
file's recovery evidence absent. A resume cursor changes read priority only; every
operation is freshly verified on each completed pass.

Eight additional writer-fed native cases are authored for present root and child
Copy, present cross-volume child Move, removed-root parent inspection and source
reappearance, discovery digest mismatch, changed destination content/metadata,
partial Copy, cancellation/fresh priority-cursor inspection, and forged descriptor
or wrong target refusal. The complete album suite becomes 57 (39 live + eight
prior descriptor + ten restart cases). The added cases isolate required ADS and
file/directory metadata mismatches. Owned source/target/journal tree snapshots
include full identities, creation/write times, attributes, main hashes and the
fixture’s known ADS; access times are excluded because reads can update them. Compilation and native execution remain
pending, including the existing current-source compiler corrections at composition.
