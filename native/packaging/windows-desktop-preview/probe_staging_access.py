"""Owned CI-only staging/access preflight; no models, compiler or ACL changes."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import uuid
from unittest.mock import patch

import package_desktop as package

TESTS = package.ROOT / 'native/apps/singles-ui/tests'
sys.path.insert(0, str(TESTS))
from windows_integrity import process_security, require_medium
from run_desktop_medium import command_for


def inventory(root):
    return {str(p.relative_to(root)): package.digest(p)
            for p in root.rglob('*') if p.is_file()}


def main():
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
        raise SystemExit('Staging access probe is restricted to owned Windows CI')
    if os.environ.get('SINGLES_STAGING_PROBE_CHILD') == '1':
        require_medium(process_security())
        root = Path(os.environ['SINGLES_STAGING_PROBE_ROOT'])
        expected = json.loads(os.environ['SINGLES_STAGING_PROBE_HASHES'])
        if inventory(root) != expected:
            raise RuntimeError('Medium child did not read every exact owned fixture byte')
        package.read_windows_acl_evidence([root, root / 'models/original', root / 'licenses/nested'])
        print('STAGING_MEDIUM_READ_PASSED', flush=True)
        return

    # Ordinary inherited workspace parent, never an existing private temp root.
    parent = package.ROOT / 'native/target' / ('staging-access-probe-' + uuid.uuid4().hex)
    parent.mkdir(parents=True)
    output = parent / 'published'
    with tempfile.TemporaryDirectory(prefix='private-', dir=parent) as private:
        stage = Path(private) / 'package'
        (stage / 'models/original').mkdir(parents=True)
        (stage / 'models/original/owned.txt').write_bytes(b'owned model-shaped fixture')
        # Exercise the nested protected tempfile used by notice construction too.
        with tempfile.TemporaryDirectory(prefix='private-license-', dir=stage) as nested:
            licenses = Path(nested) / 'licenses'
            (licenses / 'nested').mkdir(parents=True)
            (licenses / 'nested/notice.txt').write_bytes(b'owned public notice fixture')
            licenses.rename(stage / 'licenses')
        expected = inventory(stage)
        package.read_windows_acl_evidence([parent, stage, stage / 'licenses/nested'])
        # This is an access-only fixture, not a model/package acceptance claim.
        # The production publisher itself is exercised; only its package-specific
        # PE/model/license validator is replaced by an exact fixture hash inventory.
        with patch.object(package, 'validate_shape', side_effect=inventory):
            package.publish_inherited_copy(stage, output)
        package.read_windows_acl_evidence([parent, output, output / 'licenses/nested'])
        env = dict(os.environ, SINGLES_STAGING_PROBE_CHILD='1',
                   SINGLES_STAGING_PROBE_ROOT=str(output),
                   SINGLES_STAGING_PROBE_HASHES=json.dumps(expected))
        command = command_for(process_security(), Path(os.environ['SINGLES_GSUDO_EXE']),
                              sys.executable, Path(__file__).resolve())
        process = subprocess.Popen(command, env=env)
        try:
            returncode = process.wait(timeout=60)
            if returncode:
                raise subprocess.CalledProcessError(returncode, command)
        except subprocess.TimeoutExpired:
            # Same bounded owned-process-tree cleanup as run_desktop_medium.py.
            subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                           capture_output=True, timeout=10, check=True)
            process.wait(timeout=10)
            raise
    # Retain these authored public fixtures for ephemeral-runner failure diagnosis.
    print('STAGING_ACCESS_PREFLIGHT_PASSED', flush=True)


if __name__ == '__main__':
    main()
