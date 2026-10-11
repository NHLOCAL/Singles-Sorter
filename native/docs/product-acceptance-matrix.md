# Product acceptance matrix

Evidence categories are intentionally separate: released-policy parity,
read-only real-file behavior, physical execution, and actual product interaction.
A green test in one column does not establish the others. New candidate checks
below require final-head CI results; the presence of a test is not a passing run.

| Released behavior / flow | Planning and compatibility | Physical/product boundary |
| --- | --- | --- |
| Copy / Move toggle | Exact boolean and v14 virtual effects retained | Selected Windows multi-file Copy / same-volume rename candidate; cross-volume Move blocked |
| Main folder only / full tree | Original traversal/order retained | Selected plans validated whole; actual multi-file Windows gate required |
| Create singles folders | Original `סינגלים` destination component retained | Journaled directory creation candidate; native UI test asserts physical layout |
| Alphabetic sort | Original letter hierarchy retained | Backend directories supported; requires expanded native UI acceptance matrix |
| Existing folders only | Original skip/target policy retained | No fabricated missing artist destinations; explicit feature-specific runtime coverage still required |
| Duets, first artist | Original matching order retained | Ordinary single destination candidate supported |
| Duets, every artist | Original ordered destinations and deletion intention retained | Copy supported where backend preflight accepts; Move retirement cohort not enabled in first candidate |
| Original alias list + personal CSV | Ordered rows, duplicates, malformed-row diagnostics preserved | UI edit/import/delete/save/export and persistence baseline covered; no silent normalization |
| Original NER + secondary classifier | Both exact original assets validated; no fallback | CLI and Windows UI original-AI real-file preview baseline exists; new apply + AI composition requires final Windows gate |
| Disabled AI | Explicit choice; no inference | Never silently substituted for configured unavailable models |
| Albums | Original album recognition, metadata-error ordering and virtual transfer timeline | Whole album physical execution unavailable in first selected-root candidate |
| Fix names / gibberish tags | Original planned rename/tag intentions retained | Actual rename + tag rewrite remains unavailable; no “completed” label for preview |
| Name collisions | Original reservation policy; exact plan revalidated | Identity-bound no-replace publication; changed destination stops execution |
| Preview cancellation / Back / reload | Host owner remains busy until native terminal state | Existing lifecycle tests; no stale plan resurrection |
| Apply review / repeat click | Host-selected paths and retained digest only | New UI/CLI candidate; cannot repeat consumed plan |
| Apply progress / cancellation | Actual bytes; predicted rows stay predictions | Native result distinguishes committed, blocked, cancelled, partial and recovery |
| Recovery | Hash-chained journal and identity-bound reconciliation | Read-only retained-job inspection; process-restart product UI/import not implemented |
| Hebrew RTL compact layout / import editing | Existing shared UI and original labels preserved | UI unit/build checks; actual Windows screenshots required for new execution panel |
| CLI supplied-facts v1 | Existing envelopes remain model-free and non-executable | No physical authorization granted |
| CLI real-file v2 JSON | Same request fields and original planning policy | Read-only; optional selected planning adapter strengthens capability observations |
| CLI NDJSON execution | New one-process inspected-digest handshake | Opt-in Windows candidate; legacy positional compatibility still unavailable |
| Android | Shared UI remains fail-closed without native apply eligibility | Separate Android workstream; Windows proof is not Android/provider/ARM64 proof |
| Portable Windows artifact | Existing original-AI read-only package remains separate | Selected execution candidate is not silently shipped under read-only package claims |

## Current verification record

The baseline includes native original-AI CLI and Windows desktop preview work,
plus owned native Windows identity/reparse/journal/Copy/rename tests. The new
selected-root/UI/CLI integration is source implementation with explicit tests;
it is not yet a verified release until its combined final-head build and actual
Windows subprocess/WebView runs pass. Local frontend typecheck, production build
and 73 state/presentation tests passed during implementation. Local Chromium
rendering was blocked by OS socket restrictions, and local Rust installation was
blocked; neither limitation was converted into a passing verification claim.
