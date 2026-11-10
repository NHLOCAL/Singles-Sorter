#!/usr/bin/env python3
"""Test-only export from unchanged release code; never the replacement policy."""
import argparse
import builtins
from contextlib import ExitStack, contextmanager
import csv
from importlib import metadata as package_metadata
import _csv
import io
import itertools
import json
import logging
import os
import platform
import sys
import tempfile
import unicodedata
from unittest.mock import patch
import hashlib
from pathlib import Path
import subprocess

SHA = '78d001aee5ce49628bdffebd34540fa83ba32c12'
SOURCE_FILES = ['src/core/singles_sorter_v5.py', 'src/core/check_name.py',
                'src/core/jibrish_to_hebrew.py', 'src/core/app/singer-list.csv']

def verify_baseline(baseline):
    def git(*args):
        return subprocess.check_output(['git', '-C', str(baseline), *args], text=True).strip()
    if git('rev-parse', 'HEAD') != SHA:
        raise RuntimeError('Reference must be the original pinned v14.0 checkout')
    sources = {}
    for name in SOURCE_FILES:
        path = baseline / name
        blob = git('rev-parse', f'{SHA}:{name}')
        if git('hash-object', str(path)) != blob:
            raise RuntimeError(f'Refusing modified reference: {name}')
        sources[name] = {'git_blob': blob, 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'bytes': path.stat().st_size}
    return sources

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--inputs', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    export(args.baseline.resolve(), args.inputs.resolve(), args.output.resolve())

# No bytes represent a recording. All reference mutations stay in an owned
# TemporaryDirectory. Metadata saves below only change this in-memory object.
PLACEHOLDER = b"owned synthetic placeholder; not an audio recording\n"
FIELDS = ['artist', 'albumartist', 'title', 'album', 'genre', 'tracknumber']


def scalar(value):
    """Lossless tagged observations; Python integers never become machine ints."""
    if value is None:
        return {'kind': 'null'}
    if isinstance(value, int):
        return {'kind': 'integer', 'decimal': str(value)}
    return {'kind': 'text', 'text': value}


def decode(spec):
    if isinstance(spec, dict):
        if spec.get('kind') == 'integer':
            return int(spec['decimal'])
        if spec.get('kind') == 'null':
            return None
        if spec.get('kind') == 'text':
            return spec['text']
    return spec


def injected_error(name, stage):
    classes = {'OSError': OSError, 'ValueError': ValueError, 'KeyError': KeyError,
               'TypeError': TypeError, 'RuntimeError': RuntimeError}
    return classes[name](f'injected {stage} error')


class SyntheticField:
    def __init__(self, owner, name, spec):
        self.owner, self.name, self.spec = owner, name, spec
        configured = spec if isinstance(spec, dict) else {}
        value = decode(spec) if 'kind' in configured or not configured else configured.get('value', '')
        self.values = [decode(v) for v in configured.get('values', [value])]

    def __bool__(self):
        configured = self.spec if isinstance(self.spec, dict) else {}
        result = configured.get('truthy', any(self.values))
        self.owner.emit('truthy', field=self.name, result=result)
        return result

    @property
    def value(self):
        configured = self.spec if isinstance(self.spec, dict) else {}
        if 'value_error' in configured:
            self.owner.emit('value', field=self.name, error={'type': configured['value_error']})
            raise injected_error(configured['value_error'], 'value')
        value = decode(self.spec) if not configured or 'kind' in configured else decode(configured.get('value', ''))
        self.owner.emit('value', field=self.name, values=[scalar(v) for v in self.values], result=scalar(value))
        return value


