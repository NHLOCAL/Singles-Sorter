#!/usr/bin/env python3
"""Collect reviewed Windows original-AI notices and same-media native sources.

Python 3.11+; standard library only. No generic license detector and no legal
clearance claim. Unexpected graph, source, checksum or notice drift stops output.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tarfile
import tempfile
import tomllib

HERE = Path(__file__).resolve().parent
TARGET = "x86_64-pc-windows-msvc"
REVIEWED_RUST_RELEASE = "1.99.0"
RUST_LICENSE_FILES = {
    "Apache-2.0.txt", "BSD-2-Clause.txt", "CC-BY-SA-4.0.txt",
    "GCC-exception-3.1.txt", "GPL-2.0-only.txt", "GPL-3.0-or-later.txt",
    "ISC.txt", "LLVM-exception.txt", "MIT.txt", "NCSA.txt", "OFL-1.1.txt",
    "Unicode-3.0.txt",
}
PROFILES = {
    "cli": {"root": "sorter-cli", "manifest": "native/Cargo.toml", "lock": "native/Cargo.lock",
            "features": ["sorter-cli/original-ai", "sorter-cli/selected-windows-execution"], "policy": "reviewed-cargo-notices.json"},
    "desktop": {"root": "singles-ui-host", "manifest": "native/apps/singles-ui/src-tauri/Cargo.toml",
                "lock": "native/apps/singles-ui/src-tauri/Cargo.lock", "features": ["desktop", "original-ai", "selected-windows-execution", "tauri/custom-protocol"],
                "policy": "reviewed-desktop-cargo-notices.json"},
}
SOURCE_REPOSITORY = "https://github.com/NHLOCAL/Singles-Sorter"
NATIVE_PINS = {
    "taglib": ("2.3.2", "https://taglib.org/releases/taglib-2.3.2.tar.gz", "3ca2d8afaa7f1cf7f6ed10e511ebc368bfacd6dcaa3dbfa690b89e502e8963dc"),
    "zlib": ("1.3.1", "https://zlib.net/fossils/zlib-1.3.1.tar.gz", "9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23"),
}
BUILD_INPUTS = [
    "LICENSE", "native/Cargo.toml", "native/Cargo.lock", "native/rust-toolchain.toml",
    "native/crates/sorter-io/Cargo.toml", "native/crates/sorter-io/build.rs",
    "native/crates/sorter-io/bridge/CMakeLists.txt",
    "native/crates/sorter-io/bridge/taglib_bridge.cpp",
    "native/crates/sorter-io/bridge/taglib_bridge.h",
    "native/crates/sorter-io/bridge/cmake/AcquireDependency.cmake",
    "native/crates/sorter-io/bridge/cmake/zlib/ZLIBConfig.cmake",
    "native/crates/sorter-io/bridge/cmake/zlib/ZLIBConfigVersion.cmake",
    "native/crates/sorter-io/dependencies/lock.json",
    "native/crates/sorter-io/dependencies/source-manifest.json",
    "native/crates/sorter-io/dependencies/README.md",
    "native/probes/released-ner/acquire_pcre2.py",
    "native/probes/released-ner/pcre2-lock.json",
    "native/probes/released-ner/released-ner-text/build.rs",
]


class CollectionError(RuntimeError):
    """An input failed the bounded, reviewed packaging contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CollectionError(message)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_file(path: Path) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"Missing or nonregular input: {path}")
    data = path.read_bytes()
    require(bool(data.strip()), f"Empty input: {path}")
    return data


def relative_path(value: str) -> Path:
    p = PurePosixPath(value)
    require(bool(value) and not p.is_absolute() and all(x not in ("..", ".") for x in p.parts)
            and "\\" not in value and ":" not in value, f"Unsafe relative path: {value}")
    return Path(*p.parts)


def checked_under(root: Path, value: str) -> Path:
    path = root / relative_path(value)
    require(path.resolve().is_relative_to(root.resolve()), f"Path leaves input root: {value}")
    return path


def run(args: list[str], cwd: Path) -> str:
    result = subprocess.run(args, cwd=cwd, check=True, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, encoding="utf-8")
    return result.stdout.strip()


