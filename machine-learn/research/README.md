# Isolated model research utilities

New source, not recovery of the missing historical experiment workspace. These
stdlib-only utilities improve evaluation, stopping, split validation and evidence
recording. They do not train, download data, open a test set, install a model,
modify the sorter, or allocate a campaign budget. Existing legacy entrypoints
and released model/runtime behavior are untouched.

## Run the synthetic checks

From the repository root, on Python 3.10 or later:

```sh
python -m unittest discover -s machine-learn/research/tests -v
python -O -m unittest discover -s machine-learn/research/tests -v
```

All test texts, annotations and accounting inputs are owned synthetic fixtures.
Passing them establishes utility behavior only, not model quality, convergence,
dataset validity, Windows/Android inference parity or product readiness.

## Single-pass evaluation

`evaluation.Record` and `Span` use raw Python Unicode character offsets. The
validator rejects invalid, duplicate or overlapping spans instead of silently
dropping or realigning annotations. `evaluate(records, predict_many)` calls the
predictor once, then reuses each document's spans for every label and metric.
The result has strict per-label/micro precision, recall, F1, F0.5, exact span
records, exact ordered singer-string lists and negative-record singer errors.
Unexpected prediction labels count as false positives. There are no raw error
texts in the returned report.

`spacy_predictor(authenticated_nlp, batch_size=64)` adapts an already verified
spaCy pipeline with `nlp.pipe`. It neither installs spaCy nor loads a model.
Different batch sizes still require prediction-parity checks on admitted data.
Run the raw NER and NER-plus-secondary-classifier arms separately; a classifier
filter must not change role, thresholds or routing during comparison.

The old public `machine-learn/f1score.py` on branch `machine-learn` invokes the
pipeline per label for metrics and then per label for error collection. For its
five labels, one-pass evaluation avoids tenfold repeated document inference.
That is an invocation-count observation, not a measured tenfold wall-time gain.

## Development stopping

`DevelopmentStopper(StopConfig(...))` maximizes one fixed development F0.5:

- Raw best score/checkpoint selection is independent of the significant-gain
  anchor. Small gains accumulate against the unchanged anchor.
- Improvement must be strictly greater than `min_delta`, default 0.001.
- `lr_patience=2` emits a factor-0.5 LR reduction after two bad scored checks;
  `patience=4` stops after four. Counts mean scored checks, not epochs.
- `check_time(elapsed)` must run before each work unit. The time cap is inclusive
  and includes fitting, scoring, checkpointing and shutdown, not fitting alone.
- A score completed at or beyond the cap does not become a selected checkpoint.
  A resource stop reports `TIME_CAP`; patience reports `DEV_PATIENCE`. Neither
  proves an architecture has reached a global optimum.
- Terminal controllers cannot be revived. Snapshot restoration replays and
  verifies events, rather than trusting saved counters.

This is a cooperative decision helper, not a process supervisor. It cannot kill
an unresponsive optimizer call, enforce an OS-level wall clock, serialize model
weights or restore optimizer/RNG/sampler state. A real runner must implement
those independently. Defaults never constitute authorization for a new run or
an extra 1,800 seconds of campaign spending.

## New split generation and leakage checks

`build_development_split` requires a declared new dataset ID, source reference
and explicit seed. It deterministically assigns connected groups to train or
development using SHA256. The assignment is independent of input order. All
records sharing a declared identity/template key or normalized duplicate text
stay together, including transitive connections. Conflicting annotations on
identical raw text fail closed.

Normalization is grouping-only (NFC, casefold and collapsed whitespace); model
inputs and annotation offsets are never rewritten. The grouping rule can be
conservative. A large connected group may yield an imbalanced or empty split;
actual counts are reported and the utility does not silently reshuffle it.
The caller must reject unsuitable partitions before training.

Held-out/exposure exclusions may be supplied as immutable record IDs, normalized
text fingerprints and identity-key fingerprints. An intersection rejects the
entire input; excluded examples are never silently re-admitted. No sealed file
is needed or read. `validate_manifest` checks frozen record content, membership,
counts, deterministic assignments and leakage on the admitted train/dev records.

The manifest explicitly says `historical_continuation=false`, creates no test
split and does not prove unknown identity or historical training exposure is
absent. Keep manifest and exclusion-registry hashes in external trusted evidence.
Do not use this utility to manufacture a replacement historical split. Any new
public-data experiment must acknowledge that old model/source overlap may remain.

## Recoverable receipts

`receipts.create_run` requires actual existing evidence files for the dataset
manifest, training config, code provenance and external campaign ledger. It
records their hashes only, in a newly created directory. It does not validate
human approval or ledger authority from file contents and must never be treated
as a training authorization gate by itself.

`append_receipt(..., expected_head_sha256=...)` requires the caller's previously
retained head. Receipts are hash-chained, bounded and never overwritten. Writes
are flushed and atomically published via a same-directory hard link. Filesystems
without that support fail closed. Windows directory-fsync is not claimed.

`inspect_run(..., expected_head_sha256=...)` validates the chain read-only.
Checking an externally retained head detects tail truncation; a self-contained
hash chain cannot authenticate its own original head. Preserve heads outside
the run directory, in an authorized durable location.

Interrupted writers/pending files remain explicit blockers. There is no automatic
stale-lock removal, ownership takeover, ledger rewrite, reservation refund or
budget reset. A controller snapshot is not a recoverable training checkpoint.
Actual continuation additionally needs authenticated model/optimizer/scheduler/
RNG/sampler state, source/config/environment provenance and reconciled ownership.

## Training remains separately gated

The previously reported processed training/dev split, candidate checkpoints and
authoritative campaign ledger were not recovered by these utilities. Public raw
data and old scripts are not replacements for those artifacts. In particular,
no sealed evaluation split should be opened or reshuffled to make progress.

Safe independent work can continue using the exact original native model bundle.
For a new research campaign, first approve a new dataset/version and bounded
budget, preserve earlier accounting as unresolved historical evidence, construct
fresh leakage controls and explicitly label development results as development.
Do not imply continuation, independent generalization or automatic promotion.

## Primary references

- [spaCy training](https://spacy.io/usage/training): sourced components, scoring,
  tokenizer-alignment caveats and serialized training configuration.
- [spaCy configuration](https://spacy.io/api/data-formats#config-training):
  `patience` is measured in steps without evaluation-score improvement.
- [spaCy batched processing](https://spacy.io/usage/processing-pipelines#processing):
  `nlp.pipe` and multiprocessing overhead tradeoffs.
- [GroupKFold](https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.GroupKFold.html):
  disjoint group partitions. This utility implements its own hash assignment,
  not GroupKFold or an independent-test guarantee.
- [PyTorch ReduceLROnPlateau](https://docs.pytorch.org/docs/2.14/generated/torch.optim.lr_scheduler.ReduceLROnPlateau.html):
  separate metric-based learning-rate reduction from stopping.
- [ONNX quantization](https://onnxruntime.ai/docs/performance/model-optimizations/quantization.html):
  quantization can change accuracy and is not exact-original-decision preservation.
