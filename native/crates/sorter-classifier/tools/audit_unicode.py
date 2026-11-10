"""NEW exhaustive synthetic Unicode audit against pinned CPython15 semantics."""
from pathlib import Path
import argparse,hashlib,json,subprocess,struct,sys,unicodedata,re
assert sys.version_info[:2]==(3,12) and unicodedata.unidata_version=='15.0.0'
parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--evidence',type=Path,required=True);a=parser.parse_args();a.evidence.mkdir(parents=True,exist_ok=True)
keys=['lower','sigma_before','sigma_after','word_analyzer'];hashes={k:hashlib.sha256() for k in keys};count=0;pattern=re.compile(r'(?u)\b\w\w+\b')
def add(h,s):
 b=s.encode();h.update(struct.pack('<I',len(b)));h.update(b)
for cp in range(0x110000):
 if 0xd800<=cp<=0xdfff:continue
 c=chr(cp);count+=1;add(hashes['lower'],c.lower());add(hashes['sigma_before'],('A'+c+'Σ').lower());add(hashes['sigma_after'],('AΣ'+c+'A').lower())
 words=pattern.findall((c+c).lower());features=words+[' '.join(words[i:i+2]) for i in range(len(words)-1)];h=hashes['word_analyzer'];h.update(struct.pack('<I',len(features)))
 for s in features:add(h,s)
expected={k:h.hexdigest() for k,h in hashes.items()};expected['unicode_scalars']=count
actual=json.loads(subprocess.check_output([str(a.binary.resolve())],text=True));assert actual==expected,(actual,expected)
report={'status':'NEW_EXHAUSTIVE_UNICODE_PASS','oracle_python':sys.version,'unicode':unicodedata.unidata_version,'matching_hashes':actual,'contexts':'Each valid Unicode scalar: standalone lower, A+scalar+Sigma lower, A+Sigma+scalar+A lower, word analyzer of doubled scalar. No surrogate inputs accepted by Rust UTF8.'};(a.evidence/'unicode-audit.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8',newline='\n');print(json.dumps(report,indent=2))
