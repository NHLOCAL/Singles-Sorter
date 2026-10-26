"""Portable receipt helpers, not physical Windows acceptance."""
import unittest
from desktop_cross_albums import displayed_counts, STAMP, CHECKS


class CrossAlbumHelpers(unittest.TestCase):
    def test_counters_are_parsed_from_actual_display(self):
        self.assertEqual(displayed_counts('6 יעדים נכתבו · 6 קבצי מקור הועברו · 0 נכשלו · 0 לא בוצעו'),
                         dict(committed_destinations=6, moved_sources=6, failed_operations=0, blocked_operations=0))
        self.assertEqual(displayed_counts('1 יעדים נכתבו · 0 קבצי מקור הועברו · 2 נכשלו · 3 לא בוצעו')['failed_operations'], 2)

    def test_missing_or_nonterminal_counter_text_is_not_fabricated(self):
        for text in ('', 'הביצוע הושלם', '6 יעדים נכתבו', '6 יעדים נכתבו · 6 קבצי מקור הועברו · 0 נכשלו · 0 לא בוצעו extra'):
            with self.assertRaises(ValueError):
                displayed_counts(text)

    def test_every_real_cross_album_check_is_mandatory_for_package(self):
        import importlib.util
        from pathlib import Path
        path = Path(__file__).resolve().parents[3] / 'packaging/windows-desktop-preview/package_desktop.py'
        spec = importlib.util.spec_from_file_location('cross_album_package', path)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        self.assertEqual(CHECKS, module.CROSS_ALBUM_CHECKS)

    def test_fixed_timestamp_is_ntfs_representable(self):
        self.assertEqual(STAMP % 100, 0)


if __name__ == '__main__':
    unittest.main()
