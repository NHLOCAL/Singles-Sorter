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
        if metrics['view'] == 'results':
            assert metrics['resultsVisible'] and not metrics['setupVisible'], metrics
            assert metrics['resultsOverflow'] == 'auto', metrics
            assert metrics['resultsBody']['height'] >= metrics['innerHeight'] * .5, metrics
            assert all(height <= 48 for height in metrics['rowHeights']), metrics
        else:
            assert metrics['setupVisible'] and not metrics['resultsVisible'], metrics
        for name, rect in metrics['controls'].items():
            assert rect['height'] > 0, (name, rect)
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


def capture_platform_fonts(command, output):
    """Best-effort actual glyph-font proof; unsupported CDP is recorded, never treated as proof."""
    receipt = {'status':'unavailable', 'fonts':{}, 'errors':[]}
    for endpoint in ['/ms/cdp/execute', '/goog/cdp/execute']:
        try:
            def cdp(method, params=None):
                return command('POST', endpoint, {'cmd':method, 'params':params or {}})
            cdp('DOM.enable'); cdp('CSS.enable')
            document = cdp('DOM.getDocument')
            for selector in ['h1','#copy_mode-label','#folders-heading']:
                node = cdp('DOM.querySelector', {'nodeId':document['root']['nodeId'], 'selector':selector})
                receipt['fonts'][selector] = cdp('CSS.getPlatformFontsForNode', {'nodeId':node['nodeId']})
            complete = all(any(font.get('glyphCount', 0) > 0 for font in result.get('fonts', [])) for result in receipt['fonts'].values())
            receipt['status'] = 'actual_platform_fonts' if complete and len(receipt['fonts']) == 3 else 'incomplete_platform_fonts'
            receipt['interpretation'] = 'Raw nonempty glyph evidence only; review actual family names before accepting Segoe UI Hebrew rendering.'
            receipt['endpoint'] = endpoint
            break
        except Exception as error:
            receipt['errors'].append(str(error))
    (output / 'actual-platform-fonts.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2),encoding='utf-8')


def check_phone_layout(command, script, capture, output, label):
    """Narrow actual Windows WebView rendering only; this is not native Android proof."""
    original=command('GET','/window/rect')
    receipts=[]
    try:
        for width in [360,390]:
            for _ in range(3):
                metrics=script(METRICS);outer=command('GET','/window/rect')
                command('POST','/window/rect',{'width':round(outer['width']+width-metrics['innerWidth']),'height':round(outer['height']+800-metrics['innerHeight'])})
                time.sleep(.15)
            metrics=script(METRICS)
            assert abs(metrics['innerWidth']-width)<=1,metrics
            assert_layout(metrics,require_bounded=False)
            for dialog in metrics['dialogs']:
                bounds=dialog['rect']
                assert dialog['scrollWidth'] <= dialog['clientWidth']+1,dialog
                assert bounds['x'] >= 0 and bounds['right'] <= metrics['innerWidth']+1,dialog
                assert bounds['y'] >= 0 and bounds['bottom'] <= metrics['innerHeight']+1,dialog
                for control in dialog['controls']:
                    box=control['rect']
                    assert box['x'] >= bounds['x']-1 and box['right'] <= bounds['right']+1,control
                    assert box['bottom']-bounds['y']+dialog['scrollTop'] <= dialog['scrollHeight']+2,control
                    if control['tag'] in ['BUTTON','SELECT'] or control['tag']=='INPUT' and control['type'] not in ['checkbox','radio']:
                        assert box['height'] >= 48,control
            receipts.append(metrics);capture(f'{label}-{width}px')
    finally:
        command('POST','/window/rect',original)
        (output/f'{label}-phone-css-metrics.json').write_text(json.dumps({'kind':'narrow_windows_webview_not_native_android','measurements':receipts},ensure_ascii=False,indent=2),encoding='utf-8')
