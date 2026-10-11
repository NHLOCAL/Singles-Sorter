"""Failure evidence must describe the actual viewport, before restoration."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from desktop_layout import check_phone_layout, check_native_layout, assert_editor_actions

class LayoutEvidenceTests(unittest.TestCase):
    def run_failure(self, helper, suffix, expected_width):
        current = {'width': 1000, 'height': 650}
        captures = []
        def command(method, path, body=None):
            if method == 'POST': current.update(body)
            return dict(current)
        def script(_):
            return {'innerWidth': current['width'], 'innerHeight': current['height'],
                    'backgroundLocked': True, 'dialogs': [{'editor': True,
                    'rect': {'height': 100}, 'scrollHeight': 200, 'actions': []}]}
        def capture(label): captures.append((label, dict(current)))
        with tempfile.TemporaryDirectory() as folder, patch('desktop_layout.time.sleep'):
            with self.assertRaises(AssertionError):
                helper(command, script, capture, Path(folder), 'failure')
            self.assertEqual(current, {'width':1000, 'height':650})
            self.assertEqual(len(captures), 1)
            self.assertEqual(captures[0][1]['width'], expected_width)
            evidence=json.loads((Path(folder)/('failure'+suffix)).read_text(encoding='utf-8'))
            self.assertEqual(len(evidence['measurements']), 1)
            self.assertEqual(evidence['measurements'][0]['innerWidth'], expected_width)
    def test_narrow_failure_is_captured_before_restore(self):
        self.run_failure(check_phone_layout, '-phone-css-metrics.json', 360)
    def test_wide_failure_is_captured_before_restore(self):
        self.run_failure(check_native_layout, '-layout-metrics.json', 1000)
    def test_unanchored_absolute_label_is_rejected(self):
        metrics={'backgroundLocked':True,'dialogs':[{'editor':True,'rect':{'height':100},
            'scrollHeight':100,'actions':[],'absoluteRowLabels':[{'containedByButton':False}]}]}
        with self.assertRaises(AssertionError): assert_editor_actions(metrics)
        metrics['dialogs'][0]['absoluteRowLabels'][0]['containedByButton']=True
        assert_editor_actions(metrics)

if __name__ == '__main__': unittest.main()