def closure(metadata: dict, root_name: str = "sorter-cli", required_feature: str = "original-ai") -> tuple[list[dict], dict]:
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    roots = [p for p in packages.values() if p["name"] == root_name and p["source"] is None]
    require(len(roots) == 1, f"Expected exactly one local {root_name} package")
    root = roots[0]
    require(required_feature in nodes[root["id"]]["features"], f"Cargo metadata lacks {root_name}/{required_feature}")
    seen, pending = set(), [root["id"]]
    while pending:
        pkg = pending.pop()
        if pkg in seen:
            continue
        require(pkg in packages and pkg in nodes, f"Incomplete Cargo resolve graph: {pkg}")
        seen.add(pkg)
        for dep in nodes[pkg]["deps"]:
            if any(k["kind"] in (None, "normal", "build") for k in dep["dep_kinds"]):
                pending.append(dep["pkg"])
    return sorted((packages[p] for p in seen), key=lambda p: (p["name"], p["version"])), nodes


def archive_member(archive: Path, member_name: str) -> bytes:
    # Never extract archives onto the filesystem. Only read exact regular members.
    with tarfile.open(archive, "r:gz") as tf:
        matches = [m for m in tf.getmembers() if m.name == member_name]
        require(len(matches) == 1 and matches[0].isfile(), f"Missing/unsafe archive member: {member_name}")
        require(matches[0].size < 32 * 1024 * 1024, f"Oversized notice: {member_name}")
        stream = tf.extractfile(matches[0])
        require(stream is not None, f"Unreadable archive member: {member_name}")
        data = stream.read()
        require(bool(data.strip()), f"Empty archive member: {member_name}")
        return data


def verify_unmodified_source(archive: Path, source: Path, prefix: str) -> None:
    # Same-media source must be the source actually built, not merely a pristine
    # upstream archive next to a patched extracted tree. Cargo adds .cargo-ok.
    archived = set()
    with tarfile.open(archive, "r:gz") as tf:
        for member in tf.getmembers():
            require(member.name.startswith(prefix + "/"), "Unexpected Cargo source archive root")
            if member.isdir():
                continue
            require(member.isfile(), "Nonregular Cargo source archive member")
            name = member.name[len(prefix) + 1:]
            path = checked_under(source, name)
            require(path.is_file() and not path.is_symlink(), f"Missing source file: {name}")
            stream = tf.extractfile(member)
            require(stream is not None and path.read_bytes() == stream.read(), f"Modified MPL source: {name}")
            archived.add(name)
    actual = {p.relative_to(source).as_posix() for p in source.rglob("*") if p.is_file()}
    require(actual - {".cargo-ok", ".cargo-checksum.json"} == archived, "Unreviewed files in MPL source tree")


class Collector:
    def __init__(self, output: Path):
        self.output = output
        self.files: list[dict] = []

    def add(self, destination: str, data: bytes, provenance: str) -> str:
        path = checked_under(self.output, destination)
        require(not path.exists(), f"Duplicate output: {destination}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        self.files.append({"path": destination, "sha256": sha256(data), "bytes": len(data),
                           "provenance": provenance})
        return destination

    def copy(self, source: Path, destination: str, provenance: str) -> str:
        return self.add(destination, read_file(source), provenance)


def cargo_graph(packages: list[dict], nodes: dict) -> list[dict]:
    identities = {p["id"]: f"{p['name']}@{p['version']}" for p in packages}
    return [{"package": identities[p["id"]], "features": sorted(nodes[p["id"]]["features"]),
             "dependencies": sorted([{"package": identities[d["pkg"]],
                 "kinds": sorted([{"kind": k["kind"] or "normal", "target": k.get("target")}
                    for k in d["dep_kinds"] if k["kind"] in (None, "normal", "build")],
                    key=lambda k: (k["kind"], k["target"] or ""))}
                 for d in nodes[p["id"]]["deps"]
                 if any(k["kind"] in (None, "normal", "build") for k in d["dep_kinds"])],
                 key=lambda d: d["package"])}
            for p in packages]


