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


def unique_save_edit(records, pid):
    # Actual f183 Save As UIA evidence exposes FileNameControlHost/Edit1001,
    # unlike the import dialog's cmb13/1148. Do not match localized labels.
    def matching(kind, identity):
        return [index for index, item in enumerate(records)
                if item['pid'] == pid and item['control_type'] == kind
                and item['automation_id'] == identity and item['visible'] and item['enabled']]
    hosts = matching('ComboBox', 'FileNameControlHost')
    edits = matching('Edit', '1001')
    if len(hosts) != 1 or len(edits) != 1:
        raise RuntimeError(f'Expected one observed owned Save filename host/Edit, found {len(hosts)}/{len(edits)}')
    return edits[0]



def literal_keyboard_text(value):
    # pywinauto keyboard syntax treats these characters as commands/modifiers.
    # https://pywinauto.readthedocs.io/en/latest/code/pywinauto.keyboard.html
    if any(ord(character) < 32 or ord(character) == 127 for character in value):
        raise RuntimeError('Owned Save filename contains a control character')
    return ''.join('{' + character + '}' if character in '+^%~{}()' else character
                   for character in value)


def enter_owned_save_path(filename, path, pid):
    # Actual dfae evidence: ValuePattern SetValue/get_value agreed, but Save
    # returned its original default path. Use real edit input and blur before
    # invoking Save so the dialog receives the normal edit/commit notifications.
    value = str(path)
    keys = literal_keyboard_text(value)
    if filename.process_id() != pid:
        raise RuntimeError('Save filename input does not belong to the owned app')
    filename.set_focus()
    if not filename.has_keyboard_focus():
        raise RuntimeError('Owned Save filename input did not receive keyboard focus')
    filename.type_keys('^a{BACKSPACE}', set_foreground=False)
    filename.type_keys(keys, with_spaces=True, set_foreground=False, vk_packet=True)
    if filename.get_value() != value:
        raise RuntimeError('Native Save input did not retain the exact owned fixture path')
    filename.type_keys('{TAB}', set_foreground=False)
    if filename.get_value() != value:
        raise RuntimeError('Native Save input changed after the edit lost focus')


def require_new_save_path(path):
    if path.exists() or path.is_symlink() or not path.parent.is_dir():
        raise RuntimeError('Save acceptance requires a new CSV in an existing owned directory')


def interact_with_owned_picker(binary, path, output, label, *, save=False):
    # UIA calls can block inside COM; contain the entire interaction in an owned
    # helper process rather than letting a stalled control query hang the suite.
    if save:
        require_new_save_path(path)
    try:
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()),
                                 str(binary), str(path), str(output), label] + (['--save'] if save else []),
                                capture_output=True, text=True, encoding='utf-8',
                                errors='replace', timeout=75)
        if result.returncode:
            raise RuntimeError(f'Owned native picker failed: {result.stderr[-4000:]}')
    except Exception as error:
        # Leave any worker-written control tree/image intact if COM stalls later.
        (output / f'{label}-native-helper-failure.json').write_text(
            json.dumps({'status': 'failed', 'error': str(error)}, indent=2), encoding='utf-8')
        raise


def _interact_with_owned_picker(binary, path, output, label, *, save=False):
    if save:
        require_new_save_path(path)
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
                'selection_kind': 'save' if save else 'folder' if path.is_dir() else 'file'}
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
        if save:
            filename = controls[unique_save_edit(records, app.process)]
        elif path.is_dir():
            filename = controls[unique_folder_edit(records, app.process)]
        else:
            # Preserve the existing file-import dialog contract; its actual UIA
            # structure is now captured as evidence too.
            filename = dialog.child_window(auto_id='1148', control_type='ComboBox').child_window(control_type='Edit')
            filename.wait('visible enabled', timeout=10)
            filename = filename.wrapper_object()
        if filename.process_id() != app.process:
            raise RuntimeError('Path input does not belong to the owned app')
        if save:
            evidence['requested_owned_path'] = str(path)
            evidence['filename_before_input'] = filename.get_value()
            enter_owned_save_path(filename, path, app.process)
            evidence['filename_after_keyboard_and_blur'] = filename.get_value()
        else:
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
    if sys.argv[5:] not in ([], ['--save']):
        raise SystemExit('Unknown owned picker interaction mode')
    _interact_with_owned_picker(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4],
                                save=sys.argv[5:] == ['--save'])
