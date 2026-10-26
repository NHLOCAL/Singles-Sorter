"""Portable diagnostics orchestration tests; these do not simulate Rust success."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from capture_rust_format import collect, SCOPES


class FormattingDiagnostics(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.files = []
        for name, manifest in SCOPES:
            path = self.root / Path(manifest).parent / f"{name}.rs"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"fn original() {}\r\n")
            self.files.append(path)
        self.git("init", "-q")
        self.git("add", ".")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "commit", "-qm", "Owned formatter orchestration fixture")
        self.out = self.root / "diagnostics"
        self.calls = []

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root)

    def runner(self, codes):
        def run(command, root):
            self.calls.append(command)
            index = [manifest for _, manifest in SCOPES].index(command[4])
            if command[-1] == "--check":
                code = codes[index]
            else:
                self.files[index].write_bytes(b"fn corrected() {}\n")
                code = 17 if index == 0 else 0
            return {"command": command, "code": code, "output": f"scope {index}"}
        return run

    def test_all_scopes_collected_after_first_failure_and_bytes_restored(self):
        self.assertEqual(collect(self.root, self.out, self.runner([1, 1, 1])), 1)
        self.assertEqual(len(self.calls), 6)
        patch = (self.out / "native-formatting.patch").read_text()
        for name, _ in SCOPES:
            self.assertIn(f"{name}.rs", patch)
        for path in self.files:
            self.assertEqual(path.read_bytes(), b"fn original() {}\r\n")
        report = json.loads((self.out / "commands.json").read_text())
        self.assertTrue(report["source_restored"])
        self.assertEqual([item["code"] for item in report["corrections"]], [17, 0, 0])
        self.assertEqual(self.git("diff", "--binary"), b"")

    def test_nested_only_failure_still_collects_every_scope(self):
        self.assertEqual(collect(self.root, self.out, self.runner([0, 1, 0])), 1)
        self.assertEqual(len(self.calls), 6)

    def test_clean_checks_do_not_rewrite_sources(self):
        self.assertEqual(collect(self.root, self.out, self.runner([0, 0, 0])), 0)
        self.assertEqual(len(self.calls), 3)
        self.assertEqual((self.out / "native-formatting.patch").read_bytes(), b"")

    def test_dirty_source_refused_without_overwriting(self):
        self.files[0].write_bytes(b"user change\n")
        with self.assertRaisesRegex(RuntimeError, "clean tracked"):
            collect(self.root, self.out, self.runner([1, 1, 1]))
        self.assertEqual(self.calls, [])
        self.assertEqual(self.files[0].read_bytes(), b"user change\n")

    def test_exception_restores_prior_formatter_changes(self):
        run = self.runner([1, 1, 1])
        def interrupted(command, root):
            result = run(command, root)
            if len(self.calls) == 5:
                raise RuntimeError("owned simulated formatter interruption")
            return result
        with self.assertRaisesRegex(RuntimeError, "interruption"):
            collect(self.root, self.out, interrupted)
        for path in self.files:
            self.assertEqual(path.read_bytes(), b"fn original() {}\r\n")
        self.assertTrue(json.loads((self.out / "commands.json").read_text())["source_restored"])
        self.assertIn("corrected", (self.out / "native-formatting.patch").read_text())

    def test_dirty_lock_refused_without_restoring_or_formatting(self):
        lock = self.root / "native/Cargo.lock"
        lock.write_text("original lock\n")
        self.git("add", ".")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "commit", "-qm", "Owned lock fixture")
        lock.write_text("unrelated lock edit\n")
        with self.assertRaisesRegex(RuntimeError, "clean tracked"):
            collect(self.root, self.out, self.runner([1, 1, 1]))
        self.assertEqual(self.calls, [])
        self.assertEqual(lock.read_text(), "unrelated lock edit\n")

    def test_untracked_file_is_preserved(self):
        extra = self.root / "native/private-notes.rs"
        extra.write_bytes(b"owned untracked data\n")
        self.assertEqual(collect(self.root, self.out, self.runner([1, 1, 1])), 1)
        self.assertEqual(extra.read_bytes(), b"owned untracked data\n")
        self.assertNotIn("private-notes", (self.out / "native-formatting.patch").read_text())


if __name__ == "__main__":
    unittest.main()
