# Local preparation checks — 2026-10-11

These results do not claim fresh static linkage or Windows parity.

- Fresh original source export: 31 tensors, 3,829,976 bytes, 22 actions,
  33 owned synthetic documents. Frozen original source/runtime hashes pass.
- Existing reviewed Linux binaries: all 170 actions, ordered spans, metadata,
  lexical strings/IDs/hashes/buckets pass the new exact gate. Close winner
  unchanged; native final margin 0.000087738037109375.
- Existing math proof binary: 33 exact paths/spans and 114 forced-state cases.
- Eight acquisition tests pass, including caller-level no-build-on-invalid-source.
- Actual official PCRE2 10.46 ZIP acquisition succeeds through the helper with
  pinned SHA256, bounded download and complete path validation. Source extraction
  is ignored, not vendored. Original upstream license/COPYING are preserved.
- Both Cargo manifests/locks resolve offline via metadata without crate builds.
- Isolated Rust build-script checks pass for missing environment, missing library,
  correct MSVC link name (synthetic file sentinel only), and Android rejection.
- Python AST parsing and YAML structural parsing pass. This is not actionlint.

Not run: fresh PCRE2/static Rust build, Windows execution, Android execution,
installer/CRT deployment, runtime resource limits and cancellation. No CMake is
installed in this disk-constrained environment; none was installed for this check.
CI will run actual source builds and exact cross-platform checks after review.

The old Linux raw binary used for harness checks has SHA256
`a929c7f22da93e8773bed793c7896e6a58bc628b10df1330cbc48507f97bd265`.
It is 984,584 bytes and dynamically links the old host dependency; these numbers
are not the footprint of the new static Windows package.

## Reviewed optimization update

Implementation source `acefa1a4396d0c14c4f45d6e3b12bd94992a285e` copied without
algorithm edits. Only its two independently reviewed files are updated. Preserved
baseline math/raw source matches the previous sealed package exactly.

- Optimized reviewed Linux raw binary SHA256:
  `cf65ce5e347ce6521087cb1b197d7854e34a4485f2773ba6ac49e88f3b18b311`.
- Original oracle check: 33 documents, 170 actions, exact metadata/features/spans
  and unchanged close-margin winner. Same reference/input bytes as before.
- New before/after gate: 34 synthetic documents including a 302-token input,
  six error cases; full default/fast diagnostics byte-identical, spans-only and
  errors match the original.
- Common `stage-bits.rs` compiled only as two tiny probes against preexisting
  original/optimized Rust libraries. All eight token-count stage/affine/logit
  hashes and tie/one-ULP checks match. No crate, dependency or PCRE2 rebuild.
- Baseline staging source and offline Cargo metadata checked. Three staging
  tests cover source replacement, no generated-data copy, bad-hash rejection
  before copy, and existing-output refusal. Eight acquisition tests still pass.

No new Windows/Android run, static PCRE2 proof, performance benchmark or full
corpus evaluation was performed. Hosted CI still must build and execute both
versions with the actual pinned dependency before any platform claim.
