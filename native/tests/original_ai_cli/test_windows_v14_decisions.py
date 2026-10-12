"""Portable receipt projection rejection tests, not actual Windows evidence."""
import copy
import os
from pathlib import Path
import unittest

from windows_v14_decisions import accepted_decision_outcomes, original_route_outcomes
import check_windows_v14 as gate


class OriginalRouteTests(unittest.TestCase):
    def evidence(self):
        source = os.sep.join(('source', 'song.wav'))
        folder = os.sep.join(('target', 'A'))
        return {'ordered_trace': [
            {'kind': 'destination_decision', 'source': source, 'artist': 'A',
             'destination': folder, 'exist_only': True, 'is_directory': True},
            {'kind': 'copy', 'source': source, 'destination': folder+os.sep+'song.wav', 'success': True},
            {'kind': 'destination_decision', 'source': source, 'artist': 'B',
             'destination': os.sep.join(('target', 'B')), 'exist_only': True, 'is_directory': False}]}

    def test_actual_calls_distinguish_success_and_source_bound_skip(self):
        rows = original_route_outcomes(self.evidence())
        self.assertEqual([row['artist'] for row in rows], ['A', 'B'])
        self.assertEqual([row['outcome'] for row in rows], ['committed_transfer', 'no_admitted_transfer'])
        self.assertEqual(len(rows[0]['transfers']), 1)
        self.assertEqual(rows[1]['transfers'], [])

    def test_eligible_absence_or_failed_call_never_becomes_success(self):
        evidence = self.evidence()
        for success in (False, None):
            changed = copy.deepcopy(evidence)
            changed['ordered_trace'][1]['success'] = success
            self.assertEqual(original_route_outcomes(changed)[0]['outcome'], 'failed_or_missing_transfer')
        evidence['ordered_trace'].pop(1)
        self.assertEqual(original_route_outcomes(evidence)[0]['outcome'], 'failed_or_missing_transfer')

    def test_missing_source_wrong_source_or_wrong_destination_is_rejected(self):
        for key, value in (('source', 'different'), ('destination', 'different')):
            changed = self.evidence(); changed['ordered_trace'][1][key] = value
            with self.assertRaises(AssertionError): original_route_outcomes(changed)
        evidence = self.evidence(); del evidence['ordered_trace'][2]['source']
        with self.assertRaises(AssertionError): original_route_outcomes(evidence)

    def test_ineligible_transfer_is_rejected(self):
        evidence = self.evidence(); evidence['ordered_trace'][0]['is_directory'] = False
        with self.assertRaises(AssertionError): original_route_outcomes(evidence)


