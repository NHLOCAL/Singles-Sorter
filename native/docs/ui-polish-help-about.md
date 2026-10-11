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
