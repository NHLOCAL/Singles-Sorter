# Original-only Windows CLI preview packaging

This directory adds packaging to the existing `original-ai-cli.yml` proof. It
changes no runtime features and publishes no release. A downloadable CI test
artifact exists only when the real Windows job has passed all original-AI gates,
packaging, independent PE inspection and the packaged smoke test. Artifact access
follows the repository's permissions; do not describe a public-repository
artifact as private.

## Distribution boundary

The packager selects exactly the feature-enabled `singles-sorter.exe`, the five
verified runtime assets (four invariant hash pins plus a semantically verified
Windows exporter manifest whose actual digest is recorded), newly authored silent WAVs, the UTF-8 Hebrew guide,
Python-free PowerShell smoke tool, complete applicable notices/source material,
provenance and a SHA-256 manifest. It never copies an exporter directory wholesale.
A pickle, oracle corpus, Python environment, private library and candidate model
must never enter the package. The license helper separately selects only reviewed
notice/source inputs; official TagLib/zlib source archives are included to make
exact source availability independent of a moving repository link.

Model bundle ID `original-v14-ner-3.7.5` identifies the compatibility bundle, not
the model's own version: `singer_ner_he` metadata says version **2.0.0**.

This is a Windows x64 **read-only planning** CLI preview with external models and
dynamic MSVC CRT. It is not a portable single EXE, GUI, installer, signed release,
physical sorter/tag writer, Android acceptance result or arbitrary-library parity
claim. Python is needed to build/verify, never to run the packaged application.
Prerequisites and the official VC++ x64 Redistributable link are in the bundled
guide. No Microsoft DLLs are copied. Inspecting imports and clearing developer
PATH does not prove that a clean machine already has the required CRT installed.
No network isolation or absent-prerequisite clean-machine test is claimed.

## Fail-closed sequence

1. Build default-off and original-AI release executables in the existing workflow.
2. Pass the original Python classifier/NER parity gates, workspace tests and all
   44 actual release-CLI invocations using authored audio. The report must bind
   the exact executable SHA-256 and pinned original baseline.
3. Record native compiler identity; inspect actual DLL dependencies using the
   runner's official MSVC `dumpbin /dependents`. The packager separately parses
   AMD64 PE32+ normal DLL and symbol/ordinal imports. DLL sets must agree exactly;
   delayed imports or unreviewed DLLs fail instead of being silently omitted.
4. `package_preview.py build` runs only on actual Windows x64. It verifies every
   source asset (including the exact approved Windows runtime-source provenance
   in manifest.json, whose bytes differ from Linux), produces a new staging directory, gathers complete reviewed
   licenses/source, and emits provenance plus `SHA256SUMS.txt`. The manifest
   covers every file except itself. The final ZIP has its own sidecar hash.
5. Run `powershell.exe -NoProfile -NonInteractive -File smoke.ps1 -PackageRoot ...
   -ReportPath ...` under Windows PowerShell 5.1. It copies into a fresh Hebrew/
   space path and uses unrelated CWD plus System32-only child PATH. It checks
   help, feature capabilities, actual original NER, aliases/Disabled, explicit
   failures, runtime/resource binding and blocked apply. Hashes, names, ACLs,
   attributes and modification times must remain unchanged; read access time is
   intentionally excluded. Its report is outside the package.
6. `package_preview.py archive` accepts only the exact manifest and executable
   bound to a successful smoke report. It rechecks assets, PE imports, each ZIP
   entry and decompressed hashes. CI then extracts the actual ZIP with Windows
   `Expand-Archive` and repeats the PowerShell 5.1 smoke against the extracted
   contents. Only ZIP, sidecar hash and this final smoke report are
   retained as a seven-day CI artifact. No release API is called.

The stage and ZIP contain an empty `examples/target/` directory. No source/model
file is writable by the planning command as part of this smoke. Its intentionally
corrupt/missing-model copies and its preference sentinel are owned disposable
test inputs, never changes to the supplied original bundle.

## Local structural checks

From the repository root:

    python -m unittest discover -s native/packaging/windows-preview
    python -O -m unittest discover -s native/packaging/windows-preview
    python -m unittest discover -s native/packaging/licenses

Synthetic PE/unit tests are not Windows execution evidence. Linux must not
manufacture a distributable Windows artifact or claim Windows acceptance. Synthetic
unit-test PE/ZIP fixtures are disposable structural inputs, not runnable builds.
The actual Windows gates remain the authority.
