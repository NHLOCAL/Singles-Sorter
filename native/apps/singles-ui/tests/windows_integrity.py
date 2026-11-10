"""Read-only Windows process-token checks for owned CI processes."""
import os

MEDIUM_RID = 8192
# Standard-user privileges; administrator-capable privileges are rejected even
# when disabled, since a process could otherwise enable privileges it possesses.
STANDARD_PRIVILEGES = {'SeChangeNotifyPrivilege', 'SeShutdownPrivilege',
                       'SeUndockPrivilege', 'SeIncreaseWorkingSetPrivilege', 'SeTimeZonePrivilege'}
POWER_GROUPS = {'S-1-5-32-' + str(rid) for rid in (544, 547, 548, 549, 550, 551)}


def process_security(pid=None):
    import ctypes
    from ctypes import wintypes
    pid = os.getpid() if pid is None else pid
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    advapi = ctypes.WinDLL('advapi32', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    advapi.OpenProcessToken.argtypes = [wintypes.HANDLE, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)]
    advapi.OpenProcessToken.restype = wintypes.BOOL
    advapi.GetTokenInformation.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p,
                                          wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
    advapi.GetTokenInformation.restype = wintypes.BOOL
    advapi.GetSidSubAuthorityCount.argtypes = [ctypes.c_void_p]
    advapi.GetSidSubAuthorityCount.restype = ctypes.POINTER(ctypes.c_ubyte)
    advapi.GetSidSubAuthority.argtypes = [ctypes.c_void_p, wintypes.DWORD]
    advapi.GetSidSubAuthority.restype = ctypes.POINTER(wintypes.DWORD)
    process = kernel.OpenProcess(0x1000, False, pid)  # QUERY_LIMITED_INFORMATION only
    if not process:
        raise ctypes.WinError(ctypes.get_last_error())
    token = wintypes.HANDLE()
    try:
        if not advapi.OpenProcessToken(process, 0x0008, ctypes.byref(token)):  # TOKEN_QUERY only
            raise ctypes.WinError(ctypes.get_last_error())
        size = wintypes.DWORD()
        advapi.GetTokenInformation(token, 25, None, 0, ctypes.byref(size))  # TokenIntegrityLevel
        if not size.value:
            raise ctypes.WinError(ctypes.get_last_error())
        label = ctypes.create_string_buffer(size.value)
        if not advapi.GetTokenInformation(token, 25, label, size, ctypes.byref(size)):
            raise ctypes.WinError(ctypes.get_last_error())
        sid = ctypes.cast(label, ctypes.POINTER(ctypes.c_void_p))[0]
        count = advapi.GetSidSubAuthorityCount(sid)[0]
        rid = advapi.GetSidSubAuthority(sid, count - 1)[0]
        elevated = wintypes.DWORD()
        if not advapi.GetTokenInformation(token, 20, ctypes.byref(elevated), ctypes.sizeof(elevated),
                                         ctypes.byref(size)):  # TokenElevation
            raise ctypes.WinError(ctypes.get_last_error())
        def info_buffer(kind):
            length = wintypes.DWORD()
            advapi.GetTokenInformation(token, kind, None, 0, ctypes.byref(length))
            if not length.value:
                raise ctypes.WinError(ctypes.get_last_error())
            buffer = ctypes.create_string_buffer(length.value)
            if not advapi.GetTokenInformation(token, kind, buffer, length, ctypes.byref(length)):
                raise ctypes.WinError(ctypes.get_last_error())
            return buffer

        class SidAndAttributes(ctypes.Structure):
            _fields_ = [('sid', ctypes.c_void_p), ('attributes', wintypes.DWORD)]
        class TokenGroups(ctypes.Structure):
            _fields_ = [('count', wintypes.DWORD), ('groups', SidAndAttributes * 1)]
        class Luid(ctypes.Structure):
            _fields_ = [('low', wintypes.DWORD), ('high', wintypes.LONG)]
        class LuidAndAttributes(ctypes.Structure):
            _fields_ = [('luid', Luid), ('attributes', wintypes.DWORD)]
        class TokenPrivileges(ctypes.Structure):
            _fields_ = [('count', wintypes.DWORD), ('privileges', LuidAndAttributes * 1)]
        advapi.ConvertSidToStringSidW.argtypes = [ctypes.c_void_p, ctypes.POINTER(wintypes.LPWSTR)]
        advapi.ConvertSidToStringSidW.restype = wintypes.BOOL
        kernel.LocalFree.argtypes = [ctypes.c_void_p]
        kernel.LocalFree.restype = ctypes.c_void_p
        advapi.LookupPrivilegeNameW.argtypes = [wintypes.LPCWSTR, ctypes.POINTER(Luid),
                                               wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        advapi.LookupPrivilegeNameW.restype = wintypes.BOOL
        groups_buffer = info_buffer(2)
        group_count = ctypes.cast(groups_buffer, ctypes.POINTER(wintypes.DWORD))[0]
        groups = (SidAndAttributes * group_count).from_buffer(groups_buffer, TokenGroups.groups.offset)
        power_groups = []
        for group in groups:
            text = wintypes.LPWSTR()
            if not advapi.ConvertSidToStringSidW(group.sid, ctypes.byref(text)):
                raise ctypes.WinError(ctypes.get_last_error())
            try:
                if text.value in POWER_GROUPS:
                    power_groups.append({'sid': text.value, 'attributes': group.attributes})
            finally:
                kernel.LocalFree(ctypes.cast(text, ctypes.c_void_p))
        privileges_buffer = info_buffer(3)
        privilege_count = ctypes.cast(privileges_buffer, ctypes.POINTER(wintypes.DWORD))[0]
        entries = (LuidAndAttributes * privilege_count).from_buffer(privileges_buffer, TokenPrivileges.privileges.offset)
        privileges = []
        for entry in entries:
            length = wintypes.DWORD()
            advapi.LookupPrivilegeNameW(None, ctypes.byref(entry.luid), None, ctypes.byref(length))
            name = ctypes.create_unicode_buffer(length.value + 1)
            length.value += 1
            if not advapi.LookupPrivilegeNameW(None, ctypes.byref(entry.luid), name, ctypes.byref(length)):
                raise ctypes.WinError(ctypes.get_last_error())
            privileges.append({'name': name.value, 'attributes': entry.attributes})
        filtered = ctypes.cast(info_buffer(21), ctypes.POINTER(wintypes.DWORD))[0]  # TokenHasRestrictions
        elevation_type = ctypes.cast(info_buffer(18), ctypes.POINTER(wintypes.DWORD))[0]
        return {'pid': pid, 'integrity_rid': rid, 'elevated': bool(elevated.value),
                'has_restrictions': bool(filtered), 'elevation_type': elevation_type,
                'power_groups': power_groups, 'privileges': privileges}

    finally:
        if token:
            kernel.CloseHandle(token)
        kernel.CloseHandle(process)


def require_medium(security):
    # Microsoft defines the WebView2 elevated-host restriction by High IL, not
    # TokenElevation. Safer-filtered tokens may retain that historical flag.
    # Never accept the RID alone: independently reject effective admin rights.
    if security['integrity_rid'] != MEDIUM_RID:
        raise RuntimeError(f'Owned UI automation requires Medium integrity: {security}')
    for group in security['power_groups']:
        if not group['attributes'] & 0x10 or group['attributes'] & 0x4:  # SE_GROUP_USE_FOR_DENY_ONLY, cannot be re-enabled
            raise RuntimeError(f'Privileged group is not deny-only: {group}')
    for privilege in security['privileges']:
        if privilege['name'] not in STANDARD_PRIVILEGES:
            raise RuntimeError(f'Administrator-capable privilege remains present: {privilege}')
        if privilege['attributes'] & 0x2 and privilege['name'] != 'SeChangeNotifyPrivilege':
            raise RuntimeError(f'Unexpected enabled standard-user privilege: {privilege}')
    if security['elevated'] and not security['has_restrictions']:
        raise RuntimeError(f'Elevated flag without independently verified token filtering: {security}')
