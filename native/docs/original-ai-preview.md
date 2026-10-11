# Original-only opt-in AI preview

This is a read-only CLI integration, not physical apply, UI activation, Android
support, a replacement model, or a production installer. Every returned plan
remains `executable: false`. The default CLI build and v1 supplied-facts commands
remain model-free. Enabling `original-ai` compiles the original-only adapter;
models are still unavailable until an explicit local bundle validates completely.

## Build

Use the repository's pinned Rust 1.99.0, CMake/C++ metadata prerequisites, and
verified PCRE2 acquisition helper:

```sh
python probes/released-ner/acquire_pcre2.py --out probes/released-ner/.pcre2 --build
export PCRE2_STATIC_DIR="$PWD/probes/released-ner/.pcre2/install/lib"
cargo build --locked --release -p sorter-cli --features original-ai
```

On Windows set the same absolute directory in `$env:PCRE2_STATIC_DIR`. The helper
builds a static PCRE2-32 archive without JIT; this is independent of CRT linkage.
The focused Windows CI currently uses the default dynamic MSVC CRT. The workspace
includes the adapter crate, so `cargo test --workspace` also needs this prerequisite.
The default `cargo build -p sorter-cli` does not activate the optional adapter.

## Explicit model profile

A configured v2 request changes only `ai_profile`:

```json
{"Configured":{"bundle_id":"original-v14-ner-3.7.5","model_directory":"/absolute/verified-original-bundle","limits":{"max_inference_calls":10000}}}
```

On Windows supply an absolute UTF-8 JSON Windows path. The directory must contain
`manifest.json`, `tensors.f32`, `tokenizer.json`, `classifier-v14.weights.bin`, and
`classifier-v14.vocab.txt`. Both loaders verify the exact original graph/contracts,
asset lengths and pinned hashes on the same bytes they parse. No network request,
Python runtime, pickle loading, implicit search path or fallback model is used by
the application. There are no embedded weights. The source baseline is
`78d001aee5ce49628bdffebd34540fa83ba32c12`.

For developer/CI regeneration only, use the pinned NER exporter and the
`sorter-classifier/tools/export_original.py` authenticated exporter with their
separate locked Python environments. The focused original AI workflow documents
exact acquisition and export commands. Never commit exported assets or corpora.

`Disabled` retains existing behavior. Missing/wrong/partial/corrupt configured
bundles return an error, never become Disabled. Unknown fields (including caller
claims of availability, resource IDs or runtime identity) are rejected by the
strict request DTO. Only verified adapter-generated identities enter the plan.

## Execution and bounds

Both models load once per job on the calling thread. No batching or result cache
is added. Released call order, alias precedence, original class-0 classifier gate,
SINGER span order and duplicate names are preserved. Ordinary classifier model
errors preserve released fail-open behavior and add bound diagnostic warnings.
Ordinary NER failures preserve the released scan-item error boundary.

`inference_executed` means at least one native inference invocation began. It is
true even if that invocation fails its runtime bound; it is not a count of
successful classifications. Alias-only, loading failure and a zero job-call
budget rejected before the first invocation report false. Failed jobs never
return partial plans. The result flag is retained on post-invocation failure.

Limits can only tighten defaults. Available fields are `max_inference_calls`,
`ner_input_bytes`, `ner_input_chars`, `ner_tokens`, `ner_work_units`,
`ner_tensor_bytes`, `classifier_input_bytes`, `classifier_input_chars`,
`classifier_work`, and `classifier_allocation_bytes`. Fixed NER regex budgets
remain enforced. These finite limits reject entire inputs/jobs without truncation;
they do not claim parity on arbitrary unbounded inputs. Process-wide allocator
exhaustion cannot be guaranteed recoverable.

The job-token library API cooperatively cancels planning and both model runtimes.
Tokens are monotonic; retry with a new token and a fresh provider. This CLI slice
has no Ctrl-C token handler: normal OS process interruption is not a graceful
cancellation guarantee. Aborts report `PLANNING_CANCELLED`,
`PLANNING_RESOURCE_LIMIT`, or `PLANNING_ALLOCATION` and no plan.

The plan digest covers verified asset identities, original release/runtime ID,
actual linked PCRE2 archive hash, build target/profile, effective runtime limits,
and ordinary fail-open diagnostics before hashing. It remains an observation
binding, not an executable authorization or an audio-content hash.

See [integration provenance and complete notice locations](../crates/sorter-recognition/NOTICE.md)
before preparing any redistributable package.

## Verification boundary

The focused workflow builds and runs the actual application release profile on
Linux and Windows with TagLib and static PCRE2, compares both original oracles,
and checks real temporary WAV routing, immutable source/target trees, aliases,
limits, bundle failures and plan binding. Original model files remain job-local.
CI success does not establish production packaging, arbitrary-library metadata
coverage, Android, physical apply, or UI AI acceptance.