class SyntheticMetadata(dict):
    def __init__(self, path, specs, trace):
        self.path, self.trace, self.saved = path, trace, 0
        dict.__init__(self)
        for name in FIELDS:
            dict.__setitem__(self, name, SyntheticField(self, name, specs.get(name, '')))
        self.specs = specs

    def emit(self, action, **data):
        self.trace.append({'kind': 'metadata_' + action, 'source': self.path, **data})

    def get(self, name, default=None):
        return self.read(name, 'get')

    def __getitem__(self, name):
        return self.read(name, 'item')

    def read(self, name, mode):
        spec = self.specs.get(name, {})
        if isinstance(spec, dict) and 'get_error' in spec:
            self.emit(mode, field=name, error={'type': spec['get_error']})
            raise injected_error(spec['get_error'], 'get')
        self.emit(mode, field=name)
        return dict.__getitem__(self, name)

    def __setitem__(self, name, value):
        self.emit('set', field=name, result=scalar(value))
        dict.__setitem__(self, name, SyntheticField(self, name, value))

    def save(self):
        self.saved += 1
        self.emit('save_in_memory', count=self.saved)

    def snapshot(self):
        result = {}
        for name, item in dict.items(self):
            # Snapshot without calling get/bool/value and changing the oracle trace.
            spec = item.spec if isinstance(item.spec, dict) else {}
            result[name] = {'values': [scalar(v) for v in item.values]}
            if 'value_error' in spec:
                result[name]['value_error'] = spec['value_error']
            else:
                value = decode(item.spec) if not spec or 'kind' in spec else decode(spec.get('value', ''))
                result[name]['value'] = scalar(value)
        return {'fields': result, 'save_intentions': self.saved}


class SyntheticAI:
    def __init__(self, spec, trace):
        self.spec, self.trace = spec, trace

    def verify_artist_with_sklearn(self, text):
        result = self.spec.get('accept', False)
        self.trace.append({'kind': 'synthetic_classifier', 'text': text, 'result': result})
        return result

    def process_with_ner(self, text):
        result = list(self.spec.get('ner', {}).get(text, []))
        self.trace.append({'kind': 'synthetic_ner', 'text': text, 'result': result})
        return result


class CaptureLogs(logging.Handler):
    def __init__(self, root, trace):
        super().__init__(logging.WARNING)
        self.root, self.trace = root, trace

    def emit(self, record):
        self.trace.append({'kind': 'legacy_log', 'level': record.levelname,
                           'message': record.getMessage().replace(str(self.root), '$SCRATCH')})


class MutationConfinementError(BaseException):
    """Harness safety abort, deliberately outside the release's Exception catch.

    An outside-root mutation must abort the case, not become a rename failure
    which the legacy fallback or later duet cleanup could silently work around.
    """


def guard_owned(root, path):
    # Resolve lexical .. and existing symlinks before touching any object.
    Path(path).resolve().relative_to(root.resolve())


def relative(root, path):
    path = Path(path).absolute()
    return path.relative_to(root).as_posix()


