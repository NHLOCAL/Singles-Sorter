"""Observe unchanged original v14 methods; mutations are confined to one owned tree.

This is Windows test instrumentation, not a substitute policy or synthetic parser.
"""
from __future__ import annotations
import builtins
from contextlib import ExitStack, contextmanager
import io
import logging
import os
from pathlib import Path
import shutil
from unittest.mock import patch


class MutationConfinementError(BaseException):
    """Escape original Exception catches before any outside-root mutation."""


def relative(root, path):
    # Strip only the exact owned absolute prefix. No case/separator/Unicode folding.
    value, prefix = os.fspath(path), str(root) + os.sep
    if value == str(root):
        return '.'
    if not value.startswith(prefix):
        raise ValueError(f'Path does not have the exact owned prefix: {value}')
    return value[len(prefix):]


def owned(root, path, **kwargs):
    if isinstance(path, int) or any(value is not None for value in kwargs.values()):
        raise MutationConfinementError('Unresolved descriptor-relative mutation refused')
    path = Path(os.fsdecode(path))
    try:
        path.resolve().relative_to(root.resolve())
    except (ValueError, OSError) as error:
        raise MutationConfinementError(f'Mutation outside owned case refused: {path}') from error
    # Owned cases contain no links. Reject a new link rather than following it.
    if any(p.is_symlink() for p in [path, *path.parents] if p == root or root in p.parents):
        raise MutationConfinementError('Linked mutation path refused')


@contextmanager
def confine_mutations(root):
    """Guard all filesystem mutators reachable by the frozen original + shutil."""
    with ExitStack() as stack:
        def one(original):
            def wrapped(path, *args, **kwargs):
                owned(root, path, dir_fd=kwargs.get('dir_fd'))
                return original(path, *args, **kwargs)
            return wrapped

        def two(original):
            def wrapped(source, target, *args, **kwargs):
                owned(root, source, dir_fd=kwargs.get('src_dir_fd'))
                owned(root, target, dir_fd=kwargs.get('dst_dir_fd'))
                return original(source, target, *args, **kwargs)
            return wrapped

        def opened(original):
            def wrapped(path, mode='r', *args, **kwargs):
                if any(flag in mode for flag in 'wax+'):
                    owned(root, path)
                return original(path, mode, *args, **kwargs)
            return wrapped

        raw_open = os.open
        def os_open(path, flags, *args, **kwargs):
            if flags & (os.O_CREAT | os.O_TRUNC | os.O_APPEND | os.O_WRONLY | os.O_RDWR):
                owned(root, path, dir_fd=kwargs.get('dir_fd'))
            return raw_open(path, flags, *args, **kwargs)

        for name in ('mkdir', 'rmdir', 'unlink', 'remove', 'chmod', 'utime', 'truncate', 'setxattr', 'chflags'):
            if hasattr(os, name):
                stack.enter_context(patch.object(os, name, one(getattr(os, name))))
        for name in ('rename', 'replace', 'link', 'symlink'):
            if hasattr(os, name):
                stack.enter_context(patch.object(os, name, two(getattr(os, name))))
        stack.enter_context(patch.object(builtins, 'open', opened(builtins.open)))
        stack.enter_context(patch.object(io, 'open', opened(io.open)))
        stack.enter_context(patch.object(os, 'open', os_open))
        yield


class CaptureLogs(logging.Handler):
    def __init__(self, trace):
        super().__init__(logging.DEBUG)
        self.trace = trace

    def emit(self, record):
        self.trace.append({'kind': 'log', 'level': record.levelname, 'message': record.getMessage()})


