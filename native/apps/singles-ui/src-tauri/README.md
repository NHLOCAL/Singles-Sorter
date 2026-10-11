# Read-only desktop host: integration boundary

This is the first preview slice, not a complete desktop/Android release. It calls
`sorter_core::job_plan::plan_job_controlled` directly with the existing desktop storage and
metadata bridge. It contains no sorting algorithm, Python process, shell command,
file-copy/delete/tag-save adapter, or apply command.

## Contract (host schema 1)

- `select_root({role})` returns a selection or null. Cancellation leaves the old
  selection untouched. `resolve_typed_root({role,text})` accepts an explicit
  absolute desktop path. Roles are `Source` and `Target`.
- Selections expose `{token,display_name,capabilities:{read,list},platform}`.
  The host retains the actual path and directory identity. Tokens cannot encode
  arbitrary paths. Replacing a selection invalidates its prior token.
- `start_preview({request})` takes `mode`, `source_token`, `target_token`, all six
  `settings`, and `ai_profile` (`Disabled` or `Configured`). It returns `{job_id}`
  immediately after reserving a background worker. `cancel_preview({jobId})`
  signals real cooperative cancellation; the worker retains the busy guard.
- `job_snapshot({jobId})` returns `running`, `completed`, `failed` or `cancelled`. Completed
  means planning completed, never that transfers or edits happened. The snapshot
  includes blockers, warnings, plan binding, row count, and an optional error.
- `get_plan_page({jobId,offset,limit})` returns `{rows,total}`. Limits are 1–200.
  Rows preserve engine decision order and show engine-produced destinations.
  Display text must never be fed back as file authority. The complete bound plan
  remains in Rust. Only the most recent job is retained.
- `get_settings()` returns the saved development profile, or released defaults.
  The profile/personal-list section below documents the added persistence. `get_model_status()` explicitly reports unavailable native
  inference when built feature-off, or fully validates both original models
  feature-on. Configured AI never silently becomes disabled. See
  [original AI desktop contract](../../../docs/original-ai-desktop.md).

Picking, resolving, and scanning share one host-side busy guard. Metadata/scanning
runs outside the UI thread. Selected identities are rechecked against the opened
planning storage. Linux uses the explicitly test-only adapter. Android selection
and its SAF provider remain unavailable rather than converting URIs to paths.

## Build and tests

The separate Cargo workspace/lock avoids adding Tauri to the standalone CLI.
`cargo test --locked --no-default-features --lib` tests the production host
state/planning module using real owned metadata fixtures and the existing engine.
It does **not** test Tauri IPC, a native picker, WebView, or Windows execution.
`cargo test --locked` enables the actual desktop host and platform prerequisites.
The default binary is `singles-sorter-preview`; bundle creation is disabled.

The desktop feature requires platform SDKs: MSVC and WebView2 on Windows, GTK 3
and WebKitGTK 4.1 development packages on Linux. No native-desktop acceptance or
package-size claim may be inferred from the headless tests. Actual Windows build
and UI-driver acceptance, Android provider integration, automatic legacy
preference migration and executable operations remain later gates.

## Security and source evidence

Only the bundled `main` view receives the enumerated app-command permissions. The
build manifest explicitly enumerates commands because app commands are otherwise
allowed by default. No general shell/fs/dialog plugin permissions are granted to
JavaScript; the folder dialog is called only through the constrained Rust command.
Remote navigation, frames, external connections, and remote IPC are not allowed.
The release configuration has no development server or WebView debugging grant.

Primary documentation consulted 11 October 2026:
- https://v2.tauri.app/security/capabilities/
- https://v2.tauri.app/security/runtime-authority/
- https://v2.tauri.app/plugin/dialog/
- https://v2.tauri.app/develop/calling-rust/
- https://v2.tauri.app/start/prerequisites/

Exact crates verified from the official registry: Tauri 2.12.3, tauri-build 2.7.2,
tauri-plugin-dialog 2.8.2. Their source APIs were inspected, including
`AppManifest::commands`, `blocking_pick_folder`, `on_navigation`, and `devtools`.

The user-click `open_footer_link()` command opens only the exact released Shir Bot HTTPS URL in the system browser. It accepts no URL/path/application input. The official MIT/Apache-2.0 `tauri-plugin-opener` 2.7.1 is called from Rust; JavaScript receives no general opener permission. Documentation: https://v2.tauri.app/plugin/opener/ . Native opener execution remains part of platform acceptance.

## Native production build entry point

Use the pinned Tauri CLI through the UI package's desktop build script. Its
`beforeBuildCommand` builds Vite first; `frontendDist = ../dist` embeds those local
assets. A plain `cargo build --release` is not the supported production entry.
Tauri CLI 2.12.2 itself adds `tauri/custom-protocol` to Cargo features in
`src/interface/rust.rs` (line 481 of the inspected official crate); an application
feature alias named `custom-protocol` is no longer required by Tauri v2. Tauri
2.12.3 defines development mode as the absence of that dependency feature.
No native build or bundled-asset runtime pass is inferred from this source check.

