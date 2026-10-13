"""Pair fresh Apply admission with actual outcomes, never preview predictions.

The original interleaves policy calls and mutation. Native admission is a fresh
preflight interval followed by verified effects. These schedules are not equal;
only exact ordered policy values, source/artist routes and outcomes are compared.
"""
from __future__ import annotations
import os
from collections import Counter
import re

from check_cli import require


SCOPE = ('Fresh Apply-preflight policy admission followed by independently observed '
         'physical outcomes. This is not equality of interleaved runtime decision '
         'timing. Empty operation IDs mean no admitted transfer, not an inferred '
         'execution-time exists-only skip. Original raw calls and native receipts '
         'remain unchanged.')


def original_route_outcomes(original):
    """Use actual frozen scan-loop source and returned copy/move calls only."""
    routes = []
    active = None
    for row in original['ordered_trace']:
        if row['kind'] == 'destination_decision':
            require('source' in row, 'Original destination lacks its actual scan-loop source')
            active = {'source': row['source'], 'artist': row['artist'],
                      'destination_directory': row['destination'],
                      'observed_eligible': not row['exist_only'] or row['is_directory'],
                      'calls': []}
            routes.append(active)
        elif row['kind'] in ('copy', 'move') and active is not None:
            require(row['source'] == active['source'], 'Original transfer/source route differs')
            require(row['destination'].rsplit(os.sep, 1)[0] == active['destination_directory'],
                    'Original transfer/destination route differs')
            active['calls'].append(row)
    result = []
    for route in routes:
        successful = [row for row in route['calls'] if row.get('success') is True]
        if not route['observed_eligible']:
            require(not route['calls'], 'An ineligible original route made a transfer call')
            outcome = 'no_admitted_transfer'
        elif len(route['calls']) == len(successful) == 1:
            outcome = 'committed_transfer'
        else:
            # A failed call or silently absent transfer is never physical success.
            outcome = 'failed_or_missing_transfer'
        result.append({'source': route['source'], 'artist': route['artist'], 'outcome': outcome,
                       'transfers': [{key: row[key] for key in ('kind', 'source', 'destination')}
                                     for row in successful]})
    return result


def require_consistent_complete(result, label='result'):
    """Reject explicit failure evidence without inventing absent family fields."""
    require(isinstance(result, dict) and result.get('status') == 'complete',
            label + ' contradicts clean completion')
    for field in ('failed_operations', 'blocked_operations', 'recovery_operations', 'cancelled_operations'):
        if field in result:
            require(type(result[field]) is int and result[field] == 0,
                    label + ' has a nonzero or invalid ' + field)
    for field in ('error', 'failure', 'recovery_error'):
        require(result.get(field) is None, label + ' has explicit ' + field)
    if 'items' in result:
        require(isinstance(result['items'], list), label + ' items are not an ordered list')
        for index, item in enumerate(result['items']):
            require_consistent_complete(item, label + f'.items[{index}]')
    if 'whole_albums' in result:
        require(isinstance(result['whole_albums'], list), label + ' whole_albums is not an ordered list')
        for index, album in enumerate(result['whole_albums']):
            require_consistent_complete(album, label + f'.whole_albums[{index}]')
    if result.get('transfers') is not None:
        require_consistent_complete(result['transfers'], label + '.transfers')


