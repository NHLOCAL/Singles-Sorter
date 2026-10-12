#!/usr/bin/env python3
"""Stage the exact reviewed Android product notices and corresponding sources.

No network access or license inference. Both ABI graphs and all bytes fail closed.
"""
from __future__ import annotations

import argparse
import html
import json
from pathlib import Path
import re
import tempfile

import collect_licenses as common

HERE = Path(__file__).resolve().parent
TARGETS = ("x86_64-linux-android", "aarch64-linux-android")
NDK_FILES = {
    "NOTICE": "4d5224d1c0b54ffa88b0dc0088191638fdbb7227835095dfb98f7f66b416a323",
    "NOTICE.toolchain": "cbe3237be53c0a819f8df6aac5358fdee848eee3b6571b4e2ca20767ebd7465e",
    "source.properties": "fb5eec1ca5dac09442109cf79e9ce49faecd8b312dbc997e88903a394e14994a",
}
MAX_FILES, MAX_FILE, MAX_TOTAL, MAX_INVENTORY = 4096, 64 << 20, 256 << 20, 2 << 20


def validate_files(files: list[dict]) -> None:
    common.require(isinstance(files, list), "Invalid notice file list")
    common.require(len(files) <= MAX_FILES, "Too many notice files")
    seen, total = set(), 0
    for entry in files:
        common.require(isinstance(entry, dict), "Invalid notice entry")
        path = entry.get("path")
        common.require(isinstance(path, str) and all(
            re.fullmatch(r"[A-Za-z0-9._-]+", segment) and segment not in (".", "..")
            for segment in path.split("/")), "Unsafe notice asset path")
        common.require(path not in seen and path != "inventory.json", "Duplicate/reserved notice asset")
        seen.add(path)
        size = entry.get("bytes")
        common.require(type(size) is int and 0 < size <= MAX_FILE, "Invalid notice size")
        digest = entry.get("sha256")
        common.require(isinstance(digest, str) and bool(re.fullmatch(r"[0-9a-f]{64}", digest)), "Invalid notice hash")
        total += size
    common.require(total <= MAX_TOTAL, "Notice bundle too large")


def verified_snapshot(root: Path) -> list[tuple[str, bytes]]:
    """Read one bounded immutable snapshot; APK writes these exact checked bytes."""
    def unique(pairs):
        value = {}
        for key, item in pairs:
            common.require(key not in value, "Duplicate inventory key")
            value[key] = item
        return value

    common.require(root.is_dir() and not root.is_symlink(), "Invalid notice root")
    def bounded(path: Path, limit: int) -> bytes:
        common.require(path.is_file() and not path.is_symlink(), "Nonregular notice input")
        with path.open("rb") as stream:
            data = stream.read(limit + 1)
        common.require(len(data) <= limit, "Oversized notice input")
        return data

    raw = bounded(root / "inventory.json", MAX_INVENTORY)
    inventory = json.loads(raw, object_pairs_hook=unique)
    common.require(type(inventory.get("schema_version")) is int and inventory["schema_version"] == 1,
                   "Unsupported notice inventory")
    files = inventory.get("files")
    validate_files(files)
    common.require(any(f["path"] == "index.html" for f in files), "Missing notice index")
    found = set()
    for path in root.rglob("*"):
        common.require(not path.is_symlink(), "Notice symlink refused")
        common.require(path.is_dir() or path.is_file(), "Nonregular notice entry")
        if path.is_file():
            found.add(path.relative_to(root).as_posix())
    common.require(found == {f["path"] for f in files} | {"inventory.json"}, "Notice inventory closure mismatch")
    snapshot = [("inventory.json", raw)]
    for entry in files:
        data = bounded(root / entry["path"], entry["bytes"])
        common.require(len(data) == entry["bytes"] and common.sha256(data) == entry["sha256"],
                       "Notice bytes differ from inventory")
        snapshot.append((entry["path"], data))
    return snapshot


