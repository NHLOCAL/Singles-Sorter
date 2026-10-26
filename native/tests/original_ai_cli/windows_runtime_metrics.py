"""Read-only process telemetry for the existing owned Windows acceptance lane.

Microsoft PROCESS_MEMORY_COUNTERS: peak working set is an OS process-lifetime
high-water mark, not private allocation or a resettable per-operation counter.
https://learn.microsoft.com/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo
"""
from __future__ import annotations
import ctypes
import os
import subprocess
import threading
import time


class ProcessMemory:
    def __init__(self, pid):
        if os.name != 'nt':
            raise OSError('Windows process memory telemetry requires Windows')
        from ctypes import wintypes
        class Counters(ctypes.Structure):
            _fields_ = [('cb', wintypes.DWORD), ('PageFaultCount', wintypes.DWORD)] + [
                (name, ctypes.c_size_t) for name in ('PeakWorkingSetSize', 'WorkingSetSize',
                'QuotaPeakPagedPoolUsage', 'QuotaPagedPoolUsage', 'QuotaPeakNonPagedPoolUsage',
                'QuotaNonPagedPoolUsage', 'PagefileUsage', 'PeakPagefileUsage')]
        self.counters = Counters
        self.kernel = ctypes.WinDLL('kernel32', use_last_error=True)
        self.kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
        self.kernel.OpenProcess.restype = wintypes.HANDLE
        self.kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        self.kernel.CloseHandle.restype = wintypes.BOOL
        self.kernel.K32GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
        self.kernel.K32GetProcessMemoryInfo.restype = wintypes.BOOL
        # Query only the exact owned child or this harness, never change limits.
        self.handle = self.kernel.OpenProcess(0x1000, False, pid)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())

    def snapshot(self):
        value = self.counters(); value.cb = ctypes.sizeof(value)
        if not self.kernel.K32GetProcessMemoryInfo(self.handle, ctypes.byref(value), value.cb):
            raise ctypes.WinError(ctypes.get_last_error())
        return {'working_set_bytes': value.WorkingSetSize,
                'process_lifetime_peak_working_set_bytes': value.PeakWorkingSetSize,
                'process_lifetime_peak_pagefile_bytes': value.PeakPagefileUsage}

    def close(self):
        if self.handle:
            self.kernel.CloseHandle(self.handle); self.handle = None


class Sampler:
    def __init__(self, snapshot, interval=0.01):
        self.snapshot = snapshot; self.interval = interval
        self.stop_event = threading.Event(); self.thread = None
        self.samples = 0; self.maximum = {}; self.errors = []

    def sample(self):
        try:
            for name, value in self.snapshot().items():
                self.maximum[name] = max(self.maximum.get(name, 0), value)
            self.samples += 1
        except OSError as error:
            text = str(error)
            if text not in self.errors: self.errors.append(text)

    def start(self):
        self.sample()
        def poll():
            while not self.stop_event.wait(self.interval): self.sample()
        self.thread = threading.Thread(target=poll, name='owned-process-memory', daemon=True)
        self.thread.start(); return self

    def finish(self):
        self.stop_event.set()
        if self.thread: self.thread.join()
        self.sample()
        return {'status': 'measured' if self.samples else 'unavailable',
                'samples': self.samples, 'sample_interval_seconds': self.interval,
                'sampled_peak_working_set_bytes': self.maximum.get('working_set_bytes'),
                'process_lifetime_peak_working_set_bytes': self.maximum.get('process_lifetime_peak_working_set_bytes'),
                'process_lifetime_peak_pagefile_bytes': self.maximum.get('process_lifetime_peak_pagefile_bytes'),
                'errors': self.errors,
                'scope': 'one process only; child processes excluded; sampled working set is a lower bound'}


def start_sampler(pid):
    try:
        memory = ProcessMemory(pid)
        return memory, Sampler(memory.snapshot).start()
    except OSError as error:
        return None, {'status': 'unavailable', 'errors': [str(error)]}


def finish_sampler(memory, sampler):
    try:
        return sampler.finish() if memory else sampler
    finally:
        if memory: memory.close()


def measured_call(function, *, runtime):
    memory, sampler = start_sampler(os.getpid())
    started = time.perf_counter_ns()
    try:
        result = function()
    finally:
        elapsed = time.perf_counter_ns() - started
        observed = finish_sampler(memory, sampler)
    return result, dict(elapsed_ns=elapsed, memory=observed, runtime=runtime)


def measured_process(arguments, *, cwd, env, payload=None, timeout=180):
    started = time.perf_counter_ns()
    with subprocess.Popen(arguments, cwd=cwd, env=env, stdin=subprocess.PIPE if payload is not None else subprocess.DEVNULL,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
        memory, sampler = start_sampler(process.pid)
        try:
            try:
                stdout, stderr = process.communicate(payload, timeout=timeout)
            except subprocess.TimeoutExpired:
                process.kill(); process.communicate(); raise
        finally:
            elapsed = time.perf_counter_ns() - started
            observed = finish_sampler(memory, sampler)
        return subprocess.CompletedProcess(arguments, process.returncode, stdout, stderr), {
            'elapsed_ns': elapsed, 'memory': observed,
            'runtime': 'fresh process; startup included; OS filesystem cache is uncontrolled, not a cold-disk benchmark'}
