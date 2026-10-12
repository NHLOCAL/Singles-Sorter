"""Portable telemetry semantics, not proof of actual Windows memory counters."""
import os
import subprocess
from pathlib import Path
import sys
import tempfile
import zipfile
import unittest
from unittest.mock import patch
import windows_runtime_metrics as metrics


class RuntimeMetricsTests(unittest.TestCase):
    def test_package_measurement_binds_exact_archive_and_unpacked_bytes(self):
        import check_windows_v14 as gate
        import check_execution
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);package=root/'package';package.mkdir()
            binary=package/'singles-sorter.exe';binary.write_bytes(b'owned executable')
            (package/'model.bin').write_bytes(b'owned model')
            archive=root/'owned.zip'
            with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED) as zipped:
                for path in package.iterdir(): zipped.write(path,path.name)
            with patch.object(check_execution.package,'verify_package'):
                value=gate.package_measurement(binary,package,archive)
                self.assertEqual(value['unpacked_file_bytes'],len(b'owned executableowned model'))
                self.assertEqual(value['compressed_bytes'],archive.stat().st_size)
                (package/'model.bin').write_bytes(b'changed model')
                with self.assertRaises(AssertionError): gate.package_measurement(binary,package,archive)
        self.assertEqual(gate.package_measurement(Path('unused'),None,None)['status'],'unavailable')

    def test_high_water_is_distinct_from_sampled_working_set(self):
        values = iter([{'working_set_bytes':20,'process_lifetime_peak_working_set_bytes':100},
                       {'working_set_bytes':30,'process_lifetime_peak_working_set_bytes':100}])
        sampler=metrics.Sampler(lambda:next(values))
        sampler.sample(); result=sampler.finish()
        self.assertEqual(result['samples'],2)
        self.assertEqual(result['sampled_peak_working_set_bytes'],30)
        self.assertEqual(result['process_lifetime_peak_working_set_bytes'],100)

    def test_failed_query_never_becomes_zero_memory(self):
        def refused(): raise OSError('query failed')
        result=metrics.Sampler(refused).finish()
        self.assertEqual(result['status'],'unavailable')
        self.assertIsNone(result['process_lifetime_peak_working_set_bytes'])
        self.assertEqual(result['errors'],['query failed'])

    def test_measured_process_retains_real_output_and_failure(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(metrics,'start_sampler',return_value=(None,{'status':'unavailable'})):
            result, observation=metrics.measured_process([sys.executable,'-c','import sys;print("owned");sys.exit(3)'],cwd=directory,env=dict(os.environ))
        self.assertEqual(result.returncode,3);self.assertEqual(result.stdout.strip(),b'owned')
        self.assertGreater(observation['elapsed_ns'],0)
        self.assertEqual(observation['memory']['status'],'unavailable')

    def test_payload_and_closed_stdin_remain_exact(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(metrics,'start_sampler',return_value=(None,{'status':'unavailable'})):
            command=[sys.executable,'-c','import sys;sys.stdout.buffer.write(sys.stdin.buffer.read())']
            for payload in (None,b'owned payload'):
                result,_=metrics.measured_process(command,cwd=directory,env=dict(os.environ),payload=payload)
                self.assertEqual(result.stdout,payload or b'')

    def test_cold_worker_refuses_optimized_execution_before_paths(self):
        worker=Path(__file__).with_name('cold_original.py')
        result=subprocess.run([sys.executable,'-O',str(worker),'unused','unused'],capture_output=True,text=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('owned Windows CI runner and assertions',result.stderr)

    def test_shared_measurement_keeps_return_value_and_runtime_label(self):
        with patch.object(metrics,'start_sampler',return_value=(None,{'status':'unavailable'})):
            result,observation=metrics.measured_call(lambda:{'original':'value'},runtime='shared warm harness')
        self.assertEqual(result,{'original':'value'})
        self.assertEqual(observation['runtime'],'shared warm harness')
        self.assertGreaterEqual(observation['elapsed_ns'],0)


if __name__ == '__main__': unittest.main()
