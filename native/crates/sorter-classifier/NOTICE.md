# Provenance and notices

Singles-Sorter original model and sorting policy: NHLOCAL/Singles-Sorter,
MIT License, Copyright (c) 2022 NHL.OCAL11@gmail.com. The full license is retained
at the repository root LICENSE.

The native original-only classifier implementation is a new reconstruction and
bounded adaptation, not original scikit-learn source code. It consumes exported
original parameters only after verifying immutable asset fingerprints.

Frozen Unicode tables derive from CPython 3.12.10 / Unicode 15.0.0 predicates.
CPython is licensed under the Python Software Foundation License Version 2;
Unicode data is covered by the Unicode data-file/software license. Their notices
are reproduced alongside these tables in UPSTREAM-NOTICES.txt.

scikit-learn (BSD-3-Clause) and NumPy (BSD-3-Clause) are build/verification-only
Python oracle dependencies, not embedded runtime libraries. Their pinned wheels
retain their upstream notices. Rust runtime dependencies retain their own crate
licenses (sha2 MIT/Apache-2.0, libc MIT/Apache-2.0).
