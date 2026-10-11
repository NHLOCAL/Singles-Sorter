# Desktop original-AI notice/source review

Reviewed 2026-10-11 for `singles-ui-host`, Windows x64 MSVC, with `desktop` and
`original-ai,selected-windows-execution` plus the production `tauri/custom-protocol` feature added by
the Tauri CLI (also enabled in tauri-macros). This is a bounded engineering inventory, not a legal opinion or
blanket clearance. Complete upstream texts control. The CLI policy is separate.

## Exact Rust dependency closure

The desktop lock and full target-filtered normal/build identity + feature graph
are pinned in `reviewed-desktop-cargo-notices.json`: 267 packages, comprising
258 crates.io archives and 9 local crates. Dev-only edges are excluded. Build
helpers/procedural macros are conservatively retained; this is not a claim that
each package occurs in executable machine code. Every registry archive must
match its reviewed SHA256 and Cargo.lock. All selected archive license,
COPYING, COPYRIGHT, NOTICE and UNLICENSE files, including nested notices, are
retained in full (464 archive files). Archive notices are compared with the
actual extracted build source too. This includes encoding_rs WHATWG/BSD and
COPYRIGHT, dpi's libm MIT notice, regex Unicode tables, tracing-core's spin
notice, ICU/Unicode notices, and both branches of common alternative licenses.

Seven crates omit complete license files from their published archive:
alloc-stdlib, defmt-parser, selectors, tauri-plugin, webview2-com,
webview2-com-macros and webview2-com-sys. The complete missing texts are retained
under `desktop-upstream/`, with exact source URLs and hashes in the policy.
For the six non-selectors packages the source is the upstream repository at the
archive's `.cargo_vcs_info.json` commit. The collector verifies that revision.
Selectors' exact source headers refer to Mozilla's MPL 2.0; its repository also
omits a complete text, so the supplemental full text is from the license
publisher at https://www.mozilla.org/en-US/MPL/2.0/ . The policy records the
exact downloaded text URL/hash and the source commit whose headers refer to it.

The five MPL-2.0 components are cssparser 0.37.0, cssparser-macros 0.7.1,
dtoa-short 0.3.5, option-ext 0.2.0 and selectors 0.38.0. Their exact complete
Cargo source archives accompany the executable under `sources/cargo/`, with
full license notices and an explicit same-media source statement in
SOURCE-AVAILABILITY.md. Every archived source file is compared with the extracted build source; additions
and modifications fail closed. No patches are applied. Original archive notices and
per-file notices remain intact. Future modifications require the actual changed
Covered Software and notices. This source provision is conservative even for
build-only crates. See MPL 2.0 sections 3.1, 3.2 and 3.4.

The existing project, original models, reconstructed AI, TagLib 2.3.2 MPL-1.1
source route, bundled utf8cpp, zlib, PCRE2, CPython, NumPy, spaCy, Thinc and
Unicode notices/source materials remain mandatory. The actual pinned Rust
compiler's full dependent-code notice directories are also retained. See the
common REVIEW.md and SOURCE-AVAILABILITY.md for those components.

## Frontend and artwork

The complete package-lock.json and package.json are pinned separately. The
only production npm package is @tauri-apps/api 2.12.3. Its complete MIT, Apache
and SPDX notices are read from its exact official npm archive, whose SHA512 is
verified against both the policy and lock; the installed notice files must
agree. Vite 8.3.4 (including its complete third-party LICENSE.md) and Rolldown
1.2.13 LICENSE and THIRD-PARTY-LICENSE are retained conservatively for generated
module/helper code. These build tools, npm, TypeScript and test tools are not
shipped as executables or node_modules. This is the embedded frontend scope,
not an npm build-tool executable distribution or a full npm-development SBOM.
Official exact archive URLs and integrity values are in inventory.json.

The 16 statically imported Fluent SVGs are the existing unmodified subset of
@fluentui/svg-icons 1.1.344. Their existing official npm provenance receipt,
exact asset hashes and Microsoft's full MIT copyright/permission/disclaimer
are retained. All public, src/assets and Tauri icon files are hash-pinned; a
new asset or font file requires review. The application logo/icon remains
covered by the project's original MIT notice. CSS uses system Segoe UI/Arial
fallbacks; no font binaries or font service requests are part of this package.

