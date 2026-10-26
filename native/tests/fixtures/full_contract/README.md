# Frozen full-release reference

These files are independent test evidence from unchanged Singles Sorter v14.0,
commit `78d001aee5ce49628bdffebd34540fa83ba32c12`. No expected output comes from
Rust. The exact released singer CSV is copied without newline/BOM/data changes.

- `inputs.json`: authored synthetic inputs, never real music.
- `cases.jsonl`: 80 album, single, traversal and fix observations, with ordered
  listings/metadata/decision/operation traces, before/after content manifests,
  exact destinations and the release's separate legacy counters.
- `historical-unsafe.jsonl`: two destructive/false-success historical observations.
  Both say `historical_only=true` and `future_acceptance=false`. Never require a
  safe replacement executor to reproduce these mechanics.
- `csv-dialect.json`: 1,561 independent CPython default-dialect cases, including
  every string of up to four characters over `a`, comma, quote, LF, CR and space.
- `manifest.json`: source Git blobs/SHA256s, runtime versions and hashes,
  input/exporter hashes, artifact bytes/hashes, coverage and limitations.

Metadata and AI providers are explicitly synthetic. Scalar observations preserve
text/null distinctions and encode arbitrary Python integers as signed decimal
strings. Get-time and value-time failures, false-y items and partial album
accumulation stay in the ordered trace. Metadata save calls mutate only the
in-memory synthetic object. Reference file mechanics operate only on guarded,
owned disposable placeholder trees. High-level and lower-level mkdir/move/copy/
rename/remove/write/metadata entrypoints are guarded before trace path rendering;
outside-root attempts abort before the release can catch an error and fall back.
Recursive fix records the full rglob encounter order and a collection-complete
event before processing, including non-audio children and directories; the product exposes no physical execution.

Directory encounter order is the recorded Linux order, not lexical/creation
order and not a Windows guarantee. For example, the first-tie album input's
first observed child selects `שני`. A later planner must consume the recorded
order, not sort these observations. Reproducibility on a different OS/filesystem
requires separately characterized encounter order; these fixtures do not prove
Windows, Android, actual tag parsing or native inference acceptance.

## Reproduce independently

Use a detached checkout of the exact release, CPython 3.12.14 / Unicode 15.0.0,
and the test-only pins in `../../reference/requirements.lock`. Run from the
repository root, replacing paths as appropriate:

```sh
python native/tests/reference/export_full_contract.py \
  --baseline /path/to/original-v14-checkout \
  --inputs native/tests/fixtures/full_contract/inputs.json \
  --output /tmp/fresh-full-contract-1
python native/tests/reference/export_full_contract.py \
  --baseline /path/to/original-v14-checkout \
  --inputs native/tests/fixtures/full_contract/inputs.json \
  --output /tmp/fresh-full-contract-2
diff -r /tmp/fresh-full-contract-1 /tmp/fresh-full-contract-2
SINGLES_RELEASE_BASELINE=/path/to/original-v14-checkout \
  python -m pytest -q native/tests/reference
```

The exporter refuses a modified/unpinned release, an unexpected runtime,
nonempty output, and output within the frozen checkout. Keep frozen outputs
immutable: any intentional oracle change needs a new reviewed input/exporter
identity. Rust checks committed hashes and the CSV corpus without Python.

The 70 pre-existing Python assertions are ported byte-for-byte (68 policy,
2 historical mechanics). They remain independent release-reference tests and
make no claim that a future native album/traversal/fix implementation exists.
