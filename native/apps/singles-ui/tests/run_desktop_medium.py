"""Run the real acceptance process at normal user integrity without ACL changes."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
import uuid

from windows_integrity import process_security

ROOT = Path(__file__).resolve().parents[1]


def is_ci_windows():
    return os.name == 'nt' and os.environ.get('GITHUB_ACTIONS') == 'true'


def command_for(security, launcher, python, script):
    if security['integrity_rid'] == 8192 and not security['elevated']:
        return [python, str(script)]
    if security['integrity_rid'] != 12288 or not security['elevated']:
        raise RuntimeError(f'Unexpected launcher token; refusing any privilege increase: {security}')
    return [str(launcher), '--integrity', 'Medium', '--wait', '--direct', python, str(script)]


def runtime_budget_seconds(preflight, env):
    # Actual full selected lane exhausted 900s after steady progress through all
    # format/Album cases, at its first restart picker (8813a50f, 2026-10-12).
    # Keep preflight/feature-off bounds and every inner operation timeout intact.
    return 60 if preflight else 1800 if env.get('SINGLES_DESKTOP_APPLY_EXPECTED') == 'on' else 900


def main(preflight=False):
    if not is_ci_windows():
        raise SystemExit('Medium-integrity launcher is restricted to owned Windows CI')
    output = Path(os.environ.get('SINGLES_UI_OUTPUT', str(ROOT / 'test-results/native-windows')))
    output.mkdir(parents=True, exist_ok=True)
    security = process_security()
    (output / 'launcher-security.json').write_text(json.dumps(security, indent=2), encoding='utf-8')
    print(f'CI launcher token: {security}', flush=True)
    # The child creates this directory under its existing user-profile permissions.
    # It never needs write permission to the checkout or its compiled binary.
    owned = Path(os.environ['LOCALAPPDATA']) / 'Temp' / f'singles-ui-ci-{uuid.uuid4().hex}'
    env = dict(os.environ, SINGLES_UI_EVIDENCE=str(owned / 'evidence'),
               SINGLES_UI_TEMP=str(owned / 'fixtures'), PYTHONDONTWRITEBYTECODE='1')
    launcher = Path(os.environ['SINGLES_GSUDO_EXE']).resolve(strict=True)
    command = command_for(security, launcher, sys.executable, ROOT / ('tests/probe_windows_token.py' if preflight else 'tests/desktop.py'))
    budget = runtime_budget_seconds(preflight, env)
    runtime = dict(timeout_seconds=budget, preflight=preflight, selected_execution=env.get('SINGLES_DESKTOP_APPLY_EXPECTED') == 'on', status='running')
    (output / 'launcher-runtime.json').write_text(json.dumps(runtime, indent=2), encoding='utf-8')
    started = time.monotonic()
    process = subprocess.Popen(command, cwd=ROOT, env=env)
    try:
        code = process.wait(timeout=budget)
        runtime.update(status='exited', exit_code=code)
        return code
    except subprocess.TimeoutExpired:
        runtime['status'] = 'timed_out'
        subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                       capture_output=True, timeout=10, check=True)
        process.wait(timeout=10)
        raise
    finally:
        runtime['elapsed_seconds'] = round(time.monotonic() - started, 3)
        (output / 'launcher-runtime.json').write_text(json.dumps(runtime, indent=2), encoding='utf-8')
        evidence = owned / 'evidence'
        if evidence.is_dir():
            shutil.copytree(evidence, output, dirs_exist_ok=True)
        # The ephemeral runner owns this directory. Leave it for runner disposal,
        # especially on failure; do not delete or traverse any user directory.


if __name__ == '__main__':
    raise SystemExit(main(preflight=sys.argv[1:] == ['--preflight']))