class AcceptedDecisionTests(unittest.TestCase):
    root = Path('/owned')
    case = {'settings': {'duet_mode': True}}

    def evidence(self):
        raw = str(self.root/'source'/'song.wav').encode('utf-16-le')
        reference = {'WindowsPath': {'utf16': [int.from_bytes(raw[i:i+2], 'little') for i in range(0, len(raw), 2)]}}
        source = os.sep.join(('source', 'song.wav'))
        destination = os.sep.join(('target', 'A', 'song.wav'))
        return {'execution': {'exit': 0}, 'result': {
            'job_id': 'actual-job', 'status': 'complete', 'committed_operations': [0], 'recovery_error': None,
            'execution_decisions': {'schema_version': 1, 'basis': 'fresh_apply_preflight',
                'plan_binding_sha256': 'a'*64,
                'decisions': [{'Single': {'read': {'source': {'reference': reference}}, 'artists': ['A', 'B']}}],
                'single_routes': [{'source': reference, 'artist': 'A', 'operation_ids': [0]},
                                  {'source': reference, 'artist': 'B', 'operation_ids': []}]}},
            'started': {'job_id': 'actual-job', 'plan_id': 'a'*64},
            'acknowledged_transfer_order': [{'operation': 0, 'kind': 'copy', 'source': source,
                                            'destination': destination, 'receipt_kind': 'file_published'}]}

    def project(self, native):
        return accepted_decision_outcomes(self.root, self.case, native, gate.reference_path, gate.project_decisions)

    def test_fresh_decisions_and_exact_observed_outcomes_match_original_routes(self):
        native = self.evidence(); untouched = copy.deepcopy(native)
        result = self.project(native)
        self.assertTrue(result['available'])
        self.assertEqual(result['single_routes'], original_route_outcomes(OriginalRouteTests().evidence()))
        self.assertEqual(native, untouched)

    def test_preview_and_old_legacy_fields_never_supply_actual_receipt(self):
        native = self.evidence()
        receipt = native['result'].pop('execution_decisions')
        native['result']['legacy_decisions'] = receipt['decisions']
        native['preview_response'] = {'result': {'plan': {'decisions': receipt['decisions']}}}
        self.assertIsNone(self.project(native))

    def test_wrong_basis_version_or_binding_never_passes(self):
        for key, value in [('basis', 'preview'), ('schema_version', 2), ('plan_binding_sha256', 'A'*64),
                           ('plan_binding_sha256', ''), ('decisions', None), ('single_routes', None)]:
            native = self.evidence(); native['result']['execution_decisions'][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(AssertionError): self.project(native)

    def test_partial_cancelled_failed_and_missing_physical_evidence_stay_open(self):
        for key, value in [('status', 'partial'), ('status', 'cancelled'), ('status', 'needs_recovery'),
                           ('recovery_error', 'unfinished'), ('invalid_control', True)]:
            native = self.evidence(); native['result'][key] = value
            with self.subTest(key=key, value=value): self.assertFalse(self.project(native)['available'])
        native = self.evidence(); native['execution']['exit'] = 1
        self.assertFalse(self.project(native)['available'])
        native = self.evidence(); native['acknowledged_transfer_order'] = None
        self.assertFalse(self.project(native)['available'])

    def test_explicit_failed_counters_and_items_cannot_hide_under_complete_status(self):
        for nested in (False, True):
            for key in ('failed_operations', 'blocked_operations', 'recovery_operations', 'cancelled_operations'):
                native = self.evidence()
                result = native['result']
                if nested:
                    result['transfers'] = {'status': 'complete'}
                    result = result['transfers']
                result[key] = 1
                with self.subTest(nested=nested, counter=key), self.assertRaises(AssertionError): self.project(native)
            for status in ('failed', 'blocked', 'cancelled', 'source_retained', 'needs_recovery'):
                native = self.evidence()
                result = native['result']
                if nested:
                    result['transfers'] = {'status': 'complete'}
                    result = result['transfers']
                result['items'] = [{'operation': 0, 'status': status}]
                with self.subTest(nested=nested, item_status=status), self.assertRaises(AssertionError): self.project(native)
        native = self.evidence(); native['result']['transfers'] = {'status': 'partial'}
        with self.assertRaises(AssertionError): self.project(native)
        native = self.evidence(); native['result']['whole_albums'] = [{'status': 'partial'}]
        with self.assertRaises(AssertionError): self.project(native)
        native = self.evidence(); native['result']['whole_albums'] = [{'status': 'complete', 'failure': 'observed failure'}]
        with self.assertRaises(AssertionError): self.project(native)

    def test_absent_family_fields_are_not_invented_and_explicit_clean_values_work(self):
        native = self.evidence()
        self.assertTrue(self.project(native)['available'])
        self.assertNotIn('failed_operations', native['result'])
        native['result'].update(failed_operations=0, blocked_operations=0, recovery_operations=0,
                                cancelled_operations=0, items=[{'operation': 0, 'status': 'complete'}],
                                whole_albums=[], transfers=None)
        self.assertTrue(self.project(native)['available'])

    def test_missing_reordered_duplicated_artist_route_never_passes(self):
        for edit in (lambda rows: rows.pop(), lambda rows: rows.reverse(), lambda rows: rows.append(copy.deepcopy(rows[0]))):
            native = self.evidence(); edit(native['result']['execution_decisions']['single_routes'])
            with self.assertRaises(AssertionError): self.project(native)

    def test_receipt_must_bind_same_actual_started_job_and_plan(self):
        native = self.evidence(); del native['started']
        self.assertFalse(self.project(native)['available'])
        for key, value in [('job_id', 'other-job'), ('plan_id', 'b'*64)]:
            native = self.evidence(); native['started'][key] = value
            with self.assertRaises(AssertionError): self.project(native)

    def test_unknown_uncommitted_wrong_source_or_duplicated_operation_never_passes(self):
        for mutate in (
            lambda native: native['result']['execution_decisions']['single_routes'][0].update(operation_ids=[5]),
            lambda native: native['result']['execution_decisions']['single_routes'][1].update(operation_ids=[0]),
            lambda native: native['result'].update(committed_operations=[]),
            lambda native: native['acknowledged_transfer_order'][0].update(source='wrong'),
            lambda native: native['acknowledged_transfer_order'][0].update(receipt_kind='album_renamed'),
            lambda native: native['acknowledged_transfer_order'].append(copy.deepcopy(native['acknowledged_transfer_order'][0])),
        ):
            native = self.evidence(); mutate(native)
            with self.assertRaises(AssertionError): self.project(native)

    def test_unmatched_effect_is_not_hidden_by_matching_single(self):
        native = self.evidence()
        native['acknowledged_transfer_order'].append({'operation': 1, 'kind': 'move', 'source': 'album',
                                                     'destination': 'target/album', 'receipt_kind': 'album_renamed'})
        with self.assertRaises(AssertionError): self.project(native)
        native['result'].update(completed_album_operations=[1], albums_completed=1)
        self.assertTrue(self.project(native)['available'])
        native['result']['completed_album_operations'].append(2)
        native['result']['albums_completed'] = 2
        with self.assertRaises(AssertionError): self.project(native)

    def test_no_effect_path_requires_separate_basis_explicit_zero_and_no_effects(self):
        native = self.evidence()
        native['result'].update(no_op=True, plan_id='a'*64, committed_operations=[], effect_receipts=[], committed_destinations=0, moved_sources=0)
        native['result']['execution_decisions']['single_routes'][0]['operation_ids'] = []
        native['acknowledged_transfer_order'] = []
        with self.assertRaises(AssertionError): self.project(native)
        native['result']['execution_decisions']['basis'] = 'fresh_noop_preflight'
        self.assertTrue(self.project(native)['available'])
        self.assertEqual([row['outcome'] for row in self.project(native)['single_routes']], ['no_admitted_transfer']*2)
        for key, value in [('moved_sources', 1), ('committed_destinations', 1), ('effect_receipts', [{}]), ('no_op', False)]:
            changed = copy.deepcopy(native); changed['result'][key] = value
            with self.subTest(key=key), self.assertRaises(AssertionError): self.project(changed)


if __name__ == '__main__': unittest.main()
