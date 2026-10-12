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

    def test_narrow_and_touch_dialog_icons_override_compact_icon_size(self):
        root = Path(__file__).resolve().parents[1]
        css = (root / 'src/styles.css').read_text(encoding='utf-8')
        # A bare button selector loses to the compact .icon-button class.
        # Keep this structural check in addition to the actual WebView 48px gate.
        for query in ['@media (max-width: 740px) {', '@media (pointer: coarse) {']:
            section = css.split(query, 1)[1].split('@media', 1)[0]
            self.assertRegex(section, r'dialog \.icon-button\s*\{\s*min-width: 48px;\s*min-height: 48px;\s*\}')

if __name__ == '__main__':
    unittest.main()
