import http.client
import unittest
import urllib.error
from unittest.mock import Mock, patch
from driver_readiness import wait_driver_ready


class DriverReadinessTests(unittest.TestCase):
    def run_probe(self, call, process=None, timeout=1):
        clock = [0.0]
        def sleep(seconds): clock[0] += seconds
        with patch('driver_readiness.time.monotonic', side_effect=lambda: clock[0]), patch('driver_readiness.time.sleep', side_effect=sleep):
            return wait_driver_ready(call, process or Mock(poll=Mock(return_value=None)), timeout)

    def test_startup_refusal_then_empty_proxy_then_ready(self):
        call = Mock(side_effect=[urllib.error.URLError('connection refused'), http.client.RemoteDisconnected('empty proxy'), {'ready':False}, {'ready':True}])
        self.assertTrue(self.run_probe(call)['ready'])
        self.assertEqual(call.call_count, 4)
        for args in call.call_args_list: self.assertEqual(args.args, ('GET', '/status'))

    def test_persistent_disconnect_is_bounded_and_reported(self):
        call = Mock(side_effect=http.client.RemoteDisconnected('empty proxy'))
        with self.assertRaisesRegex(TimeoutError, 'RemoteDisconnected'):
            self.run_probe(call)
        self.assertEqual(call.call_count, 5)

    def test_exit_is_fatal_without_another_request(self):
        call = Mock()
        with self.assertRaisesRegex(RuntimeError, 'exit 7'):
            self.run_probe(call, Mock(poll=Mock(return_value=7)))
        call.assert_not_called()

    def test_protocol_error_is_not_retried(self):
        call = Mock(side_effect=http.client.BadStatusLine('malformed'))
        with self.assertRaises(http.client.BadStatusLine): self.run_probe(call)
        self.assertEqual(call.call_count, 1)

    def test_ready_must_be_true_boolean(self):
        call = Mock(side_effect=[{}, {'ready':1}, {'ready':'true'}, {'ready':True}])
        self.assertTrue(self.run_probe(call)['ready'])
        self.assertEqual(call.call_count, 4)


if __name__ == '__main__': unittest.main()
