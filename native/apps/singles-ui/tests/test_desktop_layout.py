"""Failure evidence must describe the actual viewport, before restoration."""
import json
from pathlib import Path
import tempfile
import subprocess
import unittest
from unittest.mock import patch
from desktop_layout import check_phone_layout, check_native_layout, assert_editor_actions, assert_layout

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

class SelectedResultsBudgetTests(unittest.TestCase):
    def metrics(self, panel=True):
        # Reproduce the recorded 026 1000x650 failure geometry. Panel fields
        # model the visible 68 px card plus its 10 px CSS bottom margin; the
        # next actual run must obtain those new fields directly from WebView.
        def rect(x, y, width, height):
            return dict(x=x, y=y, width=width, height=height, right=x+width, bottom=y+height)
        button = rect(58, 241.203125, 126, 36)
        result = dict(backgroundLocked=True, dialogs=[], reflow=False, innerHeight=650, innerWidth=1000,
                      pageHeight=650, pageWidth=1000, view='results', resultsVisible=True, setupVisible=False,
                      chrome={'header':rect(0, 0, 1000, 56), 'footer':rect(0, 612, 1000, 38)},
                      results=rect(24, 72, 952, 524), resultsBody=rect(43, 303.203125, 914, 277.796875),
                      resultsOverflow='auto', rowHeights=[44.5], controls={'review-apply':button},
                      executionPanel={'rect':rect(43, 225.203125, 914, 68), 'parentIsResults':True, 'visible':True,
                                      'marginTop':0, 'marginBottom':10, 'expectedActions':['review-apply'],
                                      'actions':[{'id':'review-apply','rect':button,'visible':True}]})
        if not panel:
            result['executionPanel'] = None
            result['controls'] = {}
            result['resultsBody'] = rect(43, 225.203125, 914, 355.796875)
        return result

    def test_selected_panel_occupancy_is_measured_without_reducing_feature_off_budget(self):
        assert_layout(self.metrics())
        assert_layout(self.metrics(False))
        # The exact failed list alone still fails when there is no real panel.
        metrics = self.metrics(); metrics['executionPanel'] = None; metrics['controls'] = {}
        with self.assertRaises(AssertionError): assert_layout(metrics)

    def test_reservation_requires_visible_nonoverlapping_owned_card_and_actions(self):
        for change in ['foreign_parent','overlap','negative_margin','nonfinite_margin','action_outside','action_missing','action_hidden','invisible_panel','invisible_action']:
            metrics = self.metrics(); panel = metrics['executionPanel']
            if change == 'foreign_parent': panel['parentIsResults'] = False
            if change == 'overlap': panel['rect']['bottom'] += 2
            if change == 'negative_margin': panel['marginTop'] = -1
            if change == 'nonfinite_margin': panel['marginBottom'] = float('nan')
            if change == 'action_outside': panel['actions'][0]['rect']['y'] = panel['rect']['y'] - 1
            if change == 'action_missing': metrics['controls'] = {}
            if change == 'action_hidden': panel['actions'] = []
            if change == 'invisible_panel': panel['visible'] = False
            if change == 'invisible_action': panel['actions'][0]['visible'] = False
            with self.subTest(change=change), self.assertRaises(AssertionError): assert_layout(metrics)

    def test_large_panel_cannot_justify_collapsed_list(self):
        metrics = self.metrics(); panel = metrics['executionPanel']['rect']; body = metrics['resultsBody']
        panel['height'] += 100; panel['bottom'] += 100
        body['y'] += 100; body['height'] -= 100
        with self.assertRaises(AssertionError): assert_layout(metrics)

    def test_fabricated_reservation_cannot_exceed_a_third_of_viewport(self):
        metrics = self.metrics(); panel = metrics['executionPanel']; body = metrics['resultsBody']
        panel['rect'].update(y=100, height=220, bottom=320)
        body.update(y=330, height=251, bottom=581)
        # List remains useful and non-overlapping; panel fraction still fails.
        with self.assertRaises(AssertionError): assert_layout(metrics)

    def test_missing_panel_measurement_is_not_assumed_to_be_hidden(self):
        metrics = self.metrics(); del metrics['executionPanel']
        with self.assertRaises(KeyError): assert_layout(metrics)

    def test_footer_list_and_horizontal_visibility_remain_strict(self):
        for change in ['floating_footer','table_under_footer','page_overflow','control_offscreen']:
            metrics = self.metrics()
            if change == 'floating_footer': metrics['chrome']['footer']['bottom'] -= 2
            if change == 'table_under_footer': metrics['results']['bottom'] = 620
            if change == 'page_overflow': metrics['pageWidth'] = 1002
            if change == 'control_offscreen': metrics['controls']['review-apply']['right'] = 1002
            with self.subTest(change=change), self.assertRaises(AssertionError): assert_layout(metrics)

    def test_metric_visibility_rejects_hidden_collapsed_and_transparent_ancestors(self):
        # Exercise the exact collected JS predicate against isolated DOM-shaped
        # values; this does not substitute for actual WebView layout evidence.
        source = Path(__file__).with_name('layout_metrics.js').read_text()
        predicate = source.split(' const renderedVisible=', 1)[1].split(';\n', 1)[0]
        program = "const getComputedStyle = node => node.style; const visible = " + predicate + r""";
const element = (visibility='visible',opacity='1',parentElement=null,rects=1) => ({
 style:{visibility,opacity},parentElement,getClientRects:()=>Array(rects)});
const result = [visible(element()), visible(element('hidden')), visible(element('collapse')),
 visible(element('visible','0')),visible(element('visible','0.0')),
 visible(element('visible','1',element('visible','0'))),
 visible(element('visible','1',element('hidden'))),visible(element('visible','1',null,0)),
 visible(element('visible','0.5'))];
process.stdout.write(JSON.stringify(result));
"""
        result = subprocess.run(['node', '-e', program], capture_output=True, text=True, check=True, timeout=10)
        self.assertEqual(json.loads(result.stdout), [True,False,False,False,False,False,False,False,True])

    def test_runtime_collects_actual_panel_and_apply_cancel_controls(self):
        source = Path(__file__).with_name('layout_metrics.js').read_text()
        self.assertIn("'review-apply','cancel-apply'", source)
        self.assertIn('panel.getClientRects().length>0', source)
        self.assertIn('parseFloat(panelStyle.marginBottom)', source)
        self.assertIn('executionPanel,', source)
        self.assertIn("style.visibility!=='visible'", source)
        self.assertIn('parseFloat(style.opacity)>0', source)
        self.assertIn('node=node.parentElement', source)
        self.assertIn('visible:renderedVisible(panel)', source)
        self.assertIn('visible:renderedVisible(el)', source)


if __name__ == '__main__': unittest.main()
