"""Checkpoint diagnostics cannot manufacture acceptance or extend inner waits."""
import json
from pathlib import Path
import tempfile
import unittest

from desktop_progress import DesktopProgress


class DesktopProgressTests(unittest.TestCase):
    def test_phase_case_action_and_wait_are_retained_with_exact_binding(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'runtime-progress.json'; now = [100.0]
            binding = dict(binary_sha256='a' * 64, package_sha256sums_sha256='b' * 64, source_commit='c' * 40)
            progress = DesktopProgress(path, binding, clock=lambda: now[0])
            progress.phase('mp4-repair')
            now[0] += 12
            progress.action('fill', '#source', case='owned/opaque-structure')
            progress.waiting(120)
            actual = json.loads(path.read_text())
            self.assertEqual(actual['binding'], binding)
            self.assertEqual(actual['status'], 'running')
            self.assertEqual(actual['active_phase'], 'mp4-repair')
            self.assertEqual(actual['active_case'], 'owned/opaque-structure')
            self.assertEqual(actual['last_action']['detail'], '#source')
            self.assertEqual(actual['active_wait'], dict(timeout_seconds=120, started_seconds=12))
            self.assertFalse((Path(tmp) / 'runtime-progress.pending').exists())
            self.assertFalse((Path(tmp) / 'acceptance.json').exists())

    def test_only_returned_phases_and_explicit_final_success_are_completed(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'progress.json'; now = [0.0]
            progress = DesktopProgress(path, {}, clock=lambda: now[0])
            progress.phase('first'); now[0] = 15
            progress.phase('second')
            self.assertEqual(progress.state['completed_phases'], [dict(phase='first', elapsed_seconds=15)])
            self.assertEqual(progress.state['status'], 'running')
            progress.waiting(45); progress.waited()
            self.assertIsNone(progress.state['active_wait'])
            now[0] = 20; progress.finish(True)
            actual = json.loads(path.read_text())
            self.assertEqual(actual['status'], 'passed')
            self.assertIsNone(actual['active_phase'])
            self.assertEqual(actual['completed_phases'][-1], dict(phase='second', elapsed_seconds=5))

    def test_failure_preserves_active_phase_and_bounded_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'progress.json'
            progress = DesktopProgress(path, {})
            progress.phase('restart'); progress.action('native-picker', '#recovery-target-picker', case='root-move')
            progress.finish(False, 'x' * 2000)
            actual = json.loads(path.read_text())
            self.assertEqual(actual['status'], 'failed')
            self.assertEqual(actual['active_phase'], 'restart')
            self.assertEqual(actual['active_case'], 'root-move')
            self.assertEqual(actual['completed_phases'], [])
            self.assertEqual(len(actual['error']), 1000)

    def test_runtime_records_phase_boundaries_without_changing_operation_limits(self):
        root = Path(__file__).parent
        source = (root / 'desktop.py').read_text(encoding='utf-8')
        for phase in ['mp3-repair','flac-repair','ogg-repair','mp4-repair','id3-container-repair','wavpack-repair',
                      'same-volume-albums','file-album-copy-restart','cross-volume-albums-and-restart',
                      'single-copied-and-fix-restart','final-package-verification']:
            self.assertIn("progress.phase('" + phase + "')", source)
        self.assertIn("progress.phase('whole-album-' + name + '-restart')", source)
        self.assertIn('def call(method, path, body=None, *, timeout=30):', source)
        self.assertIn('def wait(predicate, timeout=45):', source)
        self.assertIn('timeout=120)', source)
        self.assertLess(source.index("(OUTPUT / 'acceptance.json').write_text"), source.index('progress.finish(True)'))
        self.assertIn("progress.action('restart', label, case=label)", source)
        self.assertIn("case=value if selector=='#source' else None", source)
        picker = (root / 'native_picker.py').read_text(encoding='utf-8')
        self.assertIn('timeout=75', picker)
        workflow = (root.parents[3] / '.github/workflows/native-ui-windows.yml').read_text(encoding='utf-8')
        self.assertIn('timeout-minutes: 90', workflow)
        self.assertEqual(workflow.count('SINGLES_DESKTOP_RECOVERY_EXPECTED:'), 2)
        self.assertIn('original-staged/acceptance.json', workflow)
        self.assertIn('original-extracted/acceptance.json', workflow)


if __name__ == '__main__':
    unittest.main()
