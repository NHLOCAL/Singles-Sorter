"""Release smoke test: drive the exact extracted package through its real WebView and dialogs.

Format, recovery and edge-case coverage lives in the Rust suites. This proves the shipped
artifact starts, loads its models and completes each user-facing operation.
"""
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

from driver_readiness import wait_driver_ready
from native_picker import interact_with_owned_picker
from windows_integrity import process_security, require_medium

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parents[2]
BINARY = Path(os.environ['SINGLES_DESKTOP_BIN']).resolve(strict=True)
OUTPUT = Path(os.environ['SINGLES_UI_EVIDENCE'])
OUTPUT.mkdir(parents=True, exist_ok=True)
FIXTURES = Path(os.environ.get('SINGLES_UI_TEMP', tempfile.gettempdir()))
FIXTURES.mkdir(parents=True, exist_ok=True)
WORK = Path(tempfile.mkdtemp(prefix='singles-smoke-', dir=FIXTURES))
BASE = 'http://127.0.0.1:4444'
ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'
ARTIST = 'אברהם פריד'
session = None


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def call(method, path, body=None, timeout=60):
    data = None if body is None else json.dumps(body).encode('utf-8')
    request = urllib.request.Request(BASE + path, data=data, method=method, headers={'Content-Type': 'application/json; charset=utf-8'})
    with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request, timeout=timeout) as response:
        value = json.loads(response.read()).get('value')
    if isinstance(value, dict) and value.get('error'):
        raise RuntimeError(f"{value['error']}: {value.get('message')}")
    return value


def command(method, path, body=None):
    return call(method, f'/session/{session}{path}', body)


def script(source):
    return command('POST', '/execute/sync', {'script': source, 'args': []})


def wait(predicate, timeout=60):
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
    raise TimeoutError(f'Condition not reached: {last}; screen: {script("return document.body.innerText.slice(0, 600)")}')


def click(selector):
    element = command('POST', '/element', {'using': 'css selector', 'value': selector})[ELEMENT]
    command('POST', f'/element/{element}/click', {})


def visible(element_id):
    return script(f"const e=document.getElementById('{element_id}');return !!e&&e.offsetWidth>0&&!e.disabled")


def pick(selector, path):
    click(selector)
    interact_with_owned_picker(BINARY, path, OUTPUT, selector.removeprefix('#'))


def song(path, **tags):
    metadata = [part for key, value in tags.items() for part in ('-metadata', f'{key}={value}')]
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-f', 'lavfi', '-i', 'anullsrc=r=44100:cl=mono', '-t', '1', '-c:a', 'libmp3lame',
                    '-id3v2_version', '3', *metadata, str(path)], check=True)


def artist_tag(path):
    result = subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format_tags=artist', '-of', 'default=nw=1:nk=1', str(path)],
                            capture_output=True, check=True)
    return result.stdout.decode('utf-8').strip()


def library(name, files):
    source, target = WORK / name / 'source', WORK / name / 'target'
    source.mkdir(parents=True)
    target.mkdir()
    for filename, tags in files.items():
        song(source / filename, **tags)
    return source, target


def execute(trigger):
    click(trigger)
    wait(lambda: visible('review-apply'), 180)
    click('#review-apply')
    wait(lambda: visible('confirm-apply'))
    click('#confirm-apply')
    done = "const p=document.getElementById('execution-panel'),t=document.getElementById('execution-title')?.innerText||'';" \
           "return p&&p.offsetWidth>0&&t.length>0&&!/מבצע|מאמת|בודק/.test(t)?t:null"
    title = wait(lambda: script(done), 180)
    assert title == 'הביצוע הושלם', title
    click('#finish-results')
    wait(lambda: visible('sort') or visible('fix'))


def sort_flow(name, copy):
    filename = f'{ARTIST} - בדיקה.mp3'
    source, target = library(name, {filename: {}})
    original = sha256(source / filename)
    pick('#source-picker', source)
    pick('#target-picker', target)
    script(f"const c=document.getElementById('copy_mode');if(c.checked!=={str(copy).lower()})c.click();return true")
    execute('#sort')
    placed = next(target.rglob(filename))
    assert placed.parent.name == ARTIST or ARTIST in placed.parts, placed
    assert sha256(placed) == original
    assert (source / filename).exists() == copy, f'source retention for copy={copy}'


def fix_flow():
    gibberish = ARTIST.encode('cp1255').decode('latin-1')
    source, _ = library('fix', {'בדיקת_שם.mp3': {'artist': gibberish}})
    click('#tab-mode-fix')
    pick('#source-picker', source)
    execute('#fix')
    fixed = source / 'בדיקת שם.mp3'
    assert fixed.exists() and not (source / 'בדיקת_שם.mp3').exists(), list(source.iterdir())
    assert artist_tag(fixed) == ARTIST, artist_tag(fixed)
    assert [p for p in source.rglob('*') if p.is_file() and p != fixed], 'original backup missing'
    click('#tab-mode-sort')


def about_and_phone_layout():
    version = json.loads((REPO / 'release.json').read_text(encoding='utf-8'))['version']
    original = command('GET', '/window/rect')
    command('POST', '/window/rect', {'width': 376, 'height': 800})
    try:
        wait(lambda: script('return innerWidth < 500'))
        click('#menu')
        click('[data-help="about"]')
        wait(lambda: script("return document.getElementById('about-version').textContent") == version)
        low = script("return [...document.querySelectorAll('dialog[open] button')].filter(b=>b.offsetHeight>0&&b.getBoundingClientRect().height<47.5).map(b=>b.outerHTML.slice(0,90))")
        assert not low, low
        assert script('return document.documentElement.scrollWidth - innerWidth') <= 1
        click('dialog[open] .close-dialog')
    finally:
        command('POST', '/window/rect', original)


require_medium(process_security())
driver = subprocess.Popen([shutil.which('tauri-driver')], stdout=(OUTPUT / 'tauri-driver.log').open('wb'), stderr=subprocess.STDOUT)
try:
    wait_driver_ready(call, driver)
    session = call('POST', '/session', {'capabilities': {'alwaysMatch': {'browserName': 'wry', 'tauri:options': {'application': str(BINARY)}}}}, timeout=120)['sessionId']
    wait(lambda: script("return document.documentElement.dataset.startupState === 'ready'"), 120)
    assert script("return document.getElementById('model-status').hidden && document.getElementById('ai-profile').checked"), 'models unavailable'
    about_and_phone_layout()
    sort_flow('copy', True)
    sort_flow('move', False)
    fix_flow()
    (OUTPUT / 'acceptance.json').write_text(json.dumps({
        'status': 'passed', 'platform': 'windows-native-tauri', 'medium_integrity_verified': True,
        'ipc_mocked': False, 'ai_mode': 'on', 'binary_sha256': sha256(BINARY),
        'package_sha256sums_sha256': sha256(BINARY.parent / 'SHA256SUMS.txt'),
    }, indent=2), encoding='utf-8')
    print('Release smoke acceptance passed', flush=True)
except BaseException:
    if session:
        (OUTPUT / 'failure.png').write_bytes(__import__('base64').b64decode(command('GET', '/screenshot')))
    raise
finally:
    if session:
        command('DELETE', '')
    driver.terminate()