## Microsoft WebView2 SDK loader, distinct from the installed Runtime

The locked webview2-com-sys 0.39.1 source links WebView2LoaderStatic on MSVC.
This SDK loader is embedded in the executable. It is not the Evergreen browser
Runtime. The Rust wrapper's MIT notice does not replace Microsoft's SDK terms.

The wrapper's exact upstream generator commit identifies SDK 1.0.3800.47:
https://raw.githubusercontent.com/wravery/webview2-rs/edc2caf886175ccaebe86078c9cfe1ae2a187328/crates/update-bindings/src/main.rs

The official package inspected is:
https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/1.0.3800.47/microsoft.web.webview2.1.0.3800.47.nupkg

Its SHA256 is
56c9f26bdd07916a2d1949fb58a5c7e434dfa1173577dca879206050c4e718db.
The `build/native/x64/WebView2LoaderStatic.lib` member is byte-identical to the
locked crate's `x64/WebView2LoaderStatic.lib`, SHA256
89c6b872783b8f6c3cedbff618adb42082d455c615453ae10cfc753f1e8f25d8.
The collector rechecks the actual build-source loader against this reviewed hash.
No loader library or DLL is copied separately into the output directory.

The exact SDK LICENSE.txt permits source/binary redistribution subject to
retaining its copyright, conditions and disclaimer, and prohibits using
Microsoft/contributor names to endorse derived products without permission.
Both LICENSE.txt and the complete NOTICE.txt are supplied in
`native/microsoft-webview2-sdk-1.0.3800.47/`. The latter includes the SDK's ANTLR
and StringTemplate notices; they are retained conservatively without claiming
these tools ship in the application. The NuSpec specifies
`requireLicenseAcceptance=false`; reading/downloading it involved no new
explicit-acceptance step. These SDK terms do not establish Runtime rights.

Microsoft documents static-loader deployment here:
https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/static

## Separately installed prerequisites and distribution limits

The user supplies an installed supported Microsoft WebView2 Evergreen Runtime
and, when required by the verified build's dynamic MSVC imports, the Microsoft
Visual C++ x64 Runtime. Neither Runtime, installer, DLL, SDK archive nor SDK
binary is copied by this collector. The package must link to Microsoft's own
prerequisite pages rather than redistribute those files or silently install
or accept their terms:

- https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist

This preview creates no installer and changes no linkage. The parent packaging
gate must verify actual executable imports, prohibit accidentally bundled
runtime files, hash the complete notice/source tree and perform clean-folder
Windows tests. Retaining notices is not proof of those checks or entitlement
to redistribute other Microsoft components.


## Selected-execution closure review, 2026-10-11

The default-enabled Windows target metadata from CI run `38176429750`, source
`43a584470c478131ed85bb82af458c0c3ea508c8`, was compared before updating this policy.
The old-profile graph was identical to its earlier reviewed graph. The selected
profile adds only the local MIT `sorter-executor` crate, the corresponding host
edge/feature, and `sorter-io`'s existing native mutation feature. Every one of the
258 registry package identities, checksums, licenses, resolved features and
normal/build dependency edges remains unchanged. The complete 465 registry
entries in the nested lockfile also match the pre-integration lock; its change
adds the local executor and host dependency edge, with no registry version drift.

Selected metadata SHA256:
`e2c25b36bf845ad0d2b035d6ddd31d9841c170cbf9001e1cb56965b14645e8c4`.
The reviewed LF-normalized Cargo.lock SHA256 is
`260506768409370a5231628f84eac403a7f2d8d4c619cf90f9c216f48d7d0bbc`.
The policy now pins the actual default-enabled combined feature graph used by
Tauri's production build and the collector. Runtime archive/checksum/full-notice
checks are unchanged. This resolution review does not establish Windows runtime
acceptance; real original-AI and selected-execution package gates remain required.
