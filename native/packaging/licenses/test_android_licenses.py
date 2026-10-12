"""Portable fail-closed Android notice and APK snapshot contract tests."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import collect_android_licenses as android
import collect_licenses as common


class AndroidNoticeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.data = b"<!doctype html><title>Notices</title>"
        self.row = {"path": "index.html", "bytes": len(self.data), "sha256": common.sha256(self.data)}
        (self.root / "index.html").write_bytes(self.data)
        self.write_inventory([self.row])

    def write_inventory(self, files):
        (self.root / "inventory.json").write_bytes(json.dumps({"schema_version": 1, "files": files}).encode())

    def test_snapshot_exact_and_immutable(self):
        result = dict(android.verified_snapshot(self.root))
        (self.root / "index.html").write_bytes(b"changed")
        self.assertEqual(result["index.html"], self.data)
        self.assertEqual(set(result), {"index.html", "inventory.json"})

    def test_unsafe_paths(self):
        for value in ("", "/x", "../x", "x/./y", "x//y", "x/", "a\\b", "C:x", "x?y", "a b", "א"):
            with self.subTest(value=value), self.assertRaises(common.CollectionError):
                android.validate_files([{**self.row, "path": value}])

    def test_duplicate_reserved_paths(self):
        for rows in ([self.row, self.row], [{**self.row, "path": "inventory.json"}]):
            with self.assertRaises(common.CollectionError):
                android.validate_files(rows)

    def test_invalid_sizes_hashes(self):
        for value in (True, 0, -1, "1", android.MAX_FILE + 1):
            with self.subTest(value=value), self.assertRaises(common.CollectionError):
                android.validate_files([{**self.row, "bytes": value}])
        for value in (None, "a" * 63, "A" * 64, 1):
            with self.subTest(value=value), self.assertRaises(common.CollectionError):
                android.validate_files([{**self.row, "sha256": value}])

    def test_total_and_count_bounds(self):
        with patch.object(android, "MAX_TOTAL", 1), self.assertRaises(common.CollectionError):
            android.validate_files([self.row])
        with patch.object(android, "MAX_FILES", 0), self.assertRaises(common.CollectionError):
            android.validate_files([self.row])

    def test_altered_and_extra_files(self):
        (self.root / "index.html").write_bytes(b"x" * len(self.data))
        with self.assertRaises(common.CollectionError): android.verified_snapshot(self.root)
        (self.root / "index.html").write_bytes(self.data)
        (self.root / "extra").write_bytes(b"extra")
        with self.assertRaises(common.CollectionError): android.verified_snapshot(self.root)

    def test_missing_index_and_file(self):
        self.write_inventory([])
        with self.assertRaises(common.CollectionError): android.verified_snapshot(self.root)
        self.write_inventory([self.row])
        (self.root / "index.html").unlink()
        with self.assertRaises(common.CollectionError): android.verified_snapshot(self.root)

    def test_inventory_duplicate_keys_and_type(self):
        for data in (b'{"schema_version":1,"schema_version":1,"files":[]}',
                     b'{"schema_version":true,"files":[]}', b'{"schema_version":2,"files":[]}'):
            (self.root / "inventory.json").write_bytes(data)
            with self.assertRaises(common.CollectionError): android.verified_snapshot(self.root)

    def test_inventory_size_bound(self):
        with patch.object(android, "MAX_INVENTORY", 2), self.assertRaises(common.CollectionError):
            android.verified_snapshot(self.root)

    def test_symlinks_refused(self):
        target = self.root / "outside"
        target.write_bytes(self.data)
        link = self.root / "index.html"
        link.unlink()
        try: link.symlink_to(target)
        except OSError: self.skipTest("Host does not permit owned symlink fixtures")
        with self.assertRaises(common.CollectionError): android.verified_snapshot(self.root)

    def test_index_is_local_and_escaped(self):
        page = android.index_page([{**self.row, "path": 'x<y".txt'}]).decode()
        self.assertIn("x&lt;y&quot;.txt", page)
        self.assertNotIn("<script", page)
        self.assertNotIn("https://", page)
        self.assertIn("default-src 'none'", page)

    def test_default_windows_feature_remains_required(self):
        metadata = {"packages": [{"id": "p", "name": "sorter-cli", "source": None, "version": "1"}],
                    "resolve": {"nodes": [{"id": "p", "features": ["product"], "deps": []}]}}
        with self.assertRaises(common.CollectionError): common.closure(metadata)
        self.assertEqual(len(common.closure(metadata, required_feature="product")[0]), 1)
        metadata["resolve"]["nodes"][0]["features"] = ["original-ai"]
        self.assertEqual(len(common.closure(metadata)[0]), 1)
        with self.assertRaises(common.CollectionError): common.closure(metadata, required_feature="product")

    def test_policy_targets_and_supplement_pins(self):
        for target in android.TARGETS:
            policy = json.loads((android.HERE / f"android-cargo-{target}.json").read_bytes())
            self.assertEqual(policy["target"], target)
            self.assertEqual(policy["required_root_feature"], "product")
            self.assertEqual(policy["root_package"], "singles-android-inference")
        for item in json.loads((android.HERE / "android-supplements.json").read_bytes()):
            self.assertEqual(common.sha256((android.HERE / item["path"]).read_bytes()), item["sha256"])
        cesu = android.HERE / "android-upstream/cesu8-1.1.0"
        self.assertEqual((cesu / "COPYRIGHT").read_bytes(), (cesu / "COPYRIGHT-RUST.txt").read_bytes())


if __name__ == "__main__": unittest.main()
