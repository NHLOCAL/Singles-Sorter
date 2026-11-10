"""Standard-library-only integrity validator for the frozen local export contract."""
import hashlib,json,math,struct
from pathlib import Path
HERE=Path(__file__).resolve().parent

def check(ok,msg):
    if not ok: raise ValueError(msg)
def sha(b):return hashlib.sha256(b).hexdigest()
def verify(out):
    out=Path(out);m=json.loads((out/'manifest.json').read_text(encoding="utf-8"));lock=json.loads((HERE/'release-lock.json').read_text(encoding="utf-8"))
    check(m['schema']=='released-ner-v1' and m['dtype']=='<f4' and m['order']=='C','unsupported schema/layout')
    check(m['model_sha256']==lock['source_files']['ner/model'],'wrong source model')
    check(m['graph']==lock['graph'],'wrong graph')
    actions=['U-','O']+[f'{a}-{label}' for label in ['SINGER','SONG','MISC','ALBUM','GENRE'] for a in ['B','I','L','U']]
    check(m['actions']==actions and m['unseen_classes']==[],'wrong action ordering/mask')
    check(m['tensor_file']=='tensors.f32','unexpected tensor path')
    check(set(m['files'])=={'tensors.f32','tokenizer.json','fixtures.json','forced_states.json'},'unexpected files')
    for f,h in m['files'].items():check(sha((out/f).read_bytes())==h,'file corruption: '+f)
    b=(out/'tensors.f32').read_bytes();check(len(b)==m['parameter_bytes']==3829976,'wrong byte count')
    check(sha(b)==m['tensor_sha256'],'wrong tensor bundle digest')
    check(len(m['tensors'])==len(lock['parameters'])==31,'wrong tensor count')
    offset=0
    for t,e in zip(m['tensors'],lock['parameters']):
        for key in ['node','name','param','shape','sha256']:check(t[key]==e[key],'wrong tensor '+key)
        check(t['dtype']=='<f4' and t['offset_bytes']==offset,'wrong tensor layout')
        check(t['nbytes']==e['bytes']==math.prod(t['shape'])*4,'wrong tensor extent')
        check(sha(b[offset:offset+t['nbytes']])==e['sha256'],'wrong tensor bytes')
        offset+=t['nbytes']
    check(offset==len(b),'unclaimed tensor bytes')
    check(all(math.isfinite(v[0]) for v in struct.iter_unpack('<f',b)),'nonfinite tensor')
    return {'result':'PASS','tensors':31,'bytes':offset,'actions':len(actions)}
if __name__=='__main__':
    import sys
    print(json.dumps(verify(Path(sys.argv[1]) if len(sys.argv)>1 else HERE/'out')))
