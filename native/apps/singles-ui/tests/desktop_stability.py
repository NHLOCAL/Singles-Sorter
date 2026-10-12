"""Real native preview and viewport acceptance on disposable files only.

The notification/long-label stress cases change presentation nodes only. Native
IPC is untouched, no execution is invoked, and protected file evidence is checked.
"""
import json
import time
from pathlib import Path
from desktop_ai import ProtectedTrees, write_wave
from desktop_polish import set_developer_mode

ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'
METRICS = """
return (()=>{
 const box=e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom,right:r.right}};
 const main=document.querySelector('main'),body=document.querySelector('#results-body');
 return {width:innerWidth,height:innerHeight,pageWidth:document.documentElement.scrollWidth,pageHeight:document.documentElement.scrollHeight,
 footer:box(document.querySelector('footer')),header:box(document.querySelector('header')),main:box(main),results:box(document.querySelector('#results')),
 mainScroll:main.scrollLeft,resultScroll:body.scrollLeft,resultWidth:body.clientWidth,resultExtent:body.scrollWidth,
 footerControls:[...document.querySelectorAll('footer button')].map(box)};
})();
"""


def assert_shell(metrics, baseline=None):
    assert metrics['pageWidth'] <= metrics['width'] + 1, metrics
    assert metrics['pageHeight'] <= metrics['height'] + 1, metrics
    assert abs(metrics['footer']['bottom'] - metrics['height']) <= 1, metrics
    assert metrics['main']['bottom'] <= metrics['footer']['y'] + 1, metrics
    if baseline:
        assert abs(metrics['footer']['y'] - baseline['footer']['y']) <= 1, (metrics, baseline)
        assert abs(metrics['footer']['height'] - baseline['footer']['height']) <= 1, (metrics, baseline)
    for control in metrics['footerControls']:
        assert 0 <= control['x'] and control['right'] <= metrics['width'] + 1, control
        assert metrics['footer']['y'] <= control['y'] and control['bottom'] <= metrics['height'] + 1, control


def exercise_shell(command, script, capture, output, label, results=False):
    original = command('GET', '/window/rect')
    saved = script("return {font:document.documentElement.style.fontSize,connection:{text:document.querySelector('#connection').textContent,hidden:document.querySelector('#connection').hidden},error:{text:document.querySelector('#error').textContent,hidden:document.querySelector('#error').hidden}}")
    receipts = []
    try:
        for width, height, font in [(1000, 650, ''), (360, 800, ''), (390, 800, ''), (1000, 650, '200%')]:
            script('document.documentElement.style.fontSize=' + json.dumps(font) + ';window.dispatchEvent(new Event("resize"))')
            for _ in range(3):
                current = script(METRICS); outer = command('GET', '/window/rect')
                command('POST', '/window/rect', {'width':round(outer['width'] + width-current['width']), 'height':round(outer['height'] + height-current['height'])})
                time.sleep(.1)
            baseline = None
            for name, notices in [('none', ['', '']), ('wrapped-warning', ['יש לבדוק את הפריטים. ' * 16, '']), ('multiple', ['הבדיקה נעצרה. ' * 8, 'הפעולה לא הושלמה. ' * 8]), ('mixed-path-error', ['', 'לא ניתן לקרוא: C:\\Music\\' + 'LongPathשירים' * 60])]:
                script('const values=' + json.dumps(notices, ensure_ascii=False) + ';["connection","error"].forEach((id,index)=>{const e=document.getElementById(id);e.textContent=values[index];e.hidden=!values[index]});')
                time.sleep(.1)
                measured = script(METRICS)
                measured.update(case=name, font=font, requested_width=width)
                receipts.append(measured)
                if baseline is None: baseline = measured
                capture(f'{label}-{width}-{font or "normal"}-{name}')
                assert_shell(measured, baseline)
            # Restore notices before pointer gestures so the result rows are reachable.
            script("['connection','error'].forEach(id=>document.getElementById(id).hidden=true)")
            if results:
                script("document.querySelector('#rows tr').scrollIntoView({block:'center'});getSelection().removeAllRanges()")
                body = command('POST', '/element', {'using':'css selector','value':'#rows tr'})[ELEMENT]
                before = script(METRICS)
                for offset in [-70, 70]:
                    command('POST', '/actions', {'actions':[{'type':'pointer','id':'results-mouse','parameters':{'pointerType':'mouse'},'actions':[
                        {'type':'pointerMove','duration':0,'origin':{ELEMENT:body},'x':0,'y':0},
                        {'type':'pointerDown','button':0},
                        {'type':'pointerMove','duration':150,'origin':'pointer','x':offset,'y':0},
                        {'type':'pointerUp','button':0}]}]})
                    command('DELETE', '/actions')
                    measured = script(METRICS); assert_shell(measured)
                    assert abs(measured['results']['width']-before['results']['width']) <= 1, measured
                    assert measured['resultScroll'] == 0 and measured['mainScroll'] == 0, measured
                # Prove there is no hidden horizontal scroll range either direction.
                for amount in [-10000, 10000]:
                    script(f"document.querySelector('#results-body').scrollLeft={amount};document.querySelector('main').scrollLeft={amount};window.scrollTo({amount},0)")
                    measured = script(METRICS)
                    assert measured['resultScroll'] == 0 and measured['mainScroll'] == 0, measured
                    assert measured['resultExtent'] <= measured['resultWidth'] + 1, measured
                vertical = script("const m=document.querySelector('main'),r=document.querySelector('#results-body');const e=m.scrollHeight>m.clientHeight+1?m:r;e.scrollTop=10000;const moved=e.scrollTop>0;e.scrollTop=0;return moved")
                assert vertical, 'Long real result lists must retain vertical scrolling'
    finally:
        script('const saved=' + json.dumps(saved, ensure_ascii=False) + ';document.documentElement.style.fontSize=saved.font;["connection","error"].forEach(id=>{document.getElementById(id).textContent=saved[id].text;document.getElementById(id).hidden=saved[id].hidden});getSelection().removeAllRanges();window.dispatchEvent(new Event("resize"))')
        command('POST', '/window/rect', original)
        (Path(output) / f'{label}-stability.json').write_text(json.dumps({'kind':'actual_native_presentation_stress','ipc_mocked':False,'measurements':receipts},ensure_ascii=False,indent=2),encoding='utf-8')


