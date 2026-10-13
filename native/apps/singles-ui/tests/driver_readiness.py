"""Retry only the read-only pre-session WebDriver status probe.

The Tauri proxy can listen before its child msedgedriver does. An empty proxy
response in that interval is not an application/session failure. Never retry a
session creation, click, apply command, or an arbitrary HTTP protocol error.
References: https://www.w3.org/TR/webdriver2/#status
https://docs.python.org/3/library/http.client.html#http.client.RemoteDisconnected
"""
import http.client
import time
import urllib.error


def wait_driver_ready(call, process, timeout=45):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        code = process.poll()
        if code is not None:
            raise RuntimeError(f'Tauri driver exited before readiness (exit {code})')
        try:
            status = call('GET', '/status', timeout=max(.001, deadline-time.monotonic()))
            if isinstance(status, dict) and status.get('ready') is True:
                return status
            last = repr(status)
        except urllib.error.HTTPError:
            raise
        except (urllib.error.URLError, http.client.RemoteDisconnected) as error:
            last = f'{type(error).__name__}: {error}'
        time.sleep(.2)
    raise TimeoutError(f'WebDriver did not become ready in {timeout}s: {last}')
