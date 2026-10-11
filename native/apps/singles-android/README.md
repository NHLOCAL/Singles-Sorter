# Shared-UI Android product milestone

This source connects the existing Hebrew frontend to the original Rust planner,
TagLib metadata bridge and unchanged original recognition models. It is a
read-only development preview until its exact APK/device acceptance is recorded.
There is no Python runtime or model download in the APK.

## Coverage and remaining work

| Released feature | Android source route | Acceptance status |
| --- | --- | --- |
| Source/target selection | Native SAF tree picker, opaque UI tokens | Device proof pending |
| Six original settings | Shared UI, typed private preferences, original core Settings | Java/frontend compilation only |
| Personal artist list editing | Ordered rows, revision conflict check, original CSV parser for planning | Device proof pending |
| CSV import/export | Shared UI present | Native implementation pending |
| Sort, artist folders, ABC, existing-only, duet, copy/move intent | Unchanged `plan_job_controlled` | Product native build pending |
| Album detection and fallback | Original core traversal and metadata rules | Product native build/device oracle pending |
| FixNames predictions | Original FixNames planner and metadata projection | Product native build/device oracle pending |
| Original NER + secondary gate | Same original Rust recognition library, same five pinned assets | Prior fixture evidence does not validate this product APK |
| Provider file mutation | No capability/command | Pending provider identity, journal and rollback acceptance |
| Android 10/API 29+ | Required by provider ancestry verification | Current product runtime untested |
| ARM64 and 16 KiB pages | Two-ABI build and 16 KiB link/ZIP alignment scripts | Runtime untested |

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
restart cleanup/recovery remains to be validated before release.

Metadata uses a hash-verified private snapshot. The original SAF tree/document
reference remains provenance. Provider size/time are observations, not immutable
identity or a fabricated same-handle ReadBinding. All previews advertise
`executable:false`; provider mutations remain unavailable. Provider content can
change after a preview. Snapshot hashes and original model runtime identity are
retained in the native result, separately from frontend display paths.

## Reproduction

`build-native.sh` cross-compiles the original JNI crate with `--features product`
and authenticated PCRE2 source using official NDK r27d. `build-preview.sh` consumes
those libraries, the original five-file bundle and `vite build --mode android
--base /assets/` output. Both scripts keep build outputs outside Git. Signing is
an ephemeral development identity, never a production key. Do not upload its
private keystore. The workflow's final acceptance remains `--locked`, fmt, tests,
and strict Clippy; the optional diagnostics job only emits a proposed source diff.

The product floor is API 29 because `DocumentsContract.isChildDocument` is not
available on API 24–28. No older-Android support is claimed by changing compiler
API levels. Official architecture references: [JNI](https://developer.android.com/ndk/guides/jni-tips),
[local WebView content](https://developer.android.com/develop/ui/views/layout/webapps/load-local-content),
[SAF trees](https://developer.android.com/training/data-storage/shared/documents-files),
[NDK toolchains](https://developer.android.com/ndk/guides/other_build_systems).