def observe(root, path):
    path = Path(path)
    result = {'path': relative(root, path), 'name': path.name}
    if not path.exists():
        return {**result, 'kind': 'missing'}
    if path.is_symlink():
        return {**result, 'kind': 'link'}
    if path.is_dir():
        return {**result, 'kind': 'directory'}
    data = path.read_bytes()
    return {**result, 'kind': 'file', 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def tree_snapshot(root, path):
    """Manifests retain encounter order, with no timestamps or scratch identities."""
    result = [observe(root, path)]
    if path.is_dir():
        with os.scandir(path) as entries:
            for entry in entries:
                result.extend(tree_snapshot(root, Path(entry.path)))
    return result


@contextmanager
def release_module(baseline):
    previous = Path.cwd()
    with tempfile.TemporaryDirectory(prefix='singles-reference-import-') as directory:
        os.chdir(directory)  # Model discovery must never find the release's models.
        sys.path.insert(0, str(baseline / 'src/core'))
        try:
            import singles_sorter_v5 as released
            if not released.ai_invalid or 'ai_models' in sys.modules:
                raise RuntimeError('Actual AI imports are forbidden in this exporter')
            yield released
            if 'ai_models' in sys.modules:
                raise RuntimeError('Actual AI module was loaded')
        finally:
            sys.path.remove(str(baseline / 'src/core'))
            os.chdir(previous)


def run_case(released, case, default_aliases):
    previous = Path.cwd()
    with tempfile.TemporaryDirectory(prefix='singles-full-contract-') as directory:
        root = Path(directory)
        os.chdir(root)
        trace, metadata_objects = [], {}
        source, target = root / 'source', root / 'target'
        source.mkdir(); target.mkdir(); (root / 'app').mkdir()
        with (root / 'app/singer-list.csv').open('w', encoding='utf-8', newline='') as file:
            csv.writer(file, lineterminator='\n').writerows(case.get('aliases', default_aliases))
        if 'personal_csv' in case:
            (root / 'app/personal-singer-list.csv').write_text(case['personal_csv'], encoding='utf-8')
        file_specs = {}
        for file in case['files']:
            path = source / file['path']
            guard_owned(root, path)
            path.parent.mkdir(parents=True, exist_ok=True)
            if file.get('kind') == 'directory':
                path.mkdir(exist_ok=True)
            else:
                path.write_bytes(PLACEHOLDER)
            file_specs[relative(root, path)] = file
        if case['op'] == 'album':
            guard_owned(root, root / case['folder'])
            (root / case['folder']).mkdir(parents=True, exist_ok=True)
        for path in case.get('target_directories', []):
            guard_owned(root, target / path)
            (target / path).mkdir(parents=True, exist_ok=True)
        for path in case.get('target_files', []):
            path = target / path
            guard_owned(root, path)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(PLACEHOLDER)
        before = {'source': tree_snapshot(root, source), 'target': tree_snapshot(root, target)}
        settings = case.get('settings', {})
        logger = logging.getLogger('full-contract-' + case['id'])
        logger.handlers = [CaptureLogs(root, trace)]; logger.propagate = False
        original_iterdir, original_glob, original_rglob = Path.iterdir, Path.glob, Path.rglob
        original_walk = os.walk
        original_rename, original_unlink, original_rmdir, original_mkdir = Path.rename, Path.unlink, Path.rmdir, Path.mkdir
        original_copy2, original_copytree, original_move = released.shutil.copy2, released.shutil.copytree, released.shutil.move
        original_builtin_open, original_io_open, original_os_open = builtins.open, io.open, os.open
        primitives = {name: getattr(os, name) for name in
                      ['mkdir', 'rename', 'replace', 'unlink', 'rmdir', 'chmod', 'utime', 'setxattr', 'chflags'] if hasattr(os, name)}

        def mutation_path(path, dir_fd=None):
            # shutil.rmtree's fd-relative fallback must stay guarded too. This
            # frozen Linux reference resolves the actual descriptor through procfs;
            # an unavailable/unresolvable descriptor is rejected, never guessed.
            if isinstance(path, int):
                return Path(os.readlink(f'/proc/self/fd/{path}'))
            path = Path(os.fsdecode(path))
            if dir_fd is not None and not path.is_absolute():
                path = Path(os.readlink(f'/proc/self/fd/{dir_fd}')) / path
            return path

        def owned(path, entrypoint='mutation', role='path', dir_fd=None):
            try:
                resolved = mutation_path(path, dir_fd)
                guard_owned(root, resolved)
                return resolved
            except (ValueError, OSError) as cause:
                trace.append({'kind': 'mutation_blocked', 'entrypoint': entrypoint,
                              'role': role, 'reason': 'outside_or_unresolved_owned_root'})
                raise MutationConfinementError('mutation outside or unresolved owned case root') from cause

        def listing(method, path, iterator, pattern=None):
            event = {'kind': method, 'folder': relative(root, path), 'children': []}
            if pattern is not None: event['pattern'] = str(pattern)
            if method == 'rglob': event['completed'] = False
            trace.append(event)
            for child in iterator:
                event['children'].append(observe(root, child))
                yield child
            if method == 'rglob':
                event['completed'] = True
                trace.append({'kind': 'collection_complete', 'method': method,
                              'folder': relative(root, path), 'count': len(event['children'])})

        def iterdir(path):
            return listing('iterdir', path, original_iterdir(path))

        def glob(path, pattern, *args, **kwargs):
            return listing('glob', path, original_glob(path, pattern, *args, **kwargs), pattern)

        def rglob(path, pattern, *args, **kwargs):
            return listing('rglob', path, original_rglob(path, pattern, *args, **kwargs), pattern)

        def walk(path, *args, **kwargs):
            for folder, dirs, files in original_walk(path, *args, **kwargs):
                trace.append({'kind': 'walk', 'folder': relative(root, folder), 'directories': list(dirs), 'files': list(files)})
                yield folder, dirs, files

        def operation(kind, original, path, destination=None, *args, **kwargs):
            owned(path, kind, 'source')
            if destination is not None: owned(destination, kind, 'destination')
            event = {'kind': kind, 'source_observation': observe(root, path)}
            if destination is not None: event['destination'] = relative(root, destination)
            trace.append(event)
            try:
                arguments = (path,) if destination is None else (path, destination)
                value = original(*arguments, *args, **kwargs)
                event['outcome'] = 'success'
                return value
            except (Exception, MutationConfinementError) as error:
                event['outcome'] = 'blocked' if isinstance(error, MutationConfinementError) else 'error'
                event['error'] = {'type': type(error).__name__, 'message': str(error).replace(str(root), '$SCRATCH')}
                raise

        def rename(path, destination):
            owned(path, 'rename', 'source'); owned(destination, 'rename', 'destination')
            old, new = relative(root, path), relative(root, destination)
            value = operation('rename', original_rename, path, destination)
            # Metadata identity follows a successful rename, never an invented path.
            if old in file_specs: file_specs[new] = file_specs[old]
            return value

        def primitive_one(name, original):
            def wrapped(path, *args, **kwargs):
                display = owned(path, 'os_' + name, dir_fd=kwargs.get('dir_fd'))
                return operation('os_' + name, lambda _: original(path, *args, **kwargs), display)
            return wrapped

        def primitive_two(name, original):
            def wrapped(path, destination, *args, **kwargs):
                source_display = owned(path, 'os_' + name, 'source', kwargs.get('src_dir_fd'))
                target_display = owned(destination, 'os_' + name, 'destination', kwargs.get('dst_dir_fd'))
                return operation('os_' + name, lambda _source, _target: original(path, destination, *args, **kwargs),
                                 source_display, target_display)
            return wrapped

        def open_guard(original, name):
            def wrapped(path, mode='r', *args, **kwargs):
                if any(flag in mode for flag in 'wax+'):
                    display = owned(path, name, 'destination')
                    return operation(name, lambda _: original(path, mode, *args, **kwargs), display)
                return original(path, mode, *args, **kwargs)
            return wrapped

        def os_open(path, flags, *args, **kwargs):
            if flags & (os.O_CREAT | os.O_TRUNC | os.O_APPEND | os.O_WRONLY | os.O_RDWR):
                display = owned(path, 'os_open_write', 'destination', kwargs.get('dir_fd'))
                return operation('os_open_write', lambda _: original_os_open(path, flags, *args, **kwargs), display)
            return original_os_open(path, flags, *args, **kwargs)

        def load_file(path):
            key = relative(root, path)
            event = {'kind': 'metadata_open', 'source_observation': observe(root, path), 'provider': 'synthetic'}
            trace.append(event)
            spec = file_specs.get(key, {}).get('metadata', {})
            if 'open_error' in spec:
                event['error'] = {'type': spec['open_error']}
                raise injected_error(spec['open_error'], 'open')
            if not Path(path).is_file():
                event['error'] = {'type': 'OSError'}
                raise injected_error('OSError', 'open')
            if key not in metadata_objects:
                metadata_objects[key] = SyntheticMetadata(key, spec, trace)
            return metadata_objects[key]

        try:
            with ExitStack() as patches:
                patches.enter_context(patch.object(released, 'ai_invalid', True))
                sorter = released.MusicSorter(source, target, logger=logger, **settings)
                if 'ai' in case:
                    sorter.ai_models = SyntheticAI(case['ai'], trace)
                    patches.enter_context(patch.object(released, 'ai_invalid', False))
                patches.enter_context(patch.object(released, 'load_file', load_file))
                patches.enter_context(patch.object(Path, 'iterdir', iterdir))
                patches.enter_context(patch.object(Path, 'glob', glob))
                patches.enter_context(patch.object(Path, 'rglob', rglob))
                patches.enter_context(patch.object(os, 'walk', walk))
                patches.enter_context(patch.object(Path, 'rename', rename))
                patches.enter_context(patch.object(Path, 'mkdir', lambda path, *a, **kw: operation('mkdir', original_mkdir, path, None, *a, **kw)))
                patches.enter_context(patch.object(released.shutil, 'move', lambda path, dest, *a, **kw: operation('shutil_move', original_move, path, dest, *a, **kw)))
                for name, original in primitives.items():
                    wrapper = primitive_two(name, original) if name in ['rename', 'replace'] else primitive_one(name, original)
                    patches.enter_context(patch.object(os, name, wrapper))
                patches.enter_context(patch.object(builtins, 'open', open_guard(original_builtin_open, 'open_write')))
                patches.enter_context(patch.object(io, 'open', open_guard(original_io_open, 'io_open_write')))
                patches.enter_context(patch.object(os, 'open', os_open))
                patches.enter_context(patch.object(Path, 'unlink', lambda path, *a, **kw: operation('unlink', original_unlink, path, None, *a, **kw)))
                patches.enter_context(patch.object(Path, 'rmdir', lambda path: operation('rmdir', original_rmdir, path)))
                patches.enter_context(patch.object(released.shutil, 'copy2', lambda path, dest, *a, **kw: operation('copy2', original_copy2, path, dest, *a, **kw)))
                patches.enter_context(patch.object(released.shutil, 'copytree', lambda path, dest, *a, **kw: operation('copytree', original_copytree, path, dest, *a, **kw)))
                for name, event_kind in [('analyze_album', 'album_decision'), ('artists_from_song', 'single_decision')]:
                    original = getattr(sorter, name)
                    def wrap(path, original=original, kind=event_kind):
                        event = {'kind': kind, 'source_observation': observe(root, path)}; trace.append(event)
                        if kind == 'single_decision' and 'artists' in file_specs.get(relative(root, path), {}):
                            event['provider'] = 'synthetic_artist_override'
                            value = list(file_specs[relative(root, path)]['artists'])
                        else:
                            try: value = original(path)
                            except Exception as error:
                                event['error'] = {'type': type(error).__name__, 'message': str(error)}; raise
                        event['result'] = value
                        return value
                    patches.enter_context(patch.object(sorter, name, wrap))
                original_transfer = sorter.handle_album_transfer
                def transfer(path, name, artist):
                    event = {'kind': 'album_transfer', 'source_observation': observe(root, path), 'album_name': name, 'artist_name': artist}; trace.append(event)
                    return original_transfer(path, name, artist)
                patches.enter_context(patch.object(sorter, 'handle_album_transfer', transfer))
                if case.get('inject_move_failure'):
                    def fail_move(path, dest):
                        owned(path); owned(dest)
                        trace.append({'kind': 'injected_move_failure', 'source_observation': observe(root, path), 'destination': relative(root, dest), 'outcome': False})
                        return False
                    patches.enter_context(patch.object(sorter, 'move_file', fail_move))
                error = None
                try:
                    if case['op'] == 'album': result = sorter.analyze_album(root / case['folder'])
                    elif case['op'] == 'single': result = sorter.artists_from_song(source / case['files'][0]['path'])
                    elif case['op'] == 'fix': result = sorter.fix_names()
                    else: result = sorter.scan_dir()
                except (Exception, MutationConfinementError) as caught:
                    result = None
                    error = {'type': type(caught).__name__, 'message': str(caught).replace(str(root), '$SCRATCH')}
                summary = sorter.generate_summary()
            after = {'source': tree_snapshot(root, source), 'target': tree_snapshot(root, target)}
            return {'id': case['id'], 'op': case['op'], 'input_sha256': hashlib.sha256(canonical(case)).hexdigest(),
                    'provenance': 'unchanged_v14_release_with_synthetic_metadata_and_ai',
                    'result': result, 'error': error, 'ordered_trace': trace, 'before': before, 'after': after,
                    'legacy_counters': summary,
                    'metadata_after': {key: value.snapshot() for key, value in metadata_objects.items()},
                    'historical_only': case.get('historical_only', False), 'future_acceptance': case.get('future_acceptance', True)}
        finally:
            logger.handlers = []
            os.chdir(previous)


def canonical(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode('utf-8')


def csv_reference():
    # Frozen independent exhaustive small dialect corpus plus Unicode/BOM anchors.
    inputs = [''.join(chars) for n in range(5) for chars in itertools.product('a,"\n\r ', repeat=n)]
    inputs.extend(['\ufeffדוד, יעד ', '"דוד\r\nישראל",יעד', 'a\x00b,c', '"a"x,b', ',,', '""'])
    cases = []
    for index, value in enumerate(inputs):
        with io.TextIOWrapper(io.BytesIO(value.encode('utf-8')), encoding='utf-8', newline=None) as file:
            rows = list(csv.reader(file))
        cases.append({'id': index, 'text': value, 'rows': rows})
    return {'dialect': 'CPython csv.reader default excel; Path.open newline=None', 'field_limit': csv.field_size_limit(), 'cases': cases}


def export(baseline, inputs_path, output):
    sources = verify_baseline(baseline)
    if platform.python_implementation() != 'CPython' or platform.python_version() != '3.12.14' or unicodedata.unidata_version != '15.0.0':
        raise RuntimeError('Expected CPython 3.12.14 / Unicode 15.0.0 reference')
    if package_metadata.version('music-tag') != '0.4.3' or package_metadata.version('mutagen') != '1.47.0':
        raise RuntimeError('Expected music-tag 0.4.3 / Mutagen 1.47.0 reference')
    if output.exists() and any(output.iterdir()):
        raise RuntimeError('Refusing to overwrite frozen output; export into a fresh directory')
    if output == baseline or baseline in output.parents:
        raise RuntimeError('Never write export output into the frozen reference checkout')
    data = json.loads(inputs_path.read_text(encoding='utf-8'))
    ids = [case['id'] for case in data['cases']]
    if len(set(ids)) != len(ids): raise RuntimeError('Duplicate fixture IDs')
    with release_module(baseline) as released:
        rows = [run_case(released, case, data['default_aliases']) for case in data['cases']]
    if verify_baseline(baseline) != sources:
        raise RuntimeError('Reference source changed during export')
    output.mkdir(parents=True, exist_ok=True)
    artifacts = {
        'cases.jsonl': b''.join(canonical(row) for row in rows if not row['historical_only']),
        'historical-unsafe.jsonl': b''.join(canonical(row) for row in rows if row['historical_only']),
        'csv-dialect.json': canonical(csv_reference()),
    }
    for name, contents in artifacts.items(): (output / name).write_bytes(contents)
    manifest = {'schema_version': 2, 'purpose': 'Immutable full release oracle; expected results are never regenerated from Rust',
                'baseline': {'revision': SHA, 'repository': 'NHLOCAL/Singles-Sorter', 'files': sources},
                'runtime': {'implementation': platform.python_implementation(), 'python': platform.python_version(), 'unicode': unicodedata.unidata_version,
                            'os': platform.system(), 'music_tag': package_metadata.version('music-tag'), 'mutagen': package_metadata.version('mutagen')},
                'runtime_hashes': {name: hashlib.sha256(Path(path).read_bytes()).hexdigest() for name, path in {
                    'python_executable': Path(sys.executable).resolve(), 'csv_module': csv.__file__,
                    'csv_extension_or_builtin_host': getattr(_csv, '__file__', Path(sys.executable).resolve()),
                    'unicode_extension_or_builtin_host': getattr(unicodedata, '__file__', Path(sys.executable).resolve())}.items()},
                'runtime_module_origins': {'_csv': _csv.__spec__.origin, 'unicodedata': unicodedata.__spec__.origin},
                'reference_tooling': {name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest() for name in
                    ['requirements.lock', 'test_release_characterization.py', 'test_historical_unsafe_observations.py']},
                'characterization_port': {'total_assertion_cases': 70, 'policy_cases': 68, 'historical_unsafe_cases': 2,
                    'method': 'Byte-for-byte independent Python test port; no Rust implementation or outputs imported.'},
                'inputs': {'file': 'inputs.json', 'sha256': hashlib.sha256(inputs_path.read_bytes()).hexdigest(), 'cases': len(rows)},
                'coverage': {op: sum(row['op'] == op for row in rows) for op in ['album', 'single', 'sort', 'fix', 'unsafe']},
                'csv_cases': len(csv_reference()['cases']),
                'exporter_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'artifacts': {name: {'sha256': hashlib.sha256(value).hexdigest(), 'bytes': len(value)} for name, value in artifacts.items()},
                'actual_metadata_decoding': False, 'actual_model_inference': False,
                'mechanics': 'Every released mutation path is guarded and traced at high-level and filesystem entrypoints. Outside-root attempts abort the case before legacy fallback. Metadata save is in-memory only.',
                'limitations': ['Encounter order is an observed Linux directory order, never a promised Windows order.',
                               'Synthetic field/get/value errors do not prove any real metadata parser.',
                               'Historical unsafe mechanics are not future executor acceptance.',
                               'No native Windows, Android, actual AI, actual tag writes or physical product apply acceptance.']}
    (output / 'manifest.json').write_bytes(canonical(manifest))
    print(json.dumps({'cases': len(rows), 'output': str(output), 'artifacts': manifest['artifacts']}, ensure_ascii=False))


if __name__ == '__main__':
    sys.dont_write_bytecode = True
    main()
