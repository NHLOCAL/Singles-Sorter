"""Create a fresh job-local baseline from reviewed source, never model/binary files."""
import hashlib,json,shutil,tempfile,os
from pathlib import Path
HERE=Path(__file__).resolve().parent
# Preserve every frozen scalar byte as an exact prefix. The suffix exposes
# shared asset/control types, not a replacement numerical kernel.
LOADER_API_ADAPTER=b'\npub mod assets;\npub use assets::LoadError;\npub mod control;\n'

def prepare():
    destination=HERE/'.baseline'
    if destination.exists():raise ValueError('refusing to overwrite an existing baseline tree')
    provenance=json.loads((HERE/'optimization-provenance.json').read_text(encoding='utf-8'))
    for relative,digest in provenance['baseline_files'].items():
        if hashlib.sha256((HERE/relative).read_bytes()).hexdigest()!=digest:raise ValueError('baseline source hash mismatch')
    with tempfile.TemporaryDirectory(prefix='ner-baseline-',dir=HERE) as tmp:
        stage=Path(tmp)/'tree';stage.mkdir()
        # Explicit inputs only. Never copy ignored oracle/model/build/download directories.
        for crate in ('released-ner-math','released-ner-text'):
            source=HERE/crate;target=stage/crate;target.mkdir()
            for directory in ('src','licenses','tests'):
                if (source/directory).exists():shutil.copytree(source/directory,target/directory)
            for name in ('Cargo.toml','Cargo.lock','build.rs','model-lock.json','tokenizer.json'):
                if (source/name).exists():shutil.copyfile(source/name,target/name)
        (stage/'released-ner-math/src/lib.rs').write_bytes((HERE/'baseline/math-lib.rs').read_bytes()+LOADER_API_ADAPTER)
        shutil.copyfile(HERE/'baseline/raw-ner.rs',stage/'released-ner-text/src/bin/raw-ner.rs')
        # The original scalar has no bounded inference methods. Keep this tree a
        # diagnostic scalar comparator, never a second purported production API.
        text_lib=stage/'released-ner-text/src/lib.rs'
        if text_lib.exists():
            text_lib.write_bytes(text_lib.read_bytes().replace(b'pub mod inference;\n',b''))
        # The immutable-bundle test uses the optimized convenience method.
        # Original scalar forward returns the same affine values with tracing.
        loading=stage/'released-ner-text/tests/loading.rs'
        if loading.exists():
            loading.write_bytes(loading.read_bytes().replace(b'.forward_fast(',b'.forward('))
        for crate in ('released-ner-math','released-ner-text'):
            for name in ('bounded.rs','bounded_math.rs'):
                bounded=stage/crate/'tests'/name
                if bounded.exists():bounded.unlink()
        os.rename(stage,destination)
    print('Prepared isolated original baseline for this CI job')
if __name__=='__main__':prepare()
