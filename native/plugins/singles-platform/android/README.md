# Android read-only SAF adapter (development component)

This is the native storage component for the approved shared Tauri interface. It
is Java to compile directly against Android APIs without a separate runtime or
new UI; Kotlin/Tauri and JNI can call it normally. It contains no sorting rules.
It is not wired to the app or Rust planner yet and is not a completed Android port.

## Implemented

- Native Activity callback creates an opaque root token after a read-only
  ACTION_OPEN_DOCUMENT_TREE result. Picker requests are serialized, stale results
  rejected, and cancellation does not create a new root. The caller retains the
  previous selection on cancellation. Dispose rejects late results.
- Native-owned document tokens map to provider document IDs, never to guessed
  filesystem paths. Display names are data, not keys. Grants are rechecked on
  access and before returning a read handle. Listing and metadata observations
  recheck scope, cancellation and grant after provider cursor close. This is an
  acceptance boundary, not a guarantee against later revocation. Unknown size
  remains unknown.
- Child enumeration verifies provider ancestry and rejects duplicate IDs,
  self-links, excessive entries, provider errors and unfinished loading cursors.
  **Ancestry verification currently requires API 29.** On API 24–28 enumeration
  explicitly fails with ANCESTRY_VERIFICATION_UNSUPPORTED. The intended product
  floor remains 24; this missing adapter capability is an integration blocker.
- ContentResolver opens only `r` descriptors. Seekable descriptors stay owned by
  the read handle. Pipe descriptors spool to private app cache with a hard 64 MiB
  maximum, a caller-specified lower maximum, cancellation checks and cleanup.
  Up to 32 live handles are permitted. Closing the scope closes all owned handles;
  an in-flight open rechecks scope/grant/cancellation before adoption.
- No provider creation, deletion, rename, write grant or broad-storage permission.
  The minimal library manifest requests no permissions.

## Contract limitations

All provider I/O belongs on a worker thread. CancellationSignal is required.
Cancellation is checked at boundaries; a blocked provider stream may not stop
immediately. UI cancellation cannot be reported as finished until the call exits.
Provider metadata is observational: neither matching size/time nor a seekable FD
proves immutable bytes or core storage identity. This component must not fabricate
an EntryObservation accepted by the planner. JNI binding and provider identity
conformance are separate required work.

Persisted grants survive process death but this in-memory token registry does not.
Reopening a scope requires native selection/restoration work; no silent fake resume.
ReadHandle.descriptor is borrowed, cannot be detached, and must not be closed by
consumers separately. Callers should close handles promptly; scope close is backup.

## Checks

With JAVA_HOME (JDK 17+) and ANDROID_HOME (API 37 SDK) set, run `./check-sdk.sh`.
It compiles actual Android API calls with warnings as errors and runs 24 portable
picker/cancellation/size/loading/API-level/final-validation guard checks. It does not emulate
ContentResolver and does not prove provider behavior, revocation races, lifecycle
or Android-device acceptance.

A standalone Android AAR build is available using Gradle 9.5:
`gradle --offline --no-daemon --max-workers=1 assembleDebug`.
The offline command requires cached AGP 9.3.3/Kotlin 2.4.20 and Android SDK 37.
Build caches/output are ignored; no SDKs or third-party source are vendored here.

Required next device checks: local tree selection/cancel/stale callback, loss of
persisted grant, unknown-size and nonseekable DocumentsProvider, duplicate display
names, pending/failed listing, cancellation during provider read, activity disposal
while open is blocked, and 24–28 unsupported-capability presentation. Only then
wire provider observations and descriptors through a versioned JNI contract.

## Primary references

- https://developer.android.com/training/data-storage/shared/documents-files
- https://developer.android.com/reference/android/content/ContentResolver
- https://developer.android.com/reference/android/provider/DocumentsContract
- https://v2.tauri.app/develop/plugins/develop-mobile/
- https://developer.android.com/ndk/guides/cmake

Reviewed 2026-10-11. No native Android execution is claimed by host compilation.
