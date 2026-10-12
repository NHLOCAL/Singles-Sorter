"""Expected Partial keeps its ordinary caution; uncertain verification cannot pass."""
from pathlib import Path
import unittest

from desktop_repair import assert_repair_terminal_notice, REPAIR_PARTIAL_CAUTION


class RepairTerminalNoticeTests(unittest.TestCase):
    def state(self, partial):
        return dict(hidden=not partial, visible=partial, text=REPAIR_PARTIAL_CAUTION,
                    title='הביצוע הושלם חלקית' if partial else 'הביצוע הושלם')

    def test_complete_has_no_visible_warning(self):
        actual = self.state(False)
        self.assertEqual(assert_repair_terminal_notice(lambda _: actual, False), actual)
        for changed in [dict(hidden=False), dict(visible=True), dict(title='הקבצים טופלו; נדרשת בדיקה')]:
            with self.assertRaises(ValueError):
                assert_repair_terminal_notice(lambda _: {**actual, **changed}, False)

    def test_expected_partial_requires_exact_visible_source_backup_caution(self):
        actual = self.state(True)
        self.assertEqual(assert_repair_terminal_notice(lambda _: actual, True), actual)
        for changed in [dict(hidden=True), dict(visible=False), dict(text=''), dict(title='הביצוע הושלם'),
                        dict(text='הקבצים טופלו, אך בדיקת התוצאות לא הושלמה.')]:
            with self.assertRaises(ValueError):
                assert_repair_terminal_notice(lambda _: {**actual, **changed}, True)

    def test_malformed_notice_cannot_satisfy_either_state(self):
        for partial in [False, True]:
            for actual in [None, {}, {**self.state(partial), 'hidden': 0},
                           {**self.state(partial), 'visible': 1}, {**self.state(partial), 'text': None}]:
                with self.assertRaises(ValueError):
                    assert_repair_terminal_notice(lambda _: actual, partial)

    def test_all_three_unsupported_format_cases_bind_and_record_actual_notice(self):
        root = Path(__file__).parent
        for name, condition in [('desktop_mp4.py', 'opaque'),
                                ('desktop_id3_container.py', 'unsupported'),
                                ('desktop_wavpack.py', 'readonly')]:
            source = (root / name).read_text(encoding='utf-8')
            self.assertIn('terminal_notice = assert_repair_terminal_notice(script, ' + condition + ')', source)
            self.assertIn('terminal_notice=terminal_notice', source)
            self.assertIn('assert after == before and not backups', source)
            self.assertIn("assert 'בוצע חלקית'", source)
            self.assertNotIn('assert script("return document.querySelector(\'#execution-recovery\').hidden")', source)
        main = (root.parent / 'src/main.ts').read_text(encoding='utf-8')
        self.assertIn(REPAIR_PARTIAL_CAUTION, main)
        self.assertIn("['needs_recovery','partial','cancelled','failed'].includes(e.status)", main)


if __name__ == '__main__':
    unittest.main()
