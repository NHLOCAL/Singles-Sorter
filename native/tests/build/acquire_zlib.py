"""CI-only bounded acquisition of the unchanged official zlib archive.

CMake independently rechecks the same lock hash. No extraction or runtime network.
"""
import argparse
import base64
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
import tempfile
import time
import urllib.error
import urllib.request
from urllib.parse import urlsplit, urlunsplit

URL = 'https://zlib.net/fossils/zlib-1.3.1.tar.gz'
SHA256 = '9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23'
RELEASE_URL = 'https://github.com/madler/zlib/releases/download/v1.3.1/zlib-1.3.1.tar.gz'
MAX_BYTES = 4 * 1024 * 1024


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise urllib.error.URLError('Official archive redirect refused')



def release_asset_url(url):
    p = urlsplit(url)
    return (p.scheme == 'https' and p.hostname == 'release-assets.githubusercontent.com'
            and p.port in (None, 443) and p.username is None and p.password is None
            and p.path.startswith('/github-production-release-asset/') and not p.fragment)


class OfficialReleaseRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if req.full_url != RELEASE_URL or not release_asset_url(newurl):
            raise urllib.error.URLError('Unreviewed official release redirect refused')
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def receipt_url(url):
    # Do not retain the temporary signed release-asset query.
    p = urlsplit(url)
    return urlunsplit((p.scheme, p.netloc, p.path, '', ''))

def acquire(cache, evidence, opener=None, expected=SHA256, clock=time.monotonic, sleep=time.sleep):
    cache, evidence = Path(cache), Path(evidence)
    cache.mkdir(parents=True, exist_ok=True)
    evidence.mkdir(parents=True, exist_ok=True)
    target = cache / 'zlib-1.3.1.tar.gz'
    if target.is_symlink():
        raise ValueError('Refusing archive symlink')
    if target.exists():
        if not target.is_file() or target.stat().st_size > MAX_BYTES:
            raise ValueError('Invalid existing archive')
        if hashlib.sha256(target.read_bytes()).hexdigest() != expected:
            raise ValueError('Existing cache hash mismatch; left untouched')
        (evidence / 'cache.json').write_text(json.dumps({'verified_cached_sha256': expected}) + '\n')
        return target
    primary_opener = opener or urllib.request.build_opener(NoRedirect())
    release_opener = opener or urllib.request.build_opener(OfficialReleaseRedirect())
    deadline = clock() + 90
    for attempt in range(1, 5):
        url = URL if attempt <= 3 else RELEASE_URL
        active_opener = primary_opener if attempt <= 3 else release_opener
        receipt = {'attempt': attempt, 'url': url, 'expected_sha256': expected}
        data = bytearray()
        response = None
        try:
            if clock() >= deadline:
                raise TimeoutError('Acquisition deadline exhausted')
            try:
                response = active_opener.open(url, timeout=min(15, deadline - clock()))
            except urllib.error.HTTPError as error:
                response = error
            with response:
                receipt.update(status=response.status, response_url=receipt_url(response.url),
                               headers={k: response.headers.get(k) for k in
                                        ('Content-Type', 'Content-Length', 'Date', 'Server')})
                if response.url != url and not (url == RELEASE_URL and release_asset_url(response.url)):
                    raise ValueError('Response URL changed')
                while True:
                    if clock() >= deadline:
                        raise TimeoutError('Acquisition deadline exhausted')
                    chunk = response.read(min(65536, MAX_BYTES + 1 - len(data)))
                    if not chunk:
                        break
                    data.extend(chunk)
                    if len(data) > MAX_BYTES:
                        raise ValueError('Response exceeds byte bound')
            receipt.update(bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                           prefix_base64=base64.b64encode(data[:256]).decode(),
                           gzip_magic=data[:2] == b'\x1f\x8b')
            if receipt['status'] != 200 or receipt['sha256'] != expected:
                raise ValueError('Official response status/hash mismatch')
            # Exclusive promotion: never overwrite any existing cache entry.
            temporary = None
            try:
                with tempfile.NamedTemporaryFile(dir=cache, prefix='.zlib-', delete=False) as f:
                    temporary = Path(f.name)
                    f.write(data)
                    f.flush()
                    os.fsync(f.fileno())
                os.link(temporary, target)
            finally:
                if temporary is not None:
                    temporary.unlink()
            receipt['verified'] = True
        except (OSError, ValueError) as error:
            receipt.update(error=type(error).__name__ + ': ' + str(error), verified=False,
                           bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                           prefix_base64=base64.b64encode(data[:256]).decode())
        finally:
            (evidence / ('attempt-%d.json' % attempt)).write_text(json.dumps(receipt, indent=2) + '\n')
        if receipt.get('verified'):
            return target
        if attempt < 4 and clock() + 2 < deadline:
            sleep(2)
    raise RuntimeError('Official zlib acquisition failed; retained bounded response receipts')


def run_bounded(command, evidence, timeout=100):
    """The parent kills and reaps a stalled worker, including trickling reads."""
    try:
        subprocess.run(command, check=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        evidence = Path(evidence)
        evidence.mkdir(parents=True, exist_ok=True)
        (evidence / "outer-timeout.json").write_text(json.dumps({
            "verified": False, "error": "worker exceeded total wall-clock bound",
            "timeout_seconds": timeout, "expected_sha256": SHA256, "url": URL,
        }, indent=2) + "\n")
        raise RuntimeError("Official acquisition worker exceeded wall-clock bound") from error



def default_cache(env, platform, home):
    if env.get('SORTER_DEPENDENCY_CACHE'):
        return env['SORTER_DEPENDENCY_CACHE']
    if platform == 'win32' and env.get('LOCALAPPDATA'):
        return str(Path(env['LOCALAPPDATA']) / 'SinglesSorter' / 'cache' / 'sources')
    return str(Path(env.get('XDG_CACHE_HOME', str(Path(home) / '.cache'))) / 'singles-sorter' / 'sources')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument("--worker", action="store_true", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if not args.worker:
        run_bounded([sys.executable, str(Path(__file__).resolve()), "--worker",
                     "--evidence", str(args.evidence)], args.evidence)
        return
    lock = json.loads((Path(__file__).resolve().parents[2] / 'crates/sorter-io/dependencies/lock.json').read_text())['dependencies']['zlib']
    if lock['url'] != URL or lock['archive_sha256'] != SHA256 or lock['version'] != '1.3.1':
        raise ValueError('Acquisition helper differs from reviewed source lock')
    cache = default_cache(os.environ, sys.platform, Path.home())
    if not Path(cache).is_absolute():
        raise ValueError('Cache must be absolute')
    acquire(cache, args.evidence)


if __name__ == '__main__':
    main()
