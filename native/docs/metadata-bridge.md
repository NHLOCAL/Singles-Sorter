# Read-only metadata bridge: build and acceptance

The owned adapter links TagLib 2.3.2, bundled utf8cpp 4.2.0 and static zlib 1.3.1.
Source tarball identities, per-file hashes and licenses are in
`crates/sorter-io/dependencies/source-manifest.json`. Official archives are
automatically downloaded at build time, SHA256-verified and cached; product
runtime needs no downloads. See [cache and offline builds](../crates/sorter-io/dependencies/README.md).
CMake >=3.24, a C++17 compiler and the pinned Rust toolchain are required.
The checked Linux evidence used official CMake 4.1.3 and GCC 14.2.0.

## Native Windows CI recipe (execution still pending)

Use the existing `windows-2025` native runner with its official Visual Studio
Build Tools and CMake. No local installation or remote build is implied here.
The build script accepts native `x86_64-pc-windows-msvc`, uses CMake's installed
Visual Studio generator with `-A x64`, and builds the Release configuration with
at most two parallel jobs. All three owned/static archives are emitted to one
configuration-independent `lib` directory. It does not link GNU/Linux libraries
or treat a GNU cross-check as MSVC execution.

Run from `native` in PowerShell:

    cmake --version
    cargo +1.99.0 test --locked -j 2 -p sorter-io --test metadata_reference --test metadata_errors
    cargo +1.99.0 test --locked -j 2 --workspace
    cargo +1.99.0 clippy --locked -j 2 --workspace --all-targets -- -D warnings
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    cargo +1.99.0 test --locked -j 2 --release -p sorter-io --test metadata_reference --test metadata_errors
    cargo +1.99.0 build --locked -j 2 --release -p sorter-cli

The CMake MSVC runtime is `MultiThreaded` when Rust's effective target features
include `crt-static`, otherwise `MultiThreadedDLL`; all native code is built in
Release configuration even for Rust debug tests. The first test invocation uses
the runner's installed dynamic CRT. The release invocation is the all-static
CRT artifact gate. Archive/CRT inspection and execution must pass on the actual
runner before promoting native Windows support.

`metadata_reference` executes all frozen real-format cases and real-file
single-song route comparisons on native Linux and native Windows. On Windows it
also opens a Hebrew/supplementary-plane filename through `DesktopStorage` and
verifies bytes and modification time. Cross-target checks and Android return an
explicit `UnsupportedPlatform`, rather than pretending a host bridge was built.
Native Android FD/seek/ownership support is a separate pending gate.

## Owned boundary

`ReadStorage` supplies a bounded `ReadInput`. The bridge never opens a path and
ignores the display name for container selection. It fills exactly the bounded
available header prefix with `read_exact`, tolerating short and Interrupted
reads while preserving typed observed-length truncation errors. It retains a stable Rust input
context while the opaque C++ handle owns TagLib's read-only IOStream/file. C++
exports catch exceptions; Rust callbacks catch unwind. There is no write, save,
set, rename or transfer entry point. Every result allocation is destroyed using
the allocating C++ domain. Get/value stages remain distinct, and callers control
field order. Querying malformed track data does not poison later title reads.

A read block and aggregate returned field bytes are bounded at 16 MiB; a handle
permits 256 MiB of cumulative read requests. These I/O/result limits are not a
universal intermediate native-heap bound. Xiph, APE and MP4 currently enforce
component counts after native parsing; their intermediate allocations are not
covered by the ID3 preflight guarantee below. Shipping/general hostile-input
acceptance remains blocked on independent review of those paths. Recordings are never read wholesale
into a Rust buffer by the adapter. Streaming input fails `SeekRequired`; callers
must use the separately authorized/bounded seekable spooling adapter. OS access
times may change on reads. Budgets and unsupported representations are errors,
never invented empty tags.

