"""Original-only acceptance helpers. No IPC replacement or application test API.

The caller owns a real Windows tauri-driver session at Medium integrity. All
negative fixtures are disposable copies; the input package is never edited.
"""
import ctypes
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import time
import wave

from desktop_modes import select_sort_setup
from desktop_polish import set_developer_mode

MODEL_FILES = ('manifest.json', 'tensors.f32', 'tokenizer.json',
               'classifier-v14.weights.bin', 'classifier-v14.vocab.txt')
AI_NAME = 'ישראל כהן - שיר חדש.wav'
ALIAS_NAME = 'אברהם פריד - בדיקה.wav'
ARTIST = 'ישראל כהן'
SETTINGS = dict(copy_mode=False, main_folder_only=False, singles_folder=True,
                exist_only=False, abc_sort=False, duet_mode=False)
REQUIRED_CHECKS = ('original-models-executed', 'disabled-no-inference',
                   'cancel-no-stale-plan', 'back-no-stale-plan', 'reload-no-stale-plan', 'repeat-preview',
                   'missing-model-fail-closed', 'corrupt-model-fail-closed',
                   'protected-files-unchanged')


def require(value, message):
    if not value:
        raise AssertionError(message)


def assert_feature_off_message(text, developer):
    expected = 'מודל הזיהוי הדרוש אינו זמין. הבדיקה לא הופעלה.'
    if developer:
        expected += ' (RECOGNITION_UNAVAILABLE)'
    require(text.strip() == expected,
            f'Feature-off UI error projection differs for developer={developer}: {text!r}')


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def ai_mode(environ):
    value = environ.get('SINGLES_DESKTOP_AI_EXPECTED', 'off')
    require(value in ('off', 'on'), 'SINGLES_DESKTOP_AI_EXPECTED must be off or on')
    return value


def security_descriptor(path):
    """Read owner/group/DACL. Never alter rights, ownership or audit policy."""
    if os.name != 'nt':
        raise RuntimeError('Windows security descriptors require actual Windows')
    from ctypes import wintypes
    api = ctypes.WinDLL('advapi32', use_last_error=True)
    get = api.GetFileSecurityW
    get.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, ctypes.c_void_p,
                   wintypes.DWORD, ctypes.POINTER(wintypes.DWORD)]
    get.restype = wintypes.BOOL
    needed = wintypes.DWORD()
    get(str(path), 7, None, 0, ctypes.byref(needed))
    require(ctypes.get_last_error() == 122 and needed.value > 0,
            f'Cannot size security descriptor: {path}')
    buffer = ctypes.create_string_buffer(needed.value)
    if not get(str(path), 7, buffer, len(buffer), ctypes.byref(needed)):
        raise ctypes.WinError(ctypes.get_last_error())
    return buffer.raw[:needed.value].hex()


def windows_attribute_observation(path):
    """Independent, documented Win32 reads; never mask or set attributes.

    Keep both a path query and FILE_BASIC_INFO through an opened metadata handle.
    Raw CPython lstat attributes remain in separate diagnostic observations.
    https://learn.microsoft.com/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex
    """
    if os.name != 'nt':
        raise RuntimeError('Windows attribute observation requires actual Windows')
    from ctypes import wintypes
    class FileBasicInfo(ctypes.Structure):
        _fields_ = [('creation', ctypes.c_longlong), ('access', ctypes.c_longlong),
                    ('write', ctypes.c_longlong), ('change', ctypes.c_longlong),
                    ('attributes', wintypes.DWORD)]
    class FileIdInfo(ctypes.Structure):
        _fields_ = [('volume', ctypes.c_ulonglong), ('identity', ctypes.c_ubyte * 16)]
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.GetFileAttributesW.argtypes = [wintypes.LPCWSTR]
    kernel.GetFileAttributesW.restype = wintypes.DWORD
    kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                                  ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE]
    kernel.CreateFileW.restype = wintypes.HANDLE
    kernel.GetFileInformationByHandleEx.argtypes = [wintypes.HANDLE, ctypes.c_int,
                                                   ctypes.c_void_p, wintypes.DWORD]
    kernel.GetFileInformationByHandleEx.restype = wintypes.BOOL
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.CloseHandle.restype = wintypes.BOOL
    attributes = kernel.GetFileAttributesW(str(path))
    if attributes == 0xffffffff:
        raise ctypes.WinError(ctypes.get_last_error())
    # FILE_READ_ATTRIBUTES only; full sharing; OPEN_EXISTING; BACKUP_SEMANTICS
    # permits directory handles, OPEN_REPARSE_POINT never follows a substitute.
    handle = kernel.CreateFileW(str(path), 0x80, 7, None, 3, 0x02200000, None)
    if handle == ctypes.c_void_p(-1).value:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        basic = FileBasicInfo()
        if not kernel.GetFileInformationByHandleEx(handle, 0, ctypes.byref(basic), ctypes.sizeof(basic)):
            raise ctypes.WinError(ctypes.get_last_error())
        identity = FileIdInfo()
        if not kernel.GetFileInformationByHandleEx(handle, 18, ctypes.byref(identity), ctypes.sizeof(identity)):
            raise ctypes.WinError(ctypes.get_last_error())
        return {'win32_path_attributes': attributes, 'win32_handle_attributes': basic.attributes,
                'win32_volume_serial': identity.volume,
                'win32_file_id': int.from_bytes(bytes(identity.identity), 'little'),
                'win32_last_write_ticks': basic.write, 'win32_change_ticks': basic.change}
    finally:
        if not kernel.CloseHandle(handle):
            raise ctypes.WinError(ctypes.get_last_error())


