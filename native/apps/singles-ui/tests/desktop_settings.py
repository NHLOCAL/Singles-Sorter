"""Read actual rendered settings/list state and verify owned native CSV exports."""
import csv
import hashlib
import io
import json

from desktop_modes import select_sort_setup

MAIN_SETTINGS = ('copy_mode', 'main_folder_only', 'singles_folder', 'abc_sort', 'exist_only')
SETTINGS_CHECKS = ('all-main-settings-explicit-save-and-restart',
                   'unsaved-main-draft-not-persisted-by-immediate-duet',
                   'personal-csv-export-exact-ordered-rows',
                   'settings-baseline-restored')
EDITOR_PAGE = """return {
 rows: Array.from(document.querySelectorAll('[id^=list-alias-]')).map(input => {
   const index = Number(input.id.slice('list-alias-'.length));
   return [index, input.value, document.querySelector('#list-folder-' + index).value];
 }),
 malformed: document.querySelectorAll('#list-rows .list-row-warning').length,
 next_disabled: document.querySelector('#list-next').disabled
};"""


def read_main_settings(script):
    values = script('return Object.fromEntries(' + json.dumps(MAIN_SETTINGS) +
                    '.map(key => [key, document.getElementById(key).checked]));')
    if not isinstance(values, dict) or set(values) != set(MAIN_SETTINGS) or any(type(value) is not bool for value in values.values()):
        raise ValueError(f'Actual main settings are not five booleans: {values!r}')
    return values


def set_main_settings(script, click, wait, expected):
    if set(expected) != set(MAIN_SETTINGS) or any(type(value) is not bool for value in expected.values()):
        raise ValueError('Expected main settings must contain the exact five boolean controls')
    select_sort_setup(script, click, wait)
    for key in MAIN_SETTINGS:
        if read_main_settings(script)[key] != expected[key]:
            click('#' + key)
            wait(lambda: read_main_settings(script)[key] == expected[key])
    if read_main_settings(script) != expected:
        raise ValueError('Actual main controls did not retain the requested draft')


def editor_rows(script, click, wait):
    """A newly opened editor starts at page zero; every row is read via its DOM."""
    rows = []
    while True:
        page = script(EDITOR_PAGE)
        if not isinstance(page, dict) or page.get('malformed') != 0 or type(page.get('next_disabled')) is not bool:
            raise ValueError(f'Cannot infer exact CSV records from a malformed editor page: {page!r}')
        actual = page.get('rows')
        if not isinstance(actual, list) or len(actual) > 50:
            raise ValueError('Unexpected rendered personal-list page length')
        for row in actual:
            if not isinstance(row, list) or len(row) != 3 or type(row[0]) is not int or row[0] != len(rows) or any(not isinstance(value, str) for value in row[1:]):
                raise ValueError(f'Personal-list row index/shape is not exact: {row!r}')
            rows.append(row[1:])
        if page['next_disabled']:
            return rows
        if len(actual) != 50:
            raise ValueError('Nonterminal personal-list page is not complete')
        # The native profile is byte-bounded. This independent harness bound also
        # prevents contradictory pagination from looping indefinitely.
        if len(rows) >= 100000:
            raise ValueError('Owned personal-list fixture exceeds the acceptance bound')
        next_index = len(rows)
        click('#list-next')
        wait(lambda: script("return document.querySelector('#list-alias-" + str(next_index) + "') !== null"))



def require_export_notice(path, notice):
    expected = 'הרשימה יוצאה: ' + str(path)
    if notice != expected:
        raise ValueError(f'Native export returned a different selected path: actual={notice!r}, expected={expected!r}')


def verify_export(path, expected_rows):
    """The caller supplies a fresh owned directory, with no precreated CSV."""
    if path.is_symlink() or not path.is_file() or set(path.parent.iterdir()) != {path}:
        raise ValueError(f'Native export did not create exactly one ordinary owned CSV: requested={str(path)!r}, exists={path.exists()}, file={path.is_file()}, symlink={path.is_symlink()}, owned_directory_entries={sorted(child.name for child in path.parent.iterdir())!r}')
    if path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError('Native export exceeds the profile CSV bound')
    data = path.read_bytes()
    with path.open(encoding='utf-8', newline='') as stream:
        observed = list(csv.reader(stream))
    if observed != expected_rows:
        raise ValueError(f'Exported ordered CSV records differ: actual={observed!r}, expected={expected_rows!r}')
    exact = io.StringIO(newline='')
    csv.writer(exact, lineterminator='\r\n').writerows(expected_rows)
    if data != exact.getvalue().encode('utf-8'):
        raise ValueError('Native export differs from the UTF-8/CRLF ordered CSV encoding contract')
    return {'rows': observed, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
