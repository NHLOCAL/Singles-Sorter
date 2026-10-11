# Bounded original-asset loading

This preparation slice remains isolated from the product. No application/CLI
inference, sorting decisions, cancellation behavior, training, model promotion,
weights, tokenizer rules, floating-point expressions, or operation order change.

## Contract

- `Model::load` now returns `LoadError`; malformed external assets do not reach
  assertions or unwraps. Model tensors are private and there is no public default
  or partially initialized constructor.
- Manifest input is capped at 65,536 bytes before parsing. All fields in the
  existing `model-lock.json` must match exactly, including original release,
  model hash, graph, actions, shapes, offsets and individual tensor hashes.
  This is semantic manifest validation, not a new hash of exporter-specific
  JSON whitespace, Python patch version or synthetic-fixture receipts.
- Tensor input is capped at exactly 3,829,976 bytes. The unchanged complete hash,
  each individual tensor hash, checked dimensions/ranges, finite float32 values,
  parameter uniqueness and complete graph are checked before publishing a model.
- `Tokenizer::load` caps the exported JSON at 131,072 bytes and verifies unchanged
  export SHA256 `33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574`
  before invoking the existing tokenizer parser on those same buffered bytes.
- `ReleasedAssets::load` returns an immutable model/tokenizer pair only after
  both succeed. Existing proof executables are intentionally not switched over.
- Each asset is opened once; its opened descriptor is checked for regular-file
  type and length. Reads retain no more than the cap, and at most one extra byte
  is read to detect concurrent growth. Hashing and parsing use that same buffer.
  Linux uses nonblocking open so replacement by a FIFO cannot block open.
- The supported loading targets are Linux x86-64 and Windows x64; other targets
  fail closed. This is a byte/allocation bound, not an I/O deadline. Ordinary
  local disk paths are assumed, especially on Windows: device/UNC paths,
  adversarial reparse-point changes, hung filesystems and global process OOM
  are not covered by a wall-clock or recovery guarantee.

The unchanged low-level numeric/decoder proof API still has caller preconditions
and debug assertions. This slice is not a fully bounded production inference API.

## Frozen baseline compatibility

`prepare_baseline.py` verifies the existing baseline hashes and preserves every
original scalar-source byte as an exact prefix of the staged baseline. It appends
only `pub mod assets;` and `pub use assets::LoadError;` declarations so the shared
loader compiles. Neither preserved baseline file nor its hash is replaced.

## Local verification (Linux x86-64, Rust 1.99.0)

- 9 existing math tests unchanged, 1 streaming-bound test and 6 loader-negative
  tests pass. Cases include malformed/deep JSON, unsupported/overflowing tensor
  metadata, missing files, a sparse 4 GiB manifest, truncated/oversized/wrong-hash
  tensors, and a FIFO without a writer. Failures are checked for ordinary typed
  errors without unwinding.
- 5 existing text boundary tests and 3 new text-loading tests pass, including the
  real original bundle, malformed/truncated/changed/oversized tokenizer, partial
  bundle rejection, and continued use after on-disk paths are replaced.
- 3 staging tests pass. The actual staged baseline math crate compiles/tests;
  staged baseline text and raw proof also compile in the isolated source check.
- The retained original numeric report is byte-identical. Existing platform
  gate passes 33 synthetic documents / 170 actions with exact token metadata,
  features, actions and spans. Existing same-host optimization gate passes 34
  documents and 6 errors with exact default/fast/spans diagnostics and stage bits,
  including the one-ULP/tie checks. No corpus or held-out data was opened.

Text checks were compiled directly with cached dependencies and the existing
host PCRE2 library to keep local disk use bounded. They are source/logic checks,
not fresh pinned-static-PCRE2 packaging proof. No build script was bypassed or
altered in the distributable build. CI retains the supported pinned dependency
build and now explicitly runs the complete-bundle loading test against its own
verified original export (`ORIGINAL_NER_EXPORT`, ignored locally by default).

Remaining gates: independent final review, new-commit Linux/Windows static CI,
Windows loader edge cases, physical Android support, full runtime input/resource
and cancellation contracts, and eventual separately reviewed product wiring.

## Windows generated-export newline correction

The first new-bundle Windows CI test rejected `manifest.files`: the exporter used
platform-default `Path.write_text`, which translates generated JSON newlines to
CRLF on Windows. That changes the generated tokenizer hash despite identical
JSON values. The original source verification and synthetic oracle golden gate
had already passed; loader rejection was correct for its exact-byte contract.

The exporter now writes explicit UTF-8 bytes with LF, matching the existing frozen
export. It neither rewrites original model assets nor weakens loader hashes,
bounds or allowlists. A regression executes the actual serializer helper against
simulated Windows text I/O and requires all 73,348 frozen tokenizer bytes, plus
ordinary UTF-8/LF output and retained nonfinite-value rejection. The Windows
regression failed before the one-line serializer fix; all three tests pass after.
Actual Windows/Linux static CI must rerun on this correction before a new platform
pass is claimed.
