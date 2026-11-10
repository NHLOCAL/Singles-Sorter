"""Fail a release run unless every product manifest names the same stable version."""
import json
import os
from pathlib import Path
import re
import sys
import tomllib
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]


def problems(root, ref_type="", ref_name=""):
    release = json.loads((root / "release.json").read_text(encoding="utf-8"))
    version = release["version"]
    found = []
    if not re.fullmatch(r"\d+\.\d+\.\d+", version) or release["channel"] != "stable":
        found.append(f"not a stable identity: {version} / {release['channel']}")
    native = root / "native"
    manifest = ET.parse(native / "apps/singles-android/AndroidManifest.xml").getroot()
    ns = "{http://schemas.android.com/apk/res/android}"
    versions = {
        "Android manifest": manifest.get(ns + "versionName"),
        "frontend package": json.loads((native / "apps/singles-ui/package.json").read_text(encoding="utf-8"))["version"],
        "Tauri config": json.loads((native / "apps/singles-ui/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))["version"],
        "desktop crate": tomllib.loads((native / "apps/singles-ui/src-tauri/Cargo.toml").read_text(encoding="utf-8"))["package"]["version"],
        "workspace": tomllib.loads((native / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"],
    }
    found += [f"{name} is {value}, expected {version}" for name, value in versions.items() if value != version]
    if manifest.get(ns + "versionCode") != str(release["android_version_code"]):
        found.append("Android version code disagrees with release.json")
    if ref_type == "tag" and ref_name != f"v{version}":
        found.append(f"tag {ref_name} does not match v{version}")
    return found, release


def main():
    found, release = problems(ROOT, os.environ.get("GITHUB_REF_TYPE", ""), os.environ.get("GITHUB_REF_NAME", ""))
    if found:
        sys.exit("\n".join(found))
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as stream:
            stream.write(f"version={release['version']}\n")
            stream.write(f"app_id={release['windows_installer_app_id']}\n")
            stream.write(f"certificate={release['android_certificate_sha256']}\n")
    print(f"Release identity {release['version']} is consistent")


if __name__ == "__main__":
    main()
