"""Structural regression; actual disclosure layout remains a native CI gate."""
from pathlib import Path
import unittest


class DisclosureContract(unittest.TestCase):
    def test_closed_content_has_no_layout_but_summary_remains_available(self):
        root = Path(__file__).resolve().parents[1]
        styles = (root / 'src/styles.css').read_text(encoding='utf-8')
        self.assertIn('.developer-options:not([open]) > :not(summary){display:none}', styles)
        native = (root / 'tests/desktop_polish.py').read_text(encoding='utf-8')
        self.assertIn("'01-native-about'", native)
        self.assertIn("'01-native-developer-options'", native)
        self.assertEqual(native.count("#developer-mode').getClientRects().length===0"), 2)


if __name__ == '__main__':
    unittest.main()
