# Bounded, cancellable original inference preparation

This isolated API is not wired into the product CLI/UI, a production policy,
Android approval, classifier selection, or a change to sorting rules. Successful
requests retain the complete original input, tokenizer rules, model bytes,
float32 expressions/reduction order, chosen actions, margins and spans. Failures
return no partial result. There is no truncation, replacement model or fallback.

## Entry point and lifetime

`ReleasedAssets::infer(text, &Limits, &CancellationToken)` is the full-text
boundary. It validates limits and borrowed UTF-8 input before creating request
buffers, tokenizes with explicit regex controls, checks intermediate token
vectors, derives original features, runs checked fast math and original decoder,
then checks cancellation before returning `InferenceOutput`.

The token is atomic, monotonic and cannot be reset. Budgets are local to one
request; a new request starts fresh budgets and uses a fresh token after cancel.
The immutable assets remain reusable. `InferenceError::Cancelled` is distinct
from `ResourceLimit`, `Allocation` and `Model`; future fail-open classifier or
routing code must propagate cancellation, never convert it to ordinary failure.

The final cancellation check is the result's linearization point. Cancellation
that occurs later requires the future consumer's generation/result-delivery
check. There is no CLI/UI asynchronous delivery integration in this slice.
Legacy `tokenize`, `forward`, `forward_fast`, `linear`, logits and raw JSONL proof
helpers remain unbounded diagnostics. They are not the new inference boundary;
in particular the old JSONL reader is not represented as resource-bounded.

## Finite admission policy

Callers must supply `Limits`; default values equal the following hard maxima.
Every field can be lowered, including to zero, but never raised above these
ceilings. These are conservative configurable probe limits, not new sorting
rules or empirically optimal production defaults.

- Input: 16,384 UTF-8 bytes and 4,096 Unicode scalar values, checked before copies.
- Tokens: 1,024 in each intermediate/output token vector; no clipped token list.
- Regex: 1,000,000 match iterations, depth 512, 1,024 KiB backtracking heap per
  native call; 32,768 native calls and 4,000,000 callouts for the entire request.
- Text intermediate matches: at most input-character-limit + 1 per infix scan;
  at most regex-call-limit special-pattern candidates before sorting.
- Work: 2,000,000,000 charged units for numeric operations and text-loop work.
  Units are a reproducible budget, not CPU cycles or a wall-clock deadline.
- Numeric allocation: 32 MiB cumulative requested tensor bytes per request,
  including transient matrices and decoder logits, not just peak live tensors.

The input caps bound noninterruptible UTF-8 counting, substring copies, frozen
lexical helpers and codepoint span materialization. Special-candidate sorting is
bounded by its prior count check. BTree allocations and string/vector metadata
remain bounded by frozen assets, input and token limits. This is not an exact
whole-process allocator/RSS ceiling or a guarantee of recovery from global OOM.
Fallible reserve is used for numeric tensors and dynamic match/token/output
vectors. Immutable model/regex storage, allocator overhead, some bounded string
copies and map metadata are outside the numeric tensor budget.

For n tokens, p = 0 when n = 0 and p = n + 8 otherwise. Before tensor allocation,
forward admission checks cumulative bytes `(1696*n + 2784*p + 384)*4` and numeric
work `286336*n + 672000*p`. Tests verify actual counters against these formulas.
The token ceiling precedes those products and guarantees their representability
on the supported 64-bit targets; individual allocation-byte multiplication and
cumulative additions are checked. Logits and text processing charge separately.
At 1,024 tokens forward requests 18,440,704 bytes and 986,712,064 work units.

## Cancellation and PCRE2

The pinned 10.46, 32-bit, non-JIT engine and original translated patterns remain.
Automatic callouts observe matching without altering successful choices. A
callout returns reserved `PCRE2_ERROR_CALLOUT` on cancellation or aggregate-budget
failure; the stored typed Rust error is returned, never interpreted as no-match.
Pre/post-match checks cover optimizer paths that skip callouts. Per-match limits
remain essential: match counts reset at unanchored starting positions, so a
match limit alone is not a whole-request bound. Input size, call counts and
callout counts bound the surrounding work.

Every native call owns a fresh RAII match context and match-data block. The
backtracking heap retained by PCRE2 >=10.41 is thus released after each call;
a later lower heap limit cannot accidentally reuse a larger retained block.
No JIT is enabled, so heap/depth controls apply to the executed matching engine.
No changed Unicode tables, regex rewriting, anchoring or optimizer disabling is
used. Numeric checkpoints occur at rows/four-output blocks without reordering
reductions or bias placement. Cancellation is cooperative, not a hard deadline.

Official references checked for this implementation:
- https://www.pcre.org/current/doc/html/pcre2api.html
- https://www.pcre.org/current/doc/html/pcre2callout.html
- https://www.pcre.org/current/doc/html/pcre2perform.html

## Acceptance evidence and remaining gates

Fresh Linux x86-64, official Rust 1.99.0 and pinned source-built PCRE2 10.46:

- Exact bounded/proof actions, spans, token metadata and margin bits for the 33
  original owned synthetic fixtures; accepted 1,024-token and 4,096-codepoint
  inputs; explicit multibyte, token, tensor and regex resource rejection.
- Exact bounded/unbounded affine and logit bits at 0, 1, 17 and 1,024 tokens;
  1,025-token rejection before tensor allocation; no model mutation after errors.
- Actual nonzero match/depth/heap/callout exhaustion, zero-limit cases and smaller
  limits following larger matches; aggregate counters shared across regex calls.
- Deterministic cancellation from an actual native callout on successful-match
  and no-match paths; deterministic mid-forward cancellation; channel-coordinated
  cancellation from another thread during an active full request. No sleeps or
  timing assumptions decide these cancellation assertions; 30-second test-only
  receive timeouts prevent deadlock on an early regression. Repeated fresh-token
  reuse succeeds and canceled requests expose no result.
- Existing 34-document optimization/diagnostic/error/stage-bit checks, original
  33-document/170-action platform checks, and numeric report remain unchanged;
  the complete numeric report is byte-identical to the pre-edit original scalar.
- Independent review also compared 204 synthetic inputs against pre-edit scalar
  JSON, including 160 seeded multilingual/punctuation/URL combinations and input
  boundaries: all exact. This is owned synthetic evidence, not private-corpus
  recovery or universal-input proof.

One local accepted 1,024-token request measured 115–122 ms. The complete test
process (including model load and other fixtures) reached 38,860 KiB peak RSS.
These are illustrative Linux measurements, not worst-case latency bounds,
per-request peak-memory isolation, Windows performance or production guarantees.

The workflow explicitly runs ignored model-dependent bounded tests against its
fresh verified original export in both platform legs. New-head Windows/Linux CI
is still required. Physical Android, product cancellation/result delivery,
secondary classifier routing, broader private development evidence and physical
apply remain separate gates.

`prepare_baseline.py` keeps every original scalar source byte as its prefix,
exposes shared control/loader types, and removes the new full-inference module
and bounded-only integration tests from the scalar staging tree. That tree stays
a diagnostic comparator; it does not pretend to implement a second bounded API.
The staged immutable-loading test maps its optimized `forward_fast` convenience
call to the original scalar `forward`; its assertions are unchanged. This keeps
staged tests compilable without adding numerical code to the frozen baseline.