Sources inspected 11 October 2026:
https://docs.rs/crate/tauri-cli/2.12.2/source/src/interface/rust.rs
https://docs.rs/crate/tauri/2.12.3/source/src/lib.rs
https://v2.tauri.app/release/@tauri-apps/cli/v2.0.0-beta.5/

## Profile and personal-list slice

The application now writes **only** its separate development profile and explicit
CSV export destinations. Music preview remains read-only with no apply command.
The profile location comes from Tauri's `app_local_data_dir()` for
`com.nhlocal.singlesorter.preview`, under `profile-v1`. No source/target paths,
root tokens, model choices or executable plans are persisted there.

- `get_settings`, `save_main_settings({settings})`, `set_duet({enabled})`: the first
  five booleans save only through the save action. Duet saves independently and
  immediately. Explicit false for `singles_folder` remains false. A local mutex
  and OS file lock serialize profile I/O; corrupt/unknown schemas and string
  booleans are rejected without overwrite or guessed migration.
- Profile replacement writes/syncs an owned temporary file and persists it in the
  same directory. The preceding valid profile is retained as
  `profile.previous.json`. This is not a universal power-loss durability claim;
  Windows runtime acceptance is still outstanding.
- `get_personal_list` returns `{revision,rows,diagnostics}`. Rows are raw ordered
  arrays: source name first, destination folder second. Duplicate aliases and
  malformed imported records are retained. Diagnostics identify zero-based index,
  code and width. `save_personal_list({revision,rows})` requires explicit complete
  two-field edited rows, never silently drops incomplete rows, and rejects a
  changed revision. Settings changes do not invalidate unchanged list contents.
- `select_personal_import()` returns a native file selection and a preview token,
  or null on cancellation. Bytes stay in the host. Valid UTF-8 receives a preview;
  other data requires `preview_personal_import({token,encoding})`. The explicit
  choices are Utf8, Utf8Bom, Windows1255, Windows1252, Utf16Le and Utf16Be. Decoding
  rejects replacement-character fallbacks; matching BOM removal is explicit in
  the selected BOM/UTF16 codec. The displayed rows must be confirmed through
  `confirm_personal_import({token,encoding,revision})`. This appends exact parsed
  rows once, with no deduplication or artificial blank separator. Stale tokens,
  changed lists and unpreviewed encodings fail closed. CSV input/decoded/exported
  bytes are limited to 16 MiB; the existing Python-compatible engine parser and
  field-size rules are reused.
- `export_personal_list()` uses a native Save dialog and publishes a complete
  UTF-8 CSV to a **new** destination. Existing filenames are rejected, so this
  slice does not yet offer overwrite-export. Cancellation writes no export.
- `get_first_run_status` reads the development-version marker;
  `mark_first_run_seen` writes it only after acknowledgement.
- Planning snapshots the current personal rows as canonical CSV, then calls the
  existing alias loader with bundled rows first and personal rows second. Both
  source hashes enter the real engine's bound plan. No renderer routing logic or
  personal-first precedence was introduced.

### Released behavior and remaining migration gate

Characterized from release v14 main commit
`78d001aee5ce49628bdffebd34540fa83ba32c12`:
`src/core/main.py` lines 577–656 define the six defaults and two save timings;
lines 318–348 append imported rows (including the old extra blank separator);
lines 369–383 copy the export; `src/core/add_singer_dialog.py` displays folder/name
columns reversed but saves CSV as source/folder and historically drops incomplete
editor rows. The approved new-input repair removes only the artificially added
separator and replaces silent editor-row loss with explicit validation. Existing
imported malformed/blank rows are not silently cleaned.

The release's `requirements.txt` does not pin Flet. Actual stored RadioGroup value
types remain unverified, as recorded in the frozen compatibility contract. This
slice therefore never reads or coerces old Flet preferences. Old app storage is
untouched; personal CSV migration uses the explicit import preview. Automatic
preference migration is **not implemented or claimed**.

Primary implementation references:
https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html
https://docs.rs/encoding_rs/0.8.42/encoding_rs/struct.Encoding.html
https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock
https://v2.tauri.app/reference/config/

Profile directory review guard: the dedicated `profile-v1` directory rejects
symlinks and Windows reparse points before lock/profile writes. On Windows an
ordinary directory handle is retained with no write/delete sharing for the
operation. Legitimate OS AppData ancestors are not rejected; this is not a
hostile-ancestor sandbox. Windows junction and held-directory rename regressions
are included and require actual Windows execution. The handle flags follow
[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew).

Resource limitation: the 16 MiB CSV cap limits encoded/decoded bytes, not row
object overhead. UI pagination does not bound the full import allocation. A
streaming row-count/resource guard remains a separate acceptance item; no
maximum-import memory claim is made.
