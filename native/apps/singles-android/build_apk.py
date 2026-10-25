"""Offline Android packaging on Windows/Linux using explicit, verified inputs.

Native libraries must come from build-native.sh for this source revision. This
builder does not cross-compile Rust. QA output always carries a signing receipt.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET
from zipfile import ZipFile, ZIP_STORED, ZIP_DEFLATED

from pin_original_manifest import generate, TEMPLATE

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def run(*command):
    subprocess.run([str(p) for p in command], check=True)


def tool(directory, name):
    suffix = (".bat" if name in ("d8", "apksigner") else ".exe") if os.name == "nt" else ""
    result = directory / (name + suffix)
    if not result.is_file():
        raise ValueError(f"Missing build tool: {result}")
    return result


def main():
    required = ("ANDROID_HOME", "JAVA_HOME", "MODEL_BUNDLE", "NATIVE_LIB_X86_64",
                "UI_DIST", "LICENSES_DIR", "OUT", "SINGLES_SIGNING_MODE",
                "SINGLES_KEYSTORE", "SINGLES_KEY_ALIAS", "SINGLES_STORE_PASSWORD", "SINGLES_KEY_PASSWORD")
    for name in required:
        if not os.environ.get(name):
            raise ValueError(f"Set {name}")
    mode = os.environ["SINGLES_SIGNING_MODE"]
    if mode not in ("qa", "release"):
        raise ValueError("SINGLES_SIGNING_MODE must be qa or release")
    out = Path(os.environ["OUT"]).resolve()
    if out.exists() or out == ROOT or ROOT in out.parents:
        raise ValueError("OUT must be a fresh directory outside the repository")
    sdk, jdk = (Path(os.environ[k]) for k in ("ANDROID_HOME", "JAVA_HOME"))
    build_tools = sdk / "build-tools/35.0.0"
    tools = {name: tool(build_tools, name) for name in ("aapt2", "d8", "zipalign", "apksigner")}
    javac, jar = (tool(jdk / "bin", name) for name in ("javac", "jar"))
    platform = sdk / "platforms/android-35/android.jar"
    if not platform.is_file():
        raise ValueError("Install platforms;android-35")
    bundle = Path(os.environ["MODEL_BUNDLE"])
    generated = generate(bundle / "manifest.json", TEMPLATE.read_text(encoding="utf-8"))
    # Verify the original payload pins independently of a supplied manifest.
    pins = {
        "classifier-v14.vocab.txt": "35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3",
        "classifier-v14.weights.bin": "aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33",
        "tensors.f32": "84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745",
        "tokenizer.json": "33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574",
    }
    assets = []
    for name, digest in pins.items():
        data = (bundle / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError(f"Original asset hash mismatch: {name}")
        assets.append(("assets/original/" + name, data, ZIP_STORED))
    assets.append(("assets/original/manifest.json", (bundle / "manifest.json").read_bytes(), ZIP_STORED))
    for abi, variable, machine in (("x86_64", "NATIVE_LIB_X86_64", 62), ("arm64-v8a", "NATIVE_LIB_ARM64", 183)):
        source = os.environ.get(variable)
        if not source:
            continue
        data = Path(source).read_bytes()
        if data[:6] != b"\x7fELF\x02\x01" or int.from_bytes(data[16:18], "little") != 3 or int.from_bytes(data[18:20], "little") != machine:
            raise ValueError(f"Invalid shared native library for {abi}")
        assets.append((f"lib/{abi}/libsingles_android_inference.so", data, ZIP_STORED))
    sys.path.insert(0, str(ROOT / "native/packaging/licenses"))
    from collect_android_licenses import verified_snapshot
    for name, data in verified_snapshot(Path(os.environ["LICENSES_DIR"])):
        assets.append(("assets/licenses/" + name, data, ZIP_DEFLATED))
    ui = Path(os.environ["UI_DIST"])
    if not (ui / "index.html").is_file():
        raise ValueError("Build Android-mode UI_DIST first")
    for path in sorted(ui.rglob("*")):
        if path.is_symlink():
            raise ValueError("UI symlinks refused")
        if path.is_file():
            assets.append(("assets/ui/" + path.relative_to(ui).as_posix(), path.read_bytes(), ZIP_STORED))
    out.mkdir(parents=True)
    classes, dex = out / "build/classes", out / "build/dex"
    classes.mkdir(parents=True)
    dex.mkdir()
    generated_path = out / "build/OriginalAssets.java"
    generated_path.write_text(generated, encoding="utf-8")
    sources = sorted((HERE / "src").rglob("*.java"))
    sources += sorted((ROOT / "native/plugins/singles-platform/android/src/main/java").rglob("*.java"))
    sources += [generated_path, ROOT / "native/plugins/original-inference/android/src/com/nhlocal/singlesorter/inference/OriginalInference.java"]
    argfile = out / "build/sources.txt"
    argfile.write_text("\n".join('"' + p.as_posix() + '"' for p in sources), encoding="utf-8")
    run(javac, "--release", "8", "-encoding", "UTF-8", "-Xlint:all,-options", "-Werror", "-cp", platform, "-d", classes, "@" + str(argfile))
    run(jar, "--create", "--file", out / "build/classes.jar", "-C", classes, ".")
    run(tools["d8"], "--min-api", "29", "--lib", platform, "--output", dex, out / "build/classes.jar")
    unsigned = out / "build/unsigned.apk"
    manifest = HERE / "AndroidManifest.xml"
    if mode == "qa":
        tree = ET.parse(manifest)
        tree.getroot().find("application").set("{http://schemas.android.com/apk/res/android}debuggable", "true")
        manifest = out / "build/AndroidManifest.xml"
        tree.write(manifest, encoding="utf-8", xml_declaration=True)
    run(tools["aapt2"], "link", "--manifest", manifest, "-I", platform, "-o", unsigned)
    for path in sorted(dex.glob("*.dex")):
        assets.append((path.name, path.read_bytes(), ZIP_STORED))
    receipt = {"source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
               "runtime_acceptance": "pending-exact-apk-device-validation", "entries": []}
    with ZipFile(unsigned, "a") as apk:
        for name, data, compression in assets:
            apk.writestr(name, data, compress_type=compression)
            receipt["entries"].append({"entry": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
    (out / "inputs.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    aligned = out / "build/aligned.apk"
    run(tools["zipalign"], "-P", "16", "4", unsigned, aligned)
    version = json.loads((ROOT / "release.json").read_text(encoding="utf-8"))["version"]
    output = out / f"Singles-Sorter-{version}{'-qa' if mode == 'qa' else ''}.apk"
    run(sys.executable, HERE / "sign_apk.py", "--input", aligned, "--output", output,
        "--apksigner", tools["apksigner"], *(["--qa"] if mode == "qa" else []))
    run(tools["zipalign"], "-c", "-P", "16", "4", output)
    run(tools["aapt2"], "dump", "badging", output)
    print(output)


if __name__ == "__main__":
    main()
