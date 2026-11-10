# Fresh bounded-classifier evidence (2026-10-11)

This is new verification of the bounded adaptation, not historical source recovery.

Local Linux workspace release-profile checks passed:
- Five Rust unit tests (with actual original assets explicitly supplied), strict
  Clippy, and release example build.
- 1,519 generated original-oracle cases; exact analyzer, sparse TF-IDF, class,
  class-zero gate, and decision scores. Maximum probability/margin error
  2.220446049250313e-16 (bound 1e-14).
- All 399,084 binary64 parameter encodings bit-exact to authenticated original.
- All 1,114,112 Unicode codepoints' frozen predicates checked, plus native versus
  CPython four-context checks on every 1,112,064 valid scalar.
- Seven bad bundle variants rejected; oversized input explicitly rejected.
- Export authentication regression passes under `python -O` for bad commit,
  blob identity and content hash without reaching unpickling.

Independent fresh review additionally reported 1,707 separately generated cases
with exact native JSON parity to reconstructed source d2cd975, exact analyzer,
sparse and scores to the original Python model; 11 malformed/nonregular asset
variants; every 253 inference checkpoints on a short representative input;
10 load/hash/parse cancellation milestones; sticky token/fresh-token behavior;
and input/resource boundaries. A Unix socket fixture could not be created due
to sandbox EPERM, and is not reported as tested. Final commit seal is separate.

Runtime source SHA256 for the fresh local gate:
`be94a8e980c7d9f581f6b62b073dd62beafc3917598e0df685fe47a23e64c363`.
Frozen Unicode source SHA256:
`49e22fa19b3fa24865e9fdc5cb46bb4c06c97717b567659908bb4e029606d1ee`.
Reproducible synthetic input SHA256:
`03d9c10306f8c236d7338f8fbce4f98a5eac89556eb86e8e183ccc806dbca2be`.

No Windows execution, combined CLI proof, product activation, Android support,
private/sealed corpus parity, model quality improvement or training is claimed
by this local receipt. Those acceptance gates remain separate.
