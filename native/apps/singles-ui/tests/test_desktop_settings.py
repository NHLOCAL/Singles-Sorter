"""Portable helper contracts; actual controls and dialogs still require Windows."""
import csv
from pathlib import Path
import tempfile
import unittest

from desktop_modes import SORT_CONTROLS_READY, SORT_TAB_READY
from desktop_settings import MAIN_SETTINGS, editor_rows, read_main_settings, set_main_settings, verify_export, require_export_notice


class SettingsHelpers(unittest.TestCase):
    def test_five_main_settings_require_exact_boolean_shape(self):
        expected = dict.fromkeys(MAIN_SETTINGS, False)
        self.assertEqual(read_main_settings(lambda _: expected), expected)
        for invalid in [{}, {**expected, 'duet_mode': False}, {**expected, 'copy_mode': 0}]:
            with self.assertRaises(ValueError):
                read_main_settings(lambda _: invalid)

    def test_setting_edits_wait_for_real_sort_controls_before_each_draft(self):
        values = dict.fromkeys(MAIN_SETTINGS, False)
        mode = ['fix']
        events = []
        def script(source):
            if source == SORT_TAB_READY: return True
            if source == SORT_CONTROLS_READY: return mode[0] == 'sort'
            return dict(values)
        def click(selector):
            events.append(selector)
            if selector == '#tab-mode-sort': mode[0] = 'sort'
            else:
                self.assertEqual(mode[0], 'sort')
                values[selector[1:]] = not values[selector[1:]]
        def wait(predicate): self.assertTrue(predicate())
        expected = dict.fromkeys(MAIN_SETTINGS, True)
        set_main_settings(script, click, wait, expected)
        self.assertEqual(events, ['#tab-mode-sort'] + ['#' + key for key in MAIN_SETTINGS])
        self.assertEqual(values, expected)
        events.clear()
        set_main_settings(script, click, wait, expected)
        self.assertEqual(events, ['#tab-mode-sort'])
        self.assertEqual(values, expected)

    def editor(self, pages):
        current = [0]
        def script(_):
            return pages[current[0]]
        def click(selector):
            self.assertEqual(selector, '#list-next')
            current[0] += 1
        def wait(predicate):
            self.assertTrue(predicate())
        return editor_rows(script, click, wait)

    def test_editor_reads_all_pages_and_keeps_duplicate_order(self):
        rows = [['אמן', 'תיקייה'], ['אמן', 'תיקייה']] + [[str(i), f'שם {i}'] for i in range(51)]
        pages = [{'rows': [[i, *row] for i, row in enumerate(rows[:50])], 'malformed': 0, 'next_disabled': False},
                 {'rows': [[i, *row] for i, row in enumerate(rows[50:], 50)], 'malformed': 0, 'next_disabled': True}]
        self.assertEqual(self.editor(pages), rows)

    def test_editor_rejects_repeated_or_skipped_indices_and_hidden_extra_columns(self):
        for rows in [[[1, 'a', 'b']], [[0, 'a', 'b'], [0, 'a', 'b']]]:
            with self.assertRaises(ValueError):
                self.editor([{'rows': rows, 'malformed': 0, 'next_disabled': True}])
        with self.assertRaises(ValueError):
            self.editor([{'rows': [[0, 'a', 'b']], 'malformed': 1, 'next_disabled': True}])

    def test_editor_empty_is_valid_only_when_paging_is_finished(self):
        self.assertEqual(self.editor([{'rows': [], 'malformed': 0, 'next_disabled': True}]), [])
        with self.assertRaises(ValueError):
            self.editor([{'rows': [], 'malformed': 0, 'next_disabled': False}])

    def test_export_requires_exact_returned_path_before_reading_file(self):
        path = Path('owned') / 'רשימה.csv'
        require_export_notice(path, 'הרשימה יוצאה: ' + str(path))
        for notice in ['', 'הרשימה יוצאה: C:\\Users\\runneradmin\\Documents\\personal-singer-list.csv',
                       'הרשימה יוצאה: ' + str(path) + '.csv']:
            with self.assertRaisesRegex(ValueError, 'different selected path'):
                require_export_notice(path, notice)

    def test_export_preserves_utf8_order_duplicates_and_quoting(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'ייצוא.csv'
            rows = [['אמן, אחד', 'שם "תיקייה"'], ['אמן, אחד', 'שם "תיקייה"'], ['שורה\nשנייה', 'סוף']]
            with path.open('w', encoding='utf-8', newline='') as stream:
                csv.writer(stream, lineterminator='\r\n').writerows(rows)
            receipt = verify_export(path, rows)
            self.assertEqual(receipt['rows'], rows)
            self.assertEqual(receipt['bytes'], path.stat().st_size)
            self.assertEqual(len(receipt['sha256']), 64)
            with self.assertRaises(ValueError):
                verify_export(path, rows[::-1])
            (Path(tmp) / 'unexpected').mkdir()
            with self.assertRaises(ValueError):
                verify_export(path, rows)

    def test_runtime_exercises_save_draft_duet_restart_export_and_restore(self):
        source = Path(__file__).with_name('desktop.py').read_text(encoding='utf-8')
        block = source.split('settings_baseline = read_main_settings(script)', 1)[1].split("checks.extend(exercise_result_stability", 1)[0]
        order = ["set_main_settings(script, click, wait, expected_settings)",
                 "click('#save-settings')", "set_main_settings(script, click, wait, settings_baseline)",
                 "click('#duet-all' if expected_duet else '#duet-first')",
                 "restart_app(BINARY, 'restarted-app')", 'assert observed_settings == expected_settings',
                 "pick_owned_path('#export-list', exported_csv, save=True)", 'verify_export(exported_csv, reopened_personal)',
                 'set_main_settings(script, click, wait, settings_baseline)', "click('#save-settings')",
                 'settings_checks = list(SETTINGS_CHECKS)']
        for step in order:
            self.assertIn(step, block)
            block = block.split(step, 1)[1]

    def test_export_refuses_wrong_line_endings(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'export.csv'
            path.write_bytes(b'a,b\n')
            with self.assertRaises(ValueError):
                verify_export(path, [['a', 'b']])


if __name__ == '__main__':
    unittest.main()
