"""Verify the pinned, minimal offline Fluent SVG subset and shipped license."""
import hashlib
import json
from pathlib import Path
import re
import unittest

ROOT=Path(__file__).resolve().parents[1]
class OfflineIcons(unittest.TestCase):
    def test_selected_assets_are_exact_pinned_svg_files(self):
        folder=ROOT/'src/assets/fluent'
        receipt=json.loads((folder/'PROVENANCE.json').read_text())
        self.assertEqual(receipt['package'],'@fluentui/svg-icons')
        self.assertEqual(receipt['version'],'1.1.344')
        files=list(folder.glob('*.svg'))
        self.assertEqual(len(files),16)
        for path in files:
            raw=path.read_bytes()
            self.assertEqual(hashlib.sha256(raw).hexdigest(),receipt['files'][path.name])
            self.assertIsNone(re.search(r'<script|<foreignObject|\bon\w+=|(?:href|src)=',raw.decode(),re.I))
    def test_full_microsoft_mit_license_is_shipped(self):
        text=(ROOT/'public/licenses/fluent-system-icons-MIT.txt').read_text()
        for required in ['Copyright (c) 2020 Microsoft Corporation','Permission is hereby granted','THE SOFTWARE IS PROVIDED "AS IS"']:
            self.assertIn(required,text)
if __name__=='__main__':unittest.main()
