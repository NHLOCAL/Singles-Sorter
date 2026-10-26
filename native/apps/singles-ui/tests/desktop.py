"""Owned Windows CI acceptance via external tauri-driver, with REAL IPC.

No embedded automation plugin/server is added to the application. This script
is intentionally CI-only and refuses other platforms. Build the app with the
pinned Tauri CLI first; put tauri-driver and matching msedgedriver on PATH.
"""
if not __debug__:
    raise RuntimeError('Native desktop acceptance requires Python assertions enabled')

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

from desktop_progress import DesktopProgress
from desktop_modes import select_sort_setup
from desktop_diagnostics import snapshot, startup_probe, owned_processes, write_json
from windows_integrity import process_security, require_medium
from native_picker import interact_with_owned_picker
from native_fixtures import write_import_csv
from desktop_layout import check_native_layout, capture_platform_fonts, check_phone_layout, assert_editor_actions, METRICS
from desktop_ai import DesktopAI, ProtectedTrees, ai_mode, verify_package, REQUIRED_CHECKS
from desktop_apply import exercise_apply
from desktop_recovery import exercise_recovery
from desktop_album_recovery import AlbumRecovery
from desktop_whole_album_recovery import WholeAlbumRecovery
from desktop_repair import exercise_repair
from desktop_flac import exercise_flac
from desktop_ogg import exercise_ogg
from desktop_mp4 import exercise_mp4
from desktop_id3_container import exercise_id3_containers
from desktop_wavpack import exercise_wavpack
from desktop_copied_move import exercise_copied_move
from desktop_albums import exercise_albums
from desktop_metadata_readiness import exercise_metadata_readiness
from desktop_cross_albums import exercise_cross_albums
from desktop_polish import exercise_information, set_developer_mode
from driver_readiness import wait_driver_ready
from desktop_stability import exercise_shell, exercise_result_stability
from desktop_settings import SETTINGS_CHECKS, read_main_settings, set_main_settings, editor_rows, verify_export, require_export_notice

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
apply_checks = []
recovery_checks = []
album_recovery_checks = []
whole_album_recovery_checks = []
repair_checks = []
flac_checks = []
ogg_checks = []
mp4_checks = []
id3_container_checks = []
wavpack_checks = []
album_checks = []
cross_album_checks = []
copied_move_checks = []
metadata_readiness_checks = []
settings_checks = []
with BINARY.open('rb') as stream:
    progress_binary_sha256 = hashlib.file_digest(stream, 'sha256').hexdigest()
progress = DesktopProgress(OUTPUT / 'runtime-progress.json', dict(
    binary_sha256=progress_binary_sha256, package_sha256sums_sha256=PACKAGE_MANIFEST_SHA256,
    source_commit=os.environ.get('GITHUB_SHA'), ai_mode=AI_MODE,
    selected_execution=os.environ.get('SINGLES_DESKTOP_APPLY_EXPECTED') == 'on'))
progress.phase('startup-layout-settings-and-csv')

def call(method, path, body=None, *, timeout=30):
    data = None if body is None else json.dumps(body).encode('utf-8')
    req = urllib.request.Request(BASE + path, data=data, method=method, headers={'Content-Type':'application/json; charset=utf-8'})
    # Local native test driver only; do not inherit an external proxy.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(req, timeout=timeout) as response:
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
    progress.waiting(timeout)
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        try:
            value = predicate()
            if value:
                progress.waited()
                return value
        except (urllib.error.URLError, RuntimeError, KeyError) as error:
            last = error
        time.sleep(.2)
    raise TimeoutError(f'Native UI condition was not reached: {last}')

def find(selector):
    return command('POST','/element',{'using':'css selector','value':selector})[ELEMENT]

def click(selector):
    progress.action('click', selector)
    command('POST',f'/element/{find(selector)}/click',{})

def fill(selector, value):
    progress.action('fill', selector, case=value if selector=='#source' else None)
    element = find(selector)
    command('POST',f'/element/{element}/clear',{})
    command('POST',f'/element/{element}/value',{'text':value})

def text(selector):
    return command('GET',f'/element/{find(selector)}/text')

def script(source):
    return command('POST','/execute/sync',{'script':source,'args':[]})

def capture(name):
    progress.action('screenshot', name)
    (OUTPUT / f'{name}.png').write_bytes(base64.b64decode(command('GET','/screenshot')))

