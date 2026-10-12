"""Settings placement contract; actual rendering remains a native CI gate."""
from pathlib import Path
import unittest

class SettingsContract(unittest.TestCase):
    def test_developer_mode_is_only_in_advanced_settings(self):
        root = Path(__file__).resolve().parents[1]
        source = (root / 'src/template.ts').read_text(encoding='utf-8')
        about = source.split('<dialog id="menu-dialog"', 1)[1].split('</dialog>', 1)[0]
        advanced = source.split('<dialog id="advanced-dialog"', 1)[1].split('</dialog>', 1)[0]
        self.assertNotIn('id="developer-mode"', about)
        self.assertIn('id="developer-mode"', advanced)
        self.assertNotIn('—', source)
        native = (root / 'tests/desktop_polish.py').read_text(encoding='utf-8')
        self.assertIn("'01-native-display-settings'", native)
        self.assertIn("#menu-dialog #developer-mode", native)

if __name__ == '__main__':
    unittest.main()
