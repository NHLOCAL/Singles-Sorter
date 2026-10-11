"""Actual WebView viewport assertions. DPR is measured, never inferred from outer pixels."""
import json
import time
from pathlib import Path

METRICS = (Path(__file__).with_name('layout_metrics.js')).read_text(encoding='utf-8')

def assert_layout(metrics, require_bounded=True):
    if require_bounded:
        assert not metrics['reflow'], metrics
    assert metrics['pageWidth'] <= metrics['innerWidth'] + 1, metrics
    if not metrics['reflow']:
        assert metrics['pageHeight'] <= metrics['innerHeight'] + 1, metrics
        assert metrics['resultsOverflow'] == 'auto', metrics
        for name, rect in metrics['controls'].items():
            assert 0 <= rect['x'] and rect['right'] <= metrics['innerWidth'] + 1, (name, rect)
            assert 0 <= rect['y'] and rect['bottom'] <= metrics['innerHeight'] + 1, (name, rect)

def check_native_layout(command, script, capture, output, label):
    original = command('GET', '/window/rect')
    receipts = []
    try:
        for width, height in [(1000, 650), (1024, 768)]:
            # WebDriver resizes outer native bounds; correct using measured CSS inner dimensions.
            for _ in range(3):
                metrics = script(METRICS)
                outer = command('GET', '/window/rect')
                command('POST', '/window/rect', {'width':round(outer['width'] + width - metrics['innerWidth']), 'height':round(outer['height'] + height - metrics['innerHeight'])})
                time.sleep(.15)
            metrics = script(METRICS)
            assert abs(metrics['innerWidth'] - width) <= 1 and abs(metrics['innerHeight'] - height) <= 1, metrics
            assert_layout(metrics)
            metrics['label'] = label
            receipts.append(metrics)
            capture(f'{label}-{width}x{height}')
    finally:
        command('POST', '/window/rect', original)
        (output / f'{label}-layout-metrics.json').write_text(json.dumps({'kind':'actual_windows_webview','system_scaling':'measured DPR only; 125/150% system DPI not claimed unless DPR confirms it','measurements':receipts}, ensure_ascii=False, indent=2), encoding='utf-8')
