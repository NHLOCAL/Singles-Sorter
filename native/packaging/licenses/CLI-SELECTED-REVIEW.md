# Measured selected-execution CLI notice profile

The Windows metadata artifacts from source `574dcc764b2d73067d15bb1104fd99c13f4ee727`
were captured before compile/package checks in
[original CLI run 38192067750](https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38192067750).
The collector follows normal/build edges only, rooted at `sorter-cli`, for the
Windows target and default-enabled original-AI + selected-execution features.

- Old original-AI closure: 45 nodes, comprising 37 registry and 8 local packages.
- Selected closure: 46 nodes, adding only local MIT `sorter-executor 0.1.0`.
- Registry identities, declared licenses, existing reviewed notice hashes and
  locked archive checksums are unchanged. No newly fetched notice was assumed.
- The profile/feature dependency differences are confined to the CLI, executor
  and I/O nodes. The complete measured selected graph is now pinned, alongside
  the unchanged normalized workspace lock SHA-256
  `3b1974bbc3d2b16296b0aba5494350ad9687b3ba2d22b7231ac665cef1caa4bf`.
- Old metadata SHA-256:
  `3f0e71fee628d45d0745fa8e1e903069bd78f62ea24c101b40778533d8034e7a`.
- Selected metadata SHA-256:
  `11794bc132aaf01a953d8bc5165e38f2af9bb2bbe65e43dbcecf69f6deceb799`.

The separately measured 267-node desktop selected graph already exactly matches
its reviewed policy, including `Win32_Globalization` through existing transitives.
Its registry/licenses/checksums and lock remain unchanged; no desktop policy
repinning is necessary for this feature addition.

These checks establish the measured dependency/notice boundary. They do not
replace actual Windows execution, staged/extracted checks, or legal advice.
