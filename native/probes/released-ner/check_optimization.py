"""Same-host before/after exact diagnostics, optional modes, error and stage-bit gate."""
import argparse,json,subprocess
from pathlib import Path
HERE=Path(__file__).resolve().parent

def invoke(binary,bundle,requests,flags=()):
    result=subprocess.run([str(binary.resolve()),str(bundle.resolve()),*flags],input=requests,encoding='utf-8',text=True,capture_output=True,check=True,timeout=120)
    return result.stdout

def compare(bundle,baseline,optimized,baseline_stages=None,optimized_stages=None):
    fixtures=json.loads((bundle/'fixtures.json').read_text(encoding='utf-8'))['fixtures']
    texts=[f['text'] for f in fixtures]+[' '.join(['אברהם']*302)]
    requests=''.join(json.dumps({'text':t},ensure_ascii=False)+'\n' for t in texts)
    requests+='{}\n{"text":null}\n{"text":3}\n{"text":"abc","preset":[]}\n[]\ninvalid json\n'
    before=invoke(baseline,bundle,requests)
    after=invoke(optimized,bundle,requests)
    fast=invoke(optimized,bundle,requests,['--fast'])
    if before!=after or before!=fast:raise ValueError('baseline/optimized/default/fast complete diagnostics differ')
    rows=[json.loads(line) for line in before.splitlines()]
    if len(rows)!=len(texts)+6:raise ValueError('missing/extra baseline response rows')
    if any('error' in row for row in rows[:len(texts)]) or any(set(row)!={'error'} for row in rows[len(texts):]):raise ValueError('valid/error response contract failed')
    spans=[json.loads(line) for line in invoke(optimized,bundle,requests,['--spans']).splitlines()]
    expected=[row if 'error' in row else {'spans':row['spans']} for row in rows]
    if spans!=expected:raise ValueError('span-only/error response differs')
    stage_checked=False
    if (baseline_stages is None)!=(optimized_stages is None):raise ValueError('both stage probes required together')
    if baseline_stages is not None:
        before_bits=subprocess.check_output([str(baseline_stages.resolve()),str(bundle.resolve())],timeout=120)
        after_bits=subprocess.check_output([str(optimized_stages.resolve()),str(bundle.resolve())],timeout=120)
        if before_bits!=after_bits or b'one_ulp_margin_and_tie PASS' not in after_bits:raise ValueError('numeric stage bits/logits/ties differ')
        stage_checked=True
    print(json.dumps({'result':'PASS','documents':len(texts),'error_cases':6,'diagnostic_and_fast_exact':True,'spans_and_errors_exact':True,
                      'stage_bits_compared':stage_checked,'platform_claim':'Only the platform on which these supplied binaries executed; no production approval'}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--bundle',type=Path,required=True);p.add_argument('--baseline',type=Path,required=True);p.add_argument('--optimized',type=Path,required=True);p.add_argument('--baseline-stages',type=Path);p.add_argument('--optimized-stages',type=Path);a=p.parse_args();compare(a.bundle,a.baseline,a.optimized,a.baseline_stages,a.optimized_stages)
