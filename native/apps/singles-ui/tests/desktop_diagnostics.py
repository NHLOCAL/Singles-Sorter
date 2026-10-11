"""CI-only diagnostics; never substitute these observations for WebDriver checks."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')


def owned_processes(binary, root_pid=None, observed=()):
    # Filter by exact executable path, then include only descendants. Do not log
    # unrelated processes, their command lines, or the whole runner desktop.
    script = r'''
$all = @(Get-CimInstance Win32_Process)
$owned = @($all | Where-Object { $_.ExecutablePath -eq $env:SINGLES_DIAGNOSTIC_BINARY -and
  (-not $env:SINGLES_DIAGNOSTIC_PID -or $_.ProcessId -eq [uint32]$env:SINGLES_DIAGNOSTIC_PID) })
$known = @($env:SINGLES_DIAGNOSTIC_OBSERVED | ConvertFrom-Json)
foreach ($identity in $known) {
  $owned += @($all | Where-Object { $_.ProcessId -eq $identity.pid -and
    $_.CreationDate.ToString('o') -eq $identity.created })
}
$owned = @($owned | Sort-Object ProcessId -Unique)
$ids = @($owned | ForEach-Object { [uint32]$_.ProcessId })
if ($env:SINGLES_DIAGNOSTIC_PID) { $ids += [uint32]$env:SINGLES_DIAGNOSTIC_PID }
do {
  $children = @($all | Where-Object { $_.ParentProcessId -in $ids -and $_.ProcessId -notin $ids })
  $owned += $children
  $ids += @($children | ForEach-Object { [uint32]$_.ProcessId })
} while ($children.Count -gt 0)
@($owned | ForEach-Object {
  $p = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue
  [pscustomobject]@{ pid=$_.ProcessId; parent_pid=$_.ParentProcessId; name=$_.Name;
    executable=$_.ExecutablePath; version=$p.FileVersion; created=$_.CreationDate.ToString('o');
    main_window_handle=if ($p) { $p.MainWindowHandle.ToInt64() } else { $null } }
}) | ConvertTo-Json -Depth 4 -Compress
'''
    env = dict(os.environ, SINGLES_DIAGNOSTIC_BINARY=str(binary),
               SINGLES_DIAGNOSTIC_PID=str(root_pid) if root_pid is not None else "",
               SINGLES_DIAGNOSTIC_OBSERVED=json.dumps(list(observed)))
    result = subprocess.run(['pwsh', '-NoProfile', '-NonInteractive', '-Command', script],
                            env=env, capture_output=True, text=True, encoding='utf-8',
                            errors='replace', timeout=15, check=True)
    rows = json.loads(result.stdout) if result.stdout.strip() else []
    return rows if isinstance(rows, list) else [rows]


def capture_owned_windows(output, label, pids):
    from pywinauto import Application
    evidence = []
    for pid in pids:
        try:
            app = Application(backend='win32').connect(process=pid)
            for index, window in enumerate(app.windows()[:4]):
                if not window.is_visible() or window.process_id() != pid:
                    continue
                item = {'pid': pid, 'class': window.class_name(), 'title': window.window_text()}
                filename = f'{label}-owned-window-{pid}-{index}.png'
                try:
                    subprocess.run([sys.executable, str(Path(__file__).with_name('owned_window_capture.py')),
                                    str(window.handle), str(pid), str(output / filename)],
                                   check=True, capture_output=True, timeout=5)
                    item['screenshot'] = filename
                except Exception as error:
                    item['capture_error'] = str(error)
                    stderr = getattr(error, 'stderr', None)
                    if stderr:
                        item['capture_stderr'] = stderr.decode('utf-8', errors='replace')[-4000:]
                evidence.append(item)
        except Exception as error:
            evidence.append({'pid': pid, 'capture_error': str(error)})
    return evidence


def snapshot(binary, output, label, root_pid=None):
    evidence = {'kind': 'startup-diagnostic-only', 'binary': str(binary)}
    try:
        evidence['processes'] = owned_processes(binary, root_pid)
        pids = [row['pid'] for row in evidence['processes']
                if os.path.normcase(row.get('executable') or '') == os.path.normcase(str(binary))]
        evidence['windows'] = capture_owned_windows(output, label, pids)
    except Exception as error:
        evidence['diagnostic_error'] = str(error)
    write_json(output / f'{label}.json', evidence)
    return evidence


def startup_probe(binary, output):
    """Launch only the owned binary, observe for eight seconds, then reap it.

    This is deliberately not an acceptance check and adds no automation flags,
    remote-debugging port, app permission, or security exception.
    """
    evidence = {'kind': 'startup-diagnostic-only', 'binary': str(binary),
                'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    child = None
    observed = []
    try:
        with (output / 'startup-stdout.log').open('wb') as stdout, \
             (output / 'startup-stderr.log').open('wb') as stderr:
            child = subprocess.Popen([str(binary)], stdout=stdout, stderr=stderr)
            evidence['pid'] = child.pid
            deadline = time.monotonic() + 8
            while child.poll() is None and time.monotonic() < deadline:
                time.sleep(.25)
            evidence['exit_code_before_cleanup'] = child.poll()
            evidence['alive_before_cleanup'] = child.poll() is None
            before = snapshot(binary, output, 'startup-before-webdriver', root_pid=child.pid)
            observed = before.get('processes', [])
            if child.poll() is None:
                # taskkill roots cleanup at our still-live child PID, including
                # only its WebView2 descendants; no image-name-wide termination.
                subprocess.run(['taskkill', '/PID', str(child.pid), '/T', '/F'],
                               capture_output=True, timeout=10, check=True)
            child.wait(timeout=10)
            evidence['exit_code_after_cleanup'] = child.returncode
    except Exception as error:
        evidence['diagnostic_error'] = str(error)
    finally:
        try:
            if child is not None and child.poll() is None:
                subprocess.run(['taskkill', '/PID', str(child.pid), '/T', '/F'],
                               capture_output=True, timeout=10, check=True)
                child.wait(timeout=10)
        finally:
            write_json(output / 'startup-probe.json', evidence)

    # Fail closed if a preflight descendant survives; do not contaminate acceptance.
    residual = owned_processes(binary, child.pid, observed) if child is not None else []
    evidence['residual_processes_after_cleanup'] = residual
    write_json(output / 'startup-probe.json', evidence)
    if residual:
        raise RuntimeError('Owned startup-probe processes survived cleanup; WebDriver was not started')