def collect_cargo(c: Collector, repo: Path, metadata: dict, policy: dict, required_feature: str = "original-ai") -> list[dict]:
    packages, nodes = closure(metadata, policy.get("root_package", "sorter-cli"), required_feature)
    expected = {(p["name"], p["version"]): p for p in policy["packages"]}
    local = {(p["name"], p["version"]): p for p in policy["local_packages"]}
    found = {(p["name"], p["version"]) for p in packages}
    require(found == expected.keys() | local.keys(),
            f"Unreviewed dependency closure; added={sorted(found - expected.keys() - local.keys())}; "
            f"missing={sorted((expected.keys() | local.keys()) - found)}")
    lock_path = checked_under(repo, policy.get("lock_path", "native/Cargo.lock"))
    lock_bytes = read_file(lock_path)
    if "lock_sha256_lf" in policy:
        require(sha256(lock_bytes.replace(b"\r\n", b"\n")) == policy["lock_sha256_lf"], "Reviewed Cargo.lock changed")
        require(cargo_graph(packages, nodes) == policy["graph"], "Reviewed Cargo feature/dependency graph changed")
    lock = tomllib.loads(lock_bytes.decode("utf-8"))
    locked = {(p["name"], p["version"], p.get("source")): p for p in lock["package"]}
    rows = []
    for package in packages:
        key = (package["name"], package["version"])
        row = {"name": key[0], "version": key[1], "features": nodes[package["id"]]["features"]}
        if package["source"] is None:
            require(key in local, f"Registry dependency replaced by local source: {key}")
            manifest = checked_under(repo, local[key]["manifest_path"])
            require(Path(package["manifest_path"]).resolve() == manifest.resolve(),
                    f"Unexpected local package source: {key}")
            row.update({"source": local[key]["manifest_path"], "license": "MIT; retained component notices also apply",
                        "notices": ["retained/LICENSE"]})
        else:
            require(key in expected, f"Local dependency replaced by registry source: {key}")
            rule = expected[key]
            require(package["source"] == rule["source"] and package["license"] == rule["license"],
                    f"Unreviewed registry/license: {key}")
            lock_entry = locked.get((*key, package["source"]), {})
            require(lock_entry.get("checksum") == rule["checksum"], f"Cargo.lock checksum drift: {key}")
            source = Path(package["manifest_path"]).parent
            # Cargo registry layout is registry/src/<registry>/<crate>/Cargo.toml.
            require(source.parent.parent.name == "src", f"Unsupported registry source layout: {source}")
            archive = source.parent.parent.parent / "cache" / source.parent.name / f"{key[0]}-{key[1]}.crate"
            require(sha256(read_file(archive)) == rule["checksum"], f"Registry archive checksum mismatch: {key}")
            notices = []
            for name, digest in sorted(rule["notices"].items()):
                relative_path(name)
                data = archive_member(archive, f"{key[0]}-{key[1]}/{name}")
                require(sha256(data) == digest, f"Reviewed notice checksum mismatch: {key}/{name}")
                # Ensure the build's extracted notice is the same exact registry notice.
                require(read_file(checked_under(source, name)) == data, f"Altered registry notice: {key}/{name}")
                notices.append(c.add(f"cargo/{key[0]}-{key[1]}/{name}", data,
                                     f"crates.io {key[0]} {key[1]}, locked archive {rule['checksum']}"))
            for supplement in rule.get("upstream_notices", []):
                vcs = json.loads(archive_member(archive, f"{key[0]}-{key[1]}/.cargo_vcs_info.json"))
                require(vcs["git"]["sha1"] == supplement["vcs_revision"], f"Upstream notice revision mismatch: {key}")
                data = read_file(checked_under(HERE, supplement["path"]))
                require(sha256(data.replace(b"\r\n", b"\n")) == supplement["sha256"], f"Upstream notice drift: {key}")
                notices.append(c.add(f"cargo/{key[0]}-{key[1]}/upstream/{Path(supplement['path']).name}", data, supplement["url"]))
            if rule.get("include_source"):
                require(rule["license"] == "MPL-2.0", "Unreviewed Cargo source-distribution route")
                verify_unmodified_source(archive, source, f"{key[0]}-{key[1]}")
                row["included_source"] = c.copy(archive, f"sources/cargo/{archive.name}", "Exact Cargo.lock-authenticated MPL-2.0 source")
                row["source_license"] = "MPL-2.0"
            require(bool(notices), f"No reviewed complete notices: {key}")
            row.update({"source": rule["source"], "license": rule["license"],
                        "crate_sha256": rule["checksum"], "notices": notices,
                        "source_url": f"https://crates.io/api/v1/crates/{key[0]}/{key[1]}/download"})
        rows.append(row)
    return rows


