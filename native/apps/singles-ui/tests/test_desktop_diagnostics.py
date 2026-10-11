import json
import os
import subprocess
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import Mock, patch
import zlib

import desktop_diagnostics as diagnostics
from owned_window_capture import png_from_bgra
from windows_integrity import require_medium
import run_desktop_medium as launcher


class DiagnosticContracts(unittest.TestCase):
    def test_medium_requires_effectively_restricted_rights_not_historical_flag(self):
        normal = {'integrity_rid': 8192, 'elevated': False, 'has_restrictions': False,
                  'power_groups': [], 'privileges': [{'name': 'SeChangeNotifyPrivilege', 'attributes': 3}]}
        require_medium(normal)
        filtered = dict(normal, elevated=True, has_restrictions=True,
                        power_groups=[{'sid': 'S-1-5-32-544', 'attributes': 0x10}])
        require_medium(filtered)
        for bad in [dict(filtered, integrity_rid=12288), dict(filtered, has_restrictions=False),
                    dict(filtered, power_groups=[{'sid': 'S-1-5-32-544', 'attributes': 4}]),
                    dict(filtered, power_groups=[{'sid': 'S-1-5-32-544', 'attributes': 0}]),
                    dict(filtered, power_groups=[{'sid': 'S-1-5-32-544', 'attributes': 0x14}]),
                    dict(filtered, privileges=[{'name': 'SeDebugPrivilege', 'attributes': 0}]),
                    dict(filtered, privileges=[{'name': 'SeDebugPrivilege', 'attributes': 2}]),
                    dict(filtered, privileges=[{'name': 'SeShutdownPrivilege', 'attributes': 2}])]:
            with self.assertRaises(RuntimeError):
                require_medium(bad)
        with self.assertRaises(KeyError):
            require_medium({'integrity_rid': 8192, 'elevated': False})

    def test_launcher_only_reduces_verified_high_integrity(self):
        expected = ['gsudo.exe', '--integrity', 'Medium', '--wait', '--direct', 'python.exe', 'desktop.py']
        self.assertEqual(launcher.command_for({'integrity_rid': 12288, 'elevated': True},
                                             'gsudo.exe', 'python.exe', 'desktop.py'), expected)
        self.assertEqual(launcher.command_for({'integrity_rid': 8192, 'elevated': False},
                                             'gsudo.exe', 'python.exe', 'desktop.py'), ['python.exe', 'desktop.py'])
        for rid, elevated in [(4096, False), (16384, True), (8192, True), (12288, False)]:
            with self.assertRaisesRegex(RuntimeError, 'refusing any privilege increase'):
                launcher.command_for({'integrity_rid': rid, 'elevated': elevated}, 'g', 'p', 's')

    def test_launcher_keeps_writes_in_owned_user_temp_and_copies_evidence(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'repo'
            local = Path(temp) / 'profile'
            executable = Path(temp) / 'gsudo.exe'
            executable.write_bytes(b'fixture')
            process = Mock(pid=99)
            process.wait.return_value = 7
            def spawn(command, cwd, env):
                self.assertEqual(cwd, root)
                evidence = Path(env['SINGLES_UI_EVIDENCE'])
                self.assertTrue(evidence.is_relative_to(local))
                self.assertFalse(evidence.is_relative_to(root))
                self.assertEqual(env['PYTHONDONTWRITEBYTECODE'], '1')
                evidence.mkdir(parents=True)
                (evidence / 'failure.json').write_text('actual failure')
                return process
            with patch.object(launcher, 'ROOT', root), \
                 patch.object(launcher, 'is_ci_windows', return_value=True), \
                 patch.object(launcher, 'process_security', return_value={'pid': 1, 'integrity_rid': 12288, 'elevated': True}), \
                 patch.dict(os.environ, {'LOCALAPPDATA': str(local), 'SINGLES_GSUDO_EXE': str(executable)}), \
                 patch.object(launcher.subprocess, 'Popen', side_effect=spawn):
                self.assertEqual(launcher.main(), 7)
            self.assertEqual((root / 'test-results/native-windows/failure.json').read_text(), 'actual failure')

    def test_preflight_uses_lightweight_script_and_sixty_second_bound(self):
        with tempfile.TemporaryDirectory() as temp:
            executable = Path(temp) / 'gsudo.exe'
            executable.write_bytes(b'fixture')
            process = Mock(pid=99)
            process.wait.return_value = 0
            with patch.object(launcher, 'ROOT', Path(temp) / 'repo'), \
                 patch.object(launcher, 'is_ci_windows', return_value=True), \
                 patch.object(launcher, 'process_security', return_value={'pid': 1, 'integrity_rid': 12288, 'elevated': True}), \
                 patch.dict(os.environ, {'LOCALAPPDATA': temp, 'SINGLES_GSUDO_EXE': str(executable)}), \
                 patch.object(launcher.subprocess, 'Popen', return_value=process) as launch:
                self.assertEqual(launcher.main(preflight=True), 0)
                self.assertTrue(launch.call_args.args[0][-1].endswith('probe_windows_token.py'))
                process.wait.assert_called_once_with(timeout=60)

    def test_launcher_timeout_reaps_only_owned_launcher_tree(self):
        with tempfile.TemporaryDirectory() as temp:
            executable = Path(temp) / 'gsudo.exe'
            executable.write_bytes(b'fixture')
            process = Mock(pid=99)
            process.wait.side_effect = [subprocess.TimeoutExpired('owned-launcher', 900), 1]
            with patch.object(launcher, 'ROOT', Path(temp) / 'repo'), \
                 patch.object(launcher, 'is_ci_windows', return_value=True), \
                 patch.object(launcher, 'process_security', return_value={'pid': 1, 'integrity_rid': 12288, 'elevated': True}), \
                 patch.dict(os.environ, {'LOCALAPPDATA': temp, 'SINGLES_GSUDO_EXE': str(executable)}), \
                 patch.object(launcher.subprocess, 'Popen', return_value=process), \
                 patch.object(launcher.subprocess, 'run') as cleanup:
                with self.assertRaises(subprocess.TimeoutExpired):
                    launcher.main()
                cleanup.assert_called_once_with(['taskkill', '/PID', '99', '/T', '/F'],
                                                capture_output=True, timeout=10, check=True)

    def test_png_preserves_pixel_order_and_ignores_uninitialized_alpha(self):
        png = png_from_bgra(2, 1, bytes([0, 0, 255, 0, 0, 255, 0, 0]))
        self.assertEqual(png[:8], b'\x89PNG\r\n\x1a\n')
        offset, chunks = 8, {}
        while offset < len(png):
            size = struct.unpack('!I', png[offset:offset + 4])[0]
            kind = png[offset + 4:offset + 8]
            data = png[offset + 8:offset + 8 + size]
            self.assertEqual(struct.unpack('!I', png[offset + 8 + size:offset + 12 + size])[0],
                             zlib.crc32(kind + data))
            chunks[kind] = data
            offset += size + 12
        self.assertEqual(zlib.decompress(chunks[b'IDAT']), b'\0\xff\0\0\0\xff\0')

    def test_snapshot_only_captures_exact_binary_roots(self):
        with tempfile.TemporaryDirectory() as temp:
            binary, output = Path(temp) / 'owned.exe', Path(temp)
            rows = [{'pid': 12, 'executable': str(binary)},
                    {'pid': 13, 'executable': 'msedgewebview2.exe'}]
            with patch.object(diagnostics, 'owned_processes', return_value=rows) as processes, \
                 patch.object(diagnostics, 'capture_owned_windows', return_value=[]) as capture:
                diagnostics.snapshot(binary, output, 'probe', root_pid=12)
                processes.assert_called_once_with(binary, 12)
                capture.assert_called_once_with(output, 'probe', [12])
            self.assertEqual(json.loads((output / 'probe.json').read_text())['kind'], 'startup-diagnostic-only')

    def test_snapshot_failure_is_preserved_without_acceptance_claim(self):
        with tempfile.TemporaryDirectory() as temp:
            with patch.object(diagnostics, 'owned_processes', side_effect=RuntimeError('probe unavailable')):
                diagnostics.snapshot(Path(temp) / 'owned.exe', Path(temp), 'probe')
            result = json.loads((Path(temp) / 'probe.json').read_text())
            self.assertEqual(result['diagnostic_error'], 'probe unavailable')
            self.assertNotIn('status', result)

    def test_startup_reaps_only_launched_pid_tree_and_checks_observed_identities(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp)
            binary = output / 'owned.exe'
            binary.write_bytes(b'fixture')
            child = Mock(pid=42, returncode=0)
            child.poll.return_value = None
            observed = [{'pid': 43, 'created': 'stable-process-identity'}]
            def terminate(command, **kwargs):
                self.assertEqual(command, ['taskkill', '/PID', '42', '/T', '/F'])
                self.assertEqual(kwargs['timeout'], 10)
                child.poll.return_value = 0
            with patch.object(diagnostics.subprocess, 'Popen', return_value=child), \
                 patch.object(diagnostics.subprocess, 'run', side_effect=terminate), \
                 patch.object(diagnostics.time, 'monotonic', side_effect=[0, 9]), \
                 patch.object(diagnostics, 'snapshot', return_value={'processes': observed}), \
                 patch.object(diagnostics, 'owned_processes', return_value=[]) as residual:
                diagnostics.startup_probe(binary, output)
                residual.assert_called_once_with(binary, 42, observed)
            self.assertEqual(json.loads((output / 'startup-probe.json').read_text())['residual_processes_after_cleanup'], [])

    def test_early_exit_is_recorded_without_killing_other_processes(self):
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'owned.exe'
            binary.write_bytes(b'fixture')
            child = Mock(pid=42, returncode=101)
            child.poll.return_value = 101
            with patch.object(diagnostics.subprocess, 'Popen', return_value=child), \
                 patch.object(diagnostics.subprocess, 'run') as terminate, \
                 patch.object(diagnostics, 'snapshot', return_value={'processes': []}), \
                 patch.object(diagnostics, 'owned_processes', return_value=[]):
                diagnostics.startup_probe(binary, Path(temp))
                terminate.assert_not_called()
            self.assertEqual(json.loads((Path(temp) / 'startup-probe.json').read_text())['exit_code_before_cleanup'], 101)

    def test_residual_descendant_blocks_webdriver_preflight_completion(self):
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'owned.exe'
            binary.write_bytes(b'fixture')
            child = Mock(pid=42, returncode=101)
            child.poll.return_value = 101
            orphan = {'pid': 43, 'created': 'original-identity'}
            with patch.object(diagnostics.subprocess, 'Popen', return_value=child), \
                 patch.object(diagnostics, 'snapshot', return_value={'processes': [orphan]}), \
                 patch.object(diagnostics, 'owned_processes', return_value=[orphan]):
                with self.assertRaisesRegex(RuntimeError, 'survived cleanup'):
                    diagnostics.startup_probe(binary, Path(temp))
            self.assertEqual(json.loads((Path(temp) / 'startup-probe.json').read_text())['residual_processes_after_cleanup'], [orphan])


if __name__ == '__main__':
    unittest.main()
