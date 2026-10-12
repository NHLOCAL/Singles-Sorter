"""Portable helper contracts, not substitutes for Windows runtime acceptance."""
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch
import wave

import desktop_ai as ai


def permissions(_):
    return 'test-only-permission-identity'


class DesktopAIContracts(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def package(self):
        package = self.root / 'package'
        package.mkdir()
        (package / 'preview.exe').write_bytes(b'MZ-owned-unit-fixture')
        models = package / 'models/original'
        models.mkdir(parents=True)
        for name in ai.MODEL_FILES:
            (models / name).write_bytes(b'owned bytes for ' + name.encode())
        (package / 'examples/source').mkdir(parents=True)
        (package / 'examples/target').mkdir()
        for name in (ai.AI_NAME, ai.ALIAS_NAME):
            ai.write_wave(package / 'examples/source' / name)
        self.manifest(package)
        return package

    @staticmethod
    def manifest(package):
        entries = [p for p in package.rglob('*') if p.is_file() and p.name != 'SHA256SUMS.txt']
        (package / 'SHA256SUMS.txt').write_text(''.join(
            ai.digest(p) + '  ' + p.relative_to(package).as_posix() + '\n'
            for p in sorted(entries)), encoding='utf-8')

    def test_public_observer_avoids_documented_raw_stat_disagreement(self):
        public = dict(win32_path_attributes=16, win32_handle_attributes=16)
        self.assertEqual(ai.observed_attributes(16, public), 16)
        self.assertEqual(ai.observed_attributes(0x10000010, public), 16)
        # All public bits, including ones unknown to this test, stay significant.
        for attributes in (17, 18, 0x10000010, 0x80000010):
            changed = dict(win32_path_attributes=attributes, win32_handle_attributes=attributes)
            self.assertEqual(ai.observed_attributes(16, changed), attributes)
            self.assertNotEqual(ai.observed_attributes(16, changed), 16)
        with self.assertRaisesRegex(AssertionError, 'APIs disagree'):
            ai.observed_attributes(16, dict(win32_path_attributes=16, win32_handle_attributes=17))
        with self.assertRaisesRegex(AssertionError, 'reparse'):
            ai.observed_attributes(16, dict(win32_path_attributes=0x410, win32_handle_attributes=0x410))

    def test_field_diagnostic_keeps_attribute_identity_acl_and_mtime_changes(self):
        before = {'target': [dict(path='.', attributes=16, identity=1, permissions='acl', modified_ns=1)]}
        for field, value in [('attributes',17),('identity',2),('permissions','changed'),('modified_ns',2)]:
            after = {'target': [dict(before['target'][0], **{field:value})]}
            differences = ai.snapshot_differences(before, after)
            self.assertEqual([d['field'] for d in differences], [field])
        self.assertEqual(ai.snapshot_differences(before, {'target': []})[0]['field'], 'entry')

    def test_observation_failure_retains_partial_raw_evidence(self):
        guard = ai.ProtectedTrees.__new__(ai.ProtectedTrees)
        guard.roots = {'target': self.root}
        guard.output, guard.name = self.root, 'owned'
        def fail(root, raw_observations):
            raw_observations.append(dict(path='.', raw_lstat_attributes=16,
                                         win32_path_attributes=16, win32_handle_attributes=17))
            raise AssertionError('Public Windows attribute APIs disagree')
        with patch.object(ai, 'tree_snapshot', side_effect=fail):
            with self.assertRaisesRegex(AssertionError, 'APIs disagree'):
                guard.snapshot('after-picker')
        receipt = json.loads((self.root / 'owned-after-picker-observation-failure.json').read_text())
        self.assertEqual(receipt['raw_observations']['target'][0]['win32_handle_attributes'], 17)
        self.assertIn('APIs disagree', receipt['error'])

    def test_unavailable_ui_keeps_human_message_and_explicit_developer_code(self):
        ordinary = 'מודל הזיהוי הדרוש אינו זמין. הבדיקה לא הופעלה.'
        advanced = ordinary + ' (RECOGNITION_UNAVAILABLE)'
        ai.assert_feature_off_message(ordinary, False)
        ai.assert_feature_off_message(advanced, True)
        for text, developer in ((advanced, False), (ordinary, True), ('RECOGNITION_UNAVAILABLE', False), ('Other failure', True)):
            with self.assertRaises(AssertionError):
                ai.assert_feature_off_message(text, developer)

    def test_explicit_feature_mode_rejects_unrecognized_values(self):
        self.assertEqual(ai.ai_mode({}), 'off')
        self.assertEqual(ai.ai_mode({'SINGLES_DESKTOP_AI_EXPECTED': 'on'}), 'on')
        for value in ('', 'true', 'ON', 'auto'):
            with self.assertRaises(AssertionError):
                ai.ai_mode({'SINGLES_DESKTOP_AI_EXPECTED': value})

    def test_snapshot_reads_fresh_metadata_and_includes_directories(self):
        root = self.root / 'source'
        root.mkdir()
        file = root / 'owned.wav'
        file.write_bytes(b'original')
        before = ai.tree_snapshot(root, permissions)
        self.assertEqual([row['path'] for row in before], ['.', 'owned.wav'])
        self.assertTrue(before[0]['directory'])
        self.assertEqual(before, ai.tree_snapshot(root, permissions))
        file.write_bytes(b'mutated!')
        after = ai.tree_snapshot(root, permissions)
        self.assertNotEqual(before[1]['sha256'], after[1]['sha256'])
        old = file.stat()
        os.utime(file, ns=(old.st_atime_ns, old.st_mtime_ns + 10000000))
        self.assertNotEqual(after, ai.tree_snapshot(root, permissions))
        self.assertNotEqual(ai.tree_snapshot(root, permissions),
                            ai.tree_snapshot(root, lambda _: 'different permissions'))

    def test_snapshot_detects_add_remove_and_renames(self):
        file = self.root / 'a'
        file.write_bytes(b'fixed')
        before = ai.tree_snapshot(self.root, permissions)
        file.rename(self.root / 'b')
        self.assertNotEqual(before, ai.tree_snapshot(self.root, permissions))
        (self.root / 'empty').mkdir()
        self.assertEqual(len(ai.tree_snapshot(self.root, permissions)), 3)
        (self.root / 'b').unlink()
        self.assertEqual(len(ai.tree_snapshot(self.root, permissions)), 2)

    def test_snapshot_rejects_links_before_following_them(self):
        target = self.root / 'target'
        target.mkdir()
        link = self.root / 'link'
        try:
            link.symlink_to(target, target_is_directory=True)
        except OSError as error:
            self.skipTest(f'Host does not permit test symlinks: {error}')
        with self.assertRaisesRegex(AssertionError, 'link/reparse'):
            ai.tree_snapshot(link, permissions)
        with self.assertRaisesRegex(AssertionError, 'link/reparse'):
            ai.tree_snapshot(self.root, permissions)

    def test_reparse_attribute_is_rejected_even_without_symlink_mode(self):
        info = Mock(st_mode=0o100644, st_file_attributes=0x400)
        with patch.object(Path, 'lstat', return_value=info):
            with self.assertRaisesRegex(AssertionError, 'link/reparse'):
                ai.tree_snapshot(self.root, permissions)

    def test_exact_manifest_rejects_extras_and_corruption(self):
        package = self.package()
        expected = ai.digest(package / 'SHA256SUMS.txt')
        self.assertEqual(ai.verify_package(package, permissions), expected)
        (package / 'extra').write_bytes(b'extra')
        with self.assertRaisesRegex(AssertionError, 'missing/extra/changed'):
            ai.verify_package(package, permissions)
        (package / 'extra').unlink()
        (package / 'models/original/tensors.f32').write_bytes(b'corrupt')
        with self.assertRaisesRegex(AssertionError, 'missing/extra/changed'):
            ai.verify_package(package, permissions)

    def test_manifest_rejects_case_duplicates_and_unsafe_names(self):
        package = self.package()
        manifest = package / 'SHA256SUMS.txt'
        valid = manifest.read_text(encoding='utf-8')
        for name in ('../escape', '/absolute', 'C:/absolute', 'a\\b', 'a//b', './a',
                     'SHA256SUMS.txt', 'PREVIEW.EXE'):
            manifest.write_text(valid + '0' * 64 + '  ' + name + '\n', encoding='utf-8')
            with self.assertRaisesRegex(AssertionError, 'Unsafe or duplicate'):
                ai.verify_package(package, permissions)
        for text in ('', '0' * 63 + '  file\n', '0' * 64 + ' file\n'):
            manifest.write_text(text, encoding='utf-8')
            with self.assertRaises(AssertionError):
                ai.verify_package(package, permissions)

    def test_manifest_requires_complete_models_and_empty_target(self):
        package = self.package()
        (package / 'models/original/tokenizer.json').unlink()
        self.manifest(package)
        with self.assertRaisesRegex(AssertionError, 'complete original models'):
            ai.verify_package(package, permissions)
        (package / 'models/original/tokenizer.json').write_bytes(b'restored')
        (package / 'examples/target/unexpected').write_bytes(b'do not overwrite')
        self.manifest(package)
        with self.assertRaisesRegex(AssertionError, 'target must be empty'):
            ai.verify_package(package, permissions)

    def test_negative_cases_only_mutate_fresh_owned_copy(self):
        package = self.package()
        before = ai.tree_snapshot(package, permissions)
        for damage in ('missing', 'corrupt'):
            for asset in ('tensors.f32', 'classifier-v14.weights.bin'):
                destination = self.root / f'{damage}-{asset}'
                copied = ai.stage_negative(package / 'preview.exe', destination, asset, damage)
                self.assertEqual(ai.digest(copied), ai.digest(package / 'preview.exe'))
                target = destination / 'models/original' / asset
                if damage == 'missing':
                    self.assertFalse(target.exists())
                else:
                    original = (package / 'models/original' / asset).read_bytes()
                    changed = target.read_bytes()
                    self.assertEqual(changed[1:], original[1:])
                    self.assertEqual(changed[0], original[0] ^ 1)
                with self.assertRaises(FileExistsError):
                    ai.stage_negative(package / 'preview.exe', destination, asset, damage)
                self.assertEqual(before, ai.tree_snapshot(package, permissions))

    def test_authored_fixtures_are_metadata_free_silent_pcm(self):
        source, target, cancellation = ai.make_fixtures(self.root)
        self.assertEqual({p.name for p in source.iterdir()}, {ai.AI_NAME, ai.ALIAS_NAME})
        self.assertEqual(list(target.iterdir()), [])
        self.assertEqual(len(list(cancellation.iterdir())), 128)
        for path in source.iterdir():
            with wave.open(str(path), 'rb') as audio:
                self.assertEqual((audio.getnchannels(), audio.getsampwidth(), audio.getframerate()), (1, 2, 8000))
                self.assertEqual(audio.readframes(800), b'\0\0' * 800)
            self.assertNotIn(b'LIST', path.read_bytes())

    def test_real_ipc_helper_never_replaces_invoke_and_requires_error_code(self):
        command = Mock(return_value={'ok': True, 'value': {'available': True}})
        driver = ai.DesktopAI(command, Mock(), Mock(), Mock(), Mock(), Mock(), Mock(), self.root)
        self.assertEqual(driver.ipc('get_model_status'), {'available': True})
        method, endpoint, body = command.call_args.args
        self.assertEqual((method, endpoint), ('POST', '/execute/async'))
        self.assertIn('window.__TAURI_INTERNALS__.invoke(name, args)', body['script'])
        self.assertNotIn('__TAURI_INTERNALS__.invoke =', body['script'])
        command.return_value = {'ok': False, 'application_error': {'code': 'RECOGNITION_UNAVAILABLE'}}
        driver.ipc('start_preview', {}, 'RECOGNITION_UNAVAILABLE')
        with self.assertRaises(AssertionError):
            driver.ipc('start_preview', {}, 'HOST_BUSY')

    def test_application_error_envelope_does_not_collide_with_webdriver_error(self):
        def transport(method, endpoint, body):
            # Model the existing real WebDriver transport guard. The application's
            # expected negative response must never occupy its reserved error key.
            self.assertIn('done({ok:false,application_error:', body['script'])
            response = {'ok': False, 'application_error': {'code': 'RECOGNITION_UNAVAILABLE'}}
            if response.get('error'):
                raise RuntimeError('WebDriver transport rejected application envelope')
            return response
        driver = ai.DesktopAI(transport, Mock(), Mock(), Mock(), Mock(), Mock(), Mock(), self.root)
        self.assertEqual(driver.ipc('start_preview', {}, 'RECOGNITION_UNAVAILABLE')['code'],
                         'RECOGNITION_UNAVAILABLE')
        with self.assertRaises(AssertionError):
            driver.ipc('start_preview', {}, 'UNKNOWN_JOB')
        with self.assertRaises(AssertionError):
            driver.ipc('start_preview')

    def test_positive_artist_assertion_requires_ai_only_destination_and_alias(self):
        rows = [dict(source=ai.AI_NAME, kind='Single', reason=ai.ARTIST,
                     destination=ai.ARTIST + ' / singles / ' + ai.AI_NAME),
                dict(source=ai.ALIAS_NAME, kind='Single', reason='אברהם פריד',
                     destination='אברהם פריד / singles')]
        ai.DesktopAI.assert_artist({'rows': rows}, True)
        with self.assertRaises(AssertionError):
            ai.DesktopAI.assert_artist({'rows': rows}, False)
        rows[0].update(reason='לא נבחר אמן', destination=None)
        ai.DesktopAI.assert_artist({'rows': rows}, False)
        with self.assertRaises(AssertionError):
            ai.DesktopAI.assert_artist({'rows': rows}, True)

    def test_required_acceptance_ids_are_exact_and_unique(self):
        self.assertEqual(len(ai.REQUIRED_CHECKS), 9)
        self.assertEqual(len(set(ai.REQUIRED_CHECKS)), 9)
        self.assertIn('protected-files-unchanged', ai.REQUIRED_CHECKS)


if __name__ == '__main__':
    unittest.main()
