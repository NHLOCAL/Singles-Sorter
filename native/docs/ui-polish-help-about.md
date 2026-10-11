# Help lifetime and About information

This focused UI change leaves planning and execution semantics unchanged.

## Interaction contract

- Mouse/pen hover opens contextual help. Leaving the question-mark button hides
  it synchronously, including after one or more mouse clicks. There is no hide
  timer and clicking cannot pin mouse help.
- Keyboard focus opens help; blur closes it. Escape dismisses without moving
  focus; Enter/Space can reopen it. Keyboard-origin help is not dismissed merely
  because an unrelated pointer moves.
- Touch taps toggle help. A second tap, outside interaction, focus loss, dialog
  closure, or application focus loss dismisses it. Touch pointerleave alone is
  not dismissal.
- The explanation is plain text, has `role=tooltip`, and is associated with its
  trigger by `aria-describedby`. It does not receive focus.

Research: [WAI-ARIA tooltip pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tooltip/)
provides keyboard/focus behavior. The user's explicit immediate badge-leave
requirement differs from the hoverable-content expectation in
[WCAG 1.4.13](https://www.w3.org/WAI/WCAG22/Understanding/content-on-hover-or-focus.html).
Keyboard and touch alternatives remain available; this change is not a claim of
full WCAG conformity.

## About identity and links

The displayed build version comes from `package.json`; a test checks agreement
with native Tauri/Cargo declarations. The original rule lineage remains 14.0.
Project attribution and the MIT label are grounded in the repository README and
root LICENSE. Component acknowledgments do not replace full shipped notices or
claim that every model/dependency has the project's license.

Only the named `Source` and `License` destinations are sent to the native
`open_project_link` command. Native code independently maps these values to the
verified project repository and its LICENSE page; arbitrary URLs and shell
arguments are not accepted. Browser launch happens only after an explicit
button click. A failed launch leaves a selectable, fixed fallback URL.

## Evidence boundary

Unit tests cover input-specific lifetime, version agreement and link mappings.
Renderer acceptance covers repeated clicks, immediate dismissal, keyboard/touch,
About layout, safe link requests and launch-failure feedback using mocked IPC.
The Windows acceptance script exercises real WebView pointer/key input and
captures About/help screenshots alongside existing staged/extracted package
checks. These runtime checks must pass on the final candidate before describing
this polish as visually or natively verified. No local browser/runtime success
is inferred from unit tests or a successful build.

## Ordinary results and developer mode

The ordinary view shows the file/item name, structured artist and a short
planned/actual status. Its detail dialog gives a concise skip/error reason only
when requested. It does not expose raw classification reasons, plan hashes,
journals, recovery counters or full technical paths. Unknown warnings still
produce a concise visible caution; errors, partial completion, cancellation and
retained source files remain visible. Missing final counters never become a
claim that no effects occurred. Copy totals say copies, not distinct source
files, so duet destinations cannot inflate a completed-file count.

An explicit `מצב מפתחים` switch lives under `אפשרויות תצוגה` in the information
menu. Fresh storage defaults off. Only this display preference is saved locally;
it does not change the six sorting settings, any plan or execution authority.
Switching views refetches page zero from the same retained plan. A failed fetch
keeps the existing mode/page. Switching is disabled while work owns the host.

The native page endpoint filters structured `developer_only` rows **before**
pagination. Only non-album inspections with no associated operations and no
`FileError`/`ProtectedError` trace qualify as routine. Recognized album actions,
read errors and vanished sources remain visible. Album inspection errors gain
an `ALBUM_READ_ERROR` display marker; the underlying plan/engine is untouched.
Structured artist data is projected directly from the decision, not parsed from
human explanation text. The full developer rows remain available, as do the
unchanged CLI protocol and raw evidence. Native AI acceptance explicitly asks
for the developer projection when comparing full-plan evidence.
