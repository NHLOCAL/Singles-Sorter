# Android product notice and source package

`collect_android_licenses.py` is a separate, fail-closed entrypoint. Windows
profiles still require their existing `original-ai` root feature. Android uses
both measured `product` normal/build Cargo closures, preserving build helpers
conservatively: x86_64 has 56 packages (48 registry); aarch64 has 55 (47 registry).
The policies pin the lock, graph edges/features, registry identities, licenses,
archive checksums, exact notice bytes and local manifest locations. The CI source
and lock receipts must match the package checkout. There is no license inference
from an unfiltered Cargo.lock.

The collector requires existing authenticated inputs and performs no downloads.
It retains shared model/Unicode/PCRE2 notices, exact frontend helper/icon notices,
full build-Rust notices and pinned actual NDK r27d notices. TagLib's explicitly
offered MPL 1.1 route retains the unmodified complete TagLib and zlib archives on
the same distribution media, plus actual bridge/build recipes and the complete
upstream license texts. This does not license later modifications automatically.

Two archive omissions have explicit provenance:

- `jni-sys-macros` has no license file in its crate. Its authenticated VCS record
  binds complete MIT/Apache notices from the exact upstream repository revision.
- `cesu8` retains the complete Rust COPYRIGHT plus its same-terms README. The
  historical Rust revision's COPYRIGHT is byte-identical; its full MIT/Apache
  texts are included as pinned supplements alongside the unchanged crate texts.

`assets/licenses/index.html` is an escaped, script-free local index. The fixed
About viewer and explicit user-selected export consume `inventory.json`, schema1,
with `files: [{path, bytes, sha256, ...provenance}]`. Paths are relative to licenses;
the inventory itself is excluded from that list and is separately included in the
export. Limits are 4,096 files, 2 MiB inventory, 64 MiB per file, and 256 MiB total.
Only ASCII safe segments are accepted. No symlink or extra/missing file is allowed.
The APK builder reads one bounded snapshot, checks hashes, and writes exactly those
bytes; every added ZIP entry is recorded in `inputs.json`. Notice assets may be
compressed; native libraries remain stored and retain the 16 KiB alignment gate.

The first full collector/build and actual viewer/export checks remain required.
Portable tests establish input handling, not a completed Android distribution.
No signing keys, model binaries, source archives or dependency caches belong in Git.
