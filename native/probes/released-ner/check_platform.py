"""Exact owned-synthetic contract check. Floating score differences are diagnostic only."""
import argparse, hashlib, json, math, subprocess
from pathlib import Path
HERE=Path(__file__).resolve().parent
META=('start','end','is_space','sent_start','ent_iob')
def run(binary, args, cases):
    p=subprocess.run([str(binary.resolve()),*map(str,args)],input=''.join(json.dumps({'text':f['text']},ensure_ascii=False)+'\n' for f in cases),
                     text=True,encoding='utf-8',capture_output=True,check=True,timeout=120)
    lines=p.stdout.splitlines()
    if len(lines)!=len(cases): raise ValueError('missing/extra output lines')
    return [json.loads(line) for line in lines]
def check(bundle, raw, lexical):
    cases=json.loads((bundle/'fixtures.json').read_text(encoding='utf-8'))['fixtures']
    reference=json.loads((HERE/'linux-reference.json').read_text(encoding='utf-8'))
    if [f['text'] for f in cases] != [f['text'] for f in reference['fixtures']]: raise ValueError('fixture suite/order changed')
    results=run(raw,[bundle],cases); features=run(lexical,['tokenize'],cases)
    steps=0
    for f,ref,got,lex in zip(cases,reference['fixtures'],results,features,strict=True):
        expected_actions=[s['action_name'] for s in f['states']]
        expected_meta=[{k:t[k] for k in META} for t in f['tokens']]
        for key,want in [('actions',expected_actions),('spans',f['spans']),('tokens',expected_meta)]:
            if got.get(key)!=want or got.get(key)!=ref[key]: raise ValueError(f'cross-platform {key} mismatch at fixture {f["id"]}')
        if len(lex)!=len(f['tokens']): raise ValueError('token count mismatch')
        for i,(token,original) in enumerate(zip(lex,f['tokens'],strict=True)):
            expected={'text':original['text'],'offset':original['start'],'whitespace':original['whitespace'],'sent_start':original['sent_start'],
                      'strings':[original[k] for k in ('norm','prefix','suffix','shape')], 'ids':f['features'][i], 'hashes':f['hashes'][i],'buckets':f['buckets'][i]}
            if token!=expected or token!=ref['lexical'][i]:raise ValueError(f'lexical mismatch at fixture {f["id"]} token {i}')
        if len(got['margins'])!=len(expected_actions) or any(x is not None and not math.isfinite(x) for x in got['margins']): raise ValueError('invalid score diagnostic')
        steps+=len(expected_actions)
    close=json.loads((HERE/'close-margin.json').read_text(encoding='utf-8'))
    if list(map(ord, close['text'])) != [2407,2408,10,1488,1500,1489,1493,1501,45,1488,1489,1512,1492,1501]:
        raise ValueError('close-margin exact input code points changed')
    if cases[-1]['text']!=close['text'] or results[-1]['actions']!=[s['action_name'] for s in close['original_states']] or results[-1]['spans']!=close['original_spans']:
        raise ValueError('independent close-margin regression')
    print(json.dumps({'result':'PASS','documents':len(cases),'actions':steps,'exact_metadata_features_actions_spans':True,
                      'close_margin_native':results[-1]['margins'][-1], 'close_margin_original':8.821487426757812e-5,
                      'native_executable_bytes':raw.stat().st_size,'native_executable_sha256':hashlib.sha256(raw.read_bytes()).hexdigest(),
                      'deployment_python_required':False,'production_approved':False}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bundle',type=Path,required=True);p.add_argument('--raw',type=Path,required=True);p.add_argument('--lexical',type=Path,required=True);a=p.parse_args();check(a.bundle,a.raw,a.lexical)
