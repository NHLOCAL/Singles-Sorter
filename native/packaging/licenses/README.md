# Windows original-AI preview license collector

This is a bounded packaging gate for `sorter-cli --features original-ai` on
`x86_64-pc-windows-msvc`, not a general-purpose license scanner. It needs Python
3.11+ (standard library only), the pinned Rust toolchain, and the native archives
that the successful build has already cached. It never publishes anything.

From the repository root:

```powershell
python native/packaging/licenses/collect_licenses.py `
  --repo-root . --output-dir path/to/stage/licenses `
  --target x86_64-pc-windows-msvc `
  --dependency-cache path/to/verified-native-source-cache
```

The output must not already exist. The helper stages all inputs and renames its
output only after every check succeeds. If it fails, do not package a partial
licenses directory. `--offline` disables Cargo network resolution. Without it,
only `cargo metadata --locked` can download missing official registry packages;
the helper itself performs no network requests. Native archives are never
silently downloaded or substituted by this helper. The cache fallback is
`SORTER_DEPENDENCY_CACHE`, then the same per-platform cache as native CMake.

`--cargo-metadata path.json` can reuse output from this exact command:

```text
cargo metadata --format-version 1 --manifest-path native/Cargo.toml --locked --features sorter-cli/original-ai --filter-platform x86_64-pc-windows-msvc
```

The root must enable `original-ai`, and the normal/build dependency closure must
exactly match the reviewed 45 package identities: 37 crates.io distributions and
8 local crates. Dev-only dependency edges are ignored. Build helpers and proc
macros are retained conservatively, so this is not a linker-level binary SBOM.
Cargo's workspace feature resolution can conservatively retain additional
features, but cannot silently add/remove a package from the reviewed set.

The collector checks every registry archive against both `Cargo.lock` and the
reviewed package checksum. It reads complete notice files from those archives,
checks their reviewed content hashes, and verifies that extracted build-source
notices agree. It retains every reviewed license/notice file, including both
branches of common MIT/Apache alternatives, memchr's COPYING, typenum's additional
LICENSE, and unicode-ident's mandatory Unicode notice. Git dependencies, source
replacement, new versions/licenses, a changed closure, missing registry archives,
and notice text drift fail closed rather than using generic license templates.

Retained project/AI/native notice hashes normalize CRLF to LF only so ordinary
Git Windows checkouts work. Output always preserves the actual complete files.
The full PCRE2 source notice and CPython license are mandatory; pointer files
cannot substitute for them. The policy files are review inputs, not generated
approval: a dependency update requires reviewing its complete source/license
notices and updating this bounded inventory deliberately.

`rust/` receives the active, pinned compiler's complete COPYRIGHT.html,
COPYRIGHT-library.html, all `licenses/` files, and any root LICENSE files. This
includes Rust's dependent-code notices rather than only a top-level MIT/Apache
label. `--rust-sysroot` must match `rustc --print sysroot`; it is not a way to use
another compiler's notices.

`licenses/sources/` contains exact TagLib 2.3.2 and zlib 1.3.1 archives, verified
by SHA256, plus the actual owned bridge/acquisition/build files. The generated
SOURCE-AVAILABILITY.md explicitly selects TagLib's MPL 1.1 route, identifies
same-media source and build choices, and explains what must change if upstream
source is modified. It does not promise future third-party source hosting.

Generated `inventory.json` records component versions, source provenance,
selected native license, Cargo archive hashes, exact included file sizes/hashes,
source revision, and dirty-tree state. The inventory excludes its own recursive
hash; the parent package manifest must hash inventory.json as an ordinary file.
Keep SOURCE-AVAILABILITY.md conspicuously linked from the preview's README.
Microsoft CRT/SDK terms and entitlement to redistribute compiler runtime code
remain a separate packaging check. This helper does not add runtime DLLs or
assert legal clearance.

## Tests

```text
python -m unittest discover -s native/packaging/licenses -p 'test_*.py' -v
```

Tests use temporary directories and synthetic fixtures only. For a real
integration check, run the command above with the exact downloaded archives and
build toolchain, inspect inventory.json, and verify the parent package manifest
includes every license/source file. A Linux-hosted target-filtered inventory is
not proof that a Windows executable was built or executed.
