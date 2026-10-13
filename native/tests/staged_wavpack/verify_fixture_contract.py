"""Independent APE value/PCM fixture check; does not execute Rust."""
from pathlib import Path
import subprocess
import tempfile
if not __debug__:
    raise RuntimeError('Assertions must remain enabled')
AUDIO=(Path(__file__).resolve().parents[1]/'fixtures/media/blank.wv').read_bytes()
def item(key,value,flags=0):
    return len(value).to_bytes(4,'little')+flags.to_bytes(4,'little')+key.encode('ascii')+b'\0'+value
def fixture(text):
    items=b''.join(item(key,text.encode()) for key in ('Artist','Album Artist','Title','Album','Genre'))
    items+=item('Unknown Binary',b'opaque\0\xff',2)
    footer=b'APETAGEX'+(2000).to_bytes(4,'little')+(len(items)+32).to_bytes(4,'little')+(6).to_bytes(4,'little')+(0x80000000).to_bytes(4,'little')+bytes(8)
    header=footer[:20]+(0xa0000000).to_bytes(4,'little')+footer[24:]
    return AUDIO+header+items+footer
with tempfile.TemporaryDirectory(prefix='owned-wavpack-') as temp:
    before,after=fixture('à\0á'),fixture('א\0ב')
    assert len(before)==len(after) and before[:len(AUDIO)]==after[:len(AUDIO)]
    assert b'opaque\0\xff' in after
    outputs=[]
    for name,data in [('before.wv',before),('after.wv',after)]:
        path=Path(temp)/name;path.write_bytes(data)
        result=subprocess.run(['ffmpeg','-v','error','-i',str(path),'-map','0:a:0','-f','s16le','-'],capture_output=True,check=True)
        assert result.stdout;outputs.append(result.stdout)
    assert outputs[0]==outputs[1]
    print('WavPack APE NUL values and decoded PCM preserved')
