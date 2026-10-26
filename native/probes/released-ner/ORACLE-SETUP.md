# First hosted oracle setup repair

The first hosted proof run stopped before native inference:
https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38116011627

- Windows 2025: setup-python could not find Python 3.12.14 x64.
- Ubuntu 24.04: the three top-level pins installed, but importing spaCy failed
  with missing `click` in `spacy.cli._util`.

## Root causes and bounded correction

The official actions/python-versions manifest offers Python 3.12.10 x64 for both
Windows and Ubuntu 24.04. It offers 3.12.14 for Linux, not Windows. Python.org
identifies 3.12.10 as the last full maintenance release; later 3.12 releases are
security-only. The CPython v3.12.10 Unicode database is explicitly 15.0.0.
We now request **3.12.10** on both jobs. This is a real patch-level change from
the Linux reference's 3.12.14, not a claim that they are the same interpreter.
This older patch is only an isolated CI oracle for trusted fixed inputs; it is
not a deployed runtime or a general recommendation to downgrade Python.

The successful CI installation chose Typer 0.27.3. Its official dependency
metadata no longer includes Click, but spaCy 3.7.5 still imports Click directly.
The working local oracle has the same entire Linux package set plus Click 8.1.8.
Pinning that explicit runtime dependency addresses the proven missing import.
To avoid repeating this issue with drifting transitive dependencies, the entire
active oracle closure is now fixed, not just that one package:

- 42 Linux / 43 Windows packages; Colorama 0.4.6 is Windows-only.
- 65 Linux / 69 Windows active declared dependency edges validated against
  official PyPI metadata for Python 3.12.10 and the appropriate OS markers.
- 59 compatible wheel entries, each with official PyPI SHA256; installation is
  wheel-only with `--require-hashes`, and every package version is exact.
- All 42 Linux pins match the already working local reference environment.
- Click, Typer, Weasel, NumPy, Thinc, srsly, spaCy and spaCy CLI utility imports
  are checked before exporting or compiling anything.

`oracle-lock.json` records interpreter artifact URLs and manifest snapshot hash,
package versions, platform markers, dependency metadata and compatible wheel
hashes. `oracle-requirements.txt` is the executable pip hash lock. Metadata JSON
snapshot hashes document research provenance; the wheel hashes enforce actual
artifact integrity. No model, learned tensor, runtime source lock, original
reference, C/Rust implementation or native dependency was changed.

## Validation and next run

Eight small tests cover both OS dependency closures, the original missing-Click
failure, another missing transitive dependency, wrong Python patch and wrong
Unicode version, plus golden success/action-drift/missing-document cases. Existing local Python 3.12.14 verifies all 42 Linux pins and
imports. The unchanged 33-document / 170-action original golden passes the new
pre-build checker against the existing original export.

Neither a clean Python 3.12.10 installation nor Windows execution has been
performed locally. No large runtime or dependency installation was attempted in
the low-disk environment. The next hosted run must prove all of the following,
in order, and fail closed otherwise:

1. Actual Python version is exactly 3.12.10 and Unicode is exactly 15.0.0.
2. `pip check`, exact locked versions, active closure and imports pass.
3. Existing exporter verifies original model/runtime-source/tensor hashes.
4. New original oracle matches all frozen synthetic token metadata, lexical
   fields, actions and spans, including the unchanged close-margin fixture.
5. Only then may the existing pinned PCRE2 and native before/after parity gates run.

These setup fixes are not proof of platform parity until that run succeeds.
The reference retains its original 3.12.14 provenance. Generated model/oracle
arrays remain job-local and are not uploaded.

Official evidence:
- https://raw.githubusercontent.com/actions/python-versions/main/versions-manifest.json
- https://github.com/actions/setup-python/blob/main/docs/advanced-usage.md
- https://www.python.org/downloads/release/python-31210/
- https://www.python.org/downloads/release/python-31214/
- https://raw.githubusercontent.com/python/cpython/v3.12.10/Modules/unicodedata_db.h
- https://pypi.org/pypi/spacy/3.7.5/json
- https://pypi.org/pypi/thinc/8.2.5/json
- https://pypi.org/pypi/typer/0.27.3/json
- https://pypi.org/pypi/click/8.1.8/json
