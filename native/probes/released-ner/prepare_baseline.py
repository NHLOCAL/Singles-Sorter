"""Create a fresh job-local baseline from reviewed source, never model/binary files."""
import hashlib,json,shutil,tempfile,os
from pathlib import Path
HERE=Path(__file__).resolve().parent

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
        shutil.copyfile(HERE/'baseline/math-lib.rs',stage/'released-ner-math/src/lib.rs')
        shutil.copyfile(HERE/'baseline/raw-ner.rs',stage/'released-ner-text/src/bin/raw-ner.rs')
        os.rename(stage,destination)
    print('Prepared isolated original baseline for this CI job')
if __name__=='__main__':prepare()
