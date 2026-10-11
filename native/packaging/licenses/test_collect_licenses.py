"""Offline negative and positive checks for the bounded notice collector."""
import base64
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("collect_licenses", Path(__file__).with_name("collect_licenses.py"))
collector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(collector)


class LicenseCollectorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.output = self.root / "out"
        self.output.mkdir()
        self.writer = collector.Collector(self.output)

    def write(self, path, data=b"full notice\n"):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def archive(self, path, entries):
        path.parent.mkdir(parents=True, exist_ok=True)
        with tarfile.open(path, "w:gz") as tf:
            for name, data in entries.items():
                info = tarfile.TarInfo(name)
                info.size = len(data)
                tf.addfile(info, io.BytesIO(data))
        return path

    def fixture(self):
        local = self.write(self.repo / "native/crates/sorter-cli/Cargo.toml")
        source = self.root / "registry/src/example/demo-1.0.0"
        manifest = self.write(source / "Cargo.toml")
        notice = b"MIT License\nCopyright example\nFull fixture notice\n"
        self.write(source / "LICENSE", notice)
        archive = self.archive(self.root / "registry/cache/example/demo-1.0.0.crate", {"demo-1.0.0/LICENSE": notice})
        digest = collector.sha256(archive.read_bytes())
        registry = "registry+https://github.com/rust-lang/crates.io-index"
        rule = {"name": "demo", "version": "1.0.0", "license": "MIT", "source": registry,
                "checksum": digest, "notices": {"LICENSE": collector.sha256(notice)}}
        policy = {"packages": [rule], "local_packages": [{"name": "sorter-cli", "version": "0.1.0",
                  "manifest_path": "native/crates/sorter-cli/Cargo.toml"}]}
        metadata = {"packages": [
            {"id": "cli", "name": "sorter-cli", "version": "0.1.0", "source": None, "manifest_path": str(local)},
            {"id": "demo", "name": "demo", "version": "1.0.0", "source": registry, "license": "MIT", "manifest_path": str(manifest)},
            {"id": "test", "name": "only-test", "version": "1.0.0", "source": registry}],
            "resolve": {"nodes": [
                {"id": "cli", "features": ["original-ai"], "deps": [
                    {"pkg": "demo", "dep_kinds": [{"kind": None}]},
                    {"pkg": "test", "dep_kinds": [{"kind": "dev"}]}]},
                {"id": "demo", "features": [], "deps": []},
                {"id": "test", "features": [], "deps": []}]}}
        lock = f'[[package]]\nname = "demo"\nversion = "1.0.0"\nsource = "{registry}"\nchecksum = "{digest}"\n'
        self.write(self.repo / "native/Cargo.lock", lock.encode())
        return metadata, policy, source, archive

    def test_normal_and_build_closure_excludes_dev(self):
        metadata, policy, _, _ = self.fixture()
        rows = collector.collect_cargo(self.writer, self.repo, metadata, policy)
        self.assertEqual([r["name"] for r in rows], ["demo", "sorter-cli"])
        self.assertTrue((self.output / "cargo/demo-1.0.0/LICENSE").is_file())
        metadata["resolve"]["nodes"][0]["deps"][0]["dep_kinds"] = [{"kind": "build"}]
        self.assertEqual(len(collector.closure(metadata)[0]), 2)

    def test_original_ai_required(self):
        metadata, _, _, _ = self.fixture()
        metadata["resolve"]["nodes"][0]["features"] = []
        with self.assertRaisesRegex(collector.CollectionError, "lacks"):
            collector.closure(metadata)

    def test_changed_closure_rejected(self):
        metadata, policy, _, _ = self.fixture()
        metadata["resolve"]["nodes"][0]["deps"][1]["dep_kinds"] = [{"kind": None}]
        with self.assertRaisesRegex(collector.CollectionError, "Unreviewed dependency closure"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_missing_notice_rejected(self):
        metadata, policy, source, _ = self.fixture()
        (source / "LICENSE").unlink()
        with self.assertRaisesRegex(collector.CollectionError, "Missing"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_notice_modification_rejected(self):
        metadata, policy, source, _ = self.fixture()
        (source / "LICENSE").write_bytes(b"See upstream for license")
        with self.assertRaisesRegex(collector.CollectionError, "Altered registry notice"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_archive_modification_rejected(self):
        metadata, policy, _, archive = self.fixture()
        archive.write_bytes(archive.read_bytes() + b"changed")
        with self.assertRaisesRegex(collector.CollectionError, "archive checksum mismatch"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_license_change_rejected(self):
        metadata, policy, _, _ = self.fixture()
        metadata["packages"][1]["license"] = "Unknown"
        with self.assertRaisesRegex(collector.CollectionError, "Unreviewed registry/license"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_path_escape_rejected(self):
        for path in ("../secret", "/absolute", "C:/Windows", "a\\b"):
            with self.subTest(path=path), self.assertRaises(collector.CollectionError):
                self.writer.add(path, b"text", "test")

    def test_archive_symlink_member_rejected(self):
        path = self.root / "unsafe.tar.gz"
        with tarfile.open(path, "w:gz") as tf:
            item = tarfile.TarInfo("LICENSE")
            item.type = tarfile.SYMTYPE
            item.linkname = "../../secret"
            tf.addfile(item)
        with self.assertRaisesRegex(collector.CollectionError, "unsafe archive member"):
            collector.archive_member(path, "LICENSE")

    def test_retained_crlf_preserved_but_drift_rejected(self):
        self.write(self.repo / "NOTICE", b"Copyright\r\nFull terms\r\n")
        policy = {"components": [{"component": "fixture", "license_summary": "MIT", "notices": {
            "NOTICE": collector.sha256(b"Copyright\nFull terms\n")}}]}
        collector.collect_retained(self.writer, self.repo, policy)
        self.assertEqual((self.output / "retained/NOTICE").read_bytes(), b"Copyright\r\nFull terms\r\n")
        self.write(self.repo / "NOTICE", b"short pointer")
        with self.assertRaisesRegex(collector.CollectionError, "needs review"):
            collector.collect_retained(self.writer, self.repo, policy)

    def test_full_rust_notices_required(self):
        sysroot = self.root / "rust"
        docs = sysroot / "share/doc/rust"
        for name in ["COPYRIGHT.html", "COPYRIGHT-library.html"] + [f"licenses/{n}" for n in collector.RUST_LICENSE_FILES]:
            self.write(docs / name, f"Complete fixture: {name}".encode())
        row = collector.collect_rust(self.writer, sysroot, "rustc fixture")
        self.assertEqual(len(row["notices"]), 2 + len(collector.RUST_LICENSE_FILES))
        (docs / "COPYRIGHT-library.html").unlink()
        other = self.root / "other"
        other.mkdir()
        with self.assertRaisesRegex(collector.CollectionError, "Missing"):
            collector.collect_rust(collector.Collector(other), sysroot, "rustc fixture")

    def test_missing_rust_dependent_code_license_rejected(self):
        sysroot = self.root / "rust"
        docs = sysroot / "share/doc/rust"
        for name in ["COPYRIGHT.html", "COPYRIGHT-library.html"] + [f"licenses/{n}" for n in collector.RUST_LICENSE_FILES]:
            self.write(docs / name)
        (docs / "licenses/LLVM-exception.txt").unlink()
        with self.assertRaisesRegex(collector.CollectionError, "missing reviewed complete license"):
            collector.collect_rust(self.writer, sysroot, "rustc fixture")

    def test_native_archive_must_match_pin(self):
        lock = {"dependencies": {name: {"version": p[0], "url": p[1], "archive_sha256": p[2]}
                                 for name, p in collector.NATIVE_PINS.items()}}
        self.write(self.repo / "native/crates/sorter-io/dependencies/lock.json", json.dumps(lock).encode())
        cache = self.root / "cache"
        self.write(cache / "taglib-2.3.2.tar.gz", b"incorrect archive")
        with self.assertRaisesRegex(collector.CollectionError, "Native source archive checksum mismatch"):
            collector.collect_native(self.writer, self.repo, cache)

    def test_source_notice_is_explicit_and_not_url_only(self):
        text = collector.source_notice("a" * 40, [{"included_source": "sources/taglib-2.3.2.tar.gz", "archive_sha256": "b" * 64}])
        for required in ("MPL-1.1", "same media", "sources/taglib-2.3.2.tar.gz", "dated change", "no source patches", "COPYRIGHT-library.html"):
            self.assertIn(required, text)
        self.assertNotIn("fully compliant", text)


    def desktop_fixture(self):
        metadata, policy, source, archive = self.fixture()
        root = metadata["packages"][0]
        root["name"] = "singles-ui-host"
        root["manifest_path"] = str(self.write(self.repo / "desktop/Cargo.toml"))
        metadata["resolve"]["nodes"][0]["features"] = ["desktop", "original-ai"]
        policy["local_packages"][0].update(name="singles-ui-host", manifest_path="desktop/Cargo.toml")
        lock = (self.repo / "native/Cargo.lock").read_bytes()
        self.write(self.repo / "desktop/Cargo.lock", lock)
        policy.update(root_package="singles-ui-host", lock_path="desktop/Cargo.lock",
                      lock_sha256_lf=collector.sha256(lock),
                      graph=collector.cargo_graph(*collector.closure(metadata, "singles-ui-host")))
        return metadata, policy, source, archive

    def test_desktop_root_is_separate_and_feature_graph_locked(self):
        metadata, policy, _, _ = self.desktop_fixture()
        rows = collector.collect_cargo(self.writer, self.repo, metadata, policy)
        self.assertEqual([r["name"] for r in rows], ["demo", "singles-ui-host"])
        with self.assertRaisesRegex(collector.CollectionError, "local sorter-cli"):
            collector.closure(metadata)
        metadata["resolve"]["nodes"][0]["features"].append("new-feature")
        with self.assertRaisesRegex(collector.CollectionError, "feature/dependency graph changed"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_desktop_dependency_kind_drift_rejected(self):
        metadata, policy, _, _ = self.desktop_fixture()
        metadata["resolve"]["nodes"][0]["deps"][0]["dep_kinds"][0]["kind"] = "build"
        with self.assertRaisesRegex(collector.CollectionError, "feature/dependency graph changed"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_desktop_exact_lock_rejects_unrelated_drift(self):
        metadata, policy, _, _ = self.desktop_fixture()
        with (self.repo / "desktop/Cargo.lock").open("a") as f:
            f.write("# unrelated lock change\n")
        with self.assertRaisesRegex(collector.CollectionError, "Cargo.lock changed"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_desktop_mpl_source_is_exact_archive(self):
        metadata, policy, _, archive = self.desktop_fixture()
        policy["packages"][0].update(include_source=True, license="MPL-2.0")
        metadata["packages"][1]["license"] = "MPL-2.0"
        source = Path(metadata["packages"][1]["manifest_path"]).parent
        self.archive(archive, {"demo-1.0.0/LICENSE": (source / "LICENSE").read_bytes(),
                               "demo-1.0.0/Cargo.toml": (source / "Cargo.toml").read_bytes()})
        old = policy["packages"][0]["checksum"]
        digest = collector.sha256(archive.read_bytes())
        policy["packages"][0]["checksum"] = digest
        lock = self.repo / "desktop/Cargo.lock"
        lock.write_text(lock.read_text().replace(old, digest))
        policy["lock_sha256_lf"] = collector.sha256(lock.read_bytes())
        rows = collector.collect_cargo(self.writer, self.repo, metadata, policy)
        self.assertEqual((self.output / rows[0]["included_source"]).read_bytes(), archive.read_bytes())
        self.assertEqual(rows[0]["source_license"], "MPL-2.0")

    def frontend_fixture(self):
        root = self.repo / "frontend"
        source = root / "node_modules/demo"
        notice = b"Full original upstream license text\n"
        package = {"name": "demo", "version": "1.0.0"}
        self.write(source / "package.json", json.dumps(package).encode())
        self.write(source / "LICENSE", notice)
        archive = self.archive(self.root / "demo.tgz", {"package/LICENSE": notice})
        data = archive.read_bytes()
        digest = hashlib.sha512(data).hexdigest()
        integrity = "sha512-" + base64.b64encode(hashlib.sha512(data).digest()).decode()
        cache = self.root / "npm-cache"
        cache_path = self.write(cache / "_cacache/content-v2/sha512" / digest[:2] / digest[2:4] / digest[4:], data)
        rule = {"name": "demo", "version": "1.0.0", "lock_path": "node_modules/demo", "license": "MIT",
                "resolved": "https://registry.npmjs.org/demo/-/demo-1.0.0.tgz", "integrity": integrity,
                "scope": "embedded frontend", "notices": {"LICENSE": collector.sha256(notice)}}
        lock = {"lockfileVersion": 3, "packages": {"": {}, "node_modules/demo": rule}}
        lock_data = json.dumps(lock).encode()
        self.write(root / "package-lock.json", lock_data)
        manifest = self.write(root / "package.json", b'{"dependencies":{"demo":"1.0.0"}}')
        asset = self.write(root / "public/icon.svg", b"<svg/>\n")
        policy = {"root": "frontend", "lock_sha256_lf": collector.sha256(lock_data),
                  "manifest_sha256_lf": collector.sha256(manifest.read_bytes()),
                  "production_packages": ["node_modules/demo"], "packages": [rule],
                  "asset_directories": ["public"], "assets": {"public/icon.svg": collector.sha256(asset.read_bytes())},
                  "retained_notices": []}
        return policy, cache, cache_path, source

    def test_mpl_modified_source_rejected(self):
        source = self.root / "source"
        self.write(source / "src/lib.rs", b"modified")
        archive = self.archive(self.root / "covered.crate", {"covered-1/src/lib.rs": b"original"})
        with self.assertRaisesRegex(collector.CollectionError, "Modified MPL source"):
            collector.verify_unmodified_source(archive, source, "covered-1")

    def test_frontend_exact_archive_and_assets_succeed(self):
        policy, cache, _, _ = self.frontend_fixture()
        rows = collector.collect_frontend(self.writer, self.repo, policy, cache)
        self.assertEqual(rows[0]["component"], "demo")
        self.assertEqual((self.output / rows[0]["notices"][0]).read_bytes(), b"Full original upstream license text\n")

    def test_frontend_corrupt_archive_rejected(self):
        policy, cache, archive, _ = self.frontend_fixture()
        archive.write_bytes(archive.read_bytes() + b"drift")
        with self.assertRaisesRegex(collector.CollectionError, "integrity mismatch"):
            collector.collect_frontend(self.writer, self.repo, policy, cache)

    def test_frontend_missing_archive_rejected(self):
        policy, cache, archive, _ = self.frontend_fixture()
        archive.unlink()
        with self.assertRaisesRegex(collector.CollectionError, "Missing"):
            collector.collect_frontend(self.writer, self.repo, policy, cache)

    def test_frontend_changed_notice_rejected(self):
        policy, cache, _, source = self.frontend_fixture()
        (source / "LICENSE").write_bytes(b"Generic license label")
        with self.assertRaisesRegex(collector.CollectionError, "Installed npm notice changed"):
            collector.collect_frontend(self.writer, self.repo, policy, cache)

    def test_frontend_unreviewed_asset_rejected(self):
        policy, cache, _, _ = self.frontend_fixture()
        self.write(self.repo / "frontend/public/new-font.woff2", b"new font")
        with self.assertRaisesRegex(collector.CollectionError, "Unreviewed frontend asset set"):
            collector.collect_frontend(self.writer, self.repo, policy, cache)

    def test_frontend_lock_drift_rejected(self):
        policy, cache, _, _ = self.frontend_fixture()
        (self.repo / "frontend/package-lock.json").write_text("{}")
        with self.assertRaisesRegex(collector.CollectionError, "frontend lock changed"):
            collector.collect_frontend(self.writer, self.repo, policy, cache)

    def test_upstream_notice_requires_exact_archive_revision(self):
        metadata, policy, source, archive = self.desktop_fixture()
        data = {"demo-1.0.0/LICENSE": (source / "LICENSE").read_bytes(),
                "demo-1.0.0/.cargo_vcs_info.json": b'{"git":{"sha1":"original"}}'}
        self.archive(archive, data)
        digest = collector.sha256(archive.read_bytes())
        p = self.repo / "desktop/Cargo.lock"
        old = policy["packages"][0]["checksum"]
        p.write_text(p.read_text().replace(old, digest))
        policy["lock_sha256_lf"] = collector.sha256(p.read_bytes())
        policy["packages"][0].update(checksum=digest, upstream_notices=[{"vcs_revision": "different"}])
        with self.assertRaisesRegex(collector.CollectionError, "revision mismatch"):
            collector.collect_cargo(self.writer, self.repo, metadata, policy)

    def test_sdk_loader_must_match_exact_official_source(self):
        source = self.root / "wrapper"
        self.write(source / "Cargo.toml")
        self.write(source / "x64/loader.lib", b"modified loader")
        metadata = {"packages": [{"name": "wrapper", "version": "1", "manifest_path": str(source / "Cargo.toml")}]}
        policy = {"wrapper_package": "wrapper", "wrapper_version": "1", "loader_path": "x64/loader.lib",
                  "loader_sha256": collector.sha256(b"original loader")}
        with self.assertRaisesRegex(collector.CollectionError, "differs from exact official SDK"):
            collector.collect_desktop_sdk(self.writer, metadata, policy)

    def test_desktop_source_notice_separates_runtime_and_sdk(self):
        text = collector.desktop_source_notice([{"name": "covered", "version": "1", "included_source": "sources/cargo/covered-1.crate",
                                               "source_license": "MPL-2.0", "crate_sha256": "f" * 64}])
        for expected in ("MPL-2.0", "sources/cargo/covered-1.crate", "same media", "static WebView2 SDK loader", "no font files"):
            self.assertIn(expected, text)


if __name__ == "__main__":
    unittest.main()
