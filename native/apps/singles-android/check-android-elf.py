"""Reject a JNI library with statically embedded Bionic before packaging."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def inspect(header, dynamic, symbols, program_headers, abi):
    machine = {'x86_64': 'Advanced Micro Devices X86-64', 'arm64-v8a': 'AArch64'}[abi]
    if 'ELF64' not in header or machine not in header:
        raise ValueError('wrong ELF ABI')
    needed = re.findall(r'\(NEEDED\).*\[([^]]+)\]', dynamic)
    if 'libc.so' not in needed:
        raise ValueError('dynamic Android libc.so dependency missing')
    if not set(needed) <= {'libc.so', 'libdl.so', 'libm.so', 'liblog.so'}:
        raise ValueError('unreviewed shared-library dependency')
    forbidden = {'getauxval', '__cxa_atexit', '__libc_globals', '__libc_init'}
    for line in symbols.splitlines():
        fields = line.split()
        if len(fields) >= 8 and fields[0].endswith(':'):
            name = fields[7].split('@')[0]
            if name in forbidden and fields[6] != 'UND':
                raise ValueError('statically embedded Bionic symbol: ' + name)
    loads = [line.split() for line in program_headers.splitlines() if line.lstrip().startswith('LOAD ')]
    if not loads or any(int(fields[-1], 16) < 16384 for fields in loads):
        raise ValueError('ELF load alignment is below 16 KiB')
    return {'abi': abi, 'needed': needed, 'dynamic_bionic': True,
            'minimum_load_alignment': min(int(fields[-1], 16) for fields in loads),
            'initializer_runtime_test': 'required separately on actual device'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--readelf', required=True)
    parser.add_argument('--library', type=Path, required=True)
    parser.add_argument('--abi', choices=['x86_64', 'arm64-v8a'], required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    def report(option):
        return subprocess.check_output([args.readelf, option, str(args.library)], text=True)
    receipt = inspect(report('-h'), report('-d'), report('-sW'), report('-lW'), args.abi)
    receipt['sha256'] = hashlib.sha256(args.library.read_bytes()).hexdigest()
    args.out.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(receipt))
