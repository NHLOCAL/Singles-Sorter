"""NEW original-only regeneration from authenticated public pickle + CPython Unicode15.
This does not recover an old Git object or run training. It must not load candidate weights.
"""
from pathlib import Path
import argparse, ctypes, hashlib, json, pickle, struct, subprocess, sys, unicodedata
import numpy as np, sklearn
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--baseline', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--unicode-output', type=Path)
args = parser.parse_args()
D = args.output
D.mkdir(parents=True, exist_ok=True)
BASE = args.baseline
COMMIT = '78d001aee5ce49628bdffebd34540fa83ba32c12'
MODEL_PATH = 'src/core/models/music_entity_clf/music_entity_clf.pkl'
BLOB = '53945ae968c97125c890927e7ff97789351b2470'
if not subprocess.check_output(['git', '-C', str(BASE), 'rev-parse', COMMIT + '^{commit}'], text=True).strip() == COMMIT:
    raise ValueError("Pinned original export validation failed: subprocess.check_output(['git', '-C', str(BASE), 'rev-parse', COMMIT + '^{commit}'], text=True).strip() == COMMIT")
if not subprocess.check_output(['git', '-C', str(BASE), 'rev-parse', COMMIT + ':' + MODEL_PATH], text=True).strip() == BLOB:
    raise ValueError("Pinned original export validation failed: subprocess.check_output(['git', '-C', str(BASE), 'rev-parse', COMMIT + ':' + MODEL_PATH], text=True).strip() == BLOB")
model_bytes = subprocess.check_output(['git', '-C', str(BASE), 'cat-file', 'blob', BLOB])
MODEL_SHA = 'ec9b4b50e9308d5f7a27f12bc141389e61a16a622a4faa0cbf864f94a9cb3293'
if not hashlib.sha256(model_bytes).hexdigest() == MODEL_SHA:
    raise ValueError('Pinned original export validation failed: hashlib.sha256(model_bytes).hexdigest() == MODEL_SHA')
(D / 'original-release.pkl').write_bytes(model_bytes)
if not (sys.version_info[:2] == (3, 12) and unicodedata.unidata_version == '15.0.0' and (sklearn.__version__ == '1.5.1') and (np.__version__ == '1.26.4')):
    raise ValueError("Pinned original export validation failed: sys.version_info[:2] == (3, 12) and unicodedata.unidata_version == '15.0.0' and (sklearn.__version__ == '1.5.1') and (np.__version__ == '1.26.4')")
pipeline = pickle.loads(model_bytes)
v, c = (pipeline.steps[0][1], pipeline.steps[-1][1])
if not (v.analyzer == 'word' and v.ngram_range == (1, 2) and v.lowercase and (v.strip_accents is None) and (v.preprocessor is None) and (v.tokenizer is None) and (v.stop_words is None)):
    raise ValueError("Pinned original export validation failed: v.analyzer == 'word' and v.ngram_range == (1, 2) and v.lowercase and (v.strip_accents is None) and (v.preprocessor is None) and (v.tokenizer is None) and (v.stop_words is None)")
if not (v.token_pattern == '(?u)\\b\\w\\w+\\b' and v.norm == 'l2' and (not v.sublinear_tf) and (not v.binary) and v.use_idf):
    raise ValueError("Pinned original export validation failed: v.token_pattern == '(?u)\\\\b\\\\w\\\\w+\\\\b' and v.norm == 'l2' and (not v.sublinear_tf) and (not v.binary) and v.use_idf")
if not (c.classes_.tolist() == [0, 1, 2, 3] and c.coef_.shape == (4, 79816)):
    raise ValueError('Pinned original export validation failed: c.classes_.tolist() == [0, 1, 2, 3] and c.coef_.shape == (4, 79816)')
