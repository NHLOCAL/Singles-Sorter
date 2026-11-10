import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('android_elf', Path(__file__).with_name('check-android-elf.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class AndroidElfTests(unittest.TestCase):
    def test_dynamic_bionic_both_abis(self):
        for abi, machine in [('x86_64', 'Advanced Micro Devices X86-64'), ('arm64-v8a', 'AArch64')]:
            with self.subTest(abi=abi):
                result = m.inspect('ELF64 ' + machine, '(NEEDED) Shared library: [libc.so]',
                                   '1: 00000000 0 FUNC GLOBAL DEFAULT UND getauxval@LIBC',
                                   'LOAD 0 0 0 0 0 R E 0x4000', abi)
                self.assertTrue(result['dynamic_bionic'])

    def test_rejects_observed_zero_needed_failure(self):
        with self.assertRaisesRegex(ValueError, 'libc.so'):
            m.inspect('ELF64 AArch64', '(INIT_ARRAY) 0x100', '', 'LOAD 0 0x4000', 'arm64-v8a')

    def test_rejects_embedded_bionic_even_with_dynamic_dependency(self):
        for symbol in ['getauxval', '__cxa_atexit', '__libc_globals', '__libc_init']:
            with self.subTest(symbol=symbol), self.assertRaisesRegex(ValueError, 'embedded Bionic'):
                m.inspect('ELF64 AArch64', '(NEEDED) [libc.so]',
                          '9847: 00000000002f3300 666 FUNC LOCAL DEFAULT 11 ' + symbol,
                          'LOAD 0 0x4000', 'arm64-v8a')

    def test_rejects_wrong_abi_dependency_or_alignment(self):
        for header, needed, loads in [('ELF32 AArch64', '(NEEDED) [libc.so]', 'LOAD 0 0x4000'),
                                     ('ELF64 AArch64', '(NEEDED) [libc.so]\n(NEEDED) [libc++_shared.so]', 'LOAD 0 0x4000'),
                                     ('ELF64 AArch64', '(NEEDED) [libc.so]', 'LOAD 0 0x1000')]:
            with self.assertRaises(ValueError): m.inspect(header, needed, '', loads, 'arm64-v8a')


if __name__ == '__main__':
    unittest.main()
