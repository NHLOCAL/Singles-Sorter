"""CI structure checks; GitHub/Windows executes the actual host tests."""
from pathlib import Path
import unittest

WORKFLOW = Path(__file__).resolve().parents[4] / '.github/workflows/native-ui-windows.yml'


def steps():
    source = WORKFLOW.read_text(encoding='utf-8')
    return {block.split('\n', 1)[0]: block.split('\n', 1)[1]
            for block in source.split('      - name: ')[1:]}


class IndependentHostGate(unittest.TestCase):
    def test_host_gate_requires_successful_build_but_not_renderer_success(self):
        blocks = steps()
        build = blocks['Build actual desktop application through pinned Tauri CLI']
        host = blocks['Run selected host contracts independently of renderer acceptance']
        self.assertIn('id: default-desktop-build', build)
        self.assertNotIn('if:', build)
        self.assertIn("if: ${{ !cancelled() && steps.default-desktop-build.outcome == 'success' }}", host)
        self.assertNotIn('continue-on-error', host)
        self.assertNotIn('npm run build:desktop', host)

    def test_existing_exact_host_suites_are_moved_without_duplicate_runs(self):
        source = WORKFLOW.read_text(encoding='utf-8')
        blocks = steps()
        host = blocks['Run selected host contracts independently of renderer acceptance']
        full = 'cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features original-ai,selected-windows-execution'
        self.assertEqual(source.count(full + '\n'), 1)
        self.assertIn(full + '\n', host)
        for variable in ['restart', 'albumRestart', 'wholeRestart', 'fixRestart']:
            self.assertIn(f'${variable}Listing = ' + full + f' --lib ${variable}Case -- --exact --list', host)
            self.assertIn(full + f' --lib ${variable}Case -- --exact --test-threads=1', host)
        self.assertIn('$env:ORIGINAL_AI_BUNDLE = (Resolve-Path ../../target/desktop-original/bundle).Path', host)
        self.assertIn(full + ' --release --lib -- --ignored', host)
        self.assertEqual(host.count('if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'), 10)

    def test_product_build_and_package_gates_remain_strict(self):
        blocks = steps()
        selected = blocks['Build original-AI and selected-execution desktop through pinned Tauri CLI']
        self.assertIn("if: ${{ success() && steps.selected-host-contracts.outcome == 'success' }}", selected)
        self.assertEqual(selected.count('npm run build:desktop -- --ci --features original-ai,selected-windows-execution -- --locked'), 1)
        self.assertNotIn('cargo test', selected)
        for name in ['Stage only reviewed desktop runtime, exact models and full notices',
                     'Exercise staged original-AI desktop and actual selected Copy Move and FixNames',
                     'Archive and extract only after original-AI and actual apply acceptance',
                     'Exercise extracted original-AI desktop and actual selected Copy Move and FixNames',
                     'Retain verified original-AI selected-execution desktop package']:
            self.assertNotIn('if:', blocks[name])
            self.assertNotIn('continue-on-error', blocks[name])
        self.assertIn('--featureoff-report test-results/native-windows/feature-off/acceptance.json', blocks['Stage only reviewed desktop runtime, exact models and full notices'])


if __name__ == '__main__':
    unittest.main()
