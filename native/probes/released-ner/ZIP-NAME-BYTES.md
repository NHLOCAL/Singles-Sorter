# Exact ZIP name bytes across Windows and Linux

Hosted Windows run 38119547787 passed the original source/export guards and
33-document/170-action original golden comparison, then failed the malformed
backslash-name acquisition test before dependency acquisition or native build.

CPython 3.12.10 `ZipInfo` saves `orig_filename` and normalizes `filename`:
Windows separators become `/`, and a NUL truncates the name. This happens in
both the test writer and the archive reader. Consequently the old test writer
created a safe name on Windows, and merely fixing that writer would still
leave normalized archive names invisible to the validator.

Official implementation reference:
https://raw.githubusercontent.com/python/cpython/v3.12.10/Lib/zipfile/__init__.py
(`_sanitize_filename`, `ZipInfo.__init__`, and central-directory parsing).

The test writer now assigns its deliberately malformed filename after
`ZipInfo` construction, then checks that the archive preserves the exact
original name. The acquisition validator rejects any difference between
`orig_filename` and `filename` before the existing complete path checks and
before creating any extracted files. This also rejects NUL truncation and
alternate Unicode-name substitutions. There is no normalization-based
acceptance fallback. The official source archive digest remains unchanged.

Two regressions reproduced the previous acceptance of backslash and NUL names
under the Windows separator setting. Both pass after the fix. Additional tests
check a valid archive under that setting and the caller boundary: a malformed
archive cannot be promoted or cause a build, and staging is removed. The
11 acquisition tests and all 32 focused Python tests pass locally using the
existing Linux Python 3.12.14 runtime. Simulated separator tests exercise the
same standard-library branch; they are not a Windows execution claim.

The original model/export, source locks, runtime code, golden reference,
optimization gates, and PCRE2 acquisition hash are unchanged. Hosted Windows
native compilation and parity remain pending.
