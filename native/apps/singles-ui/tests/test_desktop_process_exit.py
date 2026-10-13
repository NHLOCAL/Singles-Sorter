"""Fail-closed exit proofs; Windows integration runs before desktop builds."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

import desktop_diagnostics as diagnostics

import windows_process_exit as exits

CREATED = '2026-10-11T16:21:45.0904470+00:00'
IMAGE = r'C:\owned\msedgewebview2.exe'


def row(pid=43, **changes):
    return dict(dict(pid=pid, created=CREATED, executable=IMAGE), **changes)


class FakeAPI:
    def __init__(self):
        self.opened, self.closed, self.waited = [], [], []
        self.creation = exits.cim_creation_microseconds(CREATED) * 10 + 7
        self.image = IMAGE
        self.result = exits.WAIT_OBJECT_0
        self.open_error = None
        self.wait_error = None

    def open(self, pid):
        if self.open_error:
            raise self.open_error
        self.opened.append(pid)
        return pid + 1000

    def identity(self, handle):
        return self.creation, self.image

    def wait(self, handle, milliseconds):
        self.waited.append((handle, milliseconds))
        if self.wait_error:
            raise self.wait_error
        return self.result

    def close(self, handle):
        self.closed.append(handle)


class ProcessExitContracts(unittest.TestCase):
    def test_creation_conversion_is_integer_microsecond_precision(self):
        value = exits.cim_creation_microseconds(CREATED)
        self.assertEqual(value, exits.cim_creation_microseconds('2026-10-11T19:21:45.090447+03:00'))
        self.assertEqual(exits.cim_creation_microseconds('1601-01-01T00:00:00.000000Z'), 0)
        for invalid in ('', None, '2026-10-11T16:21:45.0904471+00:00',
                        '2026-10-11T16:21:45.090447', '2026-10-11T16:21:45.090+00:00'):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                exits.cim_creation_microseconds(invalid)

    def test_signaled_exact_handle_proves_stale_cim_row_exited(self):
        api = FakeAPI()
        guard = exits.OwnedProcessHandles([row()], api=api)
        try:
            self.assertEqual(api.opened, [43])
            self.assertEqual(guard.records[0]['native_creation_100ns'], api.creation)
            self.assertEqual(guard.records[0]['status'], 'retained')
            guard.wait(10, clock=lambda: 0)
            self.assertEqual(guard.unconfirmed_residuals([row(executable=None)]), [])
            self.assertEqual(guard.failures, [])
            self.assertEqual(api.waited, [(1043, 10000)])
        finally:
            guard.close()
        self.assertEqual(api.closed, [1043])

    def test_only_matching_signaled_identity_can_be_ignored(self):
        api = FakeAPI()
        guard = exits.OwnedProcessHandles([row()], api=api)
        try:
            guard.wait(10)
            unknown = [row(pid=44), row(created='2026-10-11T16:21:45.0904480+00:00'),
                       row(created=None), row(executable=r'C:\other\msedgewebview2.exe')]
            self.assertEqual(guard.unconfirmed_residuals(unknown), unknown)
        finally:
            guard.close()

    def test_unwaited_matching_handle_is_not_exit_proof(self):
        api = FakeAPI()
        guard = exits.OwnedProcessHandles([row()], api=api)
        try:
            self.assertEqual(guard.unconfirmed_residuals([row(executable=None)]), [row(executable=None)])
        finally:
            guard.close()

    def test_mismatched_creation_or_image_is_fail_closed_and_closed(self):
        for field, value in [('creation', FakeAPI().creation + 10), ('image', r'C:\other.exe')]:
            api = FakeAPI()
            setattr(api, field, value)
            guard = exits.OwnedProcessHandles([row()], api=api)
            try:
                self.assertEqual(guard.records[0]['status'], 'identity-error')
                self.assertTrue(guard.failures)
                guard.wait(10)
                self.assertEqual(api.waited, [])
                self.assertEqual(guard.unconfirmed_residuals([row(executable=None)]), [row(executable=None)])
            finally:
                guard.close()
            self.assertEqual(api.closed, [1043])

    def test_open_failure_missing_identity_and_duplicate_fail_closed(self):
        api = FakeAPI()
        api.open_error = PermissionError('access denied')
        guard = exits.OwnedProcessHandles([row()], api=api)
        guard.close()
        self.assertIn('access denied', guard.records[0]['error'])
        self.assertTrue(guard.failures)
        for invalid in [row(created=None), row(executable=None), row(pid=0), row(pid=True)]:
            api = FakeAPI()
            guard = exits.OwnedProcessHandles([invalid], api=api)
            guard.close()
            self.assertTrue(guard.failures)
            self.assertEqual(api.opened, [])
        api = FakeAPI()
        guard = exits.OwnedProcessHandles([row(), row()], api=api)
        guard.close()
        self.assertTrue(guard.failures)
        self.assertEqual(api.opened, [43])
        self.assertEqual(api.closed, [1043])

    def test_later_failure_does_not_lose_prior_handle(self):
        api = FakeAPI()
        guard = exits.OwnedProcessHandles([row(), row(pid=44, executable=None)], api=api)
        guard.wait(10)
        guard.close()
        guard.close()
        self.assertEqual(api.closed, [1043])
        self.assertEqual(len(guard.failures), 1)

    def test_all_waits_share_one_deadline_and_timeout_remains_failure(self):
        api = FakeAPI()
        api.result = exits.WAIT_TIMEOUT
        guard = exits.OwnedProcessHandles([row(43), row(44), row(45)], api=api)
        ticks = iter([0.0, 0.0, 9.75, 10.0])
        try:
            guard.wait(10, clock=lambda: next(ticks))
            self.assertEqual(api.waited, [(1043, 10000), (1044, 250), (1045, 0)])
            self.assertEqual([r['status'] for r in guard.failures], ['timeout'] * 3)
            self.assertEqual(guard.unconfirmed_residuals([row()]), [row()])
        finally:
            guard.close()

    def test_wait_failure_preserves_other_process_proofs_and_closes_all(self):
        api = FakeAPI()
        api.wait_error = OSError('WaitForSingleObject failed: 5')
        guard = exits.OwnedProcessHandles([row(43), row(44)], api=api)
        try:
            guard.wait(10)
            self.assertEqual(len(guard.failures), 2)
            self.assertIn('WaitForSingleObject failed: 5', guard.records[0]['error'])
        finally:
            guard.close()
        self.assertEqual(api.closed, [1043, 1044])

    def test_unexpected_wait_status_never_becomes_success(self):
        api = FakeAPI()
        api.result = 0x80
        guard = exits.OwnedProcessHandles([row()], api=api)
        try:
            guard.wait(10)
            self.assertEqual(guard.records[0]['status'], 'wait-error')
            self.assertTrue(guard.failures)
        finally:
            guard.close()

    def test_partial_acquisition_and_close_failure_are_preserved(self):
        api = FakeAPI()
        api.identity = Mock(side_effect=[(api.creation, api.image), OSError('identity read failed')])
        api.close = Mock(side_effect=[OSError('close failed'), None])
        guard = exits.OwnedProcessHandles([row(43), row(44)], api=api)
        guard.wait(10)
        guard.close()
        self.assertEqual(api.close.call_count, 2)
        self.assertEqual(guard.records[0]['close_error'], 'close failed')
        self.assertEqual(guard.records[1]['error'], 'identity read failed')
        self.assertEqual(len(guard.failures), 2)

    def test_malformed_later_row_does_not_leak_retained_handle(self):
        api = FakeAPI()
        guard = exits.OwnedProcessHandles([row(), None], api=api)
        guard.wait(10)
        guard.close()
        self.assertEqual(api.closed, [1043])
        self.assertEqual(len(guard.failures), 1)

    def test_api_opens_only_query_and_synchronize_rights(self):
        kernel = Mock()
        kernel.OpenProcess.return_value = 123
        with patch('ctypes.WinDLL', return_value=kernel, create=True):
            api = exits.WindowsProcessAPI()
            self.assertEqual(api.open(43), 123)
        kernel.OpenProcess.assert_called_once_with(0x00101000, False, 43)

    def test_no_processes_needs_no_windows_api(self):
        guard = exits.OwnedProcessHandles([])
        guard.wait(10)
        guard.close()
        self.assertEqual(guard.records, [])
        self.assertEqual(guard.failures, [])

class StartupExitIntegrationContracts(unittest.TestCase):
    def probe(self, *, wait_result=exits.WAIT_OBJECT_0, residual_created=CREATED,
              cleanup_error=None, observation_error=None, missing_root=False):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp)
            binary = output / 'owned.exe'
            binary.write_bytes(b'fixture')
            child = Mock(pid=42, returncode=1)
            child.poll.return_value = None
            api = FakeAPI()
            api.image = str(binary)
            api.result = wait_result
            observed = [row(42, executable=str(binary)), row(43, executable=str(binary))]
            if missing_root:
                observed.pop(0)
            before = {'processes': observed}
            if observation_error:
                before['diagnostic_error'] = observation_error
            residual = [row(43, created=residual_created, executable=None)]
            if wait_result == exits.WAIT_TIMEOUT:
                residual = []  # Disappearance is insufficient when an exact handle timed out.
            def cleanup(*args, **kwargs):
                if cleanup_error:
                    raise OSError(cleanup_error)
                child.poll.return_value = 1
            with patch.object(diagnostics.subprocess, 'Popen', return_value=child), \
                 patch.object(diagnostics.subprocess, 'run', side_effect=cleanup) as kill, \
                 patch.object(diagnostics.time, 'monotonic', side_effect=[0, 9, 10, 10, 10]), \
                 patch.object(diagnostics, 'snapshot', return_value=before), \
                 patch.object(diagnostics, 'owned_processes', return_value=residual), \
                 patch.object(diagnostics, 'OwnedProcessHandles', side_effect=lambda rows: exits.OwnedProcessHandles(rows, api=api)):
                error = None
                try:
                    diagnostics.startup_probe(binary, output)
                except RuntimeError as caught:
                    error = str(caught)
                self.assertEqual(kill.call_count, 1)
            evidence = json.loads((output / 'startup-probe.json').read_text())
            self.assertEqual(sorted(api.opened), sorted(handle - 1000 for handle in api.closed))
            return error, evidence

    def test_stale_cim_row_has_retained_native_exit_proof_in_evidence(self):
        error, evidence = self.probe()
        self.assertIsNone(error)
        self.assertEqual(evidence['residual_processes_after_cleanup'][0]['pid'], 43)
        self.assertEqual(evidence['unconfirmed_residual_processes'], [])
        self.assertEqual(evidence['process_exit_failures'], [])
        self.assertTrue(all(item['status'] == 'signaled' and item['handle_closed']
                            for item in evidence['process_exit_confirmation']))

    def test_reused_identity_is_not_excused_by_previous_exit(self):
        error, evidence = self.probe(residual_created='2026-10-11T16:21:45.0904480+00:00')
        self.assertIn('survived cleanup', error)
        self.assertEqual(len(evidence['unconfirmed_residual_processes']), 1)

    def test_native_timeout_is_failure_even_without_a_residual_cim_row(self):
        error, evidence = self.probe(wait_result=exits.WAIT_TIMEOUT)
        self.assertIn('exit was not confirmed', error)
        self.assertEqual(evidence['residual_processes_after_cleanup'], [])
        self.assertEqual(len(evidence['process_exit_failures']), 2)

    def test_cleanup_failure_is_not_overwritten_by_wait_failure(self):
        error, evidence = self.probe(wait_result=exits.WAIT_TIMEOUT, cleanup_error='original taskkill failure')
        self.assertIn('exit was not confirmed', error)
        self.assertEqual(evidence['cleanup_error'], 'original taskkill failure')
        self.assertEqual(evidence['process_exit_failures'][0]['status'], 'timeout')

    def test_observation_error_or_missing_root_fails_closed_after_cleanup(self):
        for options in ({'observation_error': 'CIM failed'}, {'missing_root': True}):
            with self.subTest(options=options):
                error, evidence = self.probe(**options)
                self.assertIn('exit was not confirmed', error)
                self.assertIn('diagnostic_error', evidence)
                self.assertTrue(evidence['process_exit_confirmation'][0]['handle_closed'])


@unittest.skipUnless(os.name == 'nt', 'Requires actual Windows process handles and CIM')
class WindowsOwnedProcessExitIntegration(unittest.TestCase):
    def test_real_owned_child_tree_binds_cim_identity_and_signals_before_builds(self):
        # The descendant self-expires if taskkill is interrupted. Normal cleanup
        # uses the same one owned-root tree termination as startup_probe. On an
        # assertion failure, a still-live root cooperatively reaps its own child.
        script = r'''import json, pathlib, subprocess, sys, time
child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(45)'])
try:
    ready = pathlib.Path(sys.argv[1])
    pending = ready.with_suffix('.tmp')
    pending.write_text(json.dumps({'child_pid': child.pid}))
    pending.replace(ready)
    sys.stdin.buffer.read(1)
finally:
    if child.poll() is None:
        child.terminate()
    child.wait(timeout=10)
'''
        with tempfile.TemporaryDirectory() as temp:
            ready = Path(temp) / 'ready.json'
            child = subprocess.Popen([sys.executable, '-c', script, str(ready)],
                                     stdin=subprocess.PIPE, stdout=subprocess.DEVNULL,
                                     stderr=subprocess.PIPE)
            guard = None
            try:
                deadline = time.monotonic() + 10
                while not ready.exists() and child.poll() is None and time.monotonic() < deadline:
                    time.sleep(.05)
                self.assertTrue(ready.exists(), 'Owned Python tree did not become ready')
                descendant_pid = json.loads(ready.read_text())['child_pid']
                observed = diagnostics.owned_processes(Path(sys.executable), child.pid)
                self.assertEqual({item['pid'] for item in observed}, {child.pid, descendant_pid})
                guard = exits.OwnedProcessHandles(observed)
                self.assertTrue(all(item['status'] == 'retained' for item in guard.records), guard.records)
                # Prove these are live native objects before the one root cleanup.
                for record, handle in guard._handles:
                    self.assertEqual(guard._api.wait(handle, 0), exits.WAIT_TIMEOUT, record)
                subprocess.run(['taskkill', '/PID', str(child.pid), '/T', '/F'],
                               capture_output=True, timeout=10, check=True)
                child.wait(timeout=10)
                guard.wait(10)
                self.assertEqual(guard.failures, [])
                residual = diagnostics.owned_processes(Path(sys.executable), child.pid, observed)
                self.assertEqual(guard.unconfirmed_residuals(residual), [], residual)
                # Deterministic regression for the actual failure's null-image CIM
                # shape, using real CIM identities and signaled native handles.
                stale = [dict(item, executable=None) for item in observed]
                self.assertEqual(guard.unconfirmed_residuals(stale), [])
                changed = dict(stale[0], created='1601-01-01T00:00:00.000000Z')
                self.assertEqual(guard.unconfirmed_residuals([changed]), [changed])
                print('Windows owned-process exit proof: ' + json.dumps(guard.records))
            finally:
                try:
                    if child.poll() is None:
                        try:
                            child.communicate(input=b'x', timeout=10)
                        except subprocess.TimeoutExpired:
                            child.kill()  # Exact owned Popen handle, never a PID lookup.
                            child.wait(timeout=10)
                    if guard is not None:
                        # Failure-only hygiene: wait for self-expiring descendants
                        # on the same retained handles. Never turn a failed test or
                        # a previous production deadline timeout into success.
                        deadline = time.monotonic() + 46
                        for record, handle in guard._handles:
                            if record['status'] in ('retained', 'timeout', 'wait-error'):
                                milliseconds = max(0, int((deadline - time.monotonic()) * 1000))
                                if guard._api.wait(handle, milliseconds) != exits.WAIT_OBJECT_0:
                                    raise AssertionError('Owned test descendant failed to exit')
                finally:
                    if guard is not None:
                        guard.close()
                    child.stdin.close()
                    child.stderr.close()
            self.assertTrue(all(item.get('handle_closed') for item in guard.records), guard.records)


if __name__ == '__main__':
    unittest.main()
