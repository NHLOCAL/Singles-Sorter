# Focused result and viewport corrections

This slice changes presentation only. The complete retained plan, operation IDs,
policy, revalidation and execution counters remain authoritative and unchanged.
It is a candidate until actual native receipts from its final build pass.

- Ordinary results show action-bearing files and concise errors. A separate
  unchecked `הצג גם ללא פעולה` option includes inactive files. Developer mode is
  in advanced Settings, accessible from the header, with its existing local
  preference key and fresh-install default off. It does not enable that option.
- Desktop and Android apply both filters before offset/limit; the filtered total
  and inactive count are derived from the same projection. Routine album
  inspections remain developer-only. Album read errors, vanished files and
  item errors stay visible. Fix previews with no rename, changed tag or failure
  are inactive. Terminal Windows outcomes hide only blocked/no-effect items;
  failed, cancelled, partial, retained-source and uncertain outcomes stay visible.
- The interface no longer inserts an em dash for an absent artist. User filenames
  remain verbatim, and a truncated filename can be read in its title/detail view.
- Root cause of footer movement: notification heights changed `fitWorkspace`,
  which changed the root into intrinsic `auto auto auto` rows. Its content could
  leave unused space below the footer. The new shell always has bounded viewport
  height and `auto minmax(0,1fr) auto` rows. Its middle scrolls; no negative offset
  or overlapping fixed footer is used. VisualViewport resizing handles the
  unzoomed keyboard viewport without reflowing underneath pinch magnification.
- Root cause of horizontal risk: implicit automatic grid-column minimum and
  missing minimum-width constraints through the table/scroll chain. Explicit
  zero-minimum tracks and bounded cells remove that intrinsic expansion. Result
  content has vertical-only scrolling/gestures and no width-drag control.

## Required acceptance

`desktop_stability.py` runs in the real Windows application on 52 disposable
long Hebrew/Latin filenames plus one unrecognized WAV. It checks all active
filenames across pages, explicit inactive visibility, full filename access,
horizontal pointer drags, both horizontal scroll directions and vertical scroll.
It captures setup/results at 1000×650, 360×800, 390×800 and 200% text with no
notice, wrapped notice, multiple alerts and a long mixed-language path error.
Footer bounds must remain identical within each viewport/text-size case and
flush with the viewport. Notification stress changes presentation nodes only;
IPC remains native and protected-tree evidence must remain unchanged.

Portable unit checks do not prove these visual results. Native Android keyboard,
touch and provider behavior still need device evidence; Windows narrow layouts
are not labeled Android validation.

Implementation references: [CSS Grid automatic minimum size](https://www.w3.org/TR/css-grid-1/#min-size-auto),
[CSS overflow behavior](https://www.w3.org/TR/css-overflow-3/#overflow-properties),
[Visual Viewport](https://drafts.csswg.org/cssom-view/#the-visualviewport-interface).
