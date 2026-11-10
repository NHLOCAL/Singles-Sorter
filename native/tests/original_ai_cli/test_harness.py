"""Cheap harness-negative tests; no original model assets are required."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from check_cli import Gate, authored_wav, manifest, require


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        binary = self.root / "not-invoked"
        binary.write_bytes(b"owned executable placeholder")
        self.gate = Gate(binary, self.root, self.root)
        self.case, _ = self.gate.tree("case", [])
        self.request = self.gate.request(self.case)
        self.success = {"schema_version": 2, "status": "ok", "error": None,
                        "result": {"executable": False, "inference_executed": False,
                                   "plan": {"executable": False}, "blockers": []}}

    def process(self, payload, code=0):
        return subprocess.CompletedProcess([], code, (json.dumps(payload) + "\n").encode(),
                                           b"failure\n" if code else b"")

    def test_authored_audio_round_trips_hebrew_id3(self):
        path = self.case / "מקור/בדיקה.wav"
        authored_wav(path, "ישראל כהן", "כותרת לדוגמה")
        self.assertEqual(path.read_bytes()[:4], b"RIFF")

    def test_manifest_detects_content_without_atime_dependency(self):
        file = self.case / "יעד/existing-sentinel.bin"
        before = manifest(self.case)
        file.read_bytes()
        self.assertEqual(manifest(self.case), before)
        file.write_bytes(b"changed")
        self.assertNotEqual(manifest(self.case), before)

    def test_invocation_requires_exact_inference_boolean(self):
        response = copy.deepcopy(self.success)
        response["result"]["inference_executed"] = True
        with patch("check_cli.subprocess.run", return_value=self.process(response)):
            with self.assertRaises(AssertionError):
                self.gate.invoke(self.case, self.request, inferred=False)

    def test_fatal_response_cannot_carry_partial_plan(self):
        response = copy.deepcopy(self.success)
        response.update(status="error", error={"code": "PLANNING_RESOURCE_LIMIT"})
        response["result"]["blockers"] = ["PLANNING_RESOURCE_LIMIT"]
        with patch("check_cli.subprocess.run", return_value=self.process(response, 1)):
            with self.assertRaises(AssertionError):
                self.gate.invoke(self.case, self.request, success=False)

    def test_success_is_not_accepted_as_expected_failure(self):
        with patch("check_cli.subprocess.run", return_value=self.process(self.success)):
            with self.assertRaises(AssertionError):
                self.gate.invoke(self.case, self.request, success=False)

    def test_any_source_target_or_preference_mutation_is_rejected(self):
        def mutated(*args, **kwargs):
            (self.case / "preferences/settings.json").write_bytes(b"changed")
            return self.process(self.success)
        with patch("check_cli.subprocess.run", side_effect=mutated):
            with self.assertRaises(AssertionError):
                self.gate.invoke(self.case, self.request)

    def test_guard_remains_active_under_optimized_python(self):
        with self.assertRaises(AssertionError):
            require(False, "unconditional gate")


if __name__ == "__main__":
    unittest.main()
