# Shared Singles Sorter interface: first read-only slice

This is the new Hebrew/RTL renderer and thin local Tauri host. It preserves the
released blue header, source/target rows, paired sort/fix actions and visible
options card. It is **not a complete replacement release**.

## Implemented surface (subject to native platform gates)

- Native source/target selection or desktop path entry; cancellation preserves
  the prior selection. Android path typing is disabled when the host reports Android.
- Six settings with the release defaults, applied to the current preview.
- Sort and target-free FixNames previews, ordered host-projected rows, reasons,
  blockers, bounded result paging, explicit configured-AI failure.
- Help/About/What's New dialogs, advanced duet/AI controls, keyboard dismissal.
- No file mutation or executable plan import, shell or filesystem frontend API.

Preference persistence and ordered personal CSV editing/import/export are implemented
through native profile commands in the Task 2 continuation below. The first five
settings save explicitly; duet saves immediately. No localStorage substitute or
false migration is used. Native inference, Android SAF, transfer execution and
update handling remain incomplete. All settings/list changes are separate from
read-only music previews.

## Development and verification

```
npm ci
npm run typecheck
npm test
npm run build
npm run dev
```

`npm run test:renderer` uses Python Playwright and its installed Chromium; set
`SINGLES_CHROMIUM` to a system Chromium executable if appropriate. Its IPC is
explicitly mocked. Screenshots/results go to ignored `test-results/` or to
`SINGLES_UI_EVIDENCE`. Browser renderer success is not Windows Tauri or Android
runtime acceptance. Test snapshots must be labelled accordingly.

The native host has an independent Cargo workspace; see `src-tauri/README.md`
for its actual test/build gates. The CLI workspace remains independent from
frontend/Tauri dependencies. No npm browser download occurs during installation.

## Source and platform research

- Visual/behavior reference: v14.0 at `78d001aee5ce49628bdffebd34540fa83ba32c12`,
  `src/core/main.py`, `src/core/app/assets/general_view.png`.
- Stack decision: approved Phase 4 plan, Tauri v2 and plain TypeScript/Vite.
- Commands and worker ownership: https://v2.tauri.app/develop/calling-rust/
- Capability boundary: https://v2.tauri.app/security/capabilities/

Pinned package versions were checked at the official npm registry on
2026-10-11: Tauri JS API 2.12.3, Vite 8.3.4, TypeScript 7.0.2, Vitest 5.0.3.
The lockfile retains full transitive resolution/integrity. Renderer bytes do
not describe complete installer size or WebView memory consumption.

## Task 2 profile/list continuation (not yet native platform acceptance)

The continued interface calls a dedicated native development profile. The five
main checkboxes are saved only by **Save settings**. The duet radio writes
immediately, without persisting or overwriting the other five unsaved edits.
There is no browser localStorage and no blind legacy storage migration.

The personal-list editor retains row order, duplicate aliases, whitespace and
exact target strings. It renders up to 50 editable rows per page. Incomplete or
malformed edit rows require explicit correction/deletion before Save; Cancel
never writes. Native import first shows an encoding preview, then appends only
on confirmation. Existing malformed imported records are visible, not silently
cleaned. Export uses a native destination selection; overwriting an existing
file is currently refused rather than racing a path replacement.

This continuation includes first-run/version markers in the separate development
profile. Root tokens and selected music paths are never persisted as authority.
Browser acceptance still uses mock IPC around actual serialized host fixture
rows. Full Windows dialogs/profile persistence and Android SAF remain distinct
platform tests; a successful renderer build does not complete them.

### Native Windows acceptance harness

`npm run build:desktop` invokes the pinned Tauri CLI (`tauri build --no-bundle`),
which selects the production asset protocol. `npm run test:desktop` is CI-only:
it refuses non-Windows/non-GitHub-Actions execution. It expects official external
`tauri-driver` and a matching Edge WebDriver on PATH. It uses the real application
binary and real IPC against an owned synthetic MP3, checks source hashes and
an unchanged target, captures screenshots, and verifies preference/list reopen.
No embedded WebDriver server or mock IPC is added to the application.

This harness is prepared but **has not yet passed a native Windows run**. Its
syntax check is not runtime acceptance. Native picker automation, Narrator,
DPI, Android, apply and recovery remain separate gates. Primary driver research:
https://v2.tauri.app/develop/tests/webdriver/ and
https://v2.tauri.app/develop/tests/webdriver/example/selenium/ .

### Owned Windows CI acceptance

`.github/workflows/native-ui-windows.yml` uses the same reviewed Windows 2025
runner and pinned checkout/artifact actions as native CLI CI. It builds through
Tauri CLI, then uses external `tauri-driver` 2.1.0 and the runner's matching
Microsoft Edge driver. Native common-file-dialog automation uses PyPI
`pywinauto` 0.6.9, scoped to the exact owned preview executable. No application
IPC mocks, test-only app commands, personal desktop or real music library are
used. The script requires both Task 2 controls and successful settings/list
persistence after restart, and fails on unrecognized native dialog controls.
It uploads actual screenshots and JSON only when produced by the runtime.

This workflow source is an **unrun acceptance gate**, not proof of Windows
success. It must be integrated on the clean publication branch with the reviewed
native dependency acquisition fixes, without restoring vendor archives. Native
picker controls may require a source-reviewed correction after the first actual
Windows run. Android, Narrator, DPI and physical apply/recovery remain separate
acceptance gates.

Primary references checked 2026-10-11:
- https://v2.tauri.app/develop/tests/webdriver/ci/
- https://learn.microsoft.com/en-us/microsoft-edge/webdriver/
- https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md
- https://pywinauto.readthedocs.io/en/latest/HowTo.html

