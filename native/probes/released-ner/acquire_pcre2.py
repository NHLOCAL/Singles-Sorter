"""CI-only official source acquisition. Never downloads at application runtime."""
import argparse, hashlib, json, os, shutil, stat, subprocess, tempfile, urllib.parse, urllib.request, zipfile
from pathlib import Path, PurePosixPath
HERE = Path(__file__).resolve().parent
LOCK = json.loads((HERE / 'pcre2-lock.json').read_text(encoding='utf-8'))
ALLOWED_HOSTS = {'github.com', 'release-assets.githubusercontent.com'}
def allowed(url):
    p = urllib.parse.urlsplit(url)
    if p.scheme != 'https' or p.hostname not in ALLOWED_HOSTS or p.username or p.password or p.port not in (None, 443):
        raise ValueError('untrusted acquisition URL')
class Redirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        allowed(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)
def unpack(archive, destination, lock=LOCK):
    if archive.stat().st_size > lock['maximum_archive_bytes']:
        raise ValueError('archive exceeds bound')
    if hashlib.sha256(archive.read_bytes()).hexdigest() != lock['sha256']:
        raise ValueError('archive hash mismatch')
    with zipfile.ZipFile(archive) as z:
        entries = z.infolist()
        if len(entries) > lock['maximum_members'] or sum(e.file_size for e in entries) > lock['maximum_expanded_bytes']:
            raise ValueError('expanded archive exceeds bound')
        seen = set()
        for e in entries:
            p = PurePosixPath(e.filename)
            key = e.filename.rstrip('/').casefold()
            mode = e.external_attr >> 16
            raw_name = e.filename[:-1] if e.is_dir() else e.filename
            raw_parts = raw_name.split('/')
            reserved = {'CON', 'PRN', 'AUX', 'NUL', 'CONIN$', 'CONOUT$'} | {p + n for p in ('COM', 'LPT') for n in ('1','2','3','4','5','6','7','8','9','¹','²','³')}
            unsafe_windows = any(not part or part in ('.', '..') or
                                 part.split('.')[0].upper() in reserved or
                                 any(ord(c) < 32 or c in '<>"|?*' for c in part)
                                 for part in raw_parts)
            if (not p.parts or p.parts[0] != lock['root'] or p.is_absolute() or
                unsafe_windows or '..' in p.parts or '\\' in e.filename or ':' in e.filename or
                key in seen or stat.S_ISLNK(mode) or e.flag_bits & 1 or
                any(part.rstrip(' .') != part for part in p.parts)):
                raise ValueError('unsafe archive member')
            seen.add(key)
        # Validate all names and aggregate sizes before creating files. No extractall.
        for e in entries:
            out = destination.joinpath(*PurePosixPath(e.filename).parts)
            if e.is_dir():
                out.mkdir(parents=True, exist_ok=True)
            else:
                out.parent.mkdir(parents=True, exist_ok=True)
                with z.open(e) as source, out.open('xb') as target:
                    shutil.copyfileobj(source, target, length=65536)
def acquire(destination, archive=None):
    destination = destination.resolve()
    if destination.exists():
        raise ValueError('refusing to reuse an existing acquisition directory')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='pcre2-staging-', dir=destination.parent) as tmp:
        tmp = Path(tmp)
        if archive is None:
            archive = tmp / 'source.zip'
            allowed(LOCK['url'])
            with urllib.request.build_opener(Redirect()).open(LOCK['url'], timeout=60) as response, archive.open('xb') as out:
                allowed(response.url)
                total = 0
                while chunk := response.read(65536):
                    total += len(chunk)
                    if total > LOCK['maximum_archive_bytes']:
                        raise ValueError('download exceeds bound')
                    out.write(chunk)
        extract = tmp / 'validated'
        extract.mkdir()
        unpack(archive, extract)
        (extract / 'acquisition.json').write_text(json.dumps(LOCK, indent=2)+'\n', encoding='utf-8')
        # Promote only fully downloaded, verified, validated files; failed attempts leave no destination.
        os.rename(extract, destination)
def build(destination):
    source = destination / LOCK['root']
    windows = os.name == 'nt'
    args = ['cmake', '-S', str(source), '-B', str(destination/'build'),
            '-DBUILD_SHARED_LIBS=OFF', '-DBUILD_STATIC_LIBS=ON',
            '-DPCRE2_BUILD_PCRE2_8=OFF', '-DPCRE2_BUILD_PCRE2_16=OFF', '-DPCRE2_BUILD_PCRE2_32=ON',
            '-DPCRE2_BUILD_PCRE2GREP=OFF', '-DPCRE2_BUILD_TESTS=OFF', '-DPCRE2_SUPPORT_JIT=OFF',
            '-DPCRE2_STATIC_PIC=ON', '-DPCRE2_SUPPORT_UNICODE=ON',
            '-DPCRE2_STATIC_RUNTIME=OFF', '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_INSTALL_LIBDIR=lib',
            '-DCMAKE_INSTALL_PREFIX='+str(destination/'install')]
    if windows:
        args += ['-A', 'x64', '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL']
    for command in [args, ['cmake','--build',str(destination/'build'),'--config','Release','--target','pcre2-32-static','--parallel','2'],
                    ['cmake','--install',str(destination/'build'),'--config','Release']]:
        subprocess.run(command, check=True, timeout=300)
    library = destination/'install/lib'/('pcre2-32-static.lib' if windows else 'libpcre2-32.a')
    if not library.is_file():
        raise ValueError('expected static library absent')
    receipt = dict(LOCK, library_sha256=hashlib.sha256(library.read_bytes()).hexdigest(), library_bytes=library.stat().st_size)
    (destination/'build-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
def prepare(destination, archive=None, do_build=False):
    acquire(destination, archive)
    if do_build: build(destination.resolve())
if __name__ == '__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--archive',type=Path);p.add_argument('--build',action='store_true');a=p.parse_args()
    prepare(a.out, a.archive, a.build)
