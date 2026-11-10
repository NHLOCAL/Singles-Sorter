import importlib.util
import json
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
spec = importlib.util.spec_from_file_location("sign_apk", HERE / "sign_apk.py")
signing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signing)


class ReleaseTests(unittest.TestCase):
    def test_android_identity_and_version_are_the_release(self):
        release = json.loads((ROOT / "release.json").read_text(encoding="utf-8"))
        manifest = ET.parse(HERE / "AndroidManifest.xml").getroot()
        ns = "{http://schemas.android.com/apk/res/android}"
        self.assertEqual(manifest.attrib["package"], release["android_package"])
        self.assertEqual(manifest.attrib[ns + "versionName"], release["version"])
        self.assertEqual(int(manifest.attrib[ns + "versionCode"]), release["android_version_code"])
        self.assertGreater(release["android_version_code"], 1)
        ui = json.loads((ROOT / "native/apps/singles-ui/package.json").read_text(encoding="utf-8"))
        self.assertEqual(ui["version"], release["version"])

    def test_signer_parser_rejects_missing_and_multiple_signers(self):
        digest = "a" * 64
        line = f"Signer #1 certificate SHA-256 digest: {digest}"
        self.assertEqual(signing.certificate(line), digest)
        for output in ("", line + "\n" + line.replace("#1", "#2")):
            with self.assertRaises(ValueError):
                signing.certificate(output)


if __name__ == "__main__":
    unittest.main()
