"""Offline negative and positive checks for the bounded notice collector."""
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


if __name__ == "__main__":
    unittest.main()
