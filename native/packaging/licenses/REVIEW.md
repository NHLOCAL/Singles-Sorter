# License/source review scope and primary references

Reviewed 2026-10-11 for the bounded Windows original-AI preview. This records an
engineering packaging route and evidence, not a warranty or blanket legal
opinion. Complete upstream texts control; summaries do not relicense code.

## TagLib 2.3.2: MPL 1.1 selected

The exact official archive is pinned in the native dependency lock and supplied
in full in the output. Its per-file notices explicitly offer LGPL 2.1 or MPL
1.1; for this preview, MPL 1.1 is selected. The project's own API documentation
confirms that one of these alternatives can be chosen. The stock LGPL-only C
wrapper is disabled; the application uses its separate owned bridge.

- Official exact source: https://taglib.org/releases/taglib-2.3.2.tar.gz
- Versioned upstream README: https://raw.githubusercontent.com/taglib/taglib/v2.3.2/README.md
- Versioned full license: https://raw.githubusercontent.com/taglib/taglib/v2.3.2/COPYING.MPL
- Upstream explanation of license alternatives: https://taglib.org/api/
- License publisher's complete MPL 1.1: https://www.mozilla.org/en-US/MPL/1.1/

The conservative route is same-media delivery of the exact complete source,
with its notices and complete owned compilation scripts. MPL 1.1 defines source
in section 1.11 and covers executable notices, modifications and source access
in sections 3.1–3.7. Future actual source changes must be supplied with dated
change descriptions and retained notices. The collector does not substitute an
upstream download link for the actual source or undertake future hosting.

## Other native and reconstructed components

- zlib 1.3.1 official source: https://zlib.net/fossils/zlib-1.3.1.tar.gz
- zlib license publisher: https://zlib.net/zlib_license.html
- utf8cpp's exact BSL-1.0 text is inside the pinned TagLib archive under
  taglib-2.3.2/3rdparty/utfcpp/LICENSE; the collector byte-compares the retained
  notice with that archive. This bundled source is the authority for version
  4.2.0 used here, rather than a floating master branch.
- PCRE2 10.46 complete source license:
  https://raw.githubusercontent.com/PCRE2Project/pcre2/pcre2-10.46/LICENCE.md
- CPython 3.12.10 complete license:
  https://raw.githubusercontent.com/python/cpython/v3.12.10/LICENSE
- Unicode current full license: https://www.unicode.org/license.txt

All prior retained original-project, spaCy, Thinc, NumPy, CPython and Unicode
notices are included verbatim. The older tokenizer Unicode packaged notice is
kept along with the classifier's complete modern Unicode notice; no generic
SPDX label is used to erase differences between those retained texts. Native
predicate tables and original-model parameter exports do not imply distribution
of the Python, NumPy, spaCy, Thinc, or scikit-learn runtimes. The inventory keeps
these notices because of their reconstruction/data provenance.

## Rust and Cargo

- Official Cargo metadata format and target/feature selection:
  https://doc.rust-lang.org/cargo/commands/cargo-metadata.html
- Cargo dependency locking:
  https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html
- Rust licensing and dependent-code scope:
  https://rust-lang.org/policies/licenses/

The reviewed Cargo manifest pins each actual target closure package version,
crates.io source and archive checksum. The complete license/notice files come
from those exact original registry archives, not downloaded generic SPDX
texts. Any changed package, license, checksum or retained file fails closed.
Full notices from the actual build compiler sysroot are retained too, including
COPYRIGHT-library.html and the compiler's dependent-code notices. Conservatively
retaining toolchain/procedural-macro notices does not claim those executables
are shipped with this application.

## Boundaries

The helper does not inspect machine code to prove dependency inclusion or grant
Microsoft CRT/SDK redistribution rights. The parent packaging gate is responsible
for using the recorded compiler/flags, matching the built binary, running the
Windows clean-directory test, and hashing the entire package. Source archives
are license/source materials, not runtime model assets. Generated packages must
still undergo the ordinary distribution review before publication.
