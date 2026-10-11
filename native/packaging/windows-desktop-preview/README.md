# Original-AI Windows desktop preview package

This extends the existing Windows native UI acceptance workflow, not any release
or installer workflow. Default-off UI acceptance stays independent. The explicit
`original-ai` build consumes the five authenticated runtime files in
`<exe parent>/models/original`; no environment override or renderer-selected
model path is introduced.

`package_desktop.py build` stages a narrow allowlist only on Windows x64 after
feature-off actual UI acceptance. It binds PE DLL/symbol imports, official MSVC
`dumpbin /dependents`, compiler/source tree, original assets, PCRE2 archive and the
separate desktop license closure. Unknown DLL dependencies stop generation.
The CLI dependency allowlist and packaging behavior are unchanged.

Real Medium-integrity `tauri-driver` acceptance then exercises the staged package.
`archive` requires an acceptance receipt bound to both EXE and SHA256SUMS, all
required original-model, cancellation/navigation/repeat, corrupt/missing model
and immutability checks, and real IPC. Consumer ZIP extraction is tested again
in a Hebrew/space path. Upload happens only after final verification. Unit tests
of packaging never count as actual Windows evidence.

The WebView2 runtime and VC++ x64 runtime are prerequisites, not packaged DLLs.
WebView2's SDK loader may be statically linked by the already-used native binding;
the desktop notice inventory must cover that exact component and applicable
redistribution terms. Do not claim this executable contains no Microsoft code.
No installer/signing, clean-machine, network-isolated WebView2, Narrator, 200% DPI,
physical apply or Android acceptance claim is made here.

Source archives are generated into the artifact only, never Git. Original model
pickle, Python/oracle environments/corpora, private data, node_modules, build
intermediates and candidate models are excluded. Complete applicable notices and
source availability are audited separately, without blanket legal clearance.

Official prerequisite references:
- https://v2.tauri.app/start/prerequisites/
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist?view=msvc-170
