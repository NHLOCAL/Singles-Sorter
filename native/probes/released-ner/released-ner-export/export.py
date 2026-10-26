#!/usr/bin/env python3
"""Frozen offline oracle exporter. Reads release assets, never a corpus.
Generated model and oracle arrays are local-only and intentionally Git-ignored.
"""
import argparse, hashlib, json, platform, sys, unicodedata
from pathlib import Path
import numpy as np
import spacy, srsly, thinc
from spacy.attrs import NORM
from synthetic import SMOKE,BOUNDARY,FORCED
from runtime_sources import validate as validate_runtime_source

HERE = Path(__file__).resolve().parent
MODEL_SHA = '5f0d50400f685c3992a21d395f7fab30b45ca3a9968383c24044ff4478ec865e'
RELEASE = '78d001aee5ce49628bdffebd34540fa83ba32c12'
ACTIONS = ['U-', 'O'] + [f'{a}-{label}' for label in ['SINGER','SONG','MISC','ALBUM','GENRE'] for a in ['B','I','L','U']]
TEXTS = SMOKE
def sha(b): return hashlib.sha256(b).hexdigest()
# Generated exports must have identical UTF-8/LF bytes on Windows and Linux.
# This does not rewrite or normalize any original released model assets.
def dump(path,obj): path.write_bytes((json.dumps(obj,ensure_ascii=False,indent=2,allow_nan=False)+'\n').encode('utf-8'))
def require(ok, msg):
    if not ok: raise ValueError(msg)
def graph(nodes):
    def attrs(x):
        out = {}
        for k,v in x.attrs.items():
            if callable(v): out[k] = {'callable':v.__module__+'.'+v.__name__}
            elif isinstance(v,set): out[k] = sorted(v)
            elif isinstance(v,tuple): out[k] = repr(v)
            else: out[k]=v
        return out
    return [{'node':i,'name':n.name,'layers':[nodes.index(c) for c in n.layers],
             'dims':{k:n.get_dim(k) for k in n.dim_names if n.has_dim(k)},'attrs':attrs(n),
             'params':list(n.param_names),'shims':len(n.shims)} for i,n in enumerate(nodes)]

