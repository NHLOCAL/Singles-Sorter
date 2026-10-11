"""Real owned IFileDialog interaction, with evidence before choosing a control."""
import json
import os
from pathlib import Path
import subprocess
import sys


def unique_folder_edit(records, pid):
    # Windows SDK dlgs.h defines edt1 = 0x0480 (1152). The folder picker
    # need not expose the file-picker cmb13 parent. Require a real observed Edit.
    matches = [index for index, item in enumerate(records)
               if item['pid'] == pid and item['control_type'] == 'Edit'
               and item['automation_id'] == '1152' and item['visible'] and item['enabled']]
    if len(matches) != 1:
        raise RuntimeError(f'Expected one observed owned folder-path Edit (edt1), found {len(matches)}')
    return matches[0]


def interact_with_owned_picker(binary, path, output, label):
    # UIA calls can block inside COM; contain the entire interaction in an owned
    # helper process rather than letting a stalled control query hang the suite.
    try:
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()),
                                 str(binary), str(path), str(output), label],
                                capture_output=True, text=True, encoding='utf-8',
                                errors='replace', timeout=75)
        if result.returncode:
            raise RuntimeError(f'Owned native picker failed: {result.stderr[-4000:]}')
    except Exception as error:
        # Leave any worker-written control tree/image intact if COM stalls later.
        (output / f'{label}-native-helper-failure.json').write_text(
            json.dumps({'status': 'failed', 'error': str(error)}, indent=2), encoding='utf-8')
        raise


def _interact_with_owned_picker(binary, path, output, label):
    from pywinauto import Application
    app = Application(backend='uia').connect(path=str(binary), timeout=10)
    dialog = app.window(class_name='#32770')
    dialog.wait('visible enabled', timeout=15)
    owned = dialog.wrapper_object()
    if owned.process_id() != app.process:
        raise RuntimeError('Picker does not belong to the connected owned app')
    records, controls = [], []
    evidence = {'kind': 'actual-owned-native-dialog', 'pid': app.process,
                'hwnd': owned.handle, 'title': owned.window_text(),
                'selection_kind': 'folder' if path.is_dir() else 'file'}
    try:
        # Include control metadata, not file-list contents or values. This is the
        # actual dialog's UIA tree, not a guessed screen-wide control selector.
        controls = [control for control in owned.descendants()
                    if control.element_info.control_type in ('Edit', 'ComboBox', 'Button')]
        evidence['controls_truncated'] = len(controls) > 128
        controls = controls[:128]
        for control in controls:
            records.append({'pid': control.process_id(),
                            'control_type': control.element_info.control_type,
                            'automation_id': control.element_info.automation_id,
                            'name': control.window_text(),
                            'visible': control.is_visible(), 'enabled': control.is_enabled()})
        evidence['controls'] = records
        try:
            screenshot = f'{label}-native-dialog.png'
            subprocess.run([sys.executable, str(Path(__file__).with_name('owned_window_capture.py')),
                            str(owned.handle), str(app.process), str(output / screenshot)],
                           capture_output=True, check=True, timeout=5)
            evidence['screenshot'] = screenshot
        except Exception as error:
            evidence['screenshot_error'] = str(error)
        (output / f'{label}-native-dialog.json').write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2), encoding='utf-8')
        if evidence['controls_truncated']:
            raise RuntimeError('Owned dialog has too many controls to establish unique path input safely')
        if path.is_dir():
            filename = controls[unique_folder_edit(records, app.process)]
        else:
            # Preserve the existing file-import dialog contract; its actual UIA
            # structure is now captured as evidence too.
            filename = dialog.child_window(auto_id='1148', control_type='ComboBox').child_window(control_type='Edit')
            filename.wait('visible enabled', timeout=10)
            filename = filename.wrapper_object()
        if filename.process_id() != app.process:
            raise RuntimeError('Path input does not belong to the owned app')
        filename.set_edit_text(str(path))
        if filename.get_value() != str(path):
            raise RuntimeError('Native path input did not retain the exact owned fixture path')
        confirm = dialog.child_window(auto_id='1', control_type='Button')
        confirm.wait('visible enabled', timeout=10)
        if confirm.wrapper_object().process_id() != app.process:
            raise RuntimeError('Confirm control does not belong to the owned app')
        confirm.click()
        dialog.wait_not('visible', timeout=15)
        evidence['dialog_closed_after_native_confirm'] = True
    except Exception as error:
        evidence['error'] = str(error)
        raise
    finally:
        evidence['controls'] = records
        (output / f'{label}-native-dialog.json').write_text(
            json.dumps(evidence, ensure_ascii=False, indent=2), encoding='utf-8')


if __name__ == '__main__':
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
        raise SystemExit('Native picker interaction is restricted to owned Windows CI')
    _interact_with_owned_picker(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4])
