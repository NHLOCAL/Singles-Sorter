#!/usr/bin/env python3
"""Re-observe frozen synthetic bytes; run unchanged v14 single-song routing.
No tag saving, actual library input, or model initialization is permitted.
"""
import argparse, hashlib, json, logging, shutil, tempfile, sys, unicodedata
from pathlib import Path
from importlib.metadata import version
import music_tag
from export_full_contract import verify_baseline, release_module

def scalar(v):
    if isinstance(v, int) and not isinstance(v, bool): return {'type':'integer','decimal':str(v)}
    return {'type':type(v).__name__,'value':v}
def error(e): return {'type':type(e).__name__,'message':str(e)}
def observe(path):
    result={}
    try:
        f=music_tag.load_file(str(path));result['open']={'ok':True,'type':type(f).__name__};result['fields']={}
        for key in ['artist','albumartist','title','album','genre','tracknumber']:
            try:
                item=f[key];field={'get_ok':True,'values':list(map(scalar,item.values)),'truthy':bool(item)}
                try:field['value']={'ok':True,'scalar':scalar(item.value)}
                except Exception as e:field['value']={'ok':False,'error':error(e)}
            except Exception as e:field={'get_ok':False,'error':error(e)}
            result['fields'][key]=field
    except Exception as e:result['open']={'ok':False,'error':error(e)}
    return result

def main():
    p=argparse.ArgumentParser();p.add_argument('--baseline',type=Path,required=True);p.add_argument('--media',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
    baseline=a.baseline.resolve();media=a.media.resolve();sources=verify_baseline(baseline)
    assert sys.version_info[:3]==(3,12,14) and unicodedata.unidata_version=='15.0.0'
    assert version('music-tag')=='0.4.3' and version('mutagen')=='1.47.0'
    frozen=json.loads((media/'reference-observations.json').read_text());aliases=[['אמן בדיקה','Mapped Hebrew'],['Artist','Mapped Latin'],['Synthetic Title','Mapped Title'],['Filename Artist','Mapped Filename']]
    rows=[]
    with release_module(baseline) as released, tempfile.TemporaryDirectory(prefix='singles-media-routes-') as directory:
        sorter=released.MusicSorter.__new__(released.MusicSorter);sorter.singer_list=aliases;sorter.ai_models=None;sorter.logger=logging.getLogger('silent-reference');sorter.logger.addHandler(logging.NullHandler());sorter.logger.propagate=False
        for original in frozen['files']:
            name=Path(original['path']).name;source=media/name;before=source.read_bytes();assert hashlib.sha256(before).hexdigest()==original['sha256']
            observed=observe(source)
            assert observed['open']['ok']==original['open']['ok'], name
            if not observed['open']['ok']:assert observed['open']['error']['type']==original['open']['error']['type'],name
            if observed['open']['ok']:assert observed['fields']==original['fields'],name
            for prefix in ['', 'Filename Artist ']:
                path=Path(directory)/(prefix+name);shutil.copyfile(source,path)
                try: route={'ok':True,'artists':sorter.artists_from_song(path)}
                except Exception as e:route={'ok':False,'error':error(e)}
                assert path.read_bytes()==before
                rows.append({'fixture':name,'filename':path.name,'result':route})
            assert source.read_bytes()==before
    a.output.write_text(json.dumps({'release_sources':sources,'aliases':aliases,'scope':'Actual frozen synthetic metadata into unchanged released single-song routing, AI intentionally disabled. No album/job-v2 coverage.','routes':rows},ensure_ascii=False,sort_keys=True,indent=2)+'\n')
if __name__=='__main__':main()
