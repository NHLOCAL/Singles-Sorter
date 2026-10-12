import copy
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'apps/singles-ui/tests'))
from check_cross_albums import ARTIST, validate_effect_counts, require_effect_bindings, require_cleanup_bindings, windows_reference


class CrossAlbumReceipts(unittest.TestCase):
    def fixture(self):
        return dict(status='complete', recovery_error=None, source_items=5, committed_destinations=5,
                    moved_sources=5, albums_completed=1, artist_files=[[ARTIST, 4]],
                    legacy_summary=dict(songs_sorted=0, albums_processed=1, artist_folders_created=0, top_artists=[[ARTIST, 4]]),
                    effect_receipts=[dict(kind=kind) for _ in range(5) for kind in ('file_published', 'source_retired')])

    def test_observed_physical_and_legacy_counts_are_separate(self):
        validate_effect_counts(self.fixture(), 5, False)
        mixed = self.fixture(); mixed.update(source_items=6, committed_destinations=6, moved_sources=6)
        mixed['legacy_summary'].update(songs_sorted=1, artist_folders_created=1, top_artists=[[ARTIST, 5]])
        mixed['effect_receipts'] += [dict(kind='file_published'), dict(kind='source_retired')]
        validate_effect_counts(mixed, 6, True)

    def test_missing_effects_or_wrong_counter_refuse(self):
        original = self.fixture()
        for key in ('source_items', 'committed_destinations', 'moved_sources', 'albums_completed'):
            actual = copy.deepcopy(original); actual[key] += 1
            with self.assertRaises(ValueError): validate_effect_counts(actual, 5, False)
        for kind in ('file_published', 'source_retired'):
            actual = copy.deepcopy(original); actual['effect_receipts'].remove(dict(kind=kind))
            with self.assertRaises(ValueError): validate_effect_counts(actual, 5, False)
        actual = copy.deepcopy(original); actual['effect_receipts'].append(dict(kind='file_renamed'))
        with self.assertRaises(ValueError): validate_effect_counts(actual, 5, False)

    def test_cleanup_requires_exact_independent_reference_and_full_identity_set(self):
        first = [windows_reference('C:\\owned\\אלבום'), {'Windows': {'volume': 7, 'file_id': list(range(16))}}]
        nested = [windows_reference('C:\\owned\\אלבום\\nested'), {'Windows': {'volume': 7, 'file_id': list(range(1, 17))}}]
        expected = [first, nested]
        require_cleanup_bindings({'removed_directory_references': [nested, first]}, expected)
        wrong_id = copy.deepcopy(nested); wrong_id[1]['Windows']['file_id'][15] += 1
        wrong_path = copy.deepcopy(nested); wrong_path[0] = windows_reference('C:\\other\\nested')
        for recorded in ([first], [first, nested, first], [first, nested, wrong_path], [first, wrong_id], [first, wrong_path], []):
            with self.assertRaises(ValueError):
                require_cleanup_bindings({'removed_directory_references': recorded}, expected)

    def test_selected_reference_keeps_utf16_units_and_literal_spelling(self):
        self.assertEqual(windows_reference('a🎵'), {'WindowsPath': {'utf16': [97, 0xD83C, 0xDFB5]}})
        self.assertNotEqual(windows_reference('C:\\OWNED'), windows_reference('C:\\owned'))

    def test_recovery_uncertainty_is_not_success(self):
        actual = self.fixture(); actual['recovery_error'] = 'VERIFICATION_CANCELLED'
        with self.assertRaises(ValueError): validate_effect_counts(actual, 5, False)

    def test_identity_bijection_rejects_duplicate_missing_or_premature_effects(self):
        source = {'Test': {'id': 'source'}}; destination = {'Test': {'id': 'destination'}}
        publication = dict(kind='file_published', identity=destination, operation=3)
        retirement = dict(kind='source_retired', identity=source, operation=3)
        actual = dict(effect_receipts=[publication, retirement],
                      transfers=dict(items=[dict(status='complete', operation=3, observations=[dict(identity=destination)])]))
        require_effect_bindings(actual, [(source, destination)])
        for effects in ([], [publication], [retirement, publication], [publication, retirement, retirement]):
            changed = copy.deepcopy(actual); changed['effect_receipts'] = effects
            with self.assertRaises(ValueError): require_effect_bindings(changed, [(source, destination)])
        changed = copy.deepcopy(actual); changed['transfers']['items'] = []
        with self.assertRaises(ValueError): require_effect_bindings(changed, [(source, destination)])



if __name__ == '__main__':
    unittest.main()
