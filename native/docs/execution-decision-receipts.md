# Fresh Apply-preflight decision receipts

The live selected job exposes a Serialize-only `ExecutionDecisions` projection
only after the actual Apply-time replan has matched the retained plan digest,
resources and root identities. Album validation carries this data through its
reader-release/native-grant handoff. Reconstructed journal bindings contain no
such receipt. No receipt changes filesystem authority, policy or journal bytes.

Schema 1 contains `basis: fresh_apply_preflight`, `plan_binding_sha256`, the exact
ordered fresh `decisions`, and ordered `single_routes` with source, artist and
operation IDs. Non-duet routing retains the first selected artist; duet routing
retains every ordered occurrence, including repeated names. Empty IDs mean no
admitted transfer. They do not assert an execution-time skip reason.

The separate `validate_noop_execution_decisions` performs a controlled fresh
replan and exact resource/root checks without mutation grants, and emits
`fresh_noop_preflight` only for the shared clean-no-op admission predicate.
Cancellation or failed validation returns no accepted receipt.

Consumers must match the receipt's plan digest to the native started/final
job, then separately correlate committed operation IDs, completed album IDs,
acknowledged physical effects and terminal status. Preflight acceptance is not
transfer success. Partial, cancelled, unmatched and absent outcomes stay explicit.
This snapshot also is not the original Python implementation's interleaved
execution trace; collection, directory precreation and failure timing differences
remain observable and must not be normalized away.

Three portable integration cases are authored and separately required in the
Windows selected-feature gate: ordered repeated-artist IDs and journal absence,
controlled no-op root/resource/cancel refusal, and an accepted policy skip with
no invented reason. Existing binding-staleness and physical fault suites remain
required. Local Rust/native execution has not been performed.

## CLI transport and paired comparison

Successful positional and NDJSON Sort results expose the prepared live receipt
as `execution_decisions`. Album results expose the original whole-plan receipt,
not a reconstruction from flattened file operations. The final receipt remains
separate from committed operations and physical effects. FixNames does not use
this Sort-only evidence field.

A clean no-op confirmation re-reads aliases/models through the existing request
planner, compares the retained plan, then calls the controlled read-only no-op
validator with freshly measured resource descriptors and retained preview
descriptors. It opens no mutation grants or journals. Its receipt uses
`fresh_noop_preflight`; cancelled or stale validation emits no successful receipt.

The paired Windows lane preserves the original 36 cases and adds the remaining
30 duet-enabled settings vectors. This authors 66 cases across all 64 boolean
settings vectors. A complete comparison still requires actual Windows results;
these source changes do not retroactively close the OPEN paired-041 reports.
