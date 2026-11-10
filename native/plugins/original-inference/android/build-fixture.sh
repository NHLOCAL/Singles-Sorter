#!/usr/bin/env bash
# Build the owned original-inference fixture APK from existing local inputs only.
# No Gradle, downloads, emulator operations, production credentials, or cleanup.
# OUT must be a new directory outside this repository; failed output is retained.
#
# Example (add NATIVE_LIB_ARM64=/absolute/path/libsingles_android_inference.so
# to include arm64-v8a in the same APK):
#   ANDROID_HOME=/workspace/shared/toolchains/android-sdk \
#   JAVA_HOME=/workspace/shared/toolchains/android-jdk/usr/lib/jvm/java-21-openjdk-amd64 \
#   MODEL_BUNDLE=/workspace/shared/verified-desktop-preview/extracted/models/original \
#   NATIVE_LIB_X86_64=/absolute/path/libsingles_android_inference.so \
#   OUT=/workspace/shared/original-inference-fixture-build-1 \
#   bash native/plugins/original-inference/android/build-fixture.sh
#
# Result: $OUT/original-inference-fixture.apk, fixture-only.p12, and inputs.json.
# Signing uses the public test password "android" and a newly generated local
# fixture identity. Never distribute this fixture or use its key for production.
# Build Tools 35.0.0 and an installed Android platform >= API 30 are required.
set -euo pipefail
umask 077

fail() { printf 'build-fixture: %s\n' "$*" >&2; exit 1; }
: "${ANDROID_HOME:?Set ANDROID_HOME to the existing Android SDK}"
: "${JAVA_HOME:?Set JAVA_HOME to an existing JDK with javac, jar and keytool}"
: "${MODEL_BUNDLE:?Set MODEL_BUNDLE to the verified original five-file bundle}"
: "${NATIVE_LIB_X86_64:?Set NATIVE_LIB_X86_64 to the built x86_64 JNI library}"
: "${OUT:?Set OUT to a fresh absolute directory outside the repository}"
NATIVE_LIB_ARM64=${NATIVE_LIB_ARM64:-}
export JAVA_HOME
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/../../../.." && pwd -P)
BUILD_TOOLS="$ANDROID_HOME/build-tools/35.0.0"
command -v python3 >/dev/null || fail 'python3 is required'
for tool in aapt2 d8 zipalign apksigner; do
    [[ -x "$BUILD_TOOLS/$tool" ]] || fail "Missing executable: $BUILD_TOOLS/$tool"
done
for tool in java javac jar keytool; do
    [[ -x "$JAVA_HOME/bin/$tool" ]] || fail "Missing executable: $JAVA_HOME/bin/$tool"
done
[[ -f "$SCRIPT_DIR/AndroidManifest.xml" ]] || fail 'AndroidManifest.xml is missing'

# Validate every input before creating OUT. Select an installed stable platform;
# compiling against a newer platform does not raise the manifest or DEX min API.
ANDROID_JAR=$(python3 - "$ANDROID_HOME" <<'PY'
from pathlib import Path
import re, sys
platforms = []
for jar in (Path(sys.argv[1]) / "platforms").glob("android-*/android.jar"):
    match = re.fullmatch(r"android-(\d+)(?:\.(\d+))?", jar.parent.name)
    if match and int(match[1]) >= 30:
        platforms.append(((int(match[1]), int(match[2] or 0)), jar.resolve()))
if not platforms:
    raise SystemExit("No installed stable Android platform >= API 30")
print(max(platforms)[1])
PY
)
OUT=$(python3 - "$OUT" "$REPO_ROOT" "$MODEL_BUNDLE" "$NATIVE_LIB_X86_64" "$NATIVE_LIB_ARM64" <<'PY'
from pathlib import Path
import hashlib, sys
raw_out, root, bundle, x86, arm = sys.argv[1:]
if not Path(raw_out).is_absolute():
    raise SystemExit("OUT must be an absolute path")
out = Path(raw_out).resolve()
root = Path(root).resolve()
if out == root or root in out.parents:
    raise SystemExit("OUT must be outside the repository")
if out.exists() or Path(raw_out).is_symlink():
    raise SystemExit("OUT must be a fresh directory; existing output is never removed")
# These pins match OriginalAssets.java; only the original released bytes qualify.
pins = [
    ("classifier-v14.vocab.txt", 1268235, "35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3"),
    ("classifier-v14.weights.bin", 3192809, "aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33"),
    ("manifest.json", 32600, "3fb1801718d84c335b31cdae18ca94daf6af821fb29c315bcf485de3810b5580"),
    ("tensors.f32", 3829976, "84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745"),
    ("tokenizer.json", 73348, "33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574"),
]
for name, size, digest in pins:
    path = Path(bundle) / name
    if not path.is_file() or path.stat().st_size != size:
        raise SystemExit(f"Original asset missing or wrong size: {name}")
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise SystemExit(f"Original asset hash mismatch: {name}")
for abi, source, machine in [("x86_64", x86, 62), ("arm64-v8a", arm, 183)]:
    if not source:
        continue
    path = Path(source)
    if not path.is_file():
        raise SystemExit(f"Native library missing: {abi}: {path}")
    with path.open("rb") as stream:
        header = stream.read(20)
    if (len(header) != 20 or header[:6] != b"\x7fELF\x02\x01"
            or int.from_bytes(header[16:18], "little") != 3
            or int.from_bytes(header[18:20], "little") != machine):
        raise SystemExit(f"Expected a 64-bit little-endian shared ELF for {abi}: {path}")
print(out)
PY
)

