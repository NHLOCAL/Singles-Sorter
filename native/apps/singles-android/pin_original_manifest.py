"""Build-only original provenance verification and exact per-APK Java pin generation.

The released model bytes never change. The exporter records its actual approved
Linux/Windows source provenance, so its manifest is not byte-identical across OSes.
"""
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location(
    'original_package_verifier', ROOT / 'native/packaging/windows-preview/package_preview.py')
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)
TEMPLATE = ROOT / 'native/plugins/original-inference/android/src/com/nhlocal/singlesorter/inference/OriginalAssets.java'
OLD_SIZE = '1268235, 3192809, 32600, 3829976, 73348'
OLD_HASH = '3fb1801718d84c335b31cdae18ca94daf6af821fb29c315bcf485de3810b5580'


def manifest_pin(path):
    # Bound and reject symlinks before parsing. The shared verifier rejects
    # duplicate keys and compares every graph/tensor/runtime field exactly.
    verifier.require(path.is_file() and not path.is_symlink(), 'not an ordinary manifest')
    verifier.require(path.stat().st_size <= 262144, 'oversized original manifest')
    hash_ = verifier.digest(path)
    platform = json.loads(path.read_text(encoding='utf-8'))['runtime']['source_platform']
    verifier.check_original_manifest(path, platform)
    return path.stat().st_size, hash_


def generate(path, template):
    size, hash_ = manifest_pin(path)
    verifier.require(template.count(OLD_SIZE) == 1 and template.count(OLD_HASH) == 1,
                     'original Java pin template changed')
    return template.replace(OLD_SIZE, f'1268235, 3192809, {size}, 3829976, 73348').replace(OLD_HASH, hash_)


if __name__ == '__main__':
    manifest = Path(sys.argv[1])
    if len(sys.argv) == 2:
        size, hash_ = manifest_pin(manifest)
        print(json.dumps({'bytes': size, 'sha256': hash_}))
    else:
        output = Path(sys.argv[2])
        # Never rewrite checked-in source or an existing generated file.
        source = generate(manifest, TEMPLATE.read_text(encoding='utf-8'))
        with output.open('x', encoding='utf-8', newline='\n') as stream:
            stream.write(source)