def snapshot_differences(before, after):
    differences = []
    for root in sorted(set(before) | set(after)):
        left = {row['path']: row for row in before.get(root, [])}
        right = {row['path']: row for row in after.get(root, [])}
        for path in sorted(set(left) | set(right)):
            a, b = left.get(path), right.get(path)
            if a is None or b is None:
                differences.append(dict(root=root, path=path, field='entry', before=a, after=b))
            else:
                for field in sorted(set(a) | set(b)):
                    if a.get(field) != b.get(field):
                        differences.append(dict(root=root, path=path, field=field, before=a.get(field), after=b.get(field)))
    return differences


def observed_attributes(raw, public=None):
    if public is None:
        return raw
    require(public['win32_path_attributes'] == public['win32_handle_attributes'],
            f"Public Windows attribute APIs disagree: path={public['win32_path_attributes']:#010x}, handle={public['win32_handle_attributes']:#010x}, lstat={raw:#010x}")
    require(not public['win32_handle_attributes'] & 0x400, 'Win32 observed a reparse point')
    # No bit masks: retain every bit returned consistently by both public APIs.
    return public['win32_handle_attributes']


def tree_snapshot(root, permissions=security_descriptor, raw_observations=None, *, recursive=True):
    """Fresh lstat each pass, including directories; reject links/reparse points.

    Access time is intentionally omitted because reading may legitimately update
    it. File identity, last-write time, public attributes, content and ACLs must match.
    CPython issue #126253 documents lstat leaking a variable NTFS-only bit; on
    Windows use exact agreeing GetFileAttributesW/FILE_BASIC_INFO values instead.
    Raw lstat is retained separately, never silently masked.
    https://github.com/python/cpython/issues/126253
    """
    root = Path(root)
    rows = []

    def visit(path):
        info = path.lstat()
        require(not stat.S_ISLNK(info.st_mode) and not
                (getattr(info, 'st_file_attributes', 0) & 0x400),
                f'Protected path is a link/reparse point: {path}')
        directory = stat.S_ISDIR(info.st_mode)
        require(directory or stat.S_ISREG(info.st_mode), f'Special protected path: {path}')
        row = dict(path='.' if path == root else path.relative_to(root).as_posix(),
                   directory=directory, device=info.st_dev, identity=info.st_ino,
                   mode=info.st_mode, attributes=getattr(info, 'st_file_attributes', 0),
                   modified_ns=info.st_mtime_ns, permissions=permissions(path))
        if os.name == 'nt':
            observation = dict(path=row['path'], raw_lstat_attributes=getattr(info, 'st_file_attributes', 0),
                               lstat_device=info.st_dev, lstat_identity=info.st_ino,
                               lstat_modified_ns=info.st_mtime_ns)
            if raw_observations is not None:
                raw_observations.append(observation)
            try:
                public = windows_attribute_observation(path)
                observation.update(public)  # Preserve evidence BEFORE validating agreement.
                row['attributes'] = observed_attributes(getattr(info, 'st_file_attributes', 0), public)
                require((public['win32_volume_serial'], public['win32_file_id']) ==
                        (info.st_dev, info.st_ino), 'Independent Windows handle identifies another object')
                row.update(win32_path_attributes=public['win32_path_attributes'],
                           win32_handle_attributes=public['win32_handle_attributes'],
                           win32_last_write_ticks=public['win32_last_write_ticks'])
            except Exception as error:
                observation['error'] = repr(error)
                raise
        if not directory:
            row.update(size=info.st_size, sha256=digest(path))
        rows.append(row)
        if directory and recursive:
            for child in sorted(path.iterdir(), key=lambda p: p.name):
                visit(child)

    visit(root)
    return rows


