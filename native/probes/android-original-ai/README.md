# Android original-model inference probe

This is an **opt-in native inference experiment**, not an Android app or a claim
that Android sorting, SAF integration, JNI, Tauri, installation or UI works.
The shipping Android capability gates remain unchanged. No Python interpreter,
training code, model conversion, candidate model, private corpus, network model
fetch, or filesystem sorting belongs in the deployed probe.

## Smallest seam and rationale

Reuse `sorter-recognition` and its existing Rust NER and secondary-classifier
libraries directly. Keep all five original assets byte-identical. The secondary
classifier remains the original class-0 acceptance gate on `SINGER` spans; it
is not a replacement recognizer. The existing tests exercise the adapter,
original token/action/span oracle, resource limits and cancellation.

Cross-compile the existing test executables and `sorter-classifier`'s `parity`
example using the official NDK's Clang driver. Compile pinned PCRE2 10.46 as a
32-bit-code-unit static archive, Unicode enabled, JIT disabled. This avoids
another inference framework or changed arithmetic/model representation.

The build-script extensions and bounded asset loader permit `x86_64-linux-android` and
`aarch64-linux-android` **when `SINGLES_SORTER_ANDROID_PROBE=1` is set**. Other
Android targets remain rejected. An Android build without this flag must fail.
The loader retains regular-file, size, hash and nonblocking-open defenses.
The recognition runtime identity continues to bind the exact target, build
profile and linked PCRE2 archive hash.

Official references consulted:

