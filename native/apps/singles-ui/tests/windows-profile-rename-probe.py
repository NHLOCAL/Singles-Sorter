"""Owned Windows probe: strict directory guard versus child atomic rename APIs."""
import ctypes as c
from ctypes import wintypes as w
import os
from pathlib import Path
import tempfile

assert os.name == "nt", "This probe requires actual Windows"
k = c.WinDLL("kernel32", use_last_error=True)
n = c.WinDLL("ntdll")
k.CreateFileW.argtypes = [w.LPCWSTR, w.DWORD, w.DWORD, c.c_void_p, w.DWORD, w.DWORD, w.HANDLE]
k.CreateFileW.restype = w.HANDLE
k.CloseHandle.argtypes = [w.HANDLE]
k.CloseHandle.restype = w.BOOL
k.WriteFile.argtypes = [w.HANDLE, c.c_void_p, w.DWORD, c.POINTER(w.DWORD), c.c_void_p]
k.WriteFile.restype = w.BOOL
k.FlushFileBuffers.argtypes = [w.HANDLE]
k.FlushFileBuffers.restype = w.BOOL
k.MoveFileExW.argtypes = [w.LPCWSTR, w.LPCWSTR, w.DWORD]
k.MoveFileExW.restype = w.BOOL
k.SetFileAttributesW.argtypes = [w.LPCWSTR, w.DWORD]
k.SetFileAttributesW.restype = w.BOOL

class Status(c.Structure):
    _fields_ = [("status_or_pointer", c.c_void_p), ("information", c.c_size_t)]

class Rename(c.Structure):
    _fields_ = [("replace", w.BOOLEAN), ("root", w.HANDLE),
                ("length", w.DWORD), ("name", w.WCHAR * 1)]

n.NtSetInformationFile.argtypes = [w.HANDLE, c.POINTER(Status), c.c_void_p, w.ULONG, w.ULONG]
n.NtSetInformationFile.restype = w.LONG
n.RtlNtStatusToDosError.argtypes = [w.LONG]
n.RtlNtStatusToDosError.restype = w.ULONG


def check(ok, stage):
    if not ok:
        raise OSError(c.get_last_error(), stage)
    print("PASS:", stage, flush=True)


def open_file(path, access, share, disposition, flags):
    handle = k.CreateFileW(str(path), access, share, None, disposition, flags, None)
    if handle == c.c_void_p(-1).value:
        raise OSError(c.get_last_error(), "CreateFileW " + path.name)
    return handle


def rename_same_directory(handle, name):
    assert name in ("profile.json", "profile.previous.json")
    encoded = name.encode("utf-16-le")
    size = max(c.sizeof(Rename), Rename.name.offset + len(encoded))
    # Aligned, zero-initialized storage for native pointer-sized structure fields.
    storage = (c.c_size_t * ((size + c.sizeof(c.c_size_t) - 1) // c.sizeof(c.c_size_t)))()
    info = c.cast(storage, c.POINTER(Rename)).contents
    info.replace = 1
    info.root = None
    info.length = len(encoded)
    c.memmove(c.addressof(storage) + Rename.name.offset, encoded, len(encoded))
    ios = Status()
    status = n.NtSetInformationFile(handle, c.byref(ios), storage, size, 10)
    if status < 0:
        raise OSError(n.RtlNtStatusToDosError(status), "NtSetInformationFile basename rename")
    assert status == 0, ("Unexpected non-completion status", status)
    print("PASS: NtSetInformationFile basename rename", flush=True)


with tempfile.TemporaryDirectory(prefix="singles-profile-probe-") as owned:
    directory = Path(owned) / "profile-v1"
    directory.mkdir()
    guard = open_file(directory, 0x81, 1, 3, 0x02000000 | 0x00200000)
    try:
        for index, destination in enumerate(("profile.json", "profile.previous.json", "profile.json")):
            source = directory / ("owned-temp-" + str(index))
            handle = open_file(source, 0xC0010000, 7, 1, 0x100)  # read/write/delete; TEMPORARY
            try:
                written = w.DWORD()
                data = ("value-" + str(index)).encode()
                check(k.WriteFile(handle, data, len(data), c.byref(written), None), "write child")
                assert written.value == len(data)
                check(k.FlushFileBuffers(handle), "sync child")
                check(k.SetFileAttributesW(str(source), 0x80), "clear temporary attribute")
                result = k.MoveFileExW(str(source), str(directory / destination), 1)
                error = c.get_last_error()
                assert not result and error == 32, ("Expected parent sharing conflict", result, error)
                print("CONFIRMED: absolute MoveFileExW conflicts with strict parent guard (32)", flush=True)
                rename_same_directory(handle, destination)
                assert not source.exists()
            finally:
                k.CloseHandle(handle)
            # Python pathname reads do not share DELETE with the rename handle.
            # Match production: release the child handle, retain the parent guard.
            assert (directory / destination).read_bytes() == data
        result = k.MoveFileExW(str(directory), str(Path(owned) / "moved"), 0)
        error = c.get_last_error()
        assert not result and error == 32, ("Directory guard must reject rename", result, error)
        print("PASS: guard still prevents directory rename", flush=True)
    finally:
        k.CloseHandle(guard)
    check(k.MoveFileExW(str(directory), str(Path(owned) / "moved"), 0), "rename after guard release")
print("PASS: strict-guard native same-directory replacement probe", flush=True)
