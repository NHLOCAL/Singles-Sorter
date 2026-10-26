"""Verify a new host's original Python oracle against the frozen original golden before native builds."""
import argparse,json
from pathlib import Path
HERE=Path(__file__).resolve().parent

def check(bundle):
    fixtures=json.loads((bundle/'fixtures.json').read_text(encoding='utf-8'))['fixtures']
    golden=json.loads((HERE/'linux-reference.json').read_text(encoding='utf-8'))['fixtures']
    if len(fixtures)!=len(golden):raise ValueError('oracle fixture count changed')
    actions=0
    for f,g in zip(fixtures,golden,strict=True):
        lexical=[]
        for i,t in enumerate(f['tokens']):
            lexical.append({'text':t['text'],'offset':t['start'],'whitespace':t['whitespace'],'sent_start':t['sent_start'],
                'strings':[t[k] for k in ('norm','prefix','suffix','shape')],'ids':f['features'][i],'hashes':f['hashes'][i],'buckets':f['buckets'][i]})
        actual={'text':f['text'],'actions':[s['action_name'] for s in f['states']],'spans':f['spans'],
                'tokens':[{k:t[k] for k in ('start','end','is_space','sent_start','ent_iob')} for t in f['tokens']],'lexical':lexical}
        if actual!=g:raise ValueError(f'original oracle golden drift at fixture {f["id"]}')
        actions+=len(actual['actions'])
    print(json.dumps({'result':'PASS','documents':len(fixtures),'actions':actions,'original_oracle_golden_exact':True,'native_execution':False}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('bundle',type=Path);a=p.parse_args();check(a.bundle)