- [Rust Android target support](https://doc.rust-lang.org/rustc/platform-support/android.html)
- [NDK with other build systems](https://developer.android.com/ndk/guides/other_build_systems)
- [NDK CMake toolchain](https://developer.android.com/ndk/guides/cmake)
- [PCRE2 build configuration](https://www.pcre.org/current/doc/html/pcre2build.html)
- [Tauri mobile prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Android JNI guidance](https://developer.android.com/ndk/guides/jni-tips)
- [Headless Android emulator](https://developer.android.com/studio/run/emulator-commandline)

Tauri already has an Android route, but generating a complete app would combine
UI, asset extraction and SAF lifecycle questions with the narrower inference
question. A hand-written JNI layer adds string/thread/lifetime boundaries that
are unnecessary for this first proof. If app integration follows, keep JNI
small, off the UI thread, and handle UTF-16/modified UTF-8 deliberately.

## Reproduction contract

Use an isolated checkout. Keep SDKs, models, generated oracles, third-party
archives, build directories and results outside Git. The probe expects the
already authenticated original bundle described in
[`../../docs/original-ai-preview.md`](../../docs/original-ai-preview.md).
The generated NER `fixtures.json` is test input, not a sixth deployment asset.

Tool versions for the initial experiment: Rust 1.99.0; NDK r27d
(27.3.13750724); PCRE2 10.46; Android API 23 minimum in compiler drivers.
The Android runtime under test is separately recorded; compiler API is not the
OS version of the emulator. Download SDK components from Google's official
repository and verify its published archive checksums before extraction.
Do not use preview or Google Play/add-on images under an unrelated agreement.

Install the matching Rust standard libraries with
`rustup target add x86_64-linux-android aarch64-linux-android`.

For each ABI, configure PCRE2 with the NDK CMake toolchain:

```sh
cmake -S "$PCRE2_SOURCE" -B "$PCRE2_BUILD" \
  -DCMAKE_TOOLCHAIN_FILE="$NDK/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI=x86_64 -DANDROID_PLATFORM=android-23 \
  -DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON -DCMAKE_BUILD_TYPE=Release \
  -DPCRE2_BUILD_PCRE2_8=OFF -DPCRE2_BUILD_PCRE2_16=OFF \
  -DPCRE2_BUILD_PCRE2_32=ON -DBUILD_SHARED_LIBS=OFF \
  -DPCRE2_SUPPORT_JIT=OFF -DPCRE2_BUILD_PCRE2GREP=OFF \
  -DPCRE2_BUILD_TESTS=OFF
cmake --build "$PCRE2_BUILD" -j2
```

For ARM64 change `ANDROID_ABI` to `arm64-v8a`. Inspect the CMake configuration
and archive architecture. Never point Android at the host's PCRE2 library.
The CMake flexible-page switch applies to the C build; it does not certify
final Rust ELF or APK alignment. The initial Rust probes have 4 KiB PT_LOAD
alignment and run on a 4 KiB-page emulator. Android 16 KiB-page compatibility
requires a separate final-link configuration and runtime check.
From `native/`, using absolute paths for the environment variables:

```sh
export SINGLES_SORTER_ANDROID_PROBE=1
export PCRE2_STATIC_DIR="$PCRE2_BUILD"
export CARGO_TARGET_DIR="$OUTSIDE_REPO_TARGET"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android23-clang"
cargo test --locked --release --target x86_64-linux-android \
  -p sorter-recognition -p sorter-classifier --no-run --message-format=json
cargo build --locked --release --target x86_64-linux-android \
  -p sorter-classifier --example parity
cargo test --locked --release --target x86_64-linux-android \
  --manifest-path probes/released-ner/released-ner-text/Cargo.toml \
  --no-run --message-format=json
cargo test --locked --release --target x86_64-linux-android \
  --manifest-path probes/released-ner/released-ner-math/Cargo.toml \
  --no-run --message-format=json
```

For ARM64 use `aarch64-linux-android` and
`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER` pointing at
`aarch64-linux-android23-clang`. Do not run ARM64 binaries on an x86_64 emulator
and describe that as ARM64 validation. These are separate ABI measurements.

## Runtime proof requirements

Use only an owned emulator/device fixture directory. Set `TMPDIR` to an owned
writable directory on Android. Push the five original bundle files plus the
separate generated test oracle; hash all of them before and after execution.
Run the test executables reported by Cargo with `profile.test=true`:

```sh
ORIGINAL_CLASSIFIER_ASSETS=/data/local/tmp/original-ai-proof/models \
ORIGINAL_AI_BUNDLE=/data/local/tmp/original-ai-proof/models \
ORIGINAL_NER_ORACLE=/data/local/tmp/original-ai-proof/models/fixtures.json \
ORIGINAL_NER_EXPORT=/data/local/tmp/original-ai-proof/models \
TMPDIR=/data/local/tmp/original-ai-proof/tmp \
./TEST_EXECUTABLE --include-ignored --test-threads=1 --nocapture
```

### Known Android shell fixture limit

On the initial Android 11 x86_64 AOSP emulator, the shell denied `mkfifo` in the
owned test directory before `fifo_is_rejected_without_waiting_for_a_writer`
could exercise the loader. Preserve that failing setup log and classify this
case as **environment-blocked**, not passed. Do not change Android security
settings or route around the denial. The other five loader cases can be rerun
with `--skip fifo_is_rejected_without_waiting_for_a_writer` appended to the
command above. Keep its one filtered test explicit in the final accounting.
The preserved regular-file checks and NDK-verified `O_NONBLOCK` constant are
source evidence; they are not Android FIFO/path-swap runtime validation.

Compare the `parity` example's output on the same owned synthetic classifier
inputs against the verified desktop baseline: exact classes, accept/reject,
feature strings and sparse indices; report maximum numerical deltas separately.
Record the class histogram rather than testing only a positive example.

Record Android build fingerprint/API/ABI, kernel, page size, compiler/NDK,
PCRE2 archive hash/configuration, source commit/tree and patch, binary hashes
and sizes, asset hashes and total bytes, complete commands, exit codes, test
counts and timings. Test-binary sizes include the test harness and are **not**
APK or release-app sizes. Software-emulated timing is **not** physical ARM64
performance. Compile-only success is never runtime validation.

No publication or broad capability claim follows automatically from this
experiment. An independent review must inspect the exact source commit and
runtime evidence before any proposed publication.
