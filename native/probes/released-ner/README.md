# Isolated original NER platform proof

This is a **proof-only** copy of the reviewed original-model crates and offline
exporter. It is not a production CLI dependency, model replacement, release
package, universal-input guarantee, or Windows/Android approval.

## Scope and provenance

- Original existing repository baseline: `NHLOCAL/Singles-Sorter` commit
  `78d001aee5ce49628bdffebd34540fa83ba32c12`.
- Original source commits and copied-file SHA256 values are recorded in
  `source-provenance.json`. Those are **pre-portability** hashes, not claims that
  edited files retain identical hashes. `package-manifest.json` records this
  package's final source hashes.
- Baseline reviewed source: exporter `b9f2412`, math `395450ed`, text `2cb8497`.
- Reviewed optimization: `acefa1a4396d0c14c4f45d6e3b12bd94992a285e` replaces
  only math `src/lib.rs` and the raw proof binary source. Exact copied hashes and
  original scalar sources are preserved in `optimization-provenance.json` and
  `baseline/`. Independent scoped Linux findings are in
  `optimization-linux-review.json`; no cross-platform approval is implied.
- Preserve upstream licenses in each crate and `ORIGINAL-LICENSE.md`. The
  existing PCRE2 distro notice describes historical host provenance; for any
  redistributed source-built PCRE2 library, use its acquired upstream `LICENCE.md`
  and `COPYING` notices. No inference/model license is silently reclassified.
- No binaries, model tensors, private/development/sealed corpus, or raw original
  vocabulary are committed. `linux-reference.json` contains only owned synthetic
  text, lexical output, metadata, actions and spans. `close-margin.json` preserves
  independent fixture247 and its exact original evidence.

## Design choice

Keep the reviewed model semantics and exact float32 reduction order; package
sibling relative Cargo path crates in separate workspaces. No application workspace membership or production
inference switch is added. The portability adaptations are explicit executable arguments, UTF-8 exporter IO,
synthetic suite selection and regex linkage plumbing. The separately reviewed
optimization interleaves four independent output accumulators without changing
any individual reduction order or bias placement, and uses fixed small scratch
arrays. `forward` retains diagnostic stages; `forward_fast` omits their copies.
The raw proof remains diagnostic by default; `--fast` retains identical output,
while `--spans` is an explicitly different spans-only experiment. These are
proof flags, not a finished production CLI interface.

Alternatives considered:

1. **Host library / package-manager binary**: small source diff, but Windows and
   Android cannot assume a Linux PCRE2-32 soname. An unpinned vcpkg/MSYS2 binary
   also changes provenance and CRT choices. Rejected for this proof.
2. **Official PCRE2 source-built static 32-bit library**: explicit SHA256,
   architecture, CRT and compile flags; same C source can later be compiled by
   the Android NDK. Chosen. This does not prove future Android behavior.
3. **Switch regex engine / UTF-8 wrapper**: introduces new regex semantics and
   code-point indexing changes after exact parity work. Defer.

Pin **PCRE2 10.46**, the actual reviewed host version. Current upstream 10.49 is a
security-only release correcting a growable-JIT-stack issue. This probe disables
JIT and the bridge never uses JIT-stack APIs. This is not an assertion that 10.46
is generally secure: reassess all upstream advisories and upgrade separately,
with the complete parity gate, before production. Do not silently mix that
upgrade into the initial portability proof.

Official sources consulted:
- https://github.com/PCRE2Project/pcre2/releases/expanded_assets/pcre2-10.46
- https://github.com/PCRE2Project/pcre2/blob/pcre2-10.46/CMakeLists.txt
- https://www.pcre2.org/guide/non-autotools-build/
- https://github.com/PCRE2Project/pcre2/blob/main/SECURITY.md
- https://github.com/PCRE2Project/pcre2/releases/tag/pcre2-10.49

Build 32 only, static PCRE2, no grep/test binaries, no JIT, Unicode support on,
PIC on for the eventual NDK/shared-library direction. Rust calls the 32-bit C
ABI directly, so no C header import annotation is involved. MSVC uses
`pcre2-32-static.lib` and dynamic CRT (/MD), matching default Rust MSVC CRT;
Linux uses `libpcre2-32.a`. This is not a static-CRT installer proof.

## Acquisition containment

`acquire_pcre2.py` accepts only the checked-in official HTTPS URL and pinned
SHA256. It bounds archive and expanded sizes/member counts, rejects path
traversal, absolute paths, Windows separators/ADS, duplicate case-folded names,
symlinks and encrypted members before extraction, and validates redirect hosts.
The destination must not exist. Only verified extraction is promoted from a
fresh temporary directory. Invalid acquisition never calls CMake; build failures
stop the workflow before link-directory publication or Cargo invocation. No
skip-validation switch, cache trust shortcut or system-library fallback exists.

The SHA256 is the digest displayed for the official release ZIP, not the GitHub
automatically-generated source archive. This pins integrity against the reviewed
lock; a separate verified-signature/attestation validation flow is not implemented.
Network/signature/source acquisition is CI-only, never application startup.

## CI gate