def index_page(files: list[dict]) -> bytes:
    links = "\n".join(f'<li><a href="{html.escape(f["path"], quote=True)}">'
                      f'{html.escape(f["path"])}</a> ({f["bytes"]} bytes)</li>'
                      for f in sorted(files, key=lambda f: f["path"]))
    return ("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\">"
            "<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">"
            "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">"
            "<title>Open-source notices and source availability</title>"
            "<style>body{font:16px sans-serif;margin:1rem;overflow-wrap:anywhere}li{margin:.7rem 0}</style>"
            "<h1>Open-source notices and source availability</h1>"
            "<p>This app includes the complete notices and corresponding native source archives listed below. "
            "Use Export in this viewer to save the entire verified bundle, including source archives.</p>"
            "<p><a href=\"SOURCE-AVAILABILITY.md\">Source availability and build instructions</a></p>"
            f"<ul>{links}</ul></html>").encode("utf-8")


def source_notice(revision: str, native: list[dict]) -> str:
    archives = "\n".join(f"- {r['included_source']} (SHA256 {r['archive_sha256']})" for r in native)
    return f"""# Android corresponding source and notices

TagLib 2.3.2 is distributed under its expressly offered MPL 1.1 alternative.
The complete, unmodified Covered Code is included on this same distribution
media in the source archive below, with its original per-file notices. The full
MPL 1.1 license and the alternative LGPL license are retained under retained/.
The selected route is MPL 1.1. The LGPL-only stock C bindings are disabled.
Recipients may obtain the complete source and notices with this viewer's
explicit Export action, without a network service or future hosting promise.

{archives}

The TagLib archive includes bundled utf8cpp source and its BSL-1.0 notice. The
zlib archive retains the complete zlib source and license. No handwritten source
patches are applied. Build configuration can generate/rename zconf.h in a
disposable zlib copy; the original archive and actual recipe are both supplied.
Modified redistributions must provide the actual modified Covered Code and
dated modification descriptions, retaining the applicable MPL notices. These
archives alone are not corresponding source for a later modified TagLib.

Exact application source revision: {revision}
Source reference: https://github.com/NHLOCAL/Singles-Sorter/tree/{revision}
The included build-input files and inventory hashes identify this package.
The owned C++ bridge, Rust/JNI glue and build recipes retain their project MIT
license; they do not relicense TagLib. Rebuild with the pinned Rust toolchain,
Android NDK r27d, SDK35, original checked assets and the included
native/apps/singles-android/build-native.sh and build-preview.sh recipes.
The native recipe selects Android API29, static PCRE2-32 10.46 with JIT disabled,
and the two listed Android ABI targets. It statically links the NDK C++ runtime
archives; the actual NDK NOTICE and NOTICE.toolchain are included under ndk/.
Android system libraries and WebView are supplied by the device, not this APK.

abi/ contains full notices from the exact lock-authenticated Cargo normal/build
closures; build helpers and proc macros are retained conservatively. This is
not a linker-only SBOM. rust/ contains complete notices from the build compiler.
frontend/ includes the reviewed shared frontend helper and icon notices. No
font files, Python interpreter, spaCy/Thinc runtime or training corpus is shipped.
retained/ preserves original model, Unicode data, Python-derived table,
PCRE2 and numerical reconstruction notices. Additional cesu8 license texts are
from the exact historical Rust revision whose COPYRIGHT matches its retained
COPYRIGHT-RUST.txt byte-for-byte; its original same-terms README is included.

The complete inventory records every file hash and size. This packaging record
does not provide blanket legal clearance or change any upstream license.
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--metadata-dir", type=Path, required=True)
    parser.add_argument("--ndk", type=Path, required=True)
    parser.add_argument("--npm-cache", type=Path, required=True)
    parser.add_argument("--dependency-cache", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    repo, output = args.repo_root.resolve(), args.output_dir.resolve()
    common.require(not output.exists() and not output.is_relative_to(repo), "Output must be fresh/outside source")
    compiler = common.run(["rustc", "--version", "--verbose"], repo / "native")
    common.require(f"release: {common.REVIEWED_RUST_RELEASE}" in compiler.splitlines(), "Unreviewed Rust compiler")
    sysroot = Path(common.run(["rustc", "--print", "sysroot"], repo / "native"))
    revision = common.run(["git", "rev-parse", "HEAD"], repo)
    common.require(bool(re.fullmatch(r"[0-9a-f]{40}", revision)), "Missing source revision")
    common.require(common.read_file(args.metadata_dir / "source-commit.txt").decode().strip() == revision,
                   "Metadata source revision differs from package source")
    lock_digest = common.sha256(common.read_file(repo / "native/plugins/original-inference/Cargo.lock"))
    common.require(common.read_file(args.metadata_dir / "lock.sha256").decode().split()[0] == lock_digest,
                   "Metadata lock receipt differs from package source")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".android-notices-", dir=output.parent) as temp:
        stage = Path(temp) / "licenses"
        stage.mkdir()
        collector = common.Collector(stage)
        abi_packages = {}
        for target in TARGETS:
            metadata = json.loads(common.read_file(args.metadata_dir / f"{target}.json"))
            policy = json.loads(common.read_file(HERE / f"android-cargo-{target}.json"))
            abi = common.Collector(stage / "abi" / target)
            packages = common.collect_cargo(abi, repo, metadata, policy, required_feature="product")
            for package in packages:
                package["notices"] = [name if name.startswith("retained/") else f"abi/{target}/{name}"
                                      for name in package["notices"]]
                if "included_source" in package:
                    package["included_source"] = f"abi/{target}/{package['included_source']}"
            abi_packages[target] = packages
            collector.files.extend({**f, "path": f"abi/{target}/{f['path']}"} for f in abi.files)
        retained = common.collect_retained(collector, repo, json.loads(common.read_file(HERE / "reviewed-retained-notices.json")))
        frontend = common.collect_frontend(collector, repo, json.loads(common.read_file(HERE / "reviewed-desktop-frontend-notices.json")), args.npm_cache)
        native = common.collect_native(collector, repo, args.dependency_cache)
        rust = common.collect_rust(collector, sysroot, compiler)
        for name, digest in NDK_FILES.items():
            data = common.read_file(args.ndk / name)
            common.require(common.sha256(data) == digest, f"Unreviewed NDK notice: {name}")
            collector.add(f"ndk/{name}", data, "Official NDK r27d / 27.3.13750724, actual build installation")
        supplements = json.loads(common.read_file(HERE / "android-supplements.json"))
        for item in supplements:
            data = common.read_file(HERE / item["path"])
            common.require(common.sha256(data) == item["sha256"], "Supplement notice changed")
            collector.add(f"supplements/{item['path']}", data, item["url"])
        cesu = HERE / "android-upstream/cesu8-1.1.0"
        common.require(common.read_file(cesu / "COPYRIGHT") == common.read_file(cesu / "COPYRIGHT-RUST.txt"),
                       "Historical Rust copyright no longer matches cesu8's retained attribution")
        # Include the exact Android-specific recipe in addition to shared native inputs.
        paths = common.run(["git", "ls-files", "--", "native/apps/singles-android", "native/plugins/original-inference",
                            "native/packaging/licenses", ".github/workflows/android-product.yml"], repo).splitlines()
        for name in paths:
            if name.endswith((".rs", ".java", ".toml", ".lock", ".sh", ".py", ".xml", ".c", ".json", ".yml")):
                collector.copy(common.checked_under(repo, name), f"sources/build-inputs/{name}", name)
        collector.add("SOURCE-AVAILABILITY.md", source_notice(revision, native).encode(), "Exact Android source distribution recipe")
        validate_files(collector.files)
        collector.add("index.html", index_page(collector.files), "Generated escaped local-only index")
        validate_files(collector.files)
        inventory = {"schema_version": 1, "profile": "android-product", "source_revision": revision,
                     "targets": list(TARGETS), "features": ["product"], "abi_packages": abi_packages,
                     "retained_components": retained, "frontend_components": frontend,
                     "native_sources": native, "rust": rust, "files": sorted(collector.files, key=lambda f: f["path"])}
        data = (json.dumps(inventory, ensure_ascii=False, indent=2) + "\n").encode()
        common.require(len(data) <= MAX_INVENTORY, "Inventory exceeds viewer contract")
        (stage / "inventory.json").write_bytes(data)
        stage.rename(output)
    print(json.dumps({"licenses": str(output), "files": len(collector.files)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