def collect_frontend(c: Collector, repo: Path, policy: dict, npm_cache: Path) -> list[dict]:
    root = checked_under(repo, policy["root"])
    lock_bytes = read_file(root / "package-lock.json")
    require(sha256(lock_bytes.replace(b"\r\n", b"\n")) == policy["lock_sha256_lf"], "Reviewed frontend lock changed")
    manifest_bytes = read_file(root / "package.json")
    require(sha256(manifest_bytes.replace(b"\r\n", b"\n")) == policy["manifest_sha256_lf"], "Reviewed frontend manifest changed")
    lock = json.loads(lock_bytes)
    require(lock["lockfileVersion"] == 3, "Unsupported npm lock format")
    production = {name for name, pkg in lock["packages"].items() if name and not pkg.get("dev", False)}
    require(production == set(policy["production_packages"]), "Unreviewed frontend production closure")
    rows = []
    for rule in policy["packages"]:
        package = lock["packages"][rule["lock_path"]]
        require(all(package[k] == rule[k] for k in ("version", "resolved", "integrity", "license")),
                f"Unreviewed frontend package: {rule['lock_path']}")
        require(rule["resolved"].startswith("https://registry.npmjs.org/"), "Only official npm registry archives are supported")
        algorithm, encoded = rule["integrity"].split("-", 1)
        require(algorithm == "sha512", "Only SHA512 npm integrity is reviewed")
        digest = base64.b64decode(encoded, validate=True).hex()
        archive = npm_cache / "_cacache/content-v2/sha512" / digest[:2] / digest[2:4] / digest[4:]
        require(hashlib.sha512(read_file(archive)).hexdigest() == digest, f"npm archive integrity mismatch: {rule['name']}")
        source = checked_under(root, rule["lock_path"])
        actual = json.loads(read_file(source / "package.json"))
        require((actual["name"], actual["version"]) == (rule["name"], rule["version"]), "Installed npm package drift")
        notices = []
        for name, expected in rule["notices"].items():
            data = archive_member(archive, "package/" + name)
            require(sha256(data) == expected, f"Reviewed npm notice changed: {rule['name']}/{name}")
            require(read_file(checked_under(source, name)) == data, f"Installed npm notice changed: {rule['name']}/{name}")
            notices.append(c.add(f"frontend/{rule['name'].replace('@', '').replace('/', '-')}-{rule['version']}/{name}", data, rule["resolved"]))
        require(bool(notices), "Missing frontend notices")
        rows.append({"component": rule["name"], "version": rule["version"], "license": rule["license"],
                     "scope": rule["scope"], "source_url": rule["resolved"], "integrity": rule["integrity"], "notices": notices})
    expected_assets = policy["assets"]
    actual_assets = {p.relative_to(root).as_posix() for folder in policy["asset_directories"]
                     for p in checked_under(root, folder).rglob("*") if p.is_file()}
    require(actual_assets == set(expected_assets), "Unreviewed frontend asset set")
    for name, digest in expected_assets.items():
        data = read_file(checked_under(root, name))
        if Path(name).suffix.lower() in (".txt", ".json", ".svg"):
            data = data.replace(b"\r\n", b"\n")
        require(sha256(data) == digest, f"Reviewed frontend asset changed: {name}")
    for name in policy["retained_notices"]:
        c.copy(checked_under(root, name), f"frontend/assets/{name}", "Reviewed retained frontend icon notice/provenance")
    return rows


