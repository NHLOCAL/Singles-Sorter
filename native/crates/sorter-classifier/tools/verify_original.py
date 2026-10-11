from pathlib import Path
import argparse,hashlib,json,pickle,subprocess,random,struct,sys,unicodedata,ctypes,re,os
os.environ['OPENBLAS_NUM_THREADS']='1'
import numpy as np,sklearn
parser=argparse.ArgumentParser(description="Fresh original bounded classifier parity; no private data or training")
parser.add_argument('--assets',type=Path,required=True)
parser.add_argument('--binary',type=Path,required=True)
parser.add_argument('--evidence',type=Path,required=True)
a=parser.parse_args(); O=Path(__file__).resolve().parents[1]; D=a.assets; B=a.binary.resolve(); E=a.evidence; E.mkdir(parents=True,exist_ok=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert (sys.version_info[:2],unicodedata.unidata_version,sklearn.__version__,np.__version__)==((3,12),'15.0.0','1.5.1','1.26.4')
p=D/'original-release.pkl';assert sha(p)=='ec9b4b50e9308d5f7a27f12bc141389e61a16a622a4faa0cbf864f94a9cb3293'
m=pickle.loads(p.read_bytes());v,c=m.steps[0][1],m.steps[-1][1]
assert type(v).__name__=='TfidfVectorizer' and type(c).__name__=='LogisticRegression';assert c.classes_.tolist()==[0,1,2,3]
raw=(D/'classifier-v14.weights.bin').read_bytes();assert raw[:8]==b'SC14F64\0';assert struct.unpack_from('>III',raw,8)==(1,79816,4);assert raw[20:52].hex()==sha(p);assert raw[52:84].hex()==sha(D/'classifier-v14.vocab.txt')
pos=84
for i,name in enumerate((b'ARTIST',b'ALBUM',b'SONG',b'RANDOM')):
 ix,n=struct.unpack_from('>II',raw,pos);pos+=8;assert ix==i and raw[pos:pos+n]==name;pos+=n
expected=np.concatenate((c.intercept_,v.idf_,c.coef_.reshape(-1))).astype('>f8').tobytes();assert raw[pos:]==expected
assert (D/'classifier-v14.vocab.txt').read_text(encoding='utf-8').splitlines()==list(v.get_feature_names_out())
# Reuses the independent reconstruction review's seed and generator;  broad vocabulary, repeated terms, chained context-sensitive marks.
rng=random.Random(202610110923);names=list(v.get_feature_names_out());seps=[' ','\n','\0','-','\u0301','\u0345','\u200d','\u05b0','\u2067','_','\u2029']
texts=['','\0','a b c','__','AΣ\u0301\u0345A','AΣ\u0345\u0301!','İİ','\U00010400\U00010400','aa '*1000]
for i in range(900):
 terms=rng.sample(names,rng.randint(1,12));terms+=terms[:rng.randrange(len(terms)+1)];texts.append(rng.choice(seps).join(terms).upper() if i%2 else rng.choice(seps).join(terms))
for i in range(400):
 chars=[]
 for j in range(rng.randint(0,30)):
  cp=rng.randrange(0x110000)
  if not 0xd800<=cp<=0xdfff:chars.append(chr(cp))
 texts.append('AA'+''.join(chars)+'Σ\u0301'+('A' if i%2 else '!'))
for left in ['','A','\u0345','A\u0345','A\u0301','Σ','Σ\u200d']:
 for middle in ['','\u0301','\u0345','\u200d','\u0345\u0301','\u0301\u0345']:
  for right in ['','A','!','Σ','\u0345A']:
   texts.append(left+'Σ'+middle+right)
inp=''.join(json.dumps({'text':x},ensure_ascii=False)+'\n' for x in texts);(E/'inputs.jsonl').write_text(inp,encoding='utf-8',newline='\n')
out=subprocess.check_output([str(B),str(D)],input=inp,text=True,encoding="utf-8");(E/'outputs.jsonl').write_text(out,encoding='utf-8',newline='\n');rows=[json.loads(x) for x in out.splitlines()];assert len(rows)==len(texts)
errs=dict(sparse=0.,score=0.,probability=0.,margin=0.);counts=[0]*4
for text,row in zip(texts,rows):
 x=v.transform([text]);s=c.decision_function(x)[0];pr=m.predict_proba([text])[0];cl=int(m.predict([text])[0]);counts[cl]+=1
 assert row['analyzed_features']==v.build_analyzer()(text)
 assert [a[0] for a in row['sparse_tfidf']]==list(x.indices)
 assert row['predicted_class']==cl and row['accepted']==(cl==0)
 errs['sparse']=max([errs['sparse']]+[abs(a-b[1]) for a,b in zip(x.data,row['sparse_tfidf'],strict=True)])
 errs['score']=max(errs['score'],float(max(abs(s-np.array(row['scores'])))))
 errs['probability']=max(errs['probability'],float(max(abs(pr-np.array(row['probabilities'])))))
 p2=sorted(pr);errs['margin']=max(errs['margin'],abs(float(p2[-1]-p2[-2])-row['margin_top_two']))
assert errs['sparse']==errs['score']==0 and errs['probability']<=1e-14 and errs['margin']<=1e-14,errs
# Validate every generated scalar predicate against the independent pinned runtime.
tables={k:[int(n) for n in re.findall(r'\d+',body)] for k,body in re.findall(r'pub const (\w+): &\[u32\] = &\[(.*?)\];',(O/'src/unicode_tables.rs').read_text(encoding='utf-8'),re.S)}
def expand(k):
 a=tables[k];assert all(a[i]<=a[i+1] for i in range(0,len(a),2));assert all(a[i+1]<a[i+2] for i in range(0,len(a)-2,2));return {p for i in range(0,len(a),2) for p in range(a[i],a[i+1]+1)}
word=expand('RANGES');cased=expand('CASEDRANGES');ign=expand('IGNORABLERANGES');low=dict(zip(tables['LOWERPAIRS'][::2],tables['LOWERPAIRS'][1::2]));fs=[]
for name in ['_PyUnicode_IsCased','_PyUnicode_IsCaseIgnorable']:
 f=getattr(ctypes.pythonapi,name);f.argtypes=[ctypes.c_uint];f.restype=ctypes.c_int;fs.append(f)
for cp in range(0x110000):
 ch=chr(cp);assert (cp in word)==bool(ch.isalnum() or cp==95);assert (cp in cased)==bool(fs[0](cp));assert (cp in ign)==bool(fs[1](cp));assert ('i\u0307' if cp==304 else chr(low.get(cp,cp)))==ch.lower()
# Bad bundles: byte mutations, truncations, append, missing; no generic untrusted parser claim.
corrupt=[]
for kind in ['weights-flip','vocab-flip','weights-truncate','weights-append','vocab-truncate','vocab-append','missing']:
 folder=E/kind;folder.mkdir(exist_ok=True)
 for name in ['classifier-v14.weights.bin','classifier-v14.vocab.txt']:
  if kind=='missing':continue
  b=(D/name).read_bytes();target='weights' if '.weights.' in name else 'vocab'
  if kind.startswith(target):
   if kind.endswith('flip'):b=b[:-1]+bytes([b[-1]^1])
   elif kind.endswith('truncate'):b=b[:len(b)//2]
   elif kind.endswith('append'):b+=b'\0'
  (folder/name).write_bytes(b)
 p=subprocess.run([str(B),str(folder)],input='',text=True,encoding="utf-8",capture_output=True);assert p.returncode!=0;corrupt.append({'variant':kind,'exit':p.returncode,'stderr':p.stderr.strip()})
for args,body in [([],json.dumps({'text':'aa '*10000})+'\n'),(['--candidate-char35'],''),(['--wrong'],''),([], '{"text":123}\n'),([], '{"text":"\\ud800"}\n'),([], 'garbage\n')]:
 p=subprocess.run([str(B),str(D)]+args,input=body,text=True,encoding="utf-8",capture_output=True);assert p.returncode!=0
report={'status':'NEW_BOUNDED_ORIGINAL_ONLY_PASS','cases':len(texts),'classes':counts,'errors':errs,'parameters_bit_exact':len(expected)//8,'all_unicode_codepoints_table_checked':0x110000,'invalid_bundles':corrupt,'candidate_disabled':True,'historical_1982_or_3295_reproduced':False,'sealed_or_private_data_read':False,'training':False,'hashes':{str(p):sha(p) for p in [O/'src/lib.rs',O/'src/unicode_tables.rs',B]},'input_sha256':sha(E/'inputs.jsonl'),'python':sys.version,'numpy':np.__version__,'sklearn':sklearn.__version__}
(E/'report.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8',newline='\n');print(json.dumps(report,indent=2))
