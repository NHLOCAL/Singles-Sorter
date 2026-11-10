"""Exact official wheel byte provenance, with a separate unchanged source-equivalence check."""
import hashlib,json
from pathlib import Path
HERE=Path(__file__).resolve().parent
WINDOWS=json.loads((HERE/'windows-runtime-sources.json').read_text(encoding='utf-8'))
def digest(raw):return hashlib.sha256(raw).hexdigest()
def validate(relative,raw,reference_sha256,platform_name):
    actual=digest(raw)
    if platform_name=='win32':
        item=WINDOWS['files'].get(relative)
        if item is None or item['original_sha256']!=reference_sha256:
            raise ValueError('unrecognized Windows oracle reference source: '+relative)
        # A generic newline-normalization fallback is forbidden. Exact official raw bytes first.
        if actual!=item['windows_sha256']:
            raise ValueError(f'Windows oracle source bytes changed: {relative}; actual SHA256 {actual}')
        if digest(raw.replace(b'\r\n',b'\n'))!=reference_sha256:
            raise ValueError('Windows oracle source differs beyond reviewed line endings: '+relative)
        scheme='exact-official-windows-wheel-raw-and-original-lf'
    else:
        if actual!=reference_sha256:
            raise ValueError(f'oracle source bytes changed: {relative}; actual SHA256 {actual}')
        scheme='exact-original-source-raw'
    return {'raw_sha256':actual,'original_reference_sha256':reference_sha256,'verification':scheme}
