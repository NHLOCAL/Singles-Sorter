# Original recognition Android bridge (fixture component)

This is a **read-only development library and owned-fixture APK**, not the
Singles Sorter Android application. The production Tauri capability gates,
sorting rules, metadata backend and SAF adapter are unchanged. No file sorting,
provider mutation, storage permission, network permission, model download,
training, replacement model or Python runtime is part of this component.

## Boundary and decision

`sorter-recognition` remains the only inference implementation. A small Rust
`cdylib` exposes begin/run/cancel/abandon/status through JNI. `OriginalInference`
provides a Java worker owner usable by a later Tauri mobile plugin; it does not
require a WebView and has no callbacks from Rust into Java. Models are loaded,
used and dropped on the calling worker. Opaque monotonic IDs never contain raw
pointers. One process-wide native job is allowed; cancellation never frees a
running model. Closing an owner suppresses callbacks, cancels, and retains the
busy state until JNI returns. A generation ID rejects stale queued callbacks.

The existing Tauri host is desktop-oriented (desktop WebviewWindow setup and
filesystem roots, no generated Android project). Its metadata capability is
unavailable on Android. The separately reconstructed SAF adapter is not wired
into this host and cannot verify ancestry on API 24–28. Enabling that application
would therefore combine unrelated, unverified capabilities. This isolated
recognition seam avoids claiming that integration is complete.

JNI accepts ordinary UTF-8 byte arrays, not JNI modified UTF-8. Lengths are checked
before copying: path 4096 bytes, request 65536 bytes, up to 32 nonempty-list documents
of 4096 bytes each. Invalid UTF-8/JSON, unknown fields, null arrays, stale IDs and
repeated execution are rejected. Existing lower model budgets also apply. Native
exceptions propagate without making further JNI calls with an exception pending.
Rust inference panics are caught and the owned slot is released on unwinding;
process-level allocation failure is not claimed recoverable.

`OriginalAssets` copies exactly five APK assets to a fresh private directory,
with fixed byte lengths and SHA256 values (8,396,968 bytes total). The original
Rust loader independently validates the assets before inference. Extraction must
run off the UI thread; interruption aborts and cleans incomplete extraction.
The caller retains the directory until its worker exits, then may clean it.
The fixture retains completed assets for evidence. Production startup/recovery
and retention policy are outside this proof.

## Build

Use official Rust 1.99, NDK r27d, JDK 21, Android Build Tools 35 and an installed
android.jar. Models, tools, output, signing keys and evidence stay outside Git.
The independent Cargo workspace keeps JNI dependencies out of Windows builds.
Build with `SINGLES_SORTER_ANDROID_PROBE=1`, the ABI-matching pinned PCRE2 static
archive, the NDK API23 Clang driver as Cargo linker, and both link arguments:
`-Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384`.
Run `cargo build --locked --release --target x86_64-linux-android` and separately
`aarch64-linux-android`. `cargo test --locked --lib` tests bounds and ownership.

`android/build-fixture.sh` builds the existing Java sources without Gradle.
Its inputs are documented in the script. This **debug fixture** has min/target
API 30; that is a harness floor, not a change to the intended app's API 24 floor.
It is signed by a local fixture key, never a production signing identity.

Install only into an owned emulator, launch
`com.nhlocal.singlesorter.inferencefixture/com.nhlocal.singlesorter.inference.FixtureActivity`,
and read `files/proof.json` with `adb shell run-as` for that debug package.
The Activity displays the same result. Record APK/native hashes, exact source
commit, platform fingerprint/ABI/page size, CheckJNI logs, asset hashes, ELF
PT_LOAD alignment and tests. No compatibility claim follows from compilation.
ARM64 and 16 KiB runtime remain unverified unless separately executed; aligned ELF
segments alone do not establish Android 16 KiB runtime compatibility.

## Tests and interpretation

Fixture checks cover verified extraction, native single-job admission, pre-load
cancellation, stale IDs, JNI request length/null handling, malformed input,
document-count limits, missing/tampered bundles, Hebrew/NUL/supplementary Unicode,
observed in-inference cancellation, duplicate execution, slot recovery, and Java
owner disposal. This supplements the earlier direct native parity proof; it does
not replace original 33-document/170-action and 1519-classifier-case evidence.

No Android FIFO/path-swap, physical ARM64 performance, actual Activity recreation
or process-death recovery, production Tauri frontend, SAF provider behavior or
sorting acceptance is implied. The existing FIFO setup limitation remains
explicitly environment-blocked.

## Official references

- https://v2.tauri.app/develop/plugins/develop-mobile/
- https://developer.android.com/ndk/guides/jni-tips
- https://developer.android.com/guide/practices/page-sizes
- https://developer.android.com/ndk/guides/other_build_systems

Android recommends keeping asynchronous callbacks in the managed UI layer and
minimizing JNI/thread boundaries. Tauri documents JNI as a supported way for its
Android plugins to call shared Rust code. NDK r27 requires explicit final-link
page-size alignment flags; APK/runtime verification is a separate step.