The workflow runs fresh Windows x64 MSVC and Linux x64 legs. Each checks out
only the exact original baseline model directory, installs the pinned offline
Python oracle, verifies original asset/runtime-source/tensor hashes, and exports
33 owned synthetic documents. It builds a new pinned static PCRE2 dependency,
new Rust proof executables, and an original scalar comparison in an isolated
`.baseline` tree. `prepare_baseline.py` validates preserved source hashes and
copies only source/lock/manifest/license inputs, never old targets or model data.
Both builds use the same compiler, crate profiles and pinned static PCRE2. CI requires:

- 33 exact document paths/spans and 170 chosen actions against both that host's
 original spaCy oracle and the frozen Linux original oracle;
- all token boundaries/whitespace, lexical strings, 64-bit IDs, hashes, buckets,
 fresh sent_start, ent_iob and is_space fields;
- forced-state/context/validity tests and numerical stage diagnostics;
- byte-identical same-host original/optimized/default/fast JSON diagnostics,
  including margins, on 33 fixture documents plus one 302-token synthetic input;
- six malformed/rejected input cases and matching spans-only/error responses;
- exact stage/affine/logit bit hashes before/after at 0, 1, 2, 3, 7, 58, 129 and
  257 synthetic token counts, plus first-valid tie and one-ULP winner checks;
- fixture247's original winner and spans despite an approximately 0.0000882
 original score margin. Float tolerance never authorizes changed actions/spans.

The exact close-margin input contains an **actual newline U+000A** after the
two Devanagari digits (U+0967, U+0968). JSON represents that newline with an
escape; it is not a literal backslash followed by n. The checker pins all 14
input code points. Use the JSON, not a rendered document transcription.

Python is offline export/oracle/test tooling only. The inference binaries have
no Python runtime dependency. Generated model/oracle data remain job-local;
there are no artifact uploads or production publication steps. Unknown wheel source bytes remain a hard failure. The two exact official
Windows wheel distributions have separately pinned raw source hashes plus
unchanged original/upstream LF hashes; both checks are required. This validates
source-byte provenance without rewriting runtime or model data. See
`WINDOWS-SOURCE-BYTES.md` for all 18 source comparisons and remaining guards. The full active Python oracle closure is version- and wheel-hash-pinned in
`oracle-lock.json` / `oracle-requirements.txt`; `pip freeze` is recorded for
diagnosis. CI uses shared official Python 3.12.10 artifacts, while the original
golden retains its 3.12.14 provenance. See `ORACLE-SETUP.md` for the actual first
CI setup failures, evidence, and strict pre-build compatibility gates.

Run from this directory, supplying your own original baseline directory:

    python released-ner-export/export.py --model ORIGINAL_MODEL_DIRECTORY --out .oracle --suite platform
    python released-ner-export/verify.py .oracle
    python test_acquisition.py
    python acquire_pcre2.py --out .pcre2 --build

Set PCRE2_STATIC_DIR to the absolute `.pcre2/install/lib` directory. Then use
`cargo +1.99.0 ... --locked --manifest-path released-ner-text/Cargo.toml` and the
math sibling. See the workflow for exact commands. No developer paths are baked
into source. Acquisition cannot reuse an old output directory after failure;
inspect it and choose a fresh directory instead.

## Honest remaining blockers

- New static linkage and Windows execution are not validated by an old Linux
  binary. Until both fresh workflow legs pass, this remains an unrun platform
  draft. Local smoke results only validate the new harness against old binaries.
- Runtime size is not established: model tensors alone are 3,829,976 bytes;
  native executable bytes are recorded per CI leg. Windows CRT deployment,
  installed footprint, peak RAM and representative latency still need measuring.
- Regex match/depth/heap limits still use upstream defaults. No explicit
  per-request regex contexts, time budgets or cancellation integration exist.
- Input/token/JSON-line limits and allocation/OOM containment are unfinished.
  Long inputs must never be silently truncated or routed through an approximate
  fallback. A future bounded API must return explicit unsupported/resource-limit
  errors and preserve full-input semantics for accepted inputs.
- Model loader assertions/panics and forward-stage allocations need production
  error containment. Process timeouts in CI are not product cancellation.
- Android ARM64 needs pinned NDK/toolchain/API/ABI setup, 32-bit PCRE2 static PIC
  linkage, on-device exact parity, size/RAM/latency and cancellation tests. The
  build script rejects unproven targets rather than guessing system libraries.
- Broader original exposed-development parity and classifier/routing checks
  remain separate gates; no private corpus is added to this synthetic workflow.

## Optimization scope

The reviewed owner-run Linux exposed-development comparison preserved all
3,295 outputs and margins and reduced roughly 17.1 seconds to 5.1 seconds.
That is existing Linux evidence, not a Windows performance promise, and the
full development corpus is neither packaged nor rerun by this synthetic CI.
This package's local update reused reviewed Linux binaries and small synthetic
probes only. The original secondary-classifier Rust proof is separate future
integration work; no classifier/candidate/private weights are included here.

## Bounded loader preparation

See [LOADER-HARDENING.md](LOADER-HARDENING.md) for the isolated typed, bounded
original-asset loading API, unchanged-math checks, and remaining production gates.