def export(root,out,suite="smoke"):
    require((spacy.__version__,thinc.__version__,np.__version__,platform.python_version_tuple()[:2],unicodedata.unidata_version)==('3.7.5','8.2.5','1.26.4',('3','12'),'15.0.0'),'unsupported oracle runtime')
    require(root.resolve() not in out.resolve().parents and root.resolve()!=out.resolve(), 'output must not modify source model')
    lock=json.loads((HERE/'release-lock.json').read_text(encoding="utf-8"))
    for f,h in lock['source_files'].items(): require(sha((root/f).read_bytes())==h,'release file changed: '+f)
    require(sha((root/'ner/model').read_bytes())==MODEL_SHA,'wrong model')
    runtime_source_provenance={}
    for relative,h in lock['runtime_sources'].items():
        package,rest=relative.split('/',1); source=Path(__import__(package).__file__).parent/rest
        runtime_source_provenance[relative]=validate_runtime_source(relative,source.read_bytes(),h,sys.platform)
    nlp=spacy.load(root); ner=nlp.get_pipe('ner'); nodes=list(ner.model.walk())
    require(graph(nodes)==lock['graph'],'unsupported graph')
    actions=[ner.moves.get_class_name(i) for i in range(ner.moves.n_moves)]
    require(actions==ACTIONS,'action ordering changed')
    require(not ner.model.attrs['unseen_classes'],'unsupported unseen classes')
    require(ner.cfg['beam_width']==1,'beam search unsupported')
    out.mkdir(parents=True,exist_ok=True)
    manifest={'schema':'released-ner-v1','release':RELEASE,'model_sha256':MODEL_SHA,'dtype':'<f4','order':'C','tensor_file':'tensors.f32','graph':graph(nodes),'actions':actions,'unseen_classes':[], 'features':['NORM','PREFIX','SUFFIX','SHAPE'],'tensors':[], 'runtime':{'python':platform.python_version(),'spacy':spacy.__version__,'thinc':thinc.__version__,'numpy':np.__version__,'unicode':unicodedata.unidata_version,'source_platform':sys.platform,'source_byte_provenance':runtime_source_provenance}}
    data=bytearray()
    for expected in lock['parameters']:
        i,k=expected['node'],expected['param']; a=nodes[i].get_param(k)
        require(str(a.dtype)=='float32' and list(a.shape)==expected['shape'],'wrong tensor '+str((i,k)))
        require(bool(np.isfinite(a).all()),'nonfinite tensor '+str((i,k)))
        b=np.asarray(a,dtype='<f4',order='C').tobytes()
        require(sha(b)==expected['sha256'] and len(b)==expected['bytes'],'tensor hash differs '+str((i,k)))
        manifest['tensors'].append({'node':i,'name':nodes[i].name,'param':k,'shape':list(a.shape),'dtype':'<f4','offset_bytes':len(data),'nbytes':len(b),'sha256':sha(b)})
        data.extend(b)
    require(len(data)==3829976,'parameter byte count')
    (out/'tensors.f32').write_bytes(data); manifest['tensor_sha256']=sha(data);manifest['parameter_bytes']=len(data)
    tok=srsly.msgpack_loads(nlp.tokenizer.to_bytes());tok.pop('vocab')
    tok['exceptions']={k:[{spacy.attrs.NAMES[a]:v for a,v in entry.items()} for entry in entries] for k,entries in tok['exceptions'].items()}
    norm=nlp.vocab.lex_attr_getters[NORM]
    require(norm.func.__name__=='_get_attr_unless_lookup' and norm.args[0].__name__=='lower','unknown norm implementation')
    tok.update(schema='released-ner-tokenizer-v1',symbols={k:str(v) for k,v in spacy.symbols.IDS.items()},norm_lookups=list(norm.args[1]),unicode='15.0.0',string_hash={'algorithm':'MurmurHash64A','seed':1,'encoding':'UTF-8','symbol_ids_override_hash':True})
    dump(out/'tokenizer.json',tok)
    fixtures=[]
    for fixture_id,text in enumerate(SMOKE if suite=="smoke" else (BOUNDARY + [json.loads((HERE.parent/"close-margin.json").read_text(encoding="utf-8"))["text"]] if suite=="platform" else BOUNDARY)):
        doc=nlp.make_doc(text); features=doc.to_array(['NORM','PREFIX','SUFFIX','SHAPE']);f={'id':fixture_id,'text':text,'tokens':[{'text':t.text,'start':t.idx,'end':t.idx+len(t),'whitespace':t.whitespace_,'is_space':t.is_space,'sent_start':t.is_sent_start,'ent_iob':t.ent_iob,'norm':t.norm_,'prefix':t.prefix_,'suffix':t.suffix_,'shape':t.shape_} for t in doc], 'features':[[str(int(x)) for x in row] for row in features],'stages':{}}
        raw=[ner.model.ops.hash(features[:,i].copy(),8+i) for i in range(4)]
        f['hashes']=[[[int(x) for x in raw[j][i]] for j in range(4)] for i in range(len(doc))]
        f['buckets']=[[[int(x)%([2000,1000,1000,1000][j]) for x in raw[j][i]] for j in range(4)] for i in range(len(doc))]
        # Hook only official forward calls, preserving wrapper padding and execution order.
        watch={15:'embedding_concat',16:'mixer',17:'cnn_0',18:'cnn_1',19:'cnn_2',20:'cnn_3',6:'projection',2:'affine'}
        originals={}
        for i,name in watch.items():
            originals[i]=nodes[i]._func
            def wrapped(model,X,is_train,original=nodes[i]._func,name=name):
                Y,bp=original(model,X,is_train)
                f['stages'][name]={'shape':list(Y.shape),'values':Y.tolist()}
                return Y,bp
            nodes[i]._func=wrapped
        try: step=ner.model.predict([doc])
        finally:
            for i,fn in originals.items():nodes[i]._func=fn
        states=ner.moves.init_batch([doc]);state=states[0];f['states']=[]
        while not state.is_final():
            logits=step.predict([state])[0]; valid=[bool(ner.moves.is_valid(state,a)) for a in actions]
            candidates=[i for i,v in enumerate(valid) if v];require(bool(candidates),'no valid action')
            best=max(candidates,key=lambda i:float(logits[i]));ranked=sorted((float(logits[i]) for i in candidates),reverse=True)
            f['states'].append({'context':step.get_token_ids([state]).tolist()[0],'valid':valid,'logits':logits.tolist(),'action':best,'action_name':actions[best],'margin':ranked[0]-ranked[1] if len(ranked)>1 else None})
            ner.moves.apply_transition(state,actions[best]);require(len(f['states'])<=len(doc),'state loop exceeded tokens')
        ner.moves.set_annotations(state,state.doc)
        f['spans']=[{'start':e.start_char,'end':e.end_char,'label':e.label_,'text':e.text} for e in state.doc.ents]
        require(f['spans']==[{'start':e.start_char,'end':e.end_char,'label':e.label_,'text':e.text} for e in nlp(text).ents],'trace disagrees with pipeline')
        fixtures.append(f)
    dump(out/'fixtures.json',{'schema':'released-ner-oracle-v1','provenance':'Owned fixed synthetic suite '+suite+'; no corpus input; no randomness','fixtures':fixtures})
    forced=[]
    prefixes=[[]]+[['O']]+[[f'U-{label}'] for label in ['SINGER','SONG','MISC','ALBUM','GENRE']]+[[f'B-{label}'] for label in ['SINGER','SONG','MISC','ALBUM','GENRE']]+[[f'B-{label}',f'I-{label}'] for label in ['SINGER','SONG','MISC','ALBUM','GENRE']]
    for text in FORCED:
        for prefix in prefixes:
            doc=nlp.make_doc(text);state=ner.moves.init_batch([doc])[0];applicable=True
            for action in prefix:
                if state.is_final() or not ner.moves.is_valid(state,action): applicable=False;break
                ner.moves.apply_transition(state,action)
            if not applicable:continue
            step=ner.model.predict([doc])
            forced.append({'text':text,'prefix':prefix,'tokens':[{'text':t.text,'start':t.idx,'end':t.idx+len(t),'is_space':t.is_space,'sent_start':t.is_sent_start,'ent_iob':t.ent_iob} for t in doc],'final':bool(state.is_final()),'context':step.get_token_ids([state]).tolist()[0] if not state.is_final() else None,'valid':[bool(ner.moves.is_valid(state,a)) for a in actions] if not state.is_final() else None})
    dump(out/'forced_states.json',{'schema':'released-ner-forced-states-v1','scope':'Fresh documents only; explicitly forced legal transitions; no preset entities or sentence annotations','cases':forced})
    manifest['files']={f:sha((out/f).read_bytes()) for f in ['tensors.f32','tokenizer.json','fixtures.json','forced_states.json']}
    dump(out/'manifest.json',manifest)
    print(json.dumps({'result':'PASS','tensors':len(manifest['tensors']),'bytes':len(data),'actions':len(actions),'fixtures':len(fixtures),'out':str(out)}))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--model',type=Path,required=True);p.add_argument('--out',type=Path,default=HERE/'out');p.add_argument('--suite',choices=['smoke','boundary','platform'],default='smoke');a=p.parse_args();export(a.model,a.out,a.suite)
