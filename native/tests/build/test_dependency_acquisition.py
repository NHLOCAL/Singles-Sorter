"""Owned, tiny dependency acquisition contracts. No package build or network."""
import hashlib
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

BRIDGE = Path(__file__).resolve().parents[2] / 'crates/sorter-io/bridge'
CMAKE = os.environ.get('CMAKE', 'cmake')

class AcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.cache = self.root / 'cache with spaces'
        self.cache.mkdir()
        src = self.root / 'payload'; src.mkdir()
        (src / 'hello.txt').write_bytes(b'owned fixture\n')
        self.archive = self.cache / 'owned-1.tar.gz'
        with tarfile.open(self.archive, 'w:gz') as t:
            t.add(src, arcname='owned-1')
        self.sha = hashlib.sha256(self.archive.read_bytes()).hexdigest()
        self.project = self.root / 'project'; self.project.mkdir()
        (self.project / 'CMakeLists.txt').write_text('''cmake_minimum_required(VERSION 3.24)
project(acquisition_contract NONE)
include("''' + (BRIDGE / 'cmake/AcquireDependency.cmake').as_posix() + '''")
sorter_acquire_dependency(owned "1" "https://invalid.invalid/owned-1.tar.gz" "${EXPECTED_HASH}" result)
file(READ "${result}/hello.txt" contents)
if(NOT contents STREQUAL "owned fixture\\n")
  message(FATAL_ERROR "payload mismatch")
endif()
''')
    def tearDown(self):
        self.tmp.cleanup()
    def configure(self, *args, build='build', env=None):
        return subprocess.run([CMAKE, '-S', str(self.project), '-B', str(self.root/build),
            '-DSORTER_DEPENDENCY_CACHE='+str(self.cache), '-DSORTER_DEPENDENCIES_OFFLINE=ON',
            '-DEXPECTED_HASH='+self.sha, *args], text=True, encoding='utf-8', errors='replace',
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env)
    def test_verified_offline_archive_cold_and_warm_builds(self):
        for build in ['build', 'build', 'another-build']:
            p=self.configure(build=build); self.assertEqual(p.returncode,0,p.stdout)
        self.assertEqual(hashlib.sha256(self.archive.read_bytes()).hexdigest(),self.sha)
    def test_online_mode_reuses_verified_cache_without_reaching_invalid_host(self):
        p = self.configure('-DSORTER_DEPENDENCIES_OFFLINE=OFF')
        self.assertEqual(p.returncode, 0, p.stdout)
    def test_native_unicode_cache_path_with_spaces(self):
        native_cache = self.root / 'cache with spaces מטמון'
        self.cache.rename(native_cache)
        self.cache = native_cache
        self.archive = self.cache / 'owned-1.tar.gz'
        # Deliberately use native str(Path), including backslashes on Windows.
        p = self.configure()
        self.assertEqual(p.returncode, 0, p.stdout)
        self.assertEqual(hashlib.sha256(self.archive.read_bytes()).hexdigest(), self.sha)
    def test_environment_native_cache_path(self):
        env = dict(os.environ, SORTER_DEPENDENCY_CACHE=str(self.cache))
        p = self.configure('-DSORTER_DEPENDENCY_CACHE=', env=env)
        self.assertEqual(p.returncode, 0, p.stdout)
    def test_missing_offline_archive_is_actionable(self):
        self.archive.unlink(); p=self.configure()
        self.assertNotEqual(p.returncode,0); self.assertIn('offline cache is missing',p.stdout)
    def test_corrupt_cache_fails_without_network_or_overwrite(self):
        self.archive.write_bytes(b'broken'); p=self.configure()
        self.assertNotEqual(p.returncode,0); self.assertIn('SHA256 mismatch',p.stdout)
        self.assertEqual(self.archive.read_bytes(),b'broken')
    def test_unverified_source_override_rejected(self):
        p=self.configure('-DFETCHCONTENT_SOURCE_DIR_SORTER_OWNED='+str(self.root/'payload'))
        self.assertNotEqual(p.returncode,0); self.assertIn('source override is unsupported',p.stdout)
    def test_generic_package_search_cannot_substitute_installed_library(self):
        package = self.root / 'installed'; package.mkdir()
        (package / 'sorter_ownedConfig.cmake').write_text('set(sorter_owned_FOUND TRUE)\n')
        p = self.configure('-DFETCHCONTENT_TRY_FIND_PACKAGE_MODE=ALWAYS',
                           '-DCMAKE_PREFIX_PATH=' + str(package))
        self.assertEqual(p.returncode, 0, p.stdout)
    def test_fully_disconnected_flag_does_not_skip_initial_verified_population(self):
        p=self.configure('-DFETCHCONTENT_FULLY_DISCONNECTED=ON')
        self.assertEqual(p.returncode,0,p.stdout)

if __name__=='__main__': unittest.main()
