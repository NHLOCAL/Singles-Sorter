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