def execute_original(root, settings, template, *, ai):
    """Construct a fresh original sorter with real metadata and real loaded models."""
    import sys
    released = sys.modules['singles_sorter_v5']
    trace, decisions, operations, inference = [], [], [], []
    logger = logging.getLogger('paired-original-' + root.parent.name)
    logger.handlers = [CaptureLogs(trace)]
    logger.propagate = False
    previous = Path.cwd()
    try:
        os.chdir(root)
        # Prevent original __init__ from attempting another relative model load.
        # No routing, transfer, album, counter or summary method is replaced.
        with patch.object(released, 'ai_invalid', True):
            sorter = released.MusicSorter(root/'source', root/'target', logger=logger,
                                          log_level=logging.DEBUG, **settings)
        sorter.ai_models = template.ai_models if ai else None
        with ExitStack() as stack:
            stack.enter_context(patch.object(released, 'ai_invalid', not ai))
            for name, kind in [('analyze_album', 'album'), ('artists_from_song', 'single')]:
                original = getattr(sorter, name)
                def decision(path, original=original, kind=kind):
                    record = {'kind': kind, 'source': relative(root, path)}
                    trace.append(record)
                    result = original(path)
                    record['result'] = result
                    decisions.append(record.copy())
                    return result
                stack.enter_context(patch.object(sorter, name, decision))
            for name in ('process_with_ner', 'verify_artist_with_sklearn') if ai else ():
                original = getattr(sorter.ai_models, name)
                def model(text, original=original, name=name):
                    record = {'method': name, 'text': text}
                    inference.append(record)
                    result = original(text)
                    # sklearn returns numpy.bool_; serialize that observation as
                    # a JSON boolean without changing the value returned to v14.
                    record['result'] = bool(result) if name == 'verify_artist_with_sklearn' else result
                    return result
                stack.enter_context(patch.object(sorter.ai_models, name, model))
            target_path = sorter.get_target_path
            def destination(artist):
                path = target_path(artist)
                trace.append({'kind': 'destination_decision', 'artist': artist,
                              'destination': relative(root, path), 'is_directory': path.is_dir(),
                              'exist_only': sorter.exist_only})
                return path
            stack.enter_context(patch.object(sorter, 'get_target_path', destination))

            def operation(kind, original):
                def wrapped(source, target=None, *args, **kwargs):
                    owned(root, source)
                    if target is not None:
                        owned(root, target)
                    record = {'kind': kind, 'source': relative(root, source)}
                    if target is not None:
                        record['destination'] = relative(root, target)
                    trace.append(record)
                    try:
                        result = original(source, *args, **kwargs) if target is None else original(source, target, *args, **kwargs)
                        record['success'] = result is not False
                        if record['success']:
                            operations.append(record.copy())
                        return result
                    except Exception as error:
                        record['error'] = {'type': type(error).__name__, 'message': str(error)}
                        raise
                return wrapped
            stack.enter_context(patch.object(shutil, 'copy2', operation('copy', shutil.copy2)))
            stack.enter_context(patch.object(sorter, 'move_file', operation('move', sorter.move_file)))
            stack.enter_context(patch.object(Path, 'unlink', operation('unlink', Path.unlink)))
            stack.enter_context(patch.object(Path, 'rmdir', operation('rmdir', Path.rmdir)))
            walk = os.walk
            def walks(path, *args, **kwargs):
                for folder, directories, files in walk(path, *args, **kwargs):
                    trace.append({'kind': 'walk', 'source': relative(root, folder),
                                  'directories': list(directories), 'files': list(files)})
                    yield folder, directories, files
            stack.enter_context(patch.object(os, 'walk', walks))
            for name in ('iterdir', 'glob'):
                original = getattr(Path, name)
                def listing(path, *args, original=original, name=name, **kwargs):
                    record = {'kind': name, 'source': relative(root, path), 'children': []}
                    trace.append(record)
                    for item in original(path, *args, **kwargs):
                        record['children'].append(item.name)
                        yield item
                stack.enter_context(patch.object(Path, name, listing))
            error, summary = None, None
            with confine_mutations(root):
                try:
                    summary = sorter.scan_dir()
                except (Exception, MutationConfinementError) as caught:
                    error = {'type': type(caught).__name__, 'message': str(caught)}
        return {'error': error, 'legacy_summary': summary, 'ordered_trace': trace,
                'ordered_decisions': decisions, 'ordered_operations': operations,
                'model_calls': inference}
    finally:
        os.chdir(previous)
        logger.handlers = []
