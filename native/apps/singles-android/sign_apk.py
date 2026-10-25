"""Sign an aligned APK with an explicitly supplied, durable identity.

Passwords are read by apksigner from environment variables, never command arguments.
Release mode requires the pinned permanent product certificate. QA signing is
explicit and never implies compatibility with installations from the release.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]


def certificate(output):
    found = re.findall(r"Signer #\d+ certificate SHA-256 digest: ([0-9a-fA-F]{64})", output)
    if len(found) != 1:
        raise ValueError("Expected exactly one APK signer")
    return found[0].lower()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--apksigner", required=True)
    parser.add_argument("--qa", action="store_true")
    args = parser.parse_args()
    config = json.loads((ROOT / "release.json").read_text(encoding="utf-8"))
    for name in ("SINGLES_KEYSTORE", "SINGLES_KEY_ALIAS", "SINGLES_STORE_PASSWORD", "SINGLES_KEY_PASSWORD"):
        if not os.environ.get(name):
            parser.error(f"{name} must be set; ephemeral signing is not supported")
    if args.output.exists():
        parser.error("Output already exists")
    if not Path(os.environ["SINGLES_KEYSTORE"]).is_file():
        parser.error("Signing keystore does not exist")
    # Verify before accepting the final artifact; a wrong key never produces a
    # distributable release filename. Preserve a rejected candidate for diagnosis.
    candidate = args.output.with_suffix(".candidate.apk")
    if candidate.exists():
        parser.error("Signing candidate already exists")
    subprocess.run([args.apksigner, "sign", "--ks", os.environ["SINGLES_KEYSTORE"],
                    "--ks-key-alias", os.environ["SINGLES_KEY_ALIAS"],
                    "--ks-pass", "env:SINGLES_STORE_PASSWORD", "--key-pass", "env:SINGLES_KEY_PASSWORD",
                    "--v4-signing-enabled", "false", "--out", str(candidate), str(args.input)], check=True)
    result = subprocess.run([args.apksigner, "verify", "--verbose", "--print-certs", str(candidate)],
                            capture_output=True, text=True, check=True)
    digest = certificate(result.stdout)
    compatible = digest == config["legacy_android_certificate_sha256"]
    if not args.qa and digest != config["android_certificate_sha256"]:
        raise SystemExit("Signing certificate differs from permanent product identity; release rejected. Use --qa only for device testing.")
    candidate.rename(args.output)
    receipt = {"version": config["version"], "qa_only": args.qa,
               "legacy_update_signature_compatible": compatible, "certificate_sha256": digest,
               "apk_sha256": hashlib.sha256(args.output.read_bytes()).hexdigest()}
    args.output.with_suffix(".signing.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
