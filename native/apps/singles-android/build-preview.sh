#!/usr/bin/env bash
# Compatibility entry point. The portable builder owns validation and packaging.
# Required inputs: ANDROID_HOME JAVA_HOME MODEL_BUNDLE NATIVE_LIB_X86_64 UI_DIST
# LICENSES_DIR OUT SINGLES_SIGNING_MODE SINGLES_KEYSTORE SINGLES_KEY_ALIAS
# SINGLES_STORE_PASSWORD SINGLES_KEY_PASSWORD. NATIVE_LIB_ARM64 is optional for QA.
set -euo pipefail
umask 077
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
exec python3 "$SCRIPT_DIR/build_apk.py"
