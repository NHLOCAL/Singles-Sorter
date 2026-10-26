# Singles Sorter 15.0 Android beta

Current product identity is `15.0.0-beta.1`, package `com.nhlocal.singlesorter`,
version code `1500001`. Use `build_apk.py` on Windows/Linux or its shell wrapper
`build-preview.sh`. Manual Actions builds default to the backed-up permanent key;
pull requests use isolated QA signing. See the [signing and transition guide](docs/signing-and-v15-updates.md).

The historical checkpoint below records earlier acceptance evidence. Its
per-build temporary signing and preview version labels are superseded by the
version 15 pipeline; its ABI/provider limitations still apply unless newer
exact-artifact runtime evidence is recorded.

# Shared-UI Android product milestone

This source connects the existing Hebrew frontend to the original Rust planner,
TagLib metadata bridge and unchanged original recognition models. It is a
development preview with bounded actual provider execution. The exact 772
checkpoint passes authored API30 x86_64 software-emulator checks on the local
ExternalStorage DocumentsProvider. There is no Python runtime or model download
in the APK. See the [source/artifact-bound acceptance record](../../docs/product-acceptance-matrix.md#current-772-windows-and-android-checkpoint-2026-10-13).

## Coverage and remaining work

| Released feature | Android source route | Acceptance status |
| --- | --- | --- |
| Source/target selection | Native SAF picker, opaque UI tokens; explicit limited parent selection for selected-root Album Move | Current 772 source/target flow passed; parent-specific cases retain earlier exact APK attribution |
| Six original settings and personal CSV | Shared UI, typed private preferences, ordered Rust CSV resources | Implemented; Windows acceptance and Android cases are separately recorded, not exhaustive Android settings/CSV device proof |
| Sort / Copy / Move / original AI | Original planner/models; native provider preflight and confirmation | Current Single Move and genuine two-destination original-AI duet Move passed, including observed retirement |
| Albums | Original recognition; bounded provider copies, metadata and cleanup | Current selected-root Album Copy passed; nested Move/explicit-parent root Move and interruption cases retain prior exact APK attribution |
| Fix names and tags | Original Fix planner; staged bounded shared repair writers and original backups | Current filename+title Both repair passed; nine-format/unchanged/unsupported/interruption matrix retains prior exact APK attribution |
| Recovery and history | Read-only retained-effects review, explicit acknowledgment; no automatic replay/rollback | Three interrupted histories and nine private state/index/authority files preserved across the owned update; fresh current journals independently inspected |
| Notices | Packaged full notices/source inventory, native viewer and explicit export | Current viewer/export/readback passed; all 437 exported entries match authentic APK bytes |
| Android API 29+ | Source floor for provider ancestry verification | Actual runtime evidence is API30 x86_64 software emulator only |
| ARM64 and 16 KiB pages | Both ABIs compiled; ELF/load and ZIP alignment verified | Physical ARM64 and 16 KiB-page runtime untested |

Windows and Android use the same frontend and sorting implementation. Android's
native transport supplies local platform functions; it does not reinterpret the
sorting rules or claim native execution eligibility from a display-only plan.

## Safety boundary

Only packaged resources under the local HTTPS asset origin can execute script.
The manifest has no internet/storage-wide permission. File/content WebView access,
network loads, mixed content, remote navigation and frames are denied. Native
footer navigation is a fixed allowlisted external destination without the bridge.
Android back closes the active dialog through its existing cancel handler, then
uses existing results history, then closes the Activity.

The native owner serializes picker and job work. One JNI registry job remains
owned until actual worker completion, including Activity destruction and provider
cancellation. A blocked provider read is not called a completed cancellation.
The worker closes native resources after JNI returns, never underneath it.

SAF traversal is bounded to 2,000 entries/depth 128. The manifest is at most 2 MiB.
Only metadata requests actually made by the original planner capture files.
Each private snapshot is at most 64 MiB and a job at most 512 MiB; descriptor/pipe
spooling is also bounded. Read errors or snapshot-budget failures fail the job,
not a silently incomplete successful preview. Cleanup touches only the exact
owned temporary files/directory. Abrupt OS process death may leave cache files;
read-only interrupted-operation inspection is now verified in the bounded device
matrix. It does not authorize deletion of unknown leftovers or automatic replay.

Metadata uses a hash-verified private snapshot. The original SAF tree/document
reference remains provenance. Provider size/time are observations, not immutable
identity or a fabricated same-handle ReadBinding. The Rust snapshot-only preview's
`executable:false` never grants mutation authority. A separate native provider
admission can make an eligible retained plan ready for confirmation; fresh
preflight and actual provider observations still govern Apply. Provider content
can change after preview, and the supported contract requires quiescent writers.
Snapshot hashes/model identity remain separate from frontend display paths.
Provider operations are not atomic multi-document transactions.

## Reproduction

`build-native.sh` cross-compiles the original JNI crate with `--features product`
and authenticated PCRE2 source using official NDK r27d. `build-preview.sh` consumes
those libraries, the original five-file bundle and `vite build --mode android
--base /assets/` output. Both scripts keep build outputs outside Git. Signing is
an ephemeral development identity, never a production key. The authentic 772 APK
SHA256 is `05ff5d598868686b49f86c9d8913be48ff8650de5a375ac38687140a520389b1`;
its APK v3 signature is valid, but a fresh key is generated per build. It cannot
update an installed same-package app signed by a different key. Do not uninstall
a valued installation merely to bypass that mismatch: private history would be
lost without a separately verified preservation route.

For the owned acceptance emulator only, the identical payload was re-signed with
its existing QA key to preserve history. The installed QA artifact SHA256 is
`e40b70e10c27201f08a22522c344ec7184f735f6e0bafe9c5f3a28ad16f59147`;
all 452 non-signature entries are byte-identical to the authentic APK. Actual
device results refer to that QA artifact, not an installation of the authentic
APK. This does not establish a production signing/update channel. Do not upload
private keystores. The workflow's final acceptance remains `--locked`, fmt, tests,
and strict Clippy; the optional diagnostics job only emits a proposed source diff.

The product floor is API 29 because `DocumentsContract.isChildDocument` is not
available on API 24–28. No older-Android support is claimed by changing compiler
API levels. Official architecture references: [JNI](https://developer.android.com/ndk/guides/jni-tips),
[local WebView content](https://developer.android.com/develop/ui/views/layout/webapps/load-local-content),
[SAF trees](https://developer.android.com/training/data-storage/shared/documents-files),
[NDK toolchains](https://developer.android.com/ndk/guides/other_build_systems).

CSV import is bounded to 2 MiB/10,000 rows and preserves malformed-width rows, order, duplicates and explicit encoding selection. Export uses ACTION_CREATE_DOCUMENT and verifies readback; provider failures may leave the newly created document incomplete, and the app reports failure without deleting an unproven provider object. No broad storage or persistent write grant is requested.
