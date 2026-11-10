# Singer recognition engine

Rust port of the original `singer_ner_he` spaCy model. The two crates here,
`released-ner-text` (tokenizer and features) and `released-ner-math` (inference),
are dependencies of `crates/sorter-recognition`. The directory keeps its
historical name because license reviews and build inputs refer to these paths.

- The model itself is not stored here. Builds export it from the public
  baseline commit `78d001aee5ce49628bdffebd34540fa83ba32c12` with
  `released-ner-export/export.py` and verify every asset hash.
- `acquire_pcre2.py` downloads the hash-pinned PCRE2 release from
  `pcre2-lock.json` and builds the static library the tokenizer links against.
- Python is used only for export and comparison. The shipped binaries have no
  Python runtime dependency and never download anything.
- Upstream licenses are in each crate's `licenses/` directory and in
  `ORIGINAL-LICENSE.md`.

## Local check

    python released-ner-export/export.py --model ORIGINAL_MODEL_DIRECTORY --out .oracle --suite platform
    python released-ner-export/verify.py .oracle
    python acquire_pcre2.py --out .pcre2 --build

Set `PCRE2_STATIC_DIR` to the absolute `.pcre2/install/lib` directory, then run
`cargo +1.99.0 test --locked --manifest-path released-ner-text/Cargo.toml` and
the same for `released-ner-math`. `.github/workflows/released-ner-platform.yml`
runs these checks on Windows and Linux whenever this directory changes.
