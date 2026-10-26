# Official Windows oracle source-byte provenance

Third hosted run:
https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38118567365

Windows now proves original model checkout `i/lf w/lf` and the unchanged config
hash. All 43 package pins/imports and Python 3.12.10 pass. Export then correctly
rejects the Linux raw-byte hash of `spacy/tokenizer.pyx`. Linux passes again.

## Complete source comparison, not a one-file exemption

Downloaded both exact already-locked official CPython 3.12 Windows x64 wheels
and verified their full SHA256 before reading any members:

- spaCy 3.7.5: `07677e270a6d729453cc04b5e2247a96a86320b8845e6428d9f90f217eff0f56`
- Thinc 8.2.5: `c5f750ea2dd32ca6d46947025dacfc0f6037340c4e5f7adb9af84c75f65aa7d8`

Compared **all 18** runtime source files guarded by the exporter. Every Windows
file uses CRLF. Every file is byte-identical to the unchanged original Linux
source after CRLF-to-LF comparison only. Independently downloaded all 18 matching
files from official spaCy `v3.7.5` and Thinc `v8.2.5` tags; all upstream raw hashes
also exactly equal the existing Linux/reference hashes. No code difference was
found. Both wheel artifacts are already authorized by the reviewed hash lock.

`released-ner-export/windows-runtime-sources.json` preserves each exact raw
Windows hash, byte/CRLF counts, original LF hash, full wheel digest/download URL,
and upstream source URL/hash. It contains hashes and provenance, not wheel
binaries or copied upstream implementation code.

## Fail-closed verification

Linux/non-Windows verification stays byte-strict against the original hash.
For Windows only, each guarded source must first match its **exact known raw
Windows hash**. Only then must CRLF-to-LF comparison produce the unchanged
original source hash. Neither mixed endings nor another file that merely
normalizes to the original is accepted. Unknown paths, changed reference hashes,
code edits or changed raw distributions fail closed.

This comparison is source-provenance validation. It does not rewrite installed
sources, normalize model/tokenizer data, transform user inputs, change matching
behavior or accept arbitrary platform-specific code. The manifest now records
actual source platform, raw hashes and original reference hashes separately.
The original release lock itself is unchanged.

Eight small tests cover exact known Windows bytes, unchanged Linux strictness,
LF-only/mixed Windows rejection, code changes, wrong reference, semantic drift
with a modified raw allowlist, and complete correspondence to the existing
source and wheel locks. Local direct validation additionally passes every member
of the two actual hash-verified Windows archives; no Windows binary is executed.

## Remaining guards reviewed together

- Interpreter patch/Unicode and full wheel closure: already pass on both hosts.
- All 11 original model files: remain exact raw-byte hashes; LF checkout is fixed.
- All 18 runtime-source guards: completely reviewed as above, not skipped.
- Graph nodes/attributes, 22 action rows, greedy mode, 31 parameter shapes/raw
  float32 hashes and finiteness: unchanged and still mandatory.
- Tokenizer normalization structure and original live-pipeline spans: unchanged.
- Generated JSON may have platform line endings; it is parsed as JSON while
  its manifest hash covers actual bytes. Tensor bytes remain exact little-endian.
- Original synthetic golden, close-margin case, numerical state/bit checks,
  optimized-versus-baseline diagnostics, errors and spans: unchanged.
- PCRE2 static linking, compiler behavior and floating-point choices still need
  actual Windows execution. A source-byte match does not prove numerical parity.

A fresh local Linux export with this change preserves original tensor bytes and
passes the 33-document/170-action original golden and existing-native gate.
The next hosted Windows job must execute all remaining gates; no Windows/native
success is claimed from this provenance repair.

Primary artifact links are in `oracle-lock.json` and the source-byte receipt.
Official package metadata:
- https://pypi.org/pypi/spacy/3.7.5/json
- https://pypi.org/pypi/thinc/8.2.5/json
