# Original secondary classifier, bounded native library

Canonical original-only runtime; no char35 code or candidate model. This adapts
the NEW reconstructed source at `d2cd97570d0d1ba8fe36d09d2f81d702d73734f4`,
not restoration of missing historical source commit `59eb116e34382c065f63ae5f4b7b1c26dff19a44`.
The original public source is NHLOCAL/Singles-Sorter baseline
`78d001aee5ce49628bdffebd34540fa83ba32c12`, model Git blob
`53945ae968c97125c890927e7ff97789351b2470`. Its pickle SHA256 is
`ec9b4b50e9308d5f7a27f12bc141389e61a16a622a4faa0cbf864f94a9cb3293`.
Repository MIT notice remains applicable; see `NOTICE.md` and repository LICENSE.

## Contract

`Model::load_directory(path, &cancel)` loads the two fixed assets once.
`Model::predict(text, limits, &cancel)` returns a typed result. Cancellation can
be a closure over the caller's shared monotonic token; this library never resets
it. `ClassifierError::Model` identifies ordinary model/resource-file failures;
`Cancelled`, `ResourceLimit`, and `Allocation` are fatal request failures.
Caller policy decides ordinary fail-open behavior. **No error becomes acceptance
inside this library.** Success is class ID zero, with first-maximum tie handling.

The analyzer retains frozen CPython 3.12 / Unicode 15 lowercasing, contextual
sigma, and sklearn word-unigram/bigram behavior. Counts use a bounded sorted ID
vector instead of an allocating tree, but counts and all sparse binary64
arithmetic remain ascending-ID ordered. No f32 conversion, threshold, batching,
cache, normalization change, input truncation, or retraining is introduced.

## Bounds and cancellation

Hard ceilings, which callers may tighten but not raise: 16,384 UTF-8 input bytes,
4,096 input scalars, 4,000,000 work units, 4,000,000 cumulative requested allocation
bytes per prediction. Exceeding a bound is an explicit fatal result, not a claim
of universal released-input parity. Input is processed completely or rejected.

Work units conservatively account for frozen Unicode binary searches (64 per
scalar/scan), vocabulary binary search (20 times feature bytes plus one), sort
(16 times ID count), string copying, and numeric loops. Every scalar, sigma
context step, feature, sparse and score iteration checks cancellation. The few
bounded operations between checks include one feature lookup (at most 18 string
comparisons), one in-place sort of fewer than 8,192 IDs, or one short string copy.

Analysis allocates at most four bytes per input scalar for lowercase, at most
twice the input scalar count of feature slots (the sole expanding lowercase is
U+0130), and pre-reserves every data vector/string before insertion. Allocation
accounting counts requested capacities, not allocator headers or reserved OS
pages. Fallible reservations return `Allocation`; stricter budget exhaustion is
`ResourceLimit`. Scalar temporaries and digest bookkeeping are stack values.

Loading accepts exactly 3,192,809 weight bytes and 1,268,235 vocabulary bytes.
Each file is prechecked, opened (nonblocking on Unix), and checked again on its
opened descriptor. Reads and hashing use <=4 KiB chunks with checkpoints; one
extra byte detects growth. Parsed bytes are the exact hashed buffers; no reopen.
SHA256-pinned bytes determine 399,084 binary64 parameters, 79,816 vocabulary
strings, and four classes. All retained arrays/strings reserve fallibly; maximum
requested load heap is below 11 MiB including both input buffers, parsed strings,
vocabulary slots, parameters and two bounded path buffers. Asset directory paths
are capped at 32,768 encoded bytes. No runtime pickle or download exists.

Cancellation is cooperative, linearized at the final successful checkpoint.
The caller must check again before accepting or publishing success. This cannot
interrupt a stalled OS/filesystem call or forcibly cancel user callback code.
Use ordinary local files; this is not an OS-level deadline or process-memory cap.

## Reproduce

Use a separate Python 3.12 oracle environment. `tools/oracle-requirements.txt`
pins official PyPI wheel hashes for Linux x86_64 and Windows AMD64. Do not upgrade
or reuse the separate original NER oracle environment.

1. `python tools/export_original.py --baseline <original-git> --output <assets>`
   verifies pinned commit, blob and SHA256 with unconditional checks before
   unpickling. Export is offline from local Git objects; no training. Optional
   `--unicode-output <file>` regenerates source predicates for inspection.
2. From `native`, `cargo test -p sorter-classifier` and
   `cargo clippy -p sorter-classifier --all-targets -- -D warnings`.
   Set `ORIGINAL_CLASSIFIER_ASSETS=<assets>` to exercise actual asset unit tests.
3. `cargo build -p sorter-classifier --examples --release`.
4. `python tools/verify_original.py --assets <assets> --binary <parity-executable>
   --evidence <directory>`; also `python tools/audit_unicode.py --binary
   <unicode_audit-executable> --evidence <directory>`.
5. `python -O tools/test_export_auth.py` proves Git/blob/hash checks still reject
   before unpickling when Python assertions are disabled.

Verification generates synthetic data only; no private/sealed corpus is included
or needed. The 1,519-case generator reuses the independent reconstruction-review
seed, but replaces its 30,000-byte admitted example with a 3,000-byte case and
checks the original oversized case is rejected. Historical private 1,982-string
and 3,295-record gates remain unavailable and are not claimed. Windows integrated
CLI and independent final-commit review remain separate required release gates.