names = list(v.get_feature_names_out())
if not (len(names) == 79816 and len(set(names)) == 79816 and all(('\n' not in n for n in names))):
    raise ValueError("Pinned original export validation failed: len(names) == 79816 and len(set(names)) == 79816 and all(('\\n' not in n for n in names))")
vocab = ('\n'.join(names) + '\n').encode()
weights = bytearray(b'SC14F64\x00' + struct.pack('>III', 1, 79816, 4) + bytes.fromhex(MODEL_SHA) + hashlib.sha256(vocab).digest())
for i, label in enumerate(['ARTIST', 'ALBUM', 'SONG', 'RANDOM']):
    b = label.encode()
    weights += struct.pack('>II', i, len(b)) + b
values = np.concatenate([c.intercept_, v.idf_, c.coef_.ravel()])
weights += values.astype('>f8').tobytes()
if not hashlib.sha256(weights).hexdigest() == 'aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33':
    raise ValueError("Pinned original export validation failed: hashlib.sha256(weights).hexdigest() == 'aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33'")
if not hashlib.sha256(vocab).hexdigest() == '35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3':
    raise ValueError("Pinned original export validation failed: hashlib.sha256(vocab).hexdigest() == '35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3'")
(D / 'classifier-v14.weights.bin').write_bytes(weights)
(D / 'classifier-v14.vocab.txt').write_bytes(vocab)

def predicate(name):
    f = getattr(ctypes.pythonapi, name)
    f.argtypes = [ctypes.c_uint]
    f.restype = ctypes.c_int
    return lambda cp: bool(f(cp))
cased = predicate('_PyUnicode_IsCased')
ignorable = predicate('_PyUnicode_IsCaseIgnorable')

def ranges(fn):
    result = []
    start = None
    for cp in range(1114112):
        yes = fn(cp)
        if yes and start is None:
            start = cp
        if not yes and start is not None:
            result.extend([start, cp - 1])
            start = None
    if start is not None:
        result.extend([start, 1114111])
    return result
pairs = []
expansions = []
for cp in range(1114112):
    lo = chr(cp).lower()
    if len(lo) > 1:
        expansions.append(cp)
    elif lo != chr(cp):
        pairs.extend([cp, ord(lo)])
if not expansions == [304]:
    raise ValueError('Pinned original export validation failed: expansions == [304]')
tables = {'LOWERPAIRS': pairs, 'CASEDRANGES': ranges(cased), 'IGNORABLERANGES': ranges(ignorable), 'RANGES': ranges(lambda cp: chr(cp).isalnum() or cp == 95)}
s = '// NEW regeneration from CPython ' + sys.version.split()[0] + ' / Unicode ' + unicodedata.unidata_version + '.\n'
for name, values in tables.items():
    s += 'pub const ' + name + ': &[u32] = &[\n'
    for i in range(0, len(values), 16):
        s += '    ' + ', '.join(map(str, values[i:i + 16])) + ',\n'
    s += '];\n'
if args.unicode_output:
    args.unicode_output.write_text(s, encoding='utf-8', newline='\n')
receipt = {'status': 'NEW_REGENERATION_NOT_OLD_GIT_RECOVERY', 'baseline_commit': COMMIT, 'model_sha256': MODEL_SHA, 'public_git_blob': subprocess.check_output(['git', '-C', str(BASE), 'rev-parse', COMMIT + ':src/core/models/music_entity_clf/music_entity_clf.pkl'], text=True).strip(), 'python': sys.version, 'unicode': unicodedata.unidata_version, 'sklearn': sklearn.__version__, 'numpy': np.__version__, 'unicode_table_scalar_counts': {k: len(v) for k, v in tables.items()}, 'parameters': 399084, 'historical_export_hashes_match': True, 'training': False, 'candidate_loaded': False, 'private_corpus_read': False, 'files': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [D / 'classifier-v14.weights.bin', D / 'classifier-v14.vocab.txt']}}
(D / 'regeneration.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8', newline='\n')
print(json.dumps(receipt, indent=2))