def accepted_decision_outcomes(root, case, native, reference_path, project_decisions):
    """Consume only the live producer's fresh receipt and actual final effects.

    The separately invoked read-only preview is deliberately not an input here.
    Malformed or unmatched receipts are errors. Missing producer data or an
    unfinished run remains unavailable, never inferred from a successful preview.
    """
    result = native.get('result') or {}
    receipt = result.get('execution_decisions')
    if receipt is None:
        return None
    require(receipt.get('schema_version') == 1, 'Unknown execution decision receipt schema')
    require(receipt.get('basis') in ('fresh_apply_preflight', 'fresh_noop_preflight'),
            'Execution decision receipt is not freshly validated Apply evidence')
    require(re.fullmatch('[0-9a-f]{64}', receipt.get('plan_binding_sha256', '')) is not None,
            'Execution decision receipt lacks the exact accepted plan binding')
    require(isinstance(receipt.get('decisions'), list) and isinstance(receipt.get('single_routes'), list),
            'Execution decision receipt lacks full ordered coverage')
    decisions = project_decisions(root, receipt['decisions'])
    evidence = {'scope': SCOPE, 'basis': receipt['basis'],
                'plan_binding_sha256': receipt['plan_binding_sha256'],
                'decisions': decisions, 'single_routes': None, 'available': False}
    if (result.get('status') != 'complete' or native['execution']['exit'] != 0
            or result.get('recovery_error') is not None or result.get('invalid_control', False)):
        evidence['gap'] = 'Fresh policy receipt retained, but final execution is not cleanly complete.'
        return evidence
    require_consistent_complete(result)
    order = native.get('acknowledged_transfer_order')
    if order is None:
        evidence['gap'] = 'Fresh policy receipt retained, but acknowledged physical effect coverage is missing.'
        return evidence
    if receipt['basis'] == 'fresh_noop_preflight':
        require(result.get('no_op') is True and not order and result.get('effect_receipts') == []
                and result.get('committed_operations') == []
                and result.get('committed_destinations') == result.get('moved_sources') == 0,
                'No-effect receipt contains a physical effect or lacks explicit zero completion')
        require(result.get('plan_id') == receipt['plan_binding_sha256'],
                'No-effect completion and fresh decision receipt name different accepted plans')
    else:
        require(result.get('no_op') is not True, 'No-effect completion must use fresh_noop_preflight')
        started = native.get('started')
        if started is None:
            evidence['gap'] = 'Fresh decision receipt lacks the actual started job/accepted plan event.'
            return evidence
        require(started.get('job_id') == result.get('job_id') and result.get('job_id') is not None
                and started.get('plan_id') == receipt['plan_binding_sha256'],
                'Fresh decisions and physical result are not bound to the same started job/plan')
    require(all(row['kind'] in ('album', 'single') for row in decisions),
            'A clean accepted decision stream contains an error or vanished read')
    expected_routes = [(row['source'], artist) for row in decisions if row['kind'] == 'single'
                       for artist in (row['result'] or [])[:None if case['settings']['duet_mode'] else 1]]
    routes = receipt['single_routes']
    require([(reference_path(root, row['source']), row['artist']) for row in routes] == expected_routes,
            'Accepted source/artist routes omit, duplicate or reorder a selected policy decision')
    committed = result.get('committed_operations')
    require(isinstance(committed, list), 'Actual committed operation IDs are unavailable')
    require(Counter(committed) == Counter(row['operation'] for row in order if row['receipt_kind'] != 'album_renamed'),
            'Actual committed operation IDs differ from independently acknowledged effects')
    albums = result.get('completed_album_operations', [])
    require(isinstance(albums, list) and len(set(albums)) == len(albums),
            'Completed album operation IDs are missing or ambiguous')
    require(result.get('albums_completed', 0) == len(albums),
            'Actual album count differs from completed operation evidence')
    rows, used = [], set()
    for route in routes:
        source = reference_path(root, route['source'])
        ids = route.get('operation_ids')
        require(isinstance(ids, list) and len(ids) <= 1
                and all(type(value) is int and value >= 0 for value in ids),
                'A selected artist route must admit zero or one exact operation')
        transfers = []
        for operation in ids:
            require(operation not in used and operation not in albums,
                    'An admitted single operation is duplicated or overlaps an album')
            actual = [row for row in order if row['operation'] == operation]
            require(len(actual) == 1 and actual[0]['source'] == source
                    and actual[0]['receipt_kind'] in ('file_published', 'file_renamed'),
                    'Admitted single operation lacks its exact acknowledged source/effect')
            require(committed.count(operation) == 1, 'Admitted single operation did not commit exactly once')
            used.add(operation)
            transfers.append({key: actual[0][key] for key in ('kind', 'source', 'destination')})
        rows.append({'source': source, 'artist': route['artist'],
                     'outcome': 'committed_transfer' if ids else 'no_admitted_transfer', 'transfers': transfers})
    require({row['operation'] for row in order} == used | set(albums),
            'Acknowledged physical operations are unmatched by admitted routes/completed albums')
    evidence.update(single_routes=rows, available=True)
    return evidence