def pick_owned_path(selector, path, *, save=False):
    # Native dialog controls belong to this exact owned preview executable.
    # Never interact with a generic desktop window or another user's app.
    click(selector)
    progress.action('native-picker', selector + ': ' + str(path))
    interact_with_owned_picker(BINARY, path, OUTPUT, selector.removeprefix('#'), save=save)

def restart_app(binary, label):
    global session, active_binary
    progress.action('restart', label, case=label)
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
    wait_driver_ready(call, process)
    result = call('POST','/session',{'capabilities':{'alwaysMatch':{'browserName':'wry','tauri:options':{'application':str(BINARY)}}}})
    session = result['sessionId']
    verify_app_security('initial-app')
    wait(lambda: 'מסדר הסינגלים' in text('h1'))
    wait(lambda: script("return document.documentElement.dataset.startupState === 'ready'"))
    wait(lambda: script("return !document.querySelector('#sort').disabled"))
    assert script("return document.querySelector('#app').dataset.workMode==='sort'"), 'A fresh renderer must start in Sort mode'
    assert not script("return !!document.querySelector('dialog[open]')"), 'Startup must not open automatic help'
    check_native_layout(command, script, capture, OUTPUT, '01-native-home')
    capture_platform_fonts(command, OUTPUT)
    checks.append('Actual measured desktop CSS viewports retain controls without document overflow')
    capture('01-native-home')
    exercise_shell(command,script,capture,OUTPUT,'native-setup-stable')
    check_phone_layout(command,script,capture,OUTPUT,'01-native-home')
    checks.append('Actual Windows Tauri/WebView2 rendered local assets and initialized real IPC')
    checks.extend(exercise_information(command, script, click, wait, capture, ROOT, OUTPUT))
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
        select_sort_setup(script, click, wait)
        pick_owned_path('#source-picker', source)
        wait(lambda: script("return document.querySelector('#source').value") == str(source))
        fixture_guard.verify('after-source-picker')
        pick_owned_path('#target-picker', target)
        wait(lambda: script("return document.querySelector('#target').value") == str(target))
        fixture_guard.verify('after-target-picker')
        checks.append('Native folder dialogs selected only the owned source and target fixtures')
        click('#tab-mode-sort')
        click('#sort')
        wait(lambda: script("return document.querySelector('#app').dataset.view === 'results' && document.querySelectorAll('#rows tr').length > 0 && !document.querySelector('#sort').disabled"))
        assert 'אברהם פריד' in text('#rows')
        assert script("return document.querySelector('#error').hidden"), text('#error')
        capture('02-native-real-sort-preview')
        check_native_layout(command, script, capture, OUTPUT, '02-native-results')
        check_phone_layout(command,script,capture,OUTPUT,'02-native-results')
        assert script("return document.querySelectorAll('#rows .detail-button').length") == 0, 'Ordinary rows expose a details action'
        set_developer_mode(script, click, wait, True)
        capture('02-native-developer-results')
        click('#rows tr:last-child .detail-button')
        assert script("return document.querySelector('#row-dialog .source-path').value") == str(media)
        capture('02-native-result-details')
        click('#row-close')
        wait(lambda:not script("return document.querySelector('#row-dialog').open"))
        set_developer_mode(script, click, wait, False)
        assert script("return document.querySelector('#details').hidden")
        assert media.is_file() and hashlib.sha256(media.read_bytes()).hexdigest() == before
        assert list(target.iterdir()) == []
        fixture_guard.verify('after-sort-preview')
        checks.append('Real owned MP3 sort preview produced Hebrew singer/destination; source hash and empty target unchanged')
        click('#back-setup')
        wait(lambda:script("return document.querySelector('#app').dataset.view === 'setup'"))
        assert script("return document.querySelector('#source').value") == str(source)
        select_sort_setup(script, click, wait)
        fill('#target','')
        click('#tab-mode-fix')
        click('#fix')
        wait(lambda: script("return document.querySelector('#app').dataset.view === 'results' && !document.querySelector('#fix').disabled"))
        assert script("return document.querySelector('#error').hidden"), text('#error')
        capture('03-native-real-fix-preview')
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive === 'true' && !document.querySelector('#show-inactive').disabled"))
        assert script("return document.querySelectorAll('#rows tr').length") == 1
        click('#show-inactive')
        wait(lambda: script("return document.querySelector('#app').dataset.showInactive === 'false' && !document.querySelector('#show-inactive').disabled"))
        assert hashlib.sha256(media.read_bytes()).hexdigest() == before
        fixture_guard.verify('after-fixnames-preview')
        checks.append('Target-free real FixNames preview; owned source bytes unchanged')
        click('#back-setup')
        wait(lambda:script("return document.querySelector('#app').dataset.view === 'setup'"))
        # Task 2 uses a separate development profile. The runner has no user profile.
        # Back preserves the selected Fix tab, which correctly hides Copy settings.
        click('#tab-mode-sort')
        wait(lambda: script("return document.querySelector('#app').dataset.workMode === 'sort' && document.querySelector('#copy_mode').getClientRects().length > 0"))
        assert script("return !!document.querySelector('#save-settings')"), "Task 2 preferences UI is missing; stale binary cannot pass"
        settings_baseline = read_main_settings(script)
        expected_settings = {key: not value for key, value in settings_baseline.items()}
        set_main_settings(script, click, wait, expected_settings)
        click('#save-settings')
        wait(lambda: text('#settings-notice') == 'נשמר')
        # A different unsaved main draft must survive the immediate duet write
        # in this session, but must not be persisted by that unrelated action.
        set_main_settings(script, click, wait, settings_baseline)
        assert text('#settings-label') == 'שינויים שלא נשמרו'
        click('#open-settings')
        expected_duet = not script("return document.querySelector('#duet-all').checked")
        click('#duet-all' if expected_duet else '#duet-first')
        wait(lambda: script("return !document.querySelector('#duet-all').disabled"))
        assert script("return document.querySelector('#duet-all').checked") == expected_duet
        assert read_main_settings(script) == settings_baseline
        assert script("return document.querySelector('#settings-label').textContent") == 'שינויים שלא נשמרו'
        capture('04-native-settings')
        check_phone_layout(command,script,capture,OUTPUT,'04-native-settings')
        click('#edit-list')
        wait(lambda: script("return document.querySelector('#list-dialog').open"))
        # The same owned CI profile survives off/on/extracted package lanes.
        # Check this import's exact +2 duplicates without deleting prior results.
        personal_baseline = editor_rows(script, click, wait)
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
        wait(lambda: script("return document.querySelector('#list-dialog').open"))
        expected_personal = personal_baseline + [
            ['אמן בדיקת ממשק', 'תיקיית בדיקה'],
            ['אמן מיובא', 'תיקייה מיובאת'], ['אמן מיובא', 'תיקייה שנייה'],
            ['אמן שנערך לפני אישור', 'תיקייה ערוכה']]
        assert editor_rows(script, click, wait) == expected_personal
        fixture_guard.verify('after-settings-and-import')
        checks.append('Actual CSV draft edits/deletion applied only on confirm; original duplicate aliases remained ordered')
        click('#list-cancel'); click('#advanced-dialog .close-dialog')
        restart_app(BINARY, 'restarted-app')
        observed_settings = read_main_settings(script)
        assert observed_settings == expected_settings
        assert script("return document.querySelector('#source').value") == ''
        assert script("return document.querySelector('#target').value") == ''
        click('#open-settings')
        observed_duet = script("return document.querySelector('#duet-all').checked")
        assert observed_duet == expected_duet
        click('#edit-list')
        wait(lambda: script("return document.querySelector('#list-dialog').open"))
        reopened_personal = editor_rows(script, click, wait)
        assert reopened_personal == expected_personal
        capture('07-native-profile-after-reopen')
        checks.append('All five explicitly saved main preferences, immediate duet and exact ordered personal rows persisted; unsaved main draft and source/target authority did not persist')
        click('#list-cancel')
        export_root = scratch / 'native-profile-export'
        export_root.mkdir()
        exported_csv = export_root / 'רשימת אמנים.csv'
        assert not exported_csv.exists() and not list(export_root.iterdir())
        pick_owned_path('#export-list', exported_csv, save=True)
        wait(lambda: script("return !document.querySelector('#export-list').disabled && document.querySelector('#profile-error').hidden"))
        export_notice = text('#profile-notice')
        (OUTPUT / 'export-result-path.json').write_text(json.dumps({
            'requested_owned_path': str(exported_csv), 'actual_notice': export_notice,
            'owned_directory_entries': sorted(child.name for child in export_root.iterdir())},
            ensure_ascii=False, indent=2), encoding='utf-8')
        require_export_notice(exported_csv, export_notice)
        export_receipt = verify_export(exported_csv, reopened_personal)
        capture('08-native-personal-export')
        click('#advanced-dialog .close-dialog')
        # All three lanes share only this runner's owned profile. Restore its
        # baseline so e.g. exist_only cannot alter the next empty-target fixture.
        set_main_settings(script, click, wait, settings_baseline)
        click('#save-settings')
        wait(lambda: text('#settings-notice') == 'נשמר')
        assert read_main_settings(script) == settings_baseline
        fixture_guard.verify('after-app-restart-and-export')
        settings_checks = list(SETTINGS_CHECKS)
        (OUTPUT / 'settings-profile.json').write_text(json.dumps({
            'status': 'passed', 'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
            'ai_mode': AI_MODE, 'checks': settings_checks, 'stored_baseline': settings_baseline,
            'explicitly_saved': expected_settings, 'discarded_unsaved_draft': settings_baseline,
            'observed_after_restart': observed_settings,
            'expected_immediate_duet': expected_duet, 'observed_immediate_duet': observed_duet, 'restored_baseline': read_main_settings(script),
            'personal_baseline': personal_baseline, 'export_path': str(exported_csv),
            'export': export_receipt}, ensure_ascii=False, indent=2), encoding='utf-8')
        progress.phase('result-layout-stability')
        checks.extend(exercise_result_stability(command,script,click,fill,wait,capture,scratch/'stability-cases',OUTPUT))
        progress.phase('original-ai')
        ai_scratch = scratch / 'ai-cases'
        ai_scratch.mkdir()
        original_ai_checks = DesktopAI(command, script, click, fill, wait, capture,
                                       restart_app, OUTPUT).run(BINARY, AI_MODE, ai_scratch)
        checks.append('Real IPC AI mode ' + AI_MODE + ': ' + ', '.join(original_ai_checks))
        if os.environ.get('SINGLES_DESKTOP_APPLY_EXPECTED') == 'on':
            progress.phase('single-copy-and-move')
            apply_checks = exercise_apply(script, click, fill, wait, capture, scratch / 'apply-cases', OUTPUT, AI_MODE)
            checks.append('Real selected-root execution: ' + ', '.join(apply_checks))
            progress.phase('copied-move')
            copied_move_checks = exercise_copied_move(script, click, fill, wait, capture, scratch / 'copied-move-cases', OUTPUT, ROOT)
            checks.append('Real copied Move: ' + ', '.join(copied_move_checks))
            progress.phase('metadata-readiness')
            metadata_readiness_checks = exercise_metadata_readiness(script, click, fill, wait, capture, scratch / 'metadata-readiness', OUTPUT, ROOT.parents[1] / 'tests/fixtures/media/blank.mp3')
            checks.append('Real metadata readiness and refusal: ' + ', '.join(metadata_readiness_checks))
            progress.phase('mp3-repair')
            repair_checks = exercise_repair(command, script, click, fill, wait, capture, scratch / 'repair-cases', OUTPUT, ROOT.parents[1] / 'tests/fixtures/media/blank.mp3')
            checks.append('Real source-only repair: ' + ', '.join(repair_checks))
            progress.phase('flac-repair')
            flac_checks = exercise_flac(script, click, fill, wait, capture, scratch / 'flac-cases', OUTPUT)
            checks.append('Real FLAC repair: ' + ', '.join(flac_checks))
            progress.phase('ogg-repair')
            ogg_checks = exercise_ogg(script, click, fill, wait, capture, scratch / 'ogg-cases', OUTPUT)
            checks.append('Real Ogg repair: ' + ', '.join(ogg_checks))
            progress.phase('mp4-repair')
            mp4_checks = exercise_mp4(script, click, fill, wait, capture, scratch / 'mp4-cases', OUTPUT)
            checks.append('Real MP4 AAC/ALAC repair: ' + ', '.join(mp4_checks))
            progress.phase('id3-container-repair')
            id3_container_checks = exercise_id3_containers(script, click, fill, wait, capture, scratch / 'id3-container-cases', OUTPUT)
            checks.append('Real existing-ID3 WAVE/AIFF/DSF repair: ' + ', '.join(id3_container_checks))
            progress.phase('wavpack-repair')
            wavpack_checks = exercise_wavpack(script, click, fill, wait, capture, scratch / 'wavpack-cases', OUTPUT)
            checks.append('Real WavPack APEv2 repair: ' + ', '.join(wavpack_checks))
            progress.phase('same-volume-albums')
            album_checks = exercise_albums(script, click, fill, wait, capture, scratch / 'album-cases', OUTPUT, ROOT.parents[1] / 'tests/fixtures/media/blank.mp3')
            checks.append('Real album execution: ' + ', '.join(album_checks))
            album_recovery = None
            if os.environ.get('SINGLES_DESKTOP_RECOVERY_EXPECTED') == 'on':
                album_recovery = AlbumRecovery(script, click, wait, capture, pick_owned_path, restart_app, BINARY, OUTPUT)
                album_copy = next(case for case in json.loads((OUTPUT / 'album-execution.json').read_text(encoding='utf-8'))['cases'] if case['case'] == 'copy')
                progress.phase('file-album-copy-restart')
                album_recovery.run(album_copy)
                whole_recovery = WholeAlbumRecovery(script, click, wait, capture, pick_owned_path, restart_app, BINARY, OUTPUT)
                album_cases = json.loads((OUTPUT / 'album-execution.json').read_text(encoding='utf-8'))['cases']
                for name in ['root-move', 'mixed-move']:
                    progress.phase('whole-album-' + name + '-restart')
                    whole_recovery.run(next(case for case in album_cases if case['case'] == name))
                whole_album_recovery_checks = whole_recovery.finish()
            progress.phase('cross-volume-albums-and-restart')
            cross_album_checks = exercise_cross_albums(script, click, fill, wait, capture, scratch / 'cross-album-cases', OUTPUT, ROOT.parents[1] / 'tests/fixtures/media/blank.mp3', ROOT,
                                                       recovery=album_recovery.run if album_recovery else None)
            if album_recovery:
                album_recovery_checks = album_recovery.finish()
            checks.append('Real cross-volume albums: ' + ', '.join(cross_album_checks))
            if os.environ.get('SINGLES_DESKTOP_RECOVERY_EXPECTED') == 'on':
                progress.phase('single-copied-and-fix-restart')
                recovery_checks = exercise_recovery(script, click, wait, capture, pick_owned_path, restart_app, BINARY, OUTPUT)
                checks.append('Actual read-only process-restart inspection: ' + ', '.join(recovery_checks))
    progress.phase('final-package-verification')
    assert not script("return !!document.querySelector('vite-error-overlay')")
    if package_guard:
        package_guard.verify()
        assert verify_package(BINARY.parent) == PACKAGE_MANIFEST_SHA256
        assert set(REQUIRED_CHECKS).issubset(original_ai_checks)
    (OUTPUT / 'acceptance.json').write_text(json.dumps({'status':'passed','platform':'windows-native-tauri','binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),'ipc_mocked':False,'medium_integrity_verified':True,'ai_mode':AI_MODE,'package_sha256sums_sha256':PACKAGE_MANIFEST_SHA256,'original_ai_checks':original_ai_checks,'settings_checks':settings_checks,'apply_mode':os.environ.get('SINGLES_DESKTOP_APPLY_EXPECTED','off'),'apply_checks':apply_checks,'recovery_checks':recovery_checks,'album_recovery_checks':album_recovery_checks,'whole_album_recovery_checks':whole_album_recovery_checks,'repair_checks':repair_checks,'flac_checks':flac_checks,'ogg_checks':ogg_checks,'mp4_checks':mp4_checks,'id3_container_checks':id3_container_checks,'wavpack_checks':wavpack_checks,'album_checks':album_checks,'cross_album_checks':cross_album_checks,'copied_move_checks':copied_move_checks,'metadata_readiness_checks':metadata_readiness_checks,'checks':checks,'remaining':['Narrator','200 percent DPI','Android SAF and physical ARM64','unsupported apply modes, automatic replay, and unrecorded historical whole-Album named-stream baselines']},ensure_ascii=False,indent=2),encoding='utf-8')
    progress.finish(True)
except Exception as error:
    progress.finish(False, error)
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