### Windows startup diagnostics

The CI desktop harness performs a bounded direct launch of the exact Cargo binary
before creating the real WebDriver session. It records stdout/stderr, early exit,
owned process and WebView2 runtime versions, and up to four owned-window images.
Window images use PID-verified PrintWindow in a five-second subprocess, with no
desktop screenshot fallback. A black or unavailable image is diagnostic only.
The preflight process tree is terminated before the unchanged acceptance flow;
residual preflight processes stop the test rather than contaminate it. A failed
WebDriver session also records owned-process/window diagnostics. None of these
observations counts as a passed IPC, rendering, picker, or persistence check.

The Tauri CLI's printed output path is not sufficient proof of a renamed binary.
Pinned CLI 2.12.2 filters required-features during binary discovery and can fall
back to the Cargo package name when desktop is enabled only through defaults.
The configured Cargo binary remains singles-sorter-preview.exe; there is no
mainBinaryName override. Build adds tauri/custom-protocol and embeds frontendDist.
The launch timeout's root cause still requires the Windows diagnostic evidence.

References inspected 11 October 2026:
- https://docs.rs/crate/tauri-cli/2.12.2/source/src/interface/rust.rs
- https://github.com/tauri-apps/tauri/blob/tauri-driver-v2.1.0/crates/tauri-driver/src/server.rs
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/webdriver
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-printwindow

### Non-elevated Windows acceptance

The application rendered successfully in diagnostic run 38109858800 using the
actual WebView2 153.0.4234.48 runtime, while WebDriver session creation timed out.
This matches an upstream WebView2 150+ restriction on browser-argument environment
variables in elevated hosts. The runner token was not captured in that run, so
that cause remains a hypothesis until the new token/acceptance evidence confirms it.

CI now records the parent's actual integrity RID and elevation flag, then runs the
unchanged acceptance harness at normal Medium integrity using the SHA-256-pinned
official gsudo 2.6.1 portable binary. Already-Medium parents launch directly; other
unexpected tokens fail closed. The harness independently requires RID 8192 and verified normal-user effective
rights for itself, tauri-driver, and both actual app/native-driver processes at
initial launch and restart. Diagnostic process records include token
information when accessible. No ACL grant, registry policy, credential cache,
service installation, sandbox change, or application debug exception is used.

Evidence and owned fixture writes go into a unique directory under the CI user's
existing LocalAppData Temp permissions; only reads are needed in the checkout. The
parent copies evidence back for artifact upload and propagates the real test exit
code. Permissions failures remain failures instead of triggering broader access.
The actual native functional assertions remain required; screenshots alone cannot
pass this gate.

Primary references:
- https://github.com/tauri-apps/wry/issues/1782
- https://github.com/MicrosoftEdge/WebView2Feedback/issues/5645
- https://github.com/tauri-apps/tauri/blob/dev/.github/workflows/test-api-e2e.yml
- https://github.com/gerardog/gsudo/releases/tag/v2.6.1
- https://learn.microsoft.com/en-us/windows/win32/secauthz/mandatory-integrity-control

#### Effective-rights preflight correction

Run 38110837694 confirmed a High-integrity parent and a Medium-integrity child
whose TokenElevation flag remained true. That flag alone did not establish
effective administrator rights. Microsoft's WebView2 security documentation
defines the elevated-host boundary as High IL; pinned gsudo uses a linked token
or SAFER NormalUser token to reduce rights. Microsoft documents NormalUser as
excluding Administrator/Power User rights.

Before compilation, CI now reads the exact launched token's integrity, elevation
metadata, TokenHasRestrictions, privileged builtin group attributes, and all
present privilege names/attributes. The gate requires Medium IL; privileged groups
must be absent or irreversibly deny-only, and administrator-capable privileges
are rejected even when disabled. Only ChangeNotify may be enabled; the small
standard-user privilege allowlist is otherwise accepted only disabled. A retained
true elevation flag additionally requires evidence of token filtering. Missing or
unrecognized rights fail closed. The same gate still checks the real acceptance
processes; the preflight does not count as UI acceptance. EnableLUA is read only
to explain the runner configuration, never changed.

Additional primary sources:
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security
- https://learn.microsoft.com/en-us/windows/win32/secauthz/sid-attributes-in-an-access-token
- https://learn.microsoft.com/en-us/windows/win32/secauthz/privileges
- https://learn.microsoft.com/en-us/windows/win32/api/winsafer/nf-winsafer-safercreatelevel

### Native folder-picker acceptance

Run 38111848892 established the real WebDriver session, verified effective
Medium rights, and rendered the app. It then failed on the harness assumption
that the folder picker nests its path Edit inside the file-picker ComboBox.
The host actually calls rfd 0.16.0 IFileOpenDialog with FOS_PICKFOLDERS.
The harness now records the owned dialog's live UIA controls and exact-HWND
PrintWindow image before input. Folder selection requires one visible, enabled,
PID-matching native Edit advertising SDK edt1 (1152), without guessing its parent
or relying on an English label. Missing/ambiguous controls fail with evidence.
The native ValuePattern must retain the exact fixture path before native Confirm;
the existing application path and preview assertions still establish success.
This selector correction still requires the next actual Windows run to validate.

Sources:
- https://docs.rs/crate/rfd/0.16.0/source/src/backend/win_cid/file_dialog/dialog_ffi.rs
- https://github.com/microsoft/win32metadata/blob/main/generation/WinSDK/RecompiledIdlHeaders/um/dlgs.h
- https://pywinauto.readthedocs.io/en/latest/code/pywinauto.controls.uia_controls.html
