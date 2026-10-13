# Task-focused UI continuation

Base: the reviewed filename-first candidate `d011022`, preserved as an ancestor.

- Setup → indeterminate processing → full-height result plan. Back preserves settings and paths; reopening a valid result does not call the host again. Processing is not a history entry and there is no fake cancellation control.
- Desktop table rows use a single line. Phone rows use two lines with a 48px detail action. A keyboard/touch dialog holds the exact original source/destination and diagnostics. Large text may reflow rather than clip.
- Original `מיין שירים` and `תקן שמות` labels restored without a suffix. The result heading identifies a plan. Folder inputs are LTR and left-aligned; Hebrew labels remain RTL.
- One settings/editor/import modal is displayed at a time. Editor indices align with input controls. Import previews now expose a 50-row paged editable draft with stable original indices, preserved duplicate/order/extra fields, delete focus restoration, and explicit final confirmation.
- Import changes require the coordinated host extension: optional `set` / `delete` actions against the checked pending preview. Encoding changes and Cancel discard edits; failures preserve them. No persistence schema or sorting rule changed.

## Offline icons

Sixteen unmodified 20px regular SVGs from official `@fluentui/svg-icons@1.1.344` are vendored as a minimal static subset. The full upstream MIT license ships under `public/licenses`. Provenance, registry SHA-512 integrity and per-file SHA-256 hashes are recorded alongside the assets. The full 86MB catalog, fonts, CDN and runtime fetch are not installed or used. Only `folder_open`, import and export are RTL-mirrored; the back arrow is already right-pointing.

A production Vite comparison with the icon assets/rendering replaced by an empty stub measured 7,650 extra JavaScript bytes and 2,561 gzip bytes. This measures icon code/assets, not the entire workflow change.

## Verification boundary

TypeScript/Vite build, 54 frontend tests, 14 harness tests and two offline asset tests pass locally. Native Windows and renderer suites were extended for the new navigation, exact paths, dense rows, editable import, 360/390px rendering and copied-path selection. Those new rendered checks have not yet run in this executor; the previous Windows-green screenshots precede this change. Local Chromium execution remains blocked by the executor's socket restriction. Actual Android, screen readers and Windows OS text scaling remain separate gates.
