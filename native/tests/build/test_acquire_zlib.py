import hashlib
import io
import json
from pathlib import Path
import tempfile
import sys
import time
import unittest
import urllib.error
import acquire_zlib as a

GOOD = b'owned synthetic archive bytes'
PIN = hashlib.sha256(GOOD).hexdigest()


class Response(io.BytesIO):
    def __init__(self, data, status=200, url=a.URL):
        super().__init__(data)
        self.status, self.url = status, url
        self.headers = {'Content-Type': 'application/octet-stream', 'Content-Length': str(len(data))}


class Opener:
    def __init__(self, *responses):
        self.responses, self.calls = list(responses), 0

    def open(self, url, timeout):
        assert url in (a.URL, a.RELEASE_URL) and 0 < timeout <= 15
        self.calls += 1
        value = self.responses.pop(0)
        if isinstance(value, Exception):
            raise value
        return value


class Acquisition(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.cache, self.evidence = self.root / 'cache', self.root / 'evidence'

    def run_acquire(self, opener, **kw):
        return a.acquire(self.cache, self.evidence, opener, PIN, sleep=lambda _: None, **kw)

    def test_cache_selection_matches_cmake_platform_precedence(self):
        self.assertEqual(a.default_cache({'SORTER_DEPENDENCY_CACHE': '/explicit', 'LOCALAPPDATA': '/local'}, 'win32', '/home'), '/explicit')
        self.assertEqual(Path(a.default_cache({'LOCALAPPDATA': '/local', 'XDG_CACHE_HOME': '/xdg'}, 'win32', '/home')), Path('/local/SinglesSorter/cache/sources'))
        self.assertEqual(Path(a.default_cache({'XDG_CACHE_HOME': '/xdg'}, 'linux', '/home')), Path('/xdg/singles-sorter/sources'))
        self.assertEqual(Path(a.default_cache({}, 'linux', '/home')), Path('/home/.cache/singles-sorter/sources'))

    def test_success_and_existing_verified_cache(self):
        op = Opener(Response(GOOD))
        path = self.run_acquire(op)
        self.assertEqual(path.read_bytes(), GOOD)
        self.run_acquire(Opener())
        self.assertTrue(json.loads((self.evidence / 'attempt-1.json').read_text())['verified'])

    def test_bad_bytes_never_promoted_then_exact_retry(self):
        op = Opener(Response(b'<html>temporary error</html>'), Response(GOOD))
        self.assertEqual(self.run_acquire(op).read_bytes(), GOOD)
        receipt = json.loads((self.evidence / 'attempt-1.json').read_text())
        self.assertFalse(receipt['verified'])
        self.assertEqual(receipt['sha256'], hashlib.sha256(b'<html>temporary error</html>').hexdigest())
        self.assertIn('prefix_base64', receipt)
        self.assertEqual(op.calls, 2)

    def test_three_mismatches_fail_without_cache(self):
        op = Opener(*(Response(b'bad') for _ in range(4)))
        with self.assertRaises(RuntimeError): self.run_acquire(op)
        self.assertEqual(op.calls, 4)
        self.assertFalse((self.cache / 'zlib-1.3.1.tar.gz').exists())

    def test_official_release_fallback_requires_identical_pin(self):
        signed = 'https://release-assets.githubusercontent.com/github-production-release-asset/123/owned?sig=temporary'
        op = Opener(Response(b'html'), Response(b'html'), Response(b'html'), Response(GOOD, url=signed))
        self.assertEqual(self.run_acquire(op).read_bytes(), GOOD)
        receipt = json.loads((self.evidence / 'attempt-4.json').read_text())
        self.assertEqual(receipt['url'], a.RELEASE_URL)
        self.assertNotIn('?', receipt['response_url'])
        self.assertTrue(receipt['verified'])

    def test_release_redirect_is_one_exact_allowlisted_hop(self):
        url = 'https://release-assets.githubusercontent.com/github-production-release-asset/123/owned?sig=x'
        handler = a.OfficialReleaseRedirect()
        request = a.urllib.request.Request(a.RELEASE_URL)
        self.assertEqual(handler.redirect_request(request, None, 302, '', {}, url).full_url, url)
        for bad in ('http://release-assets.githubusercontent.com/github-production-release-asset/x',
                    'https://evil.example/github-production-release-asset/x',
                    'https://release-assets.githubusercontent.com/other/x',
                    'https://user@release-assets.githubusercontent.com/github-production-release-asset/x',
                    'https://release-assets.githubusercontent.com:444/github-production-release-asset/x'):
            with self.assertRaises(a.urllib.error.URLError):
                handler.redirect_request(request, None, 302, '', {}, bad)
        with self.assertRaises(a.urllib.error.URLError):
            handler.redirect_request(a.urllib.request.Request(url), None, 302, '', {}, url)

    def test_bad_existing_cache_is_untouched(self):
        self.cache.mkdir();p = self.cache / 'zlib-1.3.1.tar.gz';p.write_bytes(b'bad')
        with self.assertRaises(ValueError): self.run_acquire(Opener())
        self.assertEqual(p.read_bytes(), b'bad')

    def test_non_200_exact_bytes_not_accepted(self):
        with self.assertRaises(RuntimeError):
            self.run_acquire(Opener(*(Response(GOOD, 503) for _ in range(4))))
        self.assertFalse((self.cache / 'zlib-1.3.1.tar.gz').exists())

    def test_http_error_body_is_retained_then_retry(self):
        error = urllib.error.HTTPError(a.URL, 503, 'unavailable', {'Content-Type':'text/html'}, io.BytesIO(b'error'))
        self.run_acquire(Opener(error, Response(GOOD)))
        r = json.loads((self.evidence / 'attempt-1.json').read_text())
        self.assertEqual(r['status'], 503);self.assertEqual(r['bytes'], 5)

    def test_transport_error_bounded_retry(self):
        self.run_acquire(Opener(urllib.error.URLError('temporary'), Response(GOOD)))
        self.assertIn('URLError', (self.evidence / 'attempt-1.json').read_text())

    def test_changed_url_refused(self):
        with self.assertRaises(RuntimeError):
            self.run_acquire(Opener(*(Response(GOOD, url='https://example.org/') for _ in range(4))))
        self.assertFalse((self.cache / 'zlib-1.3.1.tar.gz').exists())

    def test_redirect_handler_refuses_before_fetch(self):
        with self.assertRaises(urllib.error.URLError):
            a.NoRedirect().redirect_request(None, None, 302, '', {}, 'https://example.org/')

    def test_size_bound(self):
        with self.assertRaises(RuntimeError):
            self.run_acquire(Opener(*(Response(b'x' * (a.MAX_BYTES + 1)) for _ in range(4))))
        r = json.loads((self.evidence / 'attempt-3.json').read_text())
        self.assertEqual(r['bytes'], a.MAX_BYTES + 1)
        self.assertFalse((self.cache / 'zlib-1.3.1.tar.gz').exists())

    def test_deadline_prevents_network(self):
        values = iter([0] + [100] * 12)
        op = Opener()
        with self.assertRaises(RuntimeError): self.run_acquire(op, clock=lambda: next(values))
        self.assertEqual(op.calls, 0)

    def test_outer_deadline_kills_trickling_worker(self):
        start = time.monotonic()
        with self.assertRaises(RuntimeError):
            a.run_bounded([sys.executable, "-c",
                           "import time; [(time.sleep(.01)) for _ in range(10000)]"],
                          self.evidence, timeout=.15)
        self.assertLess(time.monotonic() - start, 5)
        receipt = json.loads((self.evidence / "outer-timeout.json").read_text())
        self.assertFalse(receipt["verified"])
        self.assertEqual(receipt["timeout_seconds"], .15)

    def test_outer_worker_failure_is_not_success(self):
        import subprocess
        with self.assertRaises(subprocess.CalledProcessError):
            a.run_bounded([sys.executable, "-c", "raise SystemExit(7)"], self.evidence)

    def test_cache_race_does_not_overwrite(self):
        class Race(Opener):
            def open(inner, url, timeout):
                (self.cache / 'zlib-1.3.1.tar.gz').write_bytes(b'other')
                return super().open(url, timeout)
        with self.assertRaises(RuntimeError): self.run_acquire(Race(*(Response(GOOD) for _ in range(4))))
        self.assertEqual((self.cache / 'zlib-1.3.1.tar.gz').read_bytes(), b'other')
        self.assertEqual(list(self.cache.glob('.zlib-*')), [])


if __name__ == '__main__': unittest.main()
