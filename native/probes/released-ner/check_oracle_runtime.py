"""Fail early on CI interpreter, dependency closure or import drift."""
import importlib,importlib.metadata as metadata,json,platform,sys,unicodedata
from pathlib import Path
from packaging.markers import Marker,default_environment
from packaging.requirements import Requirement
from packaging.utils import canonicalize_name
HERE=Path(__file__).resolve().parent

def check_versions(lock,version,unicode_version,installed,environment):
    if version!=lock['python']:raise ValueError(f'oracle Python {version} != pinned {lock["python"]}')
    if unicode_version!=lock['unicode']:raise ValueError('Unicode database changed')
    expected={n:p for n,p in lock['packages'].items() if not p['marker'] or Marker(p['marker']).evaluate(environment)}
    for name,package in expected.items():
        if installed.get(name)!=package['version']:raise ValueError(f'wrong/missing oracle dependency: {name}')
        for entry in package['requires_dist']:
            r=Requirement(entry)
            if not r.marker or r.marker.evaluate(environment):
                dep=canonicalize_name(r.name)
                if dep not in expected or not r.specifier.contains(installed.get(dep,'')):
                    raise ValueError(f'unsatisfied oracle dependency: {name} -> {entry}')
    return len(expected)

def main():
    lock=json.loads((HERE/'oracle-lock.json').read_text(encoding='utf-8'))
    env=default_environment();env['extra']=''
    installed={canonicalize_name(d.metadata['Name']):d.version for d in metadata.distributions() if d.metadata['Name']}
    count=check_versions(lock,platform.python_version(),unicodedata.unidata_version,installed,env)
    # spaCy imports CLI helpers even for model inference; import these before export/build.
    for module in ('click','typer','weasel','numpy','thinc','srsly','spacy','spacy.cli._util'):
        importlib.import_module(module)
    print(json.dumps({'result':'PASS','python':platform.python_version(),'unicode':unicodedata.unidata_version,'platform':sys.platform,'locked_packages':count,'oracle_imports':True}))
if __name__=='__main__':main()