def verify_package(root, permissions=security_descriptor):
    root = Path(root)
    rows = tree_snapshot(root, permissions)
    require(rows[0]['directory'], 'Package must be a directory')
    expected = {}
    folded = set()
    for line in (root / 'SHA256SUMS.txt').read_text(encoding='utf-8').splitlines():
        require(re.fullmatch(r'[0-9a-f]{64}  .+', line), 'Malformed package manifest')
        hash_, name = line.split('  ', 1)
        require(not PurePosixPath(name).is_absolute() and '\\' not in name and ':' not in name
                and all(part not in ('', '.', '..') for part in name.split('/'))
                and name.casefold() != 'sha256sums.txt' and name.casefold() not in folded,
                'Unsafe or duplicate package manifest path')
        expected[name] = hash_
        folded.add(name.casefold())
    actual = {row['path']: row['sha256'] for row in rows
              if not row['directory'] and row['path'] != 'SHA256SUMS.txt'}
    require(expected and expected == actual, 'Package missing/extra/changed file')
    require(all('models/original/' + name in expected for name in MODEL_FILES),
            'Package lacks both complete original models')
    require((root / 'examples/target').is_dir() and
            not list((root / 'examples/target').iterdir()), 'Package example target must be empty')
    require(all('examples/source/' + name in expected for name in (AI_NAME, ALIAS_NAME)),
            'Package lacks the authored original-AI examples')
    return digest(root / 'SHA256SUMS.txt')


