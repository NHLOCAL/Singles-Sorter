import json
import csv
import io
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
from native_picker import unique_folder_edit, unique_save_edit, interact_with_owned_picker, literal_keyboard_text, enter_owned_save_path
from native_fixtures import write_import_csv


class DiagnosticContracts(unittest.TestCase):
    def test_native_import_fixture_has_exact_two_utf8_crlf_records(self):
        expected_rows = [['אמן מיובא', 'תיקייה מיובאת'], ['אמן מיובא', 'תיקייה שנייה']]
        expected = ''.join(','.join(row) + '\r\n' for row in expected_rows).encode('utf-8')
        # Reproduce the former Windows text-mode corruption on every platform.
        buffer = io.BytesIO()
        writer = io.TextIOWrapper(buffer, encoding='utf-8', newline='\r\n')
        writer.write(expected.decode('utf-8'))
        writer.flush()
        self.assertIn(b'\r\r\n', buffer.getvalue())
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'אמנים.csv'
            write_import_csv(path)
            raw = path.read_bytes()
            self.assertEqual(raw, expected)
            self.assertNotIn(b'\r\r\n', raw)
            self.assertEqual(raw.count(b'\r\n'), 2)
            self.assertEqual(list(csv.reader(io.StringIO(raw.decode('utf-8'), newline=''))), expected_rows)

    def test_native_picker_com_interaction_has_process_timeout(self):
        with patch('native_picker.subprocess.run', return_value=Mock(returncode=0)) as run:
            interact_with_owned_picker(Path('owned.exe'), Path('fixture'), Path('evidence'), 'source')
            self.assertEqual(run.call_args.kwargs['timeout'], 75)
            self.assertEqual(run.call_args.args[0][-4:], ['owned.exe', 'fixture', 'evidence', 'source'])
        with patch('native_picker.subprocess.run', return_value=Mock(returncode=1, stderr='actual UIA failure')), \
             patch.object(Path, 'write_text'):
            with self.assertRaisesRegex(RuntimeError, 'actual UIA failure'):
                interact_with_owned_picker(Path('owned.exe'), Path('fixture'), Path('evidence'), 'source')

    def test_save_picker_uses_observed_owned_filename_edit(self):
        combo = {'pid': 42, 'control_type': 'ComboBox', 'automation_id': 'FileNameControlHost',
                 'visible': True, 'enabled': True, 'name': 'File name:'}
        edit = dict(combo, control_type='Edit', automation_id='1001')
        search = dict(edit, automation_id='SearchEditBox')
        self.assertEqual(unique_save_edit([search, combo, edit], 42), 2)
        self.assertEqual(unique_save_edit([dict(combo, name='שם קובץ:'), dict(edit, name='שם קובץ:')], 42), 1)
        for records in [[combo], [edit], [combo, edit, dict(edit)], [combo, dict(combo), edit],
                        [combo, dict(edit, pid=99)], [combo, dict(edit, visible=False)],
                        [combo, dict(edit, enabled=False)], [dict(combo, pid=99), edit],
                        [dict(combo, automation_id='1148'), edit], [combo, search]]:
            with self.assertRaises(RuntimeError):
                unique_save_edit(records, 42)

    def test_save_uses_focused_literal_keyboard_input_and_blur(self):
        value = r'D:\owned\רשימה +^%~{ENTER}(שם).csv'
        control = Mock()
        control.process_id.return_value = 42
        control.has_keyboard_focus.return_value = True
        control.get_value.return_value = value
        enter_owned_save_path(control, value, 42)
        self.assertEqual(control.method_calls, [
            unittest.mock.call.process_id(), unittest.mock.call.set_focus(),
            unittest.mock.call.has_keyboard_focus(),
            unittest.mock.call.type_keys('^a{BACKSPACE}', set_foreground=False),
            unittest.mock.call.type_keys(r'D:\owned\רשימה {+}{^}{%}{~}{{}ENTER{}}{(}שם{)}.csv',
                                        with_spaces=True, set_foreground=False, vk_packet=True),
            unittest.mock.call.get_value(),
            unittest.mock.call.type_keys('{TAB}', set_foreground=False),
            unittest.mock.call.get_value()])
        control.set_edit_text.assert_not_called()

    def test_save_refuses_foreign_or_unfocused_input_before_typing(self):
        for pid, focused in [(99, True), (42, False)]:
            control = Mock()
            control.process_id.return_value = pid
            control.has_keyboard_focus.return_value = focused
            with self.assertRaises(RuntimeError):
                enter_owned_save_path(control, r'D:\owned\new.csv', 42)
            control.type_keys.assert_not_called()

    def test_save_rechecks_path_before_and_after_blur(self):
        expected = r'D:\owned\new.csv'
        for values in [['default.csv'], [expected, 'default.csv']]:
            control = Mock()
            control.process_id.return_value = 42
            control.has_keyboard_focus.return_value = True
            control.get_value.side_effect = values
            with self.assertRaisesRegex(RuntimeError, 'Native Save input'):
                enter_owned_save_path(control, expected, 42)

    def test_save_path_never_interprets_literal_modifiers_as_commands(self):
        self.assertEqual(literal_keyboard_text('+^%~{}()'), '{+}{^}{%}{~}{{}{}}{(}{)}')
        self.assertEqual(literal_keyboard_text('שלום []\\a.csv'), 'שלום []\\a.csv')
        for control in ['\0', '\r', '\n', '\t', '\x7f']:
            with self.assertRaisesRegex(RuntimeError, 'control character'):
                literal_keyboard_text('new' + control + '.csv')

    def test_save_route_is_explicit_and_refuses_existing_or_missing_parent(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            destination = root / 'new.csv'
            with patch('native_picker.subprocess.run', return_value=Mock(returncode=0)) as run:
                interact_with_owned_picker(Path('owned.exe'), destination, root, 'export', save=True)
                self.assertEqual(run.call_args.args[0][-1], '--save')
                self.assertEqual(run.call_args.kwargs['timeout'], 75)
                destination.write_bytes(b'owned original')
                with self.assertRaisesRegex(RuntimeError, 'new CSV'):
                    interact_with_owned_picker(Path('owned.exe'), destination, root, 'export', save=True)
                with self.assertRaisesRegex(RuntimeError, 'new CSV'):
                    interact_with_owned_picker(Path('owned.exe'), root / 'missing' / 'new.csv', root, 'export', save=True)
                self.assertEqual(run.call_count, 1)
                self.assertEqual(destination.read_bytes(), b'owned original')

    def test_folder_picker_uses_observed_direct_edit_without_combo_parent(self):
        edit = {'pid': 42, 'control_type': 'Edit', 'automation_id': '1152',
                'visible': True, 'enabled': True, 'name': 'Folder:'}
        search = dict(edit, automation_id='SearchEditBox', name='Search')
        self.assertEqual(unique_folder_edit([search, edit], 42), 1)
        # Labels are observed for diagnostics, never assumed to be English.
        self.assertEqual(unique_folder_edit([dict(edit, name='תיקייה:')], 42), 0)
        for records in [[], [search], [edit, dict(edit)], [dict(edit, pid=99)],
                        [dict(edit, visible=False)], [dict(edit, enabled=False)],
                        [dict(edit, control_type='ComboBox')]]:
            with self.assertRaises(RuntimeError):
                unique_folder_edit(records, 42)

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
            observed = [{'pid': 42, 'created': 'root-identity', 'executable': str(binary)},
                        {'pid': 43, 'created': 'stable-process-identity'}]
            guard = Mock(records=[], failures=[])
            guard.unconfirmed_residuals.return_value = []
            order = []
            guard.wait.side_effect = lambda _: order.append('wait')
            def retain(rows):
                self.assertEqual(rows, observed)
                order.append('retain')
                return guard
            def terminate(command, **kwargs):
                self.assertEqual(command, ['taskkill', '/PID', '42', '/T', '/F'])
                self.assertEqual(kwargs['timeout'], 10)
                order.append('kill')
                child.poll.return_value = 0
            with patch.object(diagnostics.subprocess, 'Popen', return_value=child), \
                 patch.object(diagnostics.subprocess, 'run', side_effect=terminate), \
                 patch.object(diagnostics.time, 'monotonic', side_effect=[0, 9]), \
                 patch.object(diagnostics, 'snapshot', return_value={'processes': observed}), \
                 patch.object(diagnostics, 'OwnedProcessHandles', side_effect=retain), \
                 patch.object(diagnostics, 'owned_processes', return_value=[]) as residual:
                diagnostics.startup_probe(binary, output)
                residual.assert_called_once_with(binary, 42, observed)
                guard.wait.assert_called_once_with(10)
                guard.close.assert_called_once_with()
                self.assertEqual(order, ['retain', 'kill', 'wait'])
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
                with self.assertRaisesRegex(RuntimeError, 'exit was not confirmed'):
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