def collect_desktop_sdk(c: Collector, metadata: dict, policy: dict) -> dict:
    matches = [p for p in metadata["packages"] if (p["name"], p["version"]) ==
               (policy["wrapper_package"], policy["wrapper_version"])]
    require(len(matches) == 1, "Missing reviewed WebView2 SDK wrapper")
    source = Path(matches[0]["manifest_path"]).parent
    loader = read_file(checked_under(source, policy["loader_path"]))
    require(sha256(loader) == policy["loader_sha256"], "WebView2 static loader differs from exact official SDK")
    notices = []
    for path, digest in policy["notices"].items():
        data = read_file(checked_under(HERE, path))
        require(sha256(data.replace(b"\r\n", b"\n")) == digest, "Reviewed WebView2 SDK notice changed")
        notices.append(c.add(f"native/microsoft-webview2-sdk-{policy['version']}/{Path(path).name}", data,
                             policy["source_url"] + " member " + Path(path).name))
    return {**{k: policy[k] for k in ("name", "version", "source_url", "source_archive_sha256", "loader_sha256", "license")},
            "notices": notices, "distribution": "Loader linked statically in executable; SDK archive, loader binary, Runtime and installers not copied"}


def collect_retained(c: Collector, repo: Path, policy: dict) -> list[dict]:
    rows = []
    for group in policy["components"]:
        notices = []
        for name, digest in sorted(group["notices"].items()):
            data = read_file(checked_under(repo, name))
            require(sha256(data.replace(b"\r\n", b"\n")) == digest, f"Retained notice needs review: {name}")
            notices.append(c.add(f"retained/{name}", data, name))
        rows.append({"component": group["component"], "license_summary": group["license_summary"],
                     "notices": notices})
    return rows


def collect_native(c: Collector, repo: Path, cache: Path) -> list[dict]:
    lock = json.loads(read_file(repo / "native/crates/sorter-io/dependencies/lock.json"))
    require(set(lock["dependencies"]) == set(NATIVE_PINS), "Unreviewed native dependency")
    rows = []
    for name, (version, url, digest) in NATIVE_PINS.items():
        entry = lock["dependencies"][name]
        require((entry["version"], entry["url"], entry["archive_sha256"]) == (version, url, digest),
                f"Native source pin needs review: {name}")
        filename = f"{name}-{version}.tar.gz"
        archive = cache / filename
        data = read_file(archive)
        require(sha256(data) == digest, f"Native source archive checksum mismatch: {archive}")
        destination = c.add(f"sources/{filename}", data, url)
        rows.append({"component": name, "version": version, "archive_sha256": digest,
                     "source_url": url, "included_source": destination,
                     "selected_license": "MPL-1.1" if name == "taglib" else "Zlib"})
        if name == "taglib":
            for member, retained in [("COPYING.MPL", "taglib-COPYING.MPL"),
                                     ("COPYING.LGPL", "taglib-COPYING.LGPL"),
                                     ("3rdparty/utfcpp/LICENSE", "utf8cpp-LICENSE")]:
                archived = archive_member(archive, f"taglib-{version}/{member}")
                actual = read_file(repo / "native/crates/sorter-io/dependencies/licenses" / retained)
                require(archived.replace(b"\r\n", b"\n") == actual.replace(b"\r\n", b"\n"),
                        f"Retained native notice differs from exact source: {retained}")
        else:
            # Include zlib's actual source header/README, not only an extracted license paragraph.
            for member in ("zlib.h", "README"):
                c.add(f"native/zlib-{version}/{member}", archive_member(archive, f"zlib-{version}/{member}"), url)
    for name in BUILD_INPUTS:
        c.copy(checked_under(repo, name), f"sources/build-inputs/{name}", name)
    return rows


def collect_rust(c: Collector, sysroot: Path, compiler: str) -> dict:
    docs = sysroot / "share/doc/rust"
    # Rust 1.99 ships both complete compiler and library copyright HTML plus
    # their complete license-text directory even under rustup's minimal profile.
    files = [docs / "COPYRIGHT.html", docs / "COPYRIGHT-library.html"]
    licenses = docs / "licenses"
    require(licenses.is_dir() and not licenses.is_symlink(), f"Missing Rust license directory: {licenses}")
    missing = sorted(name for name in RUST_LICENSE_FILES if not (licenses / name).is_file())
    require(not missing, f"Rust sysroot is missing reviewed complete license texts: {missing}")
    files.extend(sorted(p for p in licenses.rglob("*") if p.is_file()))
    files.extend(sorted(p for p in docs.iterdir() if p.is_file() and p.name.startswith("LICENSE")))
    notices = [c.copy(p, f"rust/{p.relative_to(docs).as_posix()}", "actual build rustc sysroot") for p in files]
    return {"component": "Rust toolchain and standard-library dependent code", "compiler": compiler,
            "notices": notices, "scope": "Full upstream toolchain notices retained conservatively; not every listed tool is shipped"}