class ProtectedTrees:
    def __init__(self, roots, output, name):
        self.roots = roots
        self.output = Path(output)
        self.name = name
        self.before = self.snapshot("before")
        self.write('before', self.before)
        self.write('before-observations', self.raw_observations)

    def snapshot(self, context):
        self.raw_observations = {}
        result = {}
        try:
            for name, root in self.roots.items():
                raw = []
                self.raw_observations[name] = raw
                result[name] = tree_snapshot(root, raw_observations=raw)
            return result
        except Exception as error:
            self.write(context + '-observation-failure', dict(error=repr(error),
                completed_snapshots=result, raw_observations=self.raw_observations))
            raise

    def write(self, suffix, value):
        (self.output / f'{self.name}-{suffix}.json').write_text(
            json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')

    def verify(self, context='after'):
        after = self.snapshot(context)
        self.write(context, after)
        self.write(context + '-observations', self.raw_observations)
        if self.before != after:
            differences = snapshot_differences(self.before, after)
            report = dict(context=context, differences=differences,
                          order_only=not differences)
            self.write(context + '-differences', report)
            print('FILESYSTEM_INVARIANCE ' + json.dumps(report, ensure_ascii=False), flush=True)
            require(False, f'{self.name} ({context}): protected contents or metadata changed')


def write_wave(path):
    with wave.open(str(path), 'wb') as audio:
        audio.setnchannels(1)
        audio.setsampwidth(2)
        audio.setframerate(8000)
        audio.writeframes(b'\0\0' * 800)


def make_fixtures(root):
    root = Path(root)
    source, target, cancellation = (root / name for name in ('מקור AI', 'יעד AI', 'מקור ביטול'))
    for directory in (source, target, cancellation):
        directory.mkdir()
    for name in (AI_NAME, ALIAS_NAME):
        write_wave(source / name)
    # Enough real, tiny files to keep cancellation from depending on one fast
    # inference. Jobs are cancelled immediately, not allowed to run to exhaustion.
    for index in range(128):
        write_wave(cancellation / f'ישראל כהן - שיר חדש {index:03d}.wav')
    return source, target, cancellation


def stage_negative(binary, destination, asset, damage):
    require(asset in MODEL_FILES and damage in ('missing', 'corrupt'), 'Invalid negative case')
    destination = Path(destination)
    destination.mkdir()  # Fail rather than reuse or overwrite an existing tree.
    copied = destination / binary.name
    shutil.copy2(binary, copied)
    for library in binary.parent.glob('*.dll'):
        require(not library.is_symlink(), 'DLL symlink cannot be staged')
        shutil.copy2(library, destination / library.name)
    models = destination / 'models/original'
    shutil.copytree(binary.parent / 'models/original', models)
    path = models / asset
    if damage == 'missing':
        path.unlink()
    else:
        with path.open('r+b') as stream:
            byte = stream.read(1)
            require(byte, 'Cannot corrupt an empty model fixture')
            stream.seek(0)
            stream.write(bytes([byte[0] ^ 1]))
    return copied


class DesktopAI:
    def __init__(self, command, script, click, fill, wait, capture, restart, output):
        self.command, self.script = command, script
        self.click, self.fill, self.wait = click, fill, wait
        self.capture, self.restart = capture, restart
        self.output = Path(output)
        self.events = []

    def javascript(self, source, args):
        # Use Tauri's existing invoke function, unchanged. This is the same
        # permission-checked IPC channel used by @tauri-apps/api/core.
        return self.command('POST', '/execute/async', {'script': """
const done = arguments[arguments.length - 1];
const input = arguments[0];
const invoke = (name, args) => window.__TAURI_INTERNALS__.invoke(name, args);
(async () => { SOURCE })().then(value => done({ok:true,value}),
  error => done({ok:false,application_error:typeof error==='object'?error:{message:String(error)}}));
""".replace('SOURCE', source), 'args': [args]})

    def ipc(self, name, args=None, error=None):
        result = self.javascript('return await invoke(input.name, input.args);',
                                 {'name': name, 'args': args or {}})
        self.events.append({'command': name, 'arguments': args or {}, 'response': result})
        if error:
            require(not result['ok'] and result['application_error'].get('code') == error,
                    f'{name}: expected {error}, got {result}')
            return result['application_error']
        require(result['ok'], f'Real IPC {name} failed: {result}')
        return result.get('value')

    def request(self, source, target, profile):
        roots = [self.ipc('resolve_typed_root', {'role': role, 'text': str(path)})
                 for role, path in (('Source', source), ('Target', target))]
        return {'mode': 'Sort', 'source_token': roots[0]['token'], 'target_token': roots[1]['token'],
                'settings': dict(SETTINGS), 'ai_profile': profile}

    def terminal(self, job_id):
        def poll():
            value = self.ipc('job_snapshot', {'jobId': job_id})
            require(value['host_schema'] == 1 and value['job_id'] == job_id,
                    'Mismatched native job identity/schema')
            require(value['status'] in ('running', 'completed', 'failed', 'cancelled'),
                    'Unknown native job state')
            return value if value['status'] != 'running' else None
        return self.wait(poll, timeout=120)

    def page(self, job_id, error=None):
        return self.ipc('get_plan_page', {'jobId': job_id, 'offset': 0, 'limit': 200, 'developerMode': True}, error)

    def preview(self, source, target, profile, expected='completed'):
        request = self.request(source, target, profile)
        job = self.ipc('start_preview', {'request': request})['job_id']
        snapshot = self.terminal(job)
        require(snapshot['status'] == expected, f'Unexpected native terminal state: {snapshot}')
        if expected != 'completed':
            require(snapshot['plan_id'] is None and snapshot['total_rows'] == 0,
                    'Failed/cancelled job retained a plan')
            self.page(job, 'JOB_NOT_COMPLETED')
            return snapshot
        require(snapshot['plan_id'] and snapshot['total_rows'] > 0, 'Completed job lacks a plan')
        require(snapshot.get('inference_executed') is (profile == 'Configured'),
                f'Native inference execution marker does not match {profile}: {snapshot}')
        page = self.page(job)
        require(page['total'] == snapshot['total_rows'] and len(page['rows']) == page['total'],
                'Expected complete bounded native result page')
        return snapshot, page

    @staticmethod
    def assert_artist(page, configured):
        def row(name):
            matches = [item for item in page['rows'] if Path(item['source']).name == name
                       and item['kind'] == 'Single']
            require(len(matches) == 1, f'Expected one real single decision: {name}')
            return matches[0]
        original, alias = row(AI_NAME), row(ALIAS_NAME)
        require('אברהם פריד' in alias['reason'] and alias['destination'],
                'Released alias precedence fixture failed')
        if configured:
            require(original['reason'] == ARTIST and ARTIST in (original['destination'] or ''),
                    'Original NER/classifier positive-gate fixture did not infer expected artist')
        else:
            require(ARTIST not in original['reason'] and original['destination'] is None,
                    'Disabled profile must not infer the original-only artist')

    def choose_profile(self, value, available):
        self.click('#advanced')
        require(self.script('return document.querySelector("#model-status").textContent') ==
                ('זמין' if available else 'לא זמין'), 'Rendered model readiness is incorrect')
        self.script('const select=document.querySelector("#ai-profile");select.value=' +
                    json.dumps(value) + ';select.dispatchEvent(new Event("change",{bubbles:true}));')
        self.click('#advanced-dialog .close-dialog')

    def setup(self, source, target, profile, available):
        if self.script('return document.querySelector("#app").dataset.view') == 'results':
            self.click('#back-setup')
        self.wait(lambda: self.script('return document.querySelector("#app").dataset.view==="setup" && !document.querySelector("#sort").disabled'))
        select_sort_setup(self.script, self.click, self.wait)
        self.fill('#source', str(source))
        self.fill('#target', str(target))
        self.choose_profile(profile, available)

    def ui_result(self):
        self.wait(lambda: self.script('return document.querySelector("#app").dataset.view==="results" && document.querySelectorAll("#rows tr").length>0 && !document.querySelector("#sort").disabled'), timeout=120)
        require(self.script('return document.querySelector("#error").hidden'), 'Preview UI shows an error')
        job_id = self.script('return history.state.jobId')
        require(isinstance(job_id, str) and job_id, 'Results history lacks actual native job ID')
        snapshot = self.terminal(job_id)
        require(snapshot['status'] == 'completed', 'UI rendered a non-completed plan')
        require(snapshot.get('inference_executed') is True,
                'Configured UI preview did not execute native inference')
        return job_id, self.page(job_id)

    def run(self, binary, mode, scratch):
        checks = []
        source, target, cancellation = make_fixtures(scratch)
        protected = ProtectedTrees({'source': source, 'target': target, 'cancel_source': cancellation},
                                   self.output, 'ai-fixtures')
        try:
            status = self.ipc('get_model_status')
            require(status['available'] is (mode == 'on') and status['platform'] == 'windows',
                    f'Wrong native feature/model availability: {status}')
            _, disabled = self.preview(source, target, 'Disabled')
            self.assert_artist(disabled, False)
            checks.append('disabled-no-inference')
            if mode == 'off':
                request = self.request(source, target, 'Configured')
                self.ipc('start_preview', {'request': request}, 'RECOGNITION_UNAVAILABLE')
                self.setup(source, target, 'Configured', False)
                self.click('#tab-mode-sort')
                self.click('#sort')
                self.wait(lambda: self.script('return !document.querySelector("#sort").disabled && !document.querySelector("#error").hidden'))
                require(self.script('return document.querySelector("#app").dataset.developerMode') == 'false',
                        'Feature-off ordinary error check requires developer mode off')
                assert_feature_off_message(self.script('return document.querySelector("#error").textContent'), False)
                self.capture('08-feature-off-ordinary-failure')
                set_developer_mode(self.script, self.click, self.wait, True)
                assert_feature_off_message(self.script('return document.querySelector("#error").textContent'), True)
                self.capture('08-feature-off-developer-failure')
                set_developer_mode(self.script, self.click, self.wait, False)
                assert_feature_off_message(self.script('return document.querySelector("#error").textContent'), False)
                require(self.script('return document.querySelectorAll("#rows tr").length===0'), 'Feature-off request showed stale rows')
                self.capture('08-feature-off-explicit-failure')
                checks.append('feature-off-fail-closed')
            else:
                _, configured = self.preview(source, target, 'Configured')
                self.assert_artist(configured, True)
                checks.append('original-models-executed')
                request = self.request(cancellation, target, 'Configured')
                cancelled = self.javascript("""
const job = await invoke('start_preview', {request:input});
await invoke('cancel_preview', {jobId:job.job_id});
await invoke('cancel_preview', {jobId:job.job_id});
return job;
""", request)
                self.events.append({'command': 'start-and-double-cancel', 'response': cancelled})
                require(cancelled['ok'], f'Real native cancellation failed: {cancelled}')
                old_job = cancelled['value']['job_id']
                result = self.terminal(old_job)
                require(result['status'] == 'cancelled' and result['plan_id'] is None
                        and result['total_rows'] == 0 and result.get('inference_executed') is None
                        and 'PLANNING_CANCELLED' in result['blockers'],
                        f'Cancelled native job retained a result: {result}')
                self.page(old_job, 'JOB_NOT_COMPLETED')
                # Reload must invoke the native page-lifetime cancellation hook.
                # The restarted renderer probes while the old worker is draining;
                # it must recover from BUSY without reviving or exposing that job.
                reload_job = self.ipc('start_preview', {'request': request})['job_id']
                self.command('POST', '/refresh', {})
                self.wait(lambda: self.script('return document.documentElement.dataset.startupState==="ready" && !document.querySelector("#sort").disabled'), timeout=120)
                reloaded = self.terminal(reload_job)
                require(reloaded['status'] == 'cancelled' and reloaded['plan_id'] is None
                        and reloaded['total_rows'] == 0,
                        f'Reload failed to discard the running native job: {reloaded}')
                self.page(reload_job, 'JOB_NOT_COMPLETED')
                require(self.script('return document.querySelector("#app").dataset.view==="setup" && document.querySelectorAll("#rows tr").length===0 && document.querySelector("#reopen-results").hidden'),
                        'Reload restored a stale plan')
                self.capture('09-ai-reload')
                checks.append('reload-no-stale-plan')
                # Actual UI Cancel and browser Back cover interrupted/repeated
                # navigation independently of the native cancellation above.
                for action in ('cancel', 'back'):
                    self.setup(cancellation, target, 'Configured', True)
                    self.script('document.querySelector("#sort").click();document.querySelector("#sort").click();')
                    self.wait(lambda: self.script('return document.querySelector("#app").dataset.view==="processing"'))
                    if action == 'cancel':
                        self.click('#cancel-preview')
                    else:
                        self.command('POST', '/back', {})
                    self.wait(lambda: self.script('return document.querySelector("#app").dataset.view==="setup" && !document.querySelector("#sort").disabled'), timeout=120)
                    require(self.script('return document.querySelectorAll("#rows tr").length===0 && document.querySelector("#reopen-results").hidden'),
                            f'{action} exposed a stale native plan')
                    self.command('POST', '/forward', {})
                    time.sleep(.3)
                    require(self.script('return document.querySelector("#app").dataset.view==="setup" && document.querySelectorAll("#rows tr").length===0'),
                            f'Forward after {action} revived a stale plan')
                    self.capture('09-ai-' + action)
                checks.extend(('cancel-no-stale-plan', 'back-no-stale-plan'))
                self.setup(source, target, 'Configured', True)
                last_job = None
                for attempt in range(2):
                    self.script('document.querySelector("#sort").click();document.querySelector("#sort").click();')
                    job, page = self.ui_result()
                    require(job != last_job, 'Repeat UI preview reused an old native job')
                    self.assert_artist(page, True)
                    if last_job:
                        self.ipc('job_snapshot', {'jobId': last_job}, 'UNKNOWN_JOB')
                    last_job = job
                    self.capture(f'10-ai-real-preview-{attempt + 1}')
                    self.click('#back-setup')
                    self.wait(lambda: self.script('return document.querySelector("#app").dataset.view==="setup"'))
                self.ipc('job_snapshot', {'jobId': old_job}, 'UNKNOWN_JOB')
                checks.append('repeat-preview')
                for damage in ('missing', 'corrupt'):
                    for asset in ('tensors.f32', 'classifier-v14.weights.bin'):
                        label = f'{damage}-{asset.split(".")[0]}'
                        copied = stage_negative(binary, scratch / label, asset, damage)
                        guard = ProtectedTrees({'negative_copy': copied.parent}, self.output, label)
                        try:
                            self.restart(copied, label)
                            require(self.ipc('get_model_status')['available'] is False,
                                    f'{label}: damaged original model reported ready')
                            failed = self.preview(source, target, 'Configured', 'failed')
                            require('RECOGNITION_UNAVAILABLE' in failed['blockers'],
                                    f'{label}: model failure silently changed profile')
                            _, no_ai = self.preview(source, target, 'Disabled')
                            self.assert_artist(no_ai, False)
                            self.setup(source, target, 'Configured', False)
                            self.click('#tab-mode-sort')
                            self.click('#sort')
                            self.wait(lambda: self.script('return !document.querySelector("#sort").disabled && !document.querySelector("#error").hidden'), timeout=120)
                            require(self.script('return document.querySelector("#app").dataset.view==="setup" && document.querySelectorAll("#rows tr").length===0'),
                                    f'{label}: error UI retained a stale result')
                            self.capture('11-ai-' + label)
                        finally:
                            # Release copied runtime before verifying/deleting its files.
                            self.restart(binary, label + '-restored')
                            guard.verify()
                    checks.append(damage + '-model-fail-closed')
        finally:
            (self.output / 'original-ai-ipc.json').write_text(
                json.dumps(self.events, ensure_ascii=False, indent=2), encoding='utf-8')
            protected.verify()
        checks.append('protected-files-unchanged')
        return checks