def exercise_result_stability(command, script, click, fill, wait, capture, root, output):
    root = Path(root); root.mkdir()
    source, target = root/'source', root/'target'; source.mkdir(); target.mkdir()
    suffix = 'LongLatinName' * 6 + 'שירעבריארוך' * 3
    names = [f'אברהם פריד - {index:02} {suffix}.wav' for index in range(52)]
    for name in names + ['unrecognizedfixture.wav']: write_wave(source/name)
    guard = ProtectedTrees({'source':source,'target':target},output,'stability-fixtures')
    if script("return document.querySelector('#app').dataset.view==='results'"):
        click('#back-setup'); wait(lambda: script("return document.querySelector('#app').dataset.view==='setup'"))
    fill('#source',str(source)); fill('#target',str(target))
    for key, value in dict(copy_mode=True,main_folder_only=False,singles_folder=True,abc_sort=False,exist_only=False).items():
        if script(f"return document.querySelector('#{key}').checked") != value: click('#'+key)
    click('#open-settings');click('#duet-first');wait(lambda: script("return !document.querySelector('#duet-first').disabled"))
    script("document.querySelector('#ai-profile').value='Disabled';document.querySelector('#ai-profile').dispatchEvent(new Event('change',{bubbles:true}))")
    click('#advanced-dialog .close-dialog');set_developer_mode(script,click,wait,False)
    click('#sort');wait(lambda: script("return document.querySelector('#app').dataset.view==='results' && !document.querySelector('#show-inactive').disabled"),timeout=120)
    assert not script("return document.querySelector('#show-inactive').checked")
    assert script("return document.querySelectorAll('#rows tr').length") == 50
    assert '52' in script("return document.querySelector('#count').textContent")
    assert 'unrecognizedfixture' not in script("return document.querySelector('#rows').textContent")
    assert '—' not in script("return document.querySelector('#rows').textContent")
    click('#next');wait(lambda: script("return !document.querySelector('#next').disabled || document.querySelector('#page-label').textContent.includes('51')"))
    wait(lambda: script("return document.querySelectorAll('#rows tr').length===2"))
    click('#show-inactive');wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='true' && !document.querySelector('#show-inactive').disabled"))
    assert '53' in script("return document.querySelector('#count').textContent")
    click('#next');wait(lambda: script("return document.querySelectorAll('#rows tr').length===3 && !document.querySelector('#show-inactive').disabled"))
    click('#show-inactive');wait(lambda: script("return document.querySelector('#app').dataset.showInactive==='false' && !document.querySelector('#show-inactive').disabled"))
    assert script("return document.querySelectorAll('#rows tr').length") == 50
    # Stable native snapshot paging must neither omit nor duplicate matching rows.
    visible = script("return [...document.querySelectorAll('#rows .source-name')].map(e=>e.textContent)")
    click('#next');wait(lambda: script("return document.querySelectorAll('#rows tr').length===2"))
    visible += script("return [...document.querySelectorAll('#rows .source-name')].map(e=>e.textContent)")
    assert set(visible) == set(names) and len(visible) == len(set(visible))
    click('#previous');wait(lambda: script("return document.querySelectorAll('#rows tr').length===50"))
    # The full long filename is accessible independently of visual ellipsis.
    assert script("return [...document.querySelectorAll('#rows .source-name')].every(e=>e.title===e.textContent && e.title.length>100)")
    exercise_shell(command,script,capture,output,'native-results-stable',results=True)
    guard.verify('after-native-preview-and-layout')
    return ['Real 52-file action list filters one no-action file before pagination; explicit option restores it',
            'Real long RTL/Latin filename list resists horizontal drag/pan and retains vertical scrolling',
            'Footer remains flush with viewport through wrapped warnings, multiple alerts, narrow widths and 200% text']
