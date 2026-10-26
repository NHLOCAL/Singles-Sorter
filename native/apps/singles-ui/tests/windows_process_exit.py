"""Read-only, retained-handle exit proofs for an already observed owned tree.

CIM_DATETIME has microsecond precision; GetProcessTimes uses 100-ns FILETIME.
Bind at the CIM precision with integer arithmetic, retain the full native value,
and never use a tolerance or reopen a PID after cleanup. A signaled retained
handle proves termination even when CIM still returns the process object.

References:
https://learn.microsoft.com/en-us/windows/win32/wmisdk/cim-datetime
https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes
https://learn.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-waitforsingleobject
https://learn.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights
"""
from datetime import datetime, timezone
import math
import ntpath
import re
import time

WAIT_OBJECT_0 = 0
WAIT_TIMEOUT = 258
WAIT_FAILED = 0xFFFFFFFF
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
SYNCHRONIZE = 0x00100000
FILETIME_EPOCH = datetime(1601, 1, 1, tzinfo=timezone.utc)
CIM_ISO = re.compile(r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{6}(?:0)?(?:Z|[+-]\d\d:\d\d)$')


def cim_creation_microseconds(value):
    """Parse CIM's .NET round-trip display without losing or rounding precision."""
    if not isinstance(value, str) or not CIM_ISO.fullmatch(value):
        raise ValueError('Missing or non-microsecond CIM creation identity')
    delta = datetime.fromisoformat(value.replace('Z', '+00:00')) - FILETIME_EPOCH
    return (delta.days * 86400 + delta.seconds) * 1000000 + delta.microseconds


def image_identity(value):
    if not isinstance(value, str) or not value or not ntpath.isabs(value):
        raise ValueError('Missing absolute process image path')
    return ntpath.normcase(ntpath.normpath(value))


class WindowsProcessAPI:
    """Only query/synchronize rights; no terminate, token, or privilege API."""
    def __init__(self):
        import ctypes
        from ctypes import wintypes
        self.ctypes, self.wintypes = ctypes, wintypes
        self.kernel = kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        kernel.OpenProcess.restype = wintypes.HANDLE
        kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
        kernel.GetProcessTimes.restype = wintypes.BOOL
        kernel.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD,
                                                    wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        kernel.QueryFullProcessImageNameW.restype = wintypes.BOOL
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        kernel.WaitForSingleObject.restype = wintypes.DWORD
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.CloseHandle.restype = wintypes.BOOL

    def error(self):
        return self.ctypes.WinError(self.ctypes.get_last_error())

    def open(self, pid):
        handle = self.kernel.OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, False, pid)
        if not handle:
            raise self.error()
        return handle

    def identity(self, handle):
        created, exited, kernel, user = [self.wintypes.FILETIME() for _ in range(4)]
        if not self.kernel.GetProcessTimes(handle, *(self.ctypes.byref(value)
                                                    for value in (created, exited, kernel, user))):
            raise self.error()
        size = self.wintypes.DWORD(32768)
        image = self.ctypes.create_unicode_buffer(size.value)
        if not self.kernel.QueryFullProcessImageNameW(handle, 0, image, self.ctypes.byref(size)):
            raise self.error()
        return (created.dwHighDateTime << 32) | created.dwLowDateTime, image.value

    def wait(self, handle, milliseconds):
        result = self.kernel.WaitForSingleObject(handle, milliseconds)
        if result == WAIT_FAILED:
            raise self.error()
        return result

    def close(self, handle):
        if not self.kernel.CloseHandle(handle):
            raise self.error()


class OwnedProcessHandles:
    """Capture before root cleanup, wait once, classify CIM, then close in finally.

    Acquisition and wait errors are kept per process, never converted to absence.
    Callers must reject ``failures`` even if the final CIM enumeration is empty.
    """
    def __init__(self, observed, api=None):
        self.records = []
        self._handles = []
        self._api = api
        self._waited = False
        seen = set()
        for observed_row in observed:
            record = {'status': 'identity-error'}
            self.records.append(record)
            try:
                record.update({key: observed_row.get(key) for key in ('pid', 'created', 'executable')})
                pid = record['pid']
                if type(pid) is not int or not 0 < pid <= 0xFFFFFFFF or pid in seen:
                    raise ValueError('Invalid or duplicate observed PID')
                seen.add(pid)
                record['cim_creation_microseconds'] = cim_creation_microseconds(record['created'])
                expected_image = image_identity(record['executable'])
                if self._api is None:
                    self._api = WindowsProcessAPI()
                handle = self._api.open(pid)
                # Keep even a mismatched handle only for guaranteed finally cleanup;
                # it can never authorize a wait proof or a termination.
                self._handles.append((record, handle))
                created, image = self._api.identity(handle)
                record['native_creation_100ns'] = created
                record['native_executable'] = image
                if created // 10 != record['cim_creation_microseconds']:
                    raise ValueError('Native creation identity does not match CIM microseconds')
                if image_identity(image) != expected_image:
                    raise ValueError('Native process image does not match observed executable')
                record['status'] = 'retained'
            except Exception as error:
                record['error'] = str(error)

    def wait(self, timeout_seconds, clock=None):
        """Wait all exact handles under one deadline, not a timeout per process."""
        if self._waited:
            raise RuntimeError('Owned-process exit wait must not be retried')
        self._waited = True
        if not math.isfinite(timeout_seconds) or not 0 <= timeout_seconds <= 60:
            raise ValueError('Owned-process wait must be bounded to at most 60 seconds')
        clock = time.monotonic if clock is None else clock
        deadline = clock() + timeout_seconds
        for record, handle in self._handles:
            if record['status'] != 'retained':
                continue
            milliseconds = max(0, math.ceil((deadline - clock()) * 1000))
            record['wait_timeout_ms'] = milliseconds
            try:
                result = self._api.wait(handle, milliseconds)
                record['wait_result'] = result
                if result == WAIT_OBJECT_0:
                    record['status'] = 'signaled'
                elif result == WAIT_TIMEOUT:
                    record['status'] = 'timeout'
                    record['error'] = 'Owned process did not signal within the shared cleanup deadline'
                else:
                    raise RuntimeError(f'Unexpected WaitForSingleObject result: {result}')
            except Exception as error:
                record['status'] = 'wait-error'
                record['error'] = str(error)

    @property
    def failures(self):
        return [record for record in self.records
                if record['status'] != 'signaled' or 'close_error' in record]

    def unconfirmed_residuals(self, residual):
        proved = {(record['pid'], record['cim_creation_microseconds']): record
                  for record in self.records if record['status'] == 'signaled'}
        unconfirmed = []
        for row in residual:
            try:
                proof = proved.get((row.get('pid'), cim_creation_microseconds(row.get('created'))))
                if proof is None:
                    raise ValueError('No signaled handle for this observed identity')
                # CIM may lose the image path after exit. If it supplies one, it
                # must still agree; an unknown/new identity is never excluded.
                if row.get('executable') is not None and image_identity(row['executable']) != image_identity(proof['executable']):
                    raise ValueError('Residual image differs from proved process')
            except (ValueError, TypeError):
                unconfirmed.append(row)
        return unconfirmed

    def close(self):
        handles, self._handles = self._handles, []
        for record, handle in handles:
            try:
                self._api.close(handle)
                record['handle_closed'] = True
            except Exception as error:
                record['close_error'] = str(error)
