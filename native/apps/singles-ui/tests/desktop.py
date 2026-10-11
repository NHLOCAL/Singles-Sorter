"""Owned Windows CI acceptance via external tauri-driver, with REAL IPC.

No embedded automation plugin/server is added to the application. This script
is intentionally CI-only and refuses other platforms. Build the app with the
pinned Tauri CLI first; put tauri-driver and matching msedgedriver on PATH.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

from desktop_diagnostics import snapshot, startup_probe, owned_processes, write_json
from windows_integrity import process_security, require_medium
from native_picker import interact_with_owned_picker
from native_fixtures import write_import_csv
from desktop_layout import check_native_layout, capture_platform_fonts, check_phone_layout, assert_editor_actions, METRICS
from desktop_ai import DesktopAI, ProtectedTrees, ai_mode, verify_package, REQUIRED_CHECKS

ROOT = Path(__file__).resolve().parents[1]
if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
    raise SystemExit('Native desktop acceptance must run in an owned Windows GitHub Actions runner.')
BINARY = Path(os.environ.get('SINGLES_DESKTOP_BIN', str(ROOT / 'src-tauri/target/release/singles-sorter-preview.exe'))).resolve(strict=True)
OUTPUT = Path(os.environ.get('SINGLES_UI_EVIDENCE', str(ROOT / 'test-results/native-windows')))
OUTPUT.mkdir(parents=True, exist_ok=True)
security = process_security()
write_json(OUTPUT / 'test-process-security.json', security)
require_medium(security)
AI_MODE = ai_mode(os.environ)
PACKAGE_MANIFEST_SHA256 = None
package_guard = None
if AI_MODE == 'on':
    assert not OUTPUT.resolve().is_relative_to(BINARY.parent), 'Evidence must be outside the input package'
    PACKAGE_MANIFEST_SHA256 = verify_package(BINARY.parent)
    package_guard = ProtectedTrees({'input_package': BINARY.parent}, OUTPUT, 'input-package')
if os.environ.get('SINGLES_UI_TEMP'):
    owned_temp = Path(os.environ['SINGLES_UI_TEMP'])
    owned_temp.mkdir(parents=True, exist_ok=True)
    tempfile.tempdir = str(owned_temp)
    os.environ['TEMP'] = os.environ['TMP'] = str(owned_temp)
DRIVER = shutil.which('tauri-driver')
if not DRIVER:
    raise SystemExit('tauri-driver is not installed; native runtime was not tested.')
BASE = 'http://127.0.0.1:4444'
ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'
session = None
active_binary = BINARY
checks = []
original_ai_checks = []

def call(method, path, body=None):
    data = None if body is None else json.dumps(body).encode('utf-8')
    req = urllib.request.Request(BASE + path, data=data, method=method, headers={'Content-Type':'application/json; charset=utf-8'})
    # Local native test driver only; do not inherit an external proxy.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(req, timeout=30) as response:
        result = json.loads(response.read())
    value = result.get('value')
    if isinstance(value, dict) and value.get('error'):
        raise RuntimeError(f"{value['error']}: {value.get('message')}")
    return value

def verify_app_security(label):
    app_roots = [row for row in owned_processes(active_binary)
                 if os.path.normcase(row.get('executable') or '') == os.path.normcase(str(active_binary))]
    app_security = [process_security(pid) for row in app_roots
                    for pid in (row['pid'], row['parent_pid'])]
    write_json(OUTPUT / f'{label}-process-security.json', app_security)
    assert app_security, 'No owned application process found for integrity verification'
    for token in app_security:
        require_medium(token)

def command(method, path, body=None):
    return call(method, f'/session/{session}{path}', body)

def wait(predicate, timeout=45):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            value = predicate()
            if value:
                return value
        except (urllib.error.URLError, RuntimeError, KeyError) as error:
            last = error
        time.sleep(.2)
    raise TimeoutError(f'Native UI condition was not reached: {last}')

def find(selector):
    return command('POST','/element',{'using':'css selector','value':selector})[ELEMENT]

def click(selector):
    command('POST',f'/element/{find(selector)}/click',{})

def fill(selector, value):
    element = find(selector)
    command('POST',f'/element/{element}/clear',{})
    command('POST',f'/element/{element}/value',{'text':value})

def text(selector):
    return command('GET',f'/element/{find(selector)}/text')

def script(source):
    return command('POST','/execute/sync',{'script':source,'args':[]})

def capture(name):
    (OUTPUT / f'{name}.png').write_bytes(base64.b64decode(command('GET','/screenshot')))

def pick_owned_path(selector, path):
    # Native dialog controls belong to this exact owned preview executable.
    # Never interact with a generic desktop window or another user's app.
    click(selector)
    interact_with_owned_picker(BINARY, path, OUTPUT, selector.removeprefix('#'))

def restart_app(binary, label):
    global session, active_binary
    if session:
        previous_binary = active_binary
        previous_processes = owned_processes(previous_binary)
        command('DELETE', '')
        session = None
        wait(lambda: not owned_processes(previous_binary, observed=previous_processes), timeout=45)
    active_binary = binary
    result = call('POST', '/session', {'capabilities': {'alwaysMatch': {
        'browserName': 'wry', 'tauri:options': {'application': str(binary)}}}})
    session = result['sessionId']
    verify_app_security(label)
    wait(lambda: script("return document.documentElement.dataset.startupState === 'ready' && !document.querySelector('#sort').disabled"), timeout=120)

startup_probe(BINARY, OUTPUT)
log = (OUTPUT / 'tauri-driver.log').open('wb')
process = subprocess.Popen([DRIVER], stdout=log, stderr=subprocess.STDOUT)
try:
    driver_security = process_security(process.pid)
    write_json(OUTPUT / 'tauri-driver-security.json', driver_security)
    require_medium(driver_security)
    wait(lambda: call('GET','/status'))
    result = call('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'wry','tauri:options':{'application':str(BINARY)}}}})
    session = result['sessionId']
    verify_app_security('initial-app')
    wait(lambda: 'מסדר הסינגלים' in text('h1'))
    wait(lambda: script("return document.documentElement.dataset.startupState === 'ready'"))
    wait(lambda: script("return !document.querySelector('#sort').disabled"))
    assert not script("return !!document.querySelector('dialog[open]')"), 'Startup must not open automatic help'
    check_native_layout(command, script, capture, OUTPUT, '01-native-home')
    capture_platform_fonts(command, OUTPUT)
    checks.append('Actual measured desktop CSS viewports retain controls without document overflow')
    capture('01-native-home')
    check_phone_layout(command,script,capture,OUTPUT,'01-native-home')
    checks.append('Actual Windows Tauri/WebView2 rendered local assets and initialized real IPC')
    with tempfile.TemporaryDirectory(prefix='singles-ui-owned-') as temporary:
        scratch = Path(temporary)
        source, target = scratch / 'מקור', scratch / 'יעד'
        source.mkdir(); target.mkdir()
        observer_control = scratch / 'unselected observer control'
        observer_control.mkdir()
        media = source / 'אברהם פריד - בדיקה.mp3'
        shutil.copyfile(ROOT.parents[1] / 'tests/fixtures/media/tagged.mp3', media)
        before = hashlib.sha256(media.read_bytes()).hexdigest()
        fixture_guard = ProtectedTrees({'source': source, 'target': target, 'unselected_control': observer_control}, OUTPUT, 'native-fixtures')
        fixture_guard.verify('before-native-picker')
        pick_owned_path('#source-picker', source)
        wait(lambda: script("return document.querySelector('#source').value") == str(source))
        fixture_guard.verify('after-source-picker')
        pick_owned_path('#target-picker', target)
        wait(lambda: script("return document.querySelector('#target').value") == str(target))
        fixture_guard.verify('after-target-picker')
        checks.append('Native folder dialogs selected only the owned source and target fixtures')
        click('#sort')
        wait(lambda: script("return document.querySelector('#app').dataset.view === 'results' && document.querySelectorAll('#rows tr').length > 0 && !document.querySelector('#sort').disabled"))
        assert 'אברהם פריד' in text('#rows')
        assert script("return document.querySelector('#error').hidden"), text('#error')
        capture('02-native-real-sort-preview')
        check_native_layout(command, script, capture, OUTPUT, '02-native-results')
        check_phone_layout(command,script,capture,OUTPUT,'02-native-results')
        click('#rows tr:last-child .detail-button')
        wait(lambda:script("return document.querySelector('#row-dialog').open"))
        assert script("return document.querySelector('#row-dialog .source-path').value") == str(media)
        capture('02-native-result-details')
        click('#row-close')
        wait(lambda:not script("return document.querySelector('#row-dialog').open"))
        assert media.is_file() and hashlib.sha256(media.read_bytes()).hexdigest() == before
        assert list(target.iterdir()) == []
        fixture_guard.verify('after-sort-preview')
        checks.append('Real owned MP3 sort preview produced Hebrew singer/destination; source hash and empty target unchanged')
        click('#back-setup')
        wait(lambda:script("return document.querySelector('#app').dataset.view === 'setup'"))
        assert script("return document.querySelector('#source').value") == str(source)
        fill('#target','')
        click('#fix')
        wait(lambda: script("return document.querySelector('#app').dataset.view === 'results' && document.querySelectorAll('#rows tr').length > 0 && !document.querySelector('#fix').disabled"))
        assert script("return document.querySelector('#error').hidden"), text('#error')
        capture('03-native-real-fix-preview')
        assert hashlib.sha256(media.read_bytes()).hexdigest() == before
        fixture_guard.verify('after-fixnames-preview')
        checks.append('Target-free real FixNames preview; owned source bytes unchanged')
        click('#back-setup')
        wait(lambda:script("return document.querySelector('#app').dataset.view === 'setup'"))
        # Task 2 uses a separate development profile. The runner has no user profile.
        assert script("return !!document.querySelector('#save-settings')"), "Task 2 preferences UI is missing; stale binary cannot pass"
        click('#copy_mode'); click('#save-settings')
        wait(lambda: text('#settings-notice') == 'נשמר')
        expected_copy = script("return document.querySelector('#copy_mode').checked")
        click('#advanced')
        expected_duet = not script("return document.querySelector('#duet-all').checked")
        click('#duet-all' if expected_duet else '#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-all').disabled"))
        assert script("return document.querySelector('#duet-all').checked") == expected_duet
        capture('04-native-settings')
        check_phone_layout(command,script,capture,OUTPUT,'04-native-settings')
        click('#edit-list')
        wait(lambda: script("return document.querySelector('#list-dialog').open"))
        # The same owned CI profile survives off/on/extracted package lanes.
        # Check this import's exact +2 duplicates without deleting prior results.
        imported_baseline = script("return Array.from(document.querySelectorAll('[id^=list-alias-]')).filter(x=>x.value==='אמן מיובא').length")
        click('#list-add')
        alias_id = script("return Array.from(document.querySelectorAll('[id^=list-alias-]')).at(-1).id")
        folder_id = alias_id.replace('alias','folder')
        fill('#'+alias_id,'אמן בדיקת ממשק'); fill('#'+folder_id,'תיקיית בדיקה')
        click('#list-save'); wait(lambda: text('#list-notice') == 'נשמר')
        assert_editor_actions(script(METRICS))
        capture('05-native-personal-list')
        check_phone_layout(command,script,capture,OUTPUT,'05-native-personal-list')
        click('#list-cancel')
        imported_csv = scratch / 'אמנים.csv'
        write_import_csv(imported_csv)
        imported_csv.write_bytes(imported_csv.read_bytes()+'אמן לעריכה,תיקייה לעריכה\r\nאמן למחיקה,תיקייה למחיקה\r\n'.encode('utf-8'))
        pick_owned_path('#import-list', imported_csv)
        wait(lambda: script("return document.querySelector('#import-dialog').open"))
        assert '4 שורות לייבוא' in text('#import-summary')
        assert script("return document.querySelector('#import-folder-0').value") == 'תיקייה מיובאת'
        fill('#import-alias-2','אמן שנערך לפני אישור')
        fill('#import-folder-2','תיקייה ערוכה')
        click('[data-import-delete="3"]')
        assert '3 שורות לייבוא' in text('#import-summary')
        assert_editor_actions(script(METRICS))
        capture('06-native-import-preview')
        check_phone_layout(command,script,capture,OUTPUT,'06-native-import-preview')
        click('#confirm-import')
        wait(lambda: script("return !document.querySelector('#import-dialog').open"))
        click('#edit-list')
        wait(lambda: script("return Array.from(document.querySelectorAll('[id^=list-alias-]')).filter(x=>x.value==='אמן מיובא').length") == imported_baseline + 2)
        assert script("return Array.from(document.querySelectorAll('[id^=list-alias-]')).some(x=>x.value==='אמן שנערך לפני אישור')")
        assert not script("return Array.from(document.querySelectorAll('[id^=list-alias-]')).some(x=>x.value==='אמן למחיקה')")
        fixture_guard.verify('after-settings-and-import')
        checks.append('Actual CSV draft edits/deletion applied only on confirm; original duplicate aliases remained ordered')
        click('#list-cancel'); click('#advanced-dialog .close-dialog')
        command('DELETE',''); session = None
        restarted = call('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'wry','tauri:options':{'application':str(BINARY)}}}})
        session = restarted['sessionId']
        verify_app_security('restarted-app')
        wait(lambda: script("return document.documentElement.dataset.startupState === 'ready' && !document.querySelector('#sort').disabled"))
        assert script("return document.querySelector('#copy_mode').checked") == expected_copy
        assert script("return document.querySelector('#source').value") == ''
        click('#advanced')
        assert script("return document.querySelector('#duet-all').checked") == expected_duet
        click('#edit-list')
        wait(lambda: 'אמן בדיקת ממשק' in script("return Array.from(document.querySelectorAll('[id^=list-alias-]')).map(x=>x.value).join('|')"))
        capture('07-native-profile-after-reopen')
        checks.append('Native explicit preference Save, immediate duet and personal list persisted after app restart; source authority not persisted')
        click('#list-cancel'); click('#advanced-dialog .close-dialog')
        fixture_guard.verify('after-app-restart')
        ai_scratch = scratch / 'ai-cases'
        ai_scratch.mkdir()
        original_ai_checks = DesktopAI(command, script, click, fill, wait, capture,
                                       restart_app, OUTPUT).run(BINARY, AI_MODE, ai_scratch)
        checks.append('Real IPC AI mode ' + AI_MODE + ': ' + ', '.join(original_ai_checks))
    assert not script("return !!document.querySelector('vite-error-overlay')")
    if package_guard:
        package_guard.verify()
        assert verify_package(BINARY.parent) == PACKAGE_MANIFEST_SHA256
        assert set(REQUIRED_CHECKS).issubset(original_ai_checks)
    (OUTPUT / 'acceptance.json').write_text(json.dumps({'status':'passed','platform':'windows-native-tauri','binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'ipc_mocked':False,'medium_integrity_verified':True,'ai_mode':AI_MODE,'package_sha256sums_sha256':PACKAGE_MANIFEST_SHA256,'original_ai_checks':original_ai_checks,'checks':checks,'remaining':['Narrator','200 percent DPI','Android SAF and physical ARM64','apply and recovery']},ensure_ascii=False,indent=2),encoding='utf-8')
except Exception as error:
    snapshot(active_binary, OUTPUT, 'webdriver-failure')
    if session:
        try:
            capture('failure')
        except Exception:
            pass
    (OUTPUT / 'failure.json').write_text(json.dumps({'status':'failed','error':str(error),'completed_checks':checks},ensure_ascii=False,indent=2),encoding='utf-8')
    raise
finally:
    if session:
        try:
            command('DELETE','')
        except Exception:
            pass
    if process.poll() is None:
        subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                       capture_output=True, timeout=10, check=True)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill(); process.wait()
    log.close()
print(f'{len(checks)} actual Windows desktop acceptance checks passed.')
