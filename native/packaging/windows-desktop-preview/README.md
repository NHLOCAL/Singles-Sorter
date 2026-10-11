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

### Windows staging access

Private construction uses Python TemporaryDirectory intentionally. On Windows,
Python 3.12.10 mode-0700 directory creation installs a protected DACL; CPython
explicitly documents that an elevated creator's directory is inaccessible to
its non-elevated token. Renaming retains those security descriptors. Before
publication, the packager therefore recreates every validated public package
file and directory (including nested licenses) under a new exclusive sibling
with ordinary parent inheritance. It copies bytes only, validates the complete
package again, and then renames it to the final output. Copy/validation failures
remove only the owned pending directory. No existing ACL, owner, integrity
label, privilege or security policy is changed. Read-only ACL diagnostics cover
private staging and published output; actual Medium-integrity UI acceptance
remains mandatory.

Primary references: [CPython owner-rights change](https://github.com/python/cpython/pull/118515),
[3.12.10 tempfile](https://github.com/python/cpython/blob/v3.12.10/Lib/tempfile.py),
[3.12.10 Windows mkdir](https://github.com/python/cpython/blob/v3.12.10/Modules/posixmodule.c),
and [Microsoft file security inheritance](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights).

The read-only Windows PowerShell ACL subprocess removes `PSModulePath` from
its child environment only. This follows Microsoft's documented
[pwsh → Python → Windows PowerShell resolution](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_psmodulepath?view=powershell-7.6#starting-windows-powershell-from-powershell-7),
so Windows PowerShell reconstructs its standard module paths instead of trying
to load incompatible PowerShell 7 modules. The parent's environment and all
execution/security policies remain unchanged. Module/ACL errors still fail the
early access preflight and preserve their captured diagnostic text.