def default_cache() -> Path:
    if os.environ.get("SORTER_DEPENDENCY_CACHE"):
        return Path(os.environ["SORTER_DEPENDENCY_CACHE"])
    if os.name == "nt":
        require(bool(os.environ.get("LOCALAPPDATA")), "LOCALAPPDATA or --dependency-cache is required")
        return Path(os.environ["LOCALAPPDATA"]) / "SinglesSorter/cache/sources"
    return Path(os.environ.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "singles-sorter/sources"


def source_notice(revision: str, native: list[dict]) -> str:
    archives = "\n".join(f"- {r['included_source']} (SHA256 {r['archive_sha256']})" for r in native)
    return f"""# Source availability and selected TagLib license

This preview uses TagLib 2.3.2 under the Mozilla Public License 1.1 (MPL-1.1)
alternative expressly offered by upstream. TagLib's Covered Code source is
available under MPL 1.1 in the full, unchanged source archive included on the
same media as this executable. Relative paths below start in this licenses
folder. The source archive retains its per-file copyright and license notices,
including the initial developers and contributors. The full MPL 1.1 text is in
retained/native/crates/sorter-io/dependencies/licenses/taglib-COPYING.MPL.
TagLib's alternative LGPL text is retained too; that does not change the chosen
MPL route. The LGPL-only stock C bindings are disabled and are not linked.

## Exact included upstream source

{archives}

TagLib's source archive includes its bundled utf8cpp 4.2.0 source and BSL-1.0
license. zlib 1.3.1 is also supplied in full under its retained zlib license.
Archive hashes are verified against the repository's locked official downloads
before copying. Official source URLs are recorded in inventory.json. These
links are provenance references; source access for this package does not depend
on a promise that an external download will stay online.

## Build choices and modifications

The checked-in recipe applies no source patches to TagLib or its bundled
utf8cpp. It builds static TagLib with WITH_ZLIB=ON, uses only the pinned bundled
utf8cpp, and disables stock C bindings, examples, and tests. The separate owned
C++ bridge and all of its CMake/acquisition/configuration files are included in
sources/build-inputs/native/crates/sorter-io/. Those owned files retain the
repository MIT license; they do not relicense TagLib.

The recipe builds zlib from a disposable copy: upstream CMake can rename
zconf.h while generating a build-specific configuration. No handwritten zlib
source patch is applied. This build-time rename/configuration is not presented
as a pristine built tree. The unmodified source archive and the complete recipe
used to create the build copy are both included. Compiler output directories
and generated headers can be regenerated with that recipe.

Repository revision recorded during packaging: {revision}
Revision reference: {SOURCE_REPOSITORY}/tree/{revision}
The exact co-packaged build inputs and their file hashes are authoritative for
this package; a commit reference is not a claim that it has been published.
See the package build manifest for compiler, flags, target, and dirty-tree state.
For a standalone bridge build, use CMake with the included bridge directory as
-S, a fresh output directory as -B, -A x64, and
-DSORTER_DEPENDENCY_CACHE=<absolute path to this folder's sources directory>
-DSORTER_DEPENDENCIES_OFFLINE=ON. Select the same CMAKE_MSVC_RUNTIME_LIBRARY as
recorded by the package build. Build --config Release --target sorter_metadata.

For later modified TagLib distributions, include the actual modified source or
complete patches and a dated change description, preserving MPL notices. An
unmodified upstream archive alone is not the source for modified Covered Code.
Changing this route to network-only source availability requires a separate
review of availability and retention obligations; this preview makes no such
future hosting promise. See MPL 1.1 sections 1.11 and 3.1–3.7.

## Other notices and provenance

The original project/model MIT notice, spaCy/Thinc MIT notices, NumPy BSD notice,
full CPython license, both retained Unicode notices, and full PCRE2 10.46 source
license are in retained/. Python, spaCy, Thinc, NumPy and scikit-learn runtimes
are not shipped. Their retained notices describe data/reconstruction provenance.
The CPython 3.12.10 / Unicode 15.0 data were frozen into native predicate tables;
tokenizer and classifier behavior were reconstructed in Rust rather than
shipping the CPython interpreter. This describes the table-format and runtime
adaptation, not an upstream-unmodified Python distribution.
PCRE2's actual complete BSD-3-Clause WITH PCRE2-exception notice is
retained/native/probes/released-ner/released-ner-text/licenses/pcre2-source-10.46.md;
the separate upstream COPYING pointer is not used as a substitute.

cargo/ contains the complete reviewed license/notice files read from the exact
Cargo.lock-authenticated crates.io archives. The inventory covers the Windows
original-ai normal/build dependency closure and conservatively retains build
helper/procedural-macro notices. rust/ contains the build toolchain's full
COPYRIGHT.html, COPYRIGHT-library.html, and license-text directory, including
dependent-code notices. Full compiler notices can name tools not in the binary.

This inventory and source package document this bounded build. They do not
provide blanket legal clearance or resolve Microsoft's separate compiler/SDK/
CRT redistribution terms. They add no Microsoft runtime DLLs. The packager must
use authorized toolchains and honor the terms of the selected CRT linkage.
"""


def desktop_source_notice(cargo: list[dict]) -> str:
    sources = "\n".join(f"- {p['name']} {p['version']}: {p['included_source']} (SHA256 {p['crate_sha256']})"
                        for p in cargo if p.get("source_license") == "MPL-2.0")
    return f"""

## Additional desktop MPL 2.0 source and frontend notices

The desktop normal/build graph includes the following MPL-2.0 components.
Their complete, unmodified Cargo source archives are provided on the same media,
under MPL 2.0, including original source notices. No source patches are applied.
The exact locked manifests and build inputs accompany them. The complete MPL
text is retained with their cargo/ notices. Executable-form recipients may use,
modify and distribute this covered source under MPL 2.0; no application terms
limit those rights. See MPL 2.0 sections 3.1, 3.2 and 3.4. Some of these crates
are build helpers; source is retained conservatively rather than asserting
that each is linked into the executable. Modified distributions must supply
their actual modified Covered Software and retain the applicable notices.

{sources}

frontend/ retains the exact npm @tauri-apps/api notices, Vite and Rolldown notices
for build-generated helpers, and the vendored Fluent icon license/provenance.
The application uses system Segoe UI/Arial fonts; no font files are bundled.
See DESKTOP-REVIEW.md for the static WebView2 SDK loader and separately installed
WebView2 Runtime / Visual C++ Runtime prerequisites. Neither Microsoft runtime
nor a Microsoft runtime installer/DLL is added to this folder by the collector.
"""


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=HERE.parents[2])
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--profile", choices=sorted(PROFILES), default="cli")
    parser.add_argument("--npm-cache", type=Path, help="npm cache used by npm ci; desktop only")
    parser.add_argument("--target", choices=[TARGET], default=TARGET)
    parser.add_argument("--cargo-metadata", type=Path, help="Previously generated target-filtered, locked original-ai metadata")
    parser.add_argument("--rust-sysroot", type=Path)
    parser.add_argument("--dependency-cache", type=Path)
    parser.add_argument("--offline", action="store_true", help="Add --offline to Cargo metadata resolution")
    args = parser.parse_args(argv)
    repo = args.repo_root.resolve()
    output = args.output_dir.resolve()
    require(not output.exists(), f"Output must not already exist: {output}")
    require(not output.is_relative_to(HERE), "Do not write generated notices into the collector source directory")
    profile = PROFILES[args.profile]
    cargo_policy = json.loads(read_file(HERE / profile["policy"]))
    retained_policy = json.loads(read_file(HERE / "reviewed-retained-notices.json"))
    command = ["cargo", "metadata", "--format-version", "1", "--manifest-path", str(repo / profile["manifest"]),
               "--locked", "--features", ",".join(profile["features"]), "--filter-platform", args.target]
    if args.offline:
        command.append("--offline")
    metadata = json.loads(read_file(args.cargo_metadata)) if args.cargo_metadata else json.loads(run(command, repo / "native"))
    compiler = run(["rustc", "--version", "--verbose"], repo / "native")
    required_rust = tomllib.loads(read_file(repo / "native/rust-toolchain.toml").decode("utf-8"))["toolchain"]["channel"]
    require(required_rust == REVIEWED_RUST_RELEASE, "Rust release changed; dependent-code notice review required")
    require(f"release: {required_rust}" in compiler.splitlines(), "Active rustc does not match the pinned toolchain")
    active_sysroot = Path(run(["rustc", "--print", "sysroot"], repo / "native")).resolve()
    sysroot = args.rust_sysroot.resolve() if args.rust_sysroot else active_sysroot
    require(sysroot == active_sysroot, "--rust-sysroot must be the active build compiler's sysroot")
    revision = run(["git", "rev-parse", "HEAD"], repo)
    require(bool(re.fullmatch(r"[0-9a-f]{40}", revision)), "Cannot identify exact source commit")
    dirty = bool(run(["git", "status", "--porcelain", "--untracked-files=normal"], repo))
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".license-stage-", dir=output.parent) as temp:
        stage = Path(temp) / "licenses"
        stage.mkdir()
        collector = Collector(stage)
        retained = collect_retained(collector, repo, retained_policy)
        cargo = collect_cargo(collector, repo, metadata, cargo_policy)
        frontend = []
        embedded_sdk = None
        if args.profile == "desktop":
            npm_cache = args.npm_cache or Path(run(["npm.cmd" if os.name == "nt" else "npm", "config", "get", "cache"], repo / "native/apps/singles-ui"))
            embedded_sdk = collect_desktop_sdk(collector, metadata, json.loads(read_file(HERE / "reviewed-desktop-sdk-notices.json")))
            frontend = collect_frontend(collector, repo, json.loads(read_file(HERE / "reviewed-desktop-frontend-notices.json")), npm_cache)
            for name in [profile["manifest"], profile["lock"], "native/apps/singles-ui/package.json", "native/apps/singles-ui/package-lock.json", "native/apps/singles-ui/vite.config.ts", "native/apps/singles-ui/src-tauri/build.rs"]:
                collector.copy(checked_under(repo, name), f"sources/build-inputs/{name}", name)
            collector.copy(HERE / "DESKTOP-REVIEW.md", "DESKTOP-REVIEW.md", "Desktop dependency/source and prerequisite review")
        native = collect_native(collector, repo, args.dependency_cache or default_cache())
        rust = collect_rust(collector, sysroot, compiler)
        collector.add("SOURCE-AVAILABILITY.md", (source_notice(revision, native) + (desktop_source_notice(cargo) if args.profile == "desktop" else "")).encode("utf-8"), "generated source-availability route")
        collector.copy(HERE / "REVIEW.md", "REVIEW.md", "reviewed primary-source references and scope")
        inventory = {"schema_version": 1, "target": args.target, "features": profile["features"], "profile": args.profile,
                     "source_revision": revision, "source_tree_dirty": dirty,
                     "cargo_lock_sha256": sha256(read_file(repo / profile["lock"])),
                     "cargo_metadata_command": [x if x != str(repo / profile["manifest"]) else profile["manifest"] for x in command],
                     "scope": cargo_policy["scope"], "cargo_packages": cargo,
                     "frontend_components": frontend, "embedded_sdk": embedded_sdk, "retained_components": retained, "native_sources": native, "rust": rust,
                     "files": sorted(collector.files, key=lambda f: f["path"])}
        (stage / "inventory.json").write_text(json.dumps(inventory, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        stage.rename(output)
    print(json.dumps({"licenses": str(output), "cargo_packages": len(cargo), "notice_and_source_files": len(collector.files)}))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CollectionError, OSError, ValueError, KeyError, subprocess.CalledProcessError, tarfile.TarError) as exc:
        print(f"License collection failed: {exc}", file=sys.stderr)
        if isinstance(exc, subprocess.CalledProcessError) and exc.stderr:
            print(exc.stderr, file=sys.stderr)
        raise SystemExit(1)
