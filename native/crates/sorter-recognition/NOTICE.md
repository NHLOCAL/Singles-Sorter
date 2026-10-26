# Original AI integration provenance and notices

This adapter reuses the existing bounded original NER libraries directly. It does
not embed model weights, a pickle interpreter, CPython, NumPy, spaCy, scikit-learn,
or a replacement classifier. Exporters and Python oracles are developer-only.

- Original Singles-Sorter application/model release: NHLOCAL/Singles-Sorter,
  commit `78d001aee5ce49628bdffebd34540fa83ba32c12`, MIT; retain the
  [repository license](../../../LICENSE).
- NER numeric reconstruction provenance and upstream spaCy/Thinc MIT and NumPy BSD
  notices: [NER math notices](../../probes/released-ner/released-ner-math/licenses).
- Tokenizer reconstruction, frozen CPython/Unicode data: [NER text notices](../../probes/released-ner/released-ner-text/licenses),
  including full `CPython-LICENSE.txt` and `Unicode-data-packaged.txt`.
- Statically linked PCRE2 10.46: BSD-3-Clause WITH PCRE2-exception. The complete
  actual source notice is [pcre2-source-10.46.md](../../probes/released-ner/released-ner-text/licenses/pcre2-source-10.46.md).
  `pcre2-COPYING.txt` alone is only an upstream pointer and is insufficient.
- Original secondary reconstruction/Unicode tables:
  [classifier provenance](../sorter-classifier/NOTICE.md) and
  [full retained notices](../sorter-classifier/UPSTREAM-NOTICES.txt).
- The actual CLI also links the owned metadata bridge and TagLib/zlib/utf8cpp:
  [native dependency obligations](../sorter-io/dependencies/README.md) and
  [retained license texts](../sorter-io/dependencies/licenses).
- Rust library dependency licenses remain in their exact Cargo registry source
  distributions, with versions and checksums bound by `native/Cargo.lock`.

These references make review discoverable; any future redistributable binary
package must include applicable complete notices and fulfill upstream
source-availability obligations. This read-only integration does not approve a
production package or supersede those obligations. No artifact containing model
assets is published by the focused verification workflow.
