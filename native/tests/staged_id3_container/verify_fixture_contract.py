"""Independent owned-container decode check; does not execute the Rust writer."""
import hashlib
from pathlib import Path
import subprocess
import tempfile
if not __debug__:
    raise RuntimeError('Assertions must remain enabled')
BASE = Path(__file__).resolve().parents[1] / 'fixtures' / 'media'
def sync(n):
    return bytes((n >> shift) & 127 for shift in (21, 14, 7, 0))
def tag(text):
    body = b''
    for key in (b'TPE1', b'TPE2', b'TIT2', b'TALB', b'TCON'):
        payload = b'\x03' + text.encode('utf8')
        body += key + sync(len(payload)) + b'\0\0' + payload
    body += b'PRIV' + sync(12) + b'\0\0' + b'owned\0opaque'
    body += bytes(512-len(body))
    return b'ID3\x04\0\0' + sync(len(body)) + body
def container(kind, block):
    data = bytearray((BASE / ('blank.' + kind)).read_bytes())
    if kind == 'dsf':
        data[20:28] = len(data).to_bytes(8,'little')
        data += block
        data[12:20] = len(data).to_bytes(8,'little')
    else:
        order = 'little' if kind == 'wav' else 'big'
        data += b'ID3 ' + len(block).to_bytes(4,order) + block
        if len(block) % 2:
            data += b'\0'
        data[4:8] = (len(data)-8).to_bytes(4,order)
    return bytes(data)
def decode(path):
    result = subprocess.run(['ffmpeg','-v','error','-i',str(path),'-map','0:a:0','-f','s16le','-'],check=True,capture_output=True)
    assert result.stdout
    return result.stdout
with tempfile.TemporaryDirectory(prefix='owned-id3-container-') as tmp:
    for kind in ('wav','aiff','dsf'):
        before, after = container(kind,tag('à')), container(kind,tag('א'))
        assert len(before) == len(after)
        start = before.index(b'ID3\x04')
        assert before[:start] == after[:start]
        a, b = Path(tmp)/('before.'+kind), Path(tmp)/('after.'+kind)
        a.write_bytes(before); b.write_bytes(after)
        assert decode(a) == decode(b)
        assert b'owned\0opaque' in after
        print(kind, 'PCM preserved', hashlib.sha256(decode(b)).hexdigest())
