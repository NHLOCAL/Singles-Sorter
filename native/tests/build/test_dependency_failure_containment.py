"""Full production-caller failure containment, using tiny owned archives.

Only compiler discovery is disabled. Every COPY is intercepted before execution,
so even the historical empty-path regression cannot copy filesystem roots.
Run this on native Windows as well as Linux; no network or package build occurs.
"""
import hashlib,json,os,pathlib,shutil,subprocess,tarfile,tempfile
P=pathlib.Path
BASE=P(__file__).resolve().parents[2] / 'crates/sorter-io/bridge'
CMAKE=os.environ.get('CMAKE', 'cmake')
_scratch=tempfile.TemporaryDirectory(prefix='sorter-full-configure-')
OUT=P(_scratch.name)
results=[]
def run(case, fault=None, stage='taglib', mock=None):
 with tempfile.TemporaryDirectory(dir=OUT) as tmp:
  root=P(tmp); bridge=root/'project/bridge'; bridge.mkdir(parents=True); cache=root/'cache'; cache.mkdir(); dep=bridge.parent/'dependencies'; dep.mkdir(); (bridge/'cmake').mkdir()
  helper=(BASE/'cmake/AcquireDependency.cmake').read_text()
  if mock == 'symlink_source':
   helper+='\nfunction(sorter_acquire_dependency name version url sha256 result)\nfile(MAKE_DIRECTORY \"${CMAKE_BINARY_DIR}/_deps\")\nfile(CREATE_LINK \"${CMAKE_CURRENT_SOURCE_DIR}\" \"${CMAKE_BINARY_DIR}/_deps/sorter_${name}-src\" SYMBOLIC)\nset(${result} \"${CMAKE_BINARY_DIR}/_deps/sorter_${name}-src\" PARENT_SCOPE)\nset(${result}_archive_sha256 \"${sha256}\" PARENT_SCOPE)\nendfunction()\n'
  elif mock == 'unwind':
   helper+='\nfunction(sorter_acquire_dependency name version url sha256 result)\nset(${result} \"\" PARENT_SCOPE)\nfile(MAKE_DIRECTORY \"${CMAKE_CURRENT_SOURCE_DIR}/CMakeLists.txt/impossible\")\nendfunction()\n'
  elif mock is not None:
   helper+='\nfunction(sorter_acquire_dependency name version url sha256 result)\n'
   helper+='set(${result} "'+mock+'" PARENT_SCOPE)\nset(${result}_archive_sha256 "${sha256}" PARENT_SCOPE)\nendfunction()\n'
  (bridge/'cmake/AcquireDependency.cmake').write_text(helper)
  locks={}
  for name in ['taglib','zlib']:
   src=root/name;src.mkdir();(src/'CMakeLists.txt').write_text('# SAFE OWNED FIXTURE\n');(src/'sentinel').write_text(name)
   archive=cache/f'{name}-1.tar.gz'
   with tarfile.open(archive,'w:gz') as t:t.add(src,arcname=name+'-1')
   digest=hashlib.sha256(archive.read_bytes()).hexdigest()
   locks[name]={'version':'1','url':'https://invalid.invalid/'+archive.name,'archive_sha256':digest}
  (dep/'lock.json').write_text(json.dumps({'dependencies':locks}))
  archive=cache/f'{stage}-1.tar.gz'
  if fault=='missing':archive.unlink()
  if fault=='corrupt':archive.write_bytes(b'corrupt owned fixture')
  if fault=='bad_archive':
   archive.write_bytes(b'not an archive');locks[stage]['archive_sha256']=hashlib.sha256(archive.read_bytes()).hexdigest();(dep/'lock.json').write_text(json.dumps({'dependencies':locks}))
  if fault=='no_marker':
   archive.unlink();src=root/stage;(src/'CMakeLists.txt').unlink()
   with tarfile.open(archive,'w:gz') as t:t.add(src,arcname=stage+'-1')
   locks[stage]['archive_sha256']=hashlib.sha256(archive.read_bytes()).hexdigest();(dep/'lock.json').write_text(json.dumps({'dependencies':locks}))
  if fault=='parent_file':
   cache=root/'parent';cache.write_text('owned sentinel');cache=cache/'subdir'
  if fault=='symlink_cache':
   linked=root/'cache-link';linked.symlink_to(cache,target_is_directory=True);cache=linked
  if fault=='lock_dir':(cache/'acquire.lock').mkdir()
  if fault=='archive_dir':archive.unlink();archive.mkdir()
  content=(BASE/'CMakeLists.txt').read_text().replace('project(sorter_metadata LANGUAGES C CXX)','project(sorter_metadata NONE)')
  trap='''\nmacro(file)\n if("${ARGV0}" STREQUAL "COPY")\n  _file(WRITE "${CMAKE_BINARY_DIR}/COPY_REACHED" "${ARGV1}")\n  message(FATAL_ERROR "SAFE_COPY_TRAP_NO_COPY_PERFORMED")\n endif()\n _file(${ARGV})\nendmacro()\n'''
  content=content.replace('include(cmake/AcquireDependency.cmake)',trap+'\ninclude(cmake/AcquireDependency.cmake)')
  (bridge/'CMakeLists.txt').write_text(content)
  proc=subprocess.run([CMAKE,'-S',str(bridge),'-B',str(root/'build'),'-DSORTER_DEPENDENCY_CACHE='+str(cache),'-DSORTER_DEPENDENCIES_OFFLINE=ON'],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,encoding="utf-8",errors="replace",timeout=35)
  reached=(root/'build/COPY_REACHED').exists(); expect=case in ('positive', 'symlink_cache')
  result={'case':case,'exit':proc.returncode,'copy_reached':reached,'passed':proc.returncode!=0 and reached==expect}
  (OUT/(case+'.log')).write_text(proc.stdout);results.append(result)
for stage in ['taglib','zlib']:
 for fault in ['missing','corrupt','bad_archive','no_marker','archive_dir']:run(stage+'_'+fault,fault,stage)
for fault in ['parent_file','lock_dir']:run(fault,fault)
for label,source in [('empty',''),('root','/'),('drive_root','C:/'),('unc_root','//server/share/'),('relative','.'),('build','${CMAKE_BINARY_DIR}'),('source','${CMAKE_CURRENT_SOURCE_DIR}')]:run('mock_'+label,mock=source)
run('mock_unwind',mock='unwind')
try:
 (OUT/'symlink-probe').symlink_to(OUT, target_is_directory=True)
except OSError as exc:
 print('SKIP symlink cases: host cannot create directory symlinks:', exc)
else:
 (OUT/'symlink-probe').unlink()
 run('mock_symlink_source',mock='symlink_source')
 run('symlink_cache',fault='symlink_cache')
run('positive')
print(json.dumps(results,indent=2))
for result in results:
 if not result['passed']:
  print((OUT/(result['case']+'.log')).read_text())
assert all(r['passed'] for r in results)
_scratch.cleanup()
