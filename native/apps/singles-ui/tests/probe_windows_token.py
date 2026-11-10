"""Lightweight, read-only rights preflight before any native compilation."""
import json
import os
from pathlib import Path

from windows_integrity import process_security, require_medium


def main():
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
        raise SystemExit('Token preflight is restricted to owned Windows CI')
    import winreg
    output = Path(os.environ['SINGLES_UI_EVIDENCE'])
    output.mkdir(parents=True, exist_ok=True)
    security = process_security()
    evidence = {'process': security}
    try:
        # Observe only; never change UAC, registry policy, accounts, or permissions.
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE,
                            r'SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System',
                            0, winreg.KEY_READ) as key:
            evidence['enable_lua'] = winreg.QueryValueEx(key, 'EnableLUA')[0]
    except OSError as error:
        evidence['uac_query_error'] = str(error)
    (output / 'token-preflight.json').write_text(json.dumps(evidence, indent=2), encoding='utf-8')
    print(json.dumps(evidence), flush=True)
    require_medium(security)


if __name__ == '__main__':
    main()
