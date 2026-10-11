# Native dependencies

The product links TagLib 2.3.2 (with bundled utf8cpp 4.2.0) and zlib 1.3.1
statically. Their source trees and build outputs are not tracked in Git.
`lock.json` is the small authoritative download/version/SHA256 lock. CMake
FetchContent downloads official release archives at build time and reuses a
verified archive cache; nothing is downloaded when an end user runs the program.
The lock is separate from Cargo.lock, which controls Rust crates only.

## Build and cache

CMake 3.24+, a C++17 compiler and the native Rust toolchain are required. Normal
`cargo build --locked` acquires missing native sources automatically. The default
source cache is `%LOCALAPPDATA%/SinglesSorter/cache/sources` on Windows,
`$XDG_CACHE_HOME/singles-sorter/sources` when set, or
`$HOME/.cache/singles-sorter/sources` elsewhere. Set the absolute
`SORTER_DEPENDENCY_CACHE` environment variable to choose another directory.
The archive cache is shared; extracted and compiled objects are isolated inside
Cargo's OUT_DIR for each target and CRT configuration.

For disconnected/filtered networks, place the exact official archives named in
`lock.json` into that cache and set `SORTER_DEPENDENCIES_OFFLINE=ON`. Hashes are
checked even in offline mode. A missing or corrupt archive stops with an explicit
error; certificates are never bypassed and no system-library fallback occurs.
Cargo's `--offline`/`--frozen` govern Cargo itself, so also set this native offline
option when a build must make no dependency-network requests.

CMake standalone builds accept the same values as `-D` options. Generic
`FETCHCONTENT_SOURCE_DIR_*` overrides are rejected: use a verified cached archive
instead. CMake's generic fully-disconnected switch alone cannot populate a fresh
build tree, so our explicit offline archive path handles that case. Downloads
have bounded connection inactivity/total time and a cache population lock.

## Integrity and distribution

`source-manifest.json` retains the previously reviewed source inventory. The
metadata tests verify all 702 retained upstream files against this inventory in
the extracted build sources, plus the unchanged owned media fixtures. Archive
verification also covers upstream data omitted from the original Git snapshot;
those upstream recordings are neither our test corpus nor product assets.

Original notices are retained in `licenses/` and in downloaded archives. TagLib
core permits LGPL 2.1 or MPL 1.1; its stock LGPL-only C wrapper is not linked.
The owned wrapper does not relicense upstream. utf8cpp uses BSL-1.0 and zlib its
own zlib license. Release packaging must retain applicable notices and fulfill
source-availability obligations for the selected upstream license. Removing
vendored files from Git does not remove those obligations. No final distribution
license choice or platform acceptance is asserted here.

## Why this acquisition mechanism

Cargo remains the Rust package manager. CMake FetchContent is its native build's
existing build-system mechanism, avoiding a second manager and custom package
recipe for only two C/C++ sources. Checked on 2026-10-11: the official vcpkg
TagLib port was 2.3.1 and Conan Center offered 2.0, whereas our frozen parser is
2.3.2. Adopting either unmodified would change the validated parser; maintaining
an overlay just for that would add work. Reassess a manifest-based vcpkg/Conan
integration if the native dependency graph grows. Automatic retrieval and exact
version selection are compatible; silently floating to latest is not required.

Primary references:
- https://cmake.org/cmake/help/latest/module/FetchContent.html
- https://cmake.org/cmake/help/latest/module/ExternalProject.html#download-step-options
- https://doc.rust-lang.org/cargo/reference/build-scripts.html
- https://learn.microsoft.com/en-us/vcpkg/concepts/manifest-mode
- https://raw.githubusercontent.com/microsoft/vcpkg/master/ports/taglib/vcpkg.json
- https://raw.githubusercontent.com/conan-io/conan-center-index/master/recipes/taglib/config.yml
- https://developer.android.com/ndk/guides/cmake
- https://taglib.org/api/
- https://zlib.net/zlib_license.html

The acquisition path works with CMake/NDK toolchains, but the Rust bridge still
rejects unsupported cross-targets. Android runtime acceptance remains separate.