TagLib drops empty ID3 text components. The owned bridge therefore retains
original bounded v2.3/v2.4 text frame bytes and Rust decodes them without that
normalization. A bounded raw-byte preflight now runs before every ID3-bearing
TagLib file constructor (MPEG, WAV, AIFF and DSF). It caps tag bytes at 16 MiB,
and both frame counts and aggregate text components at 4,096 (including ignored text fields),
with conservative charges for genre-expansion syntax. Unsupported flags on any
frame, v2.2, duplicate relevant frames, structured frame classes
CHAP/CTOC/SYLT/ETCO/RVA2 and malformed encoding layouts are blocked before native
tag parsing.
RIFF/AIFF envelope chunk counts and WAV INFO field counts are capped. Duplicate
ID3 chunks and MPEG APE trailers are conservatively unsupported pending their
own preflight proof. LIST/BEXT/iXML bytes have a separate aggregate 16 MiB
budget; INFO remains ignored by the legacy metadata projection.
The exact preflight header/body windows are retained in a bounded replay cache
(maximum 32 MiB + 64 KiB and 8,196 regions), so a changing provider cannot replace
validated metadata with an over-budget payload before TagLib parses it. This is
a metadata snapshot, not a claim that the entire recording is immutable. The
post-query raw decoder still validates returned values independently. Xiph and
APE use the format-specific ordered value APIs;
MP4 uses its format-specific item/tuple API. The frozen multi-tuple case proves
only that class. The coverage file lists remaining representation limitations.

## Sanitizer recipe

    cmake -S crates/sorter-io/bridge -B /tmp/sorter-metadata-sanitized -DSORTER_BRIDGE_TESTS=ON -DCMAKE_BUILD_TYPE=Debug -DCMAKE_CXX_FLAGS='-fsanitize=address,undefined -fno-omit-frame-pointer' -DCMAKE_C_FLAGS='-fsanitize=address,undefined -fno-omit-frame-pointer' -DCMAKE_EXE_LINKER_FLAGS='-fsanitize=address,undefined'
    cmake --build /tmp/sorter-metadata-sanitized --target sorter_bridge_tests --parallel 2
    ASAN_OPTIONS=detect_leaks=1 /tmp/sorter-metadata-sanitized/sorter_bridge_tests tests/fixtures/media/*.mp3 tests/fixtures/media/*.flac

The harness accepts the full 60-file media list, repeatedly opens/queries/destroys
handles, exercises truncations, 3,000 deterministic malformed byte buffers, and
invalid ABI versions/lengths. Passing it is finite evidence, not exhaustive fuzz
proof or a substitute for native Windows/Android tests.

### Round-2 native discovery boundary

ID3v2.3 `IPLS` is charged as its effective native text class `TIPL` before
construction. The other pinned v2.3 conversions (`TORY`, `TYER`) and the
`TRDC` typo conversion already start with `T`. All v2.3 three-character IDs
padded with NUL or space are explicitly unsupported: TagLib's iTunes
compatibility path otherwise applies the separate v2.2 alias table, including
text, genre, Apple and structured-frame aliases. Ordinary v2.2 remains gated.

MPEG construction explicitly uses `AudioProperties::Fast` with audio properties
disabled. Pinned `findID3v2` then checks offset zero and returns without scanning
for displaced ID3. Only that offset-zero ID3 is parsed and preflight-cached.
Subsequent audio-frame scanning does not construct additional ID3 tags. For
inputs without offset-zero ID3, reserved version/layer/sample-rate/bitrate fields
and free-format bitrate zero are conservatively unsupported. The review's
`ff e2 00 00` prefix is therefore rejected, although pinned Python opens it with
empty metadata; this is an explicit representation limitation, not a claimed
legacy output mismatch. Plausible MPEG prefixes can still open with empty
metadata, but displaced ID3 is never passed to the native ID3 parser. Frozen
normal container projection remains unchanged. No universal heap bound or new
platform/sanitizer acceptance follows from these checks.