mapfile -d '' JAVA_SOURCES < <(find "$SCRIPT_DIR/src" -type f -name '*.java' -print0 | sort -z)
[[ ${#JAVA_SOURCES[@]} -gt 0 ]] || fail 'No Java sources found'
mkdir -p -- "$(dirname -- "$OUT")"
mkdir -- "$OUT"
trap 'printf "Build failed; partial output preserved at %s\n" "$OUT" >&2' ERR
mkdir -- "$OUT/build" "$OUT/build/classes" "$OUT/build/dex"
printf 'Android platform: %s\nOutput: %s\n' "$ANDROID_JAR" "$OUT"

# --release 8 supplies the Java lambda compiler bootstrap API. Using android.jar
# as javac -bootclasspath instead fails because LambdaMetafactory is absent.
"$JAVA_HOME/bin/javac" --release 8 -Xlint:all,-options -Werror \
    -cp "$ANDROID_JAR" -d "$OUT/build/classes" "${JAVA_SOURCES[@]}"
"$JAVA_HOME/bin/jar" --create --file "$OUT/build/classes.jar" -C "$OUT/build/classes" .
"$BUILD_TOOLS/d8" --min-api 30 --lib "$ANDROID_JAR" \
    --output "$OUT/build/dex" "$OUT/build/classes.jar"
"$BUILD_TOOLS/aapt2" link --manifest "$SCRIPT_DIR/AndroidManifest.xml" \
    -I "$ANDROID_JAR" -o "$OUT/build/resources.apk"

python3 - "$OUT" "$MODEL_BUNDLE" "$NATIVE_LIB_X86_64" "$NATIVE_LIB_ARM64" "$ANDROID_JAR" <<'PY'
from pathlib import Path
from zipfile import ZipFile, ZIP_STORED
import hashlib, json, shutil, sys
out, bundle, x86, arm, platform = sys.argv[1:]
out, bundle = Path(out), Path(bundle)
shutil.copyfile(out / "build/resources.apk", out / "build/unsigned.apk")
entries = [(p, p.name) for p in sorted((out / "build/dex").glob("*.dex"))]
for name in ["classifier-v14.vocab.txt", "classifier-v14.weights.bin", "manifest.json", "tensors.f32", "tokenizer.json"]:
    entries.append((bundle / name, "assets/original/" + name))
entries.append((Path(x86), "lib/x86_64/libsingles_android_inference.so"))
if arm:
    entries.append((Path(arm), "lib/arm64-v8a/libsingles_android_inference.so"))
receipt = {"fixture_only": True, "build_tools": "35.0.0", "android_jar": platform, "entries": []}
with ZipFile(out / "build/unsigned.apk", "a") as apk:
    for source, name in entries:
        data = source.read_bytes()
        apk.writestr(name, data, compress_type=ZIP_STORED)
        receipt["entries"].append({"entry": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
(out / "inputs.json").write_text(json.dumps(receipt, indent=2) + "\n")
PY

"$BUILD_TOOLS/zipalign" -P 16 4 "$OUT/build/unsigned.apk" "$OUT/build/aligned.apk"
"$JAVA_HOME/bin/keytool" -genkeypair -noprompt -keystore "$OUT/fixture-only.p12" \
    -storetype PKCS12 -storepass android -keypass android -alias fixture-only \
    -keyalg RSA -keysize 2048 -validity 3650 \
    -dname 'CN=Owned Inference Fixture,OU=Test Only,O=Local Fixture,C=ZZ'
"$BUILD_TOOLS/apksigner" sign --ks "$OUT/fixture-only.p12" \
    --ks-key-alias fixture-only --ks-pass pass:android --key-pass pass:android \
    --v4-signing-enabled false --out "$OUT/original-inference-fixture.apk" "$OUT/build/aligned.apk"
"$BUILD_TOOLS/zipalign" -c -P 16 -v 4 "$OUT/original-inference-fixture.apk"
"$BUILD_TOOLS/apksigner" verify --verbose --print-certs "$OUT/original-inference-fixture.apk"
"$BUILD_TOOLS/aapt2" dump badging "$OUT/original-inference-fixture.apk"
printf '\nFixture APK: %s\n' "$OUT/original-inference-fixture.apk"
