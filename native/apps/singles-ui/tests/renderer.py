"""Renderer-only Playwright acceptance. IPC is explicitly mocked; no native claim.
Run after npm run build. Requires the standard Python Playwright package and an
installed browser. Set SINGLES_CHROMIUM to use a system Chromium executable.
"""
import functools
import http.server
import json
import os
from pathlib import Path
import threading
from playwright.sync_api import sync_playwright, expect
from desktop_layout import METRICS, assert_layout

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = Path(os.environ.get('SINGLES_UI_EVIDENCE', str(ROOT / 'test-results')))
OUTPUT.mkdir(parents=True, exist_ok=True)
REAL_PAGE = json.loads((ROOT / 'src-tauri/tests/fixtures/real-preview-page.json').read_text(encoding='utf-8'))
MOCK = r"""
window.__calls=[];
window.__pickerCancel=false;
window.__storedSettings={copy_mode:false,main_folder_only:false,singles_folder:true,exist_only:false,abc_sort:false,duet_mode:false};
window.__personalList={revision:'r1',rows:[['אמן ראשון','אמן ראשון']],diagnostics:[]};
window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
 window.__calls.push({command,args});
 const settings={copy_mode:false,main_folder_only:false,singles_folder:true,exist_only:false,abc_sort:false,duet_mode:false};
 if(command==='get_settings')return {...window.__storedSettings};
 if(command==='save_main_settings'){window.__storedSettings={...window.__storedSettings,...args.settings};return {...window.__storedSettings};}
 if(command==='set_duet'){window.__storedSettings.duet_mode=args.enabled;return {...window.__storedSettings};}
 if(command==='get_first_run_status'||command==='mark_first_run_seen')return {version:'0.1.0',seen:false};
 if(command==='get_personal_list')return structuredClone(window.__personalList);
 if(command==='save_personal_list'){window.__personalList={revision:'r2',rows:args.rows,diagnostics:[]};return structuredClone(window.__personalList);}
 if(command==='export_personal_list')return null;
 if(command==='select_personal_import')return {token:'i1',filename:'sample.csv',encoding_required:true,preview:null};
 if(command==='preview_personal_import')return {token:'i1',encoding:args.encoding,rows:[['זמר נוסף','תיקייה']],diagnostics:[]};
 if(command==='confirm_personal_import'){window.__personalList.rows.push(['זמר נוסף','תיקייה']);window.__personalList.revision='r3';return structuredClone(window.__personalList);}
 if(command==='open_footer_link')return null;
 if(command==='get_model_status')return {available:false,message:'בדיקת ממשק בלבד עם IPC מדומה. לא בוצעה הסקה.',platform:'linux_test'};
 if(command==='select_root')return window.__pickerCancel?null:{token:args.role,display_name:args.role==='Source'?'C:\\Music\\שירים':'D:\\מוזיקה',capabilities:{read:true,list:true},platform:'linux_test'};
 if(command==='resolve_typed_root'){if(args.text==='bad')throw 'INVALID_ROOT';return {token:args.role,display_name:args.text,capabilities:{read:true,list:true},platform:'linux_test'};}
 if(command==='start_preview'){
  if(args.request.ai_profile==='Configured')throw 'RECOGNITION_UNAVAILABLE';
  await new Promise(resolve=>setTimeout(resolve,80));return {job_id:'renderer-fixture'};
 }
 if(command==='job_snapshot')return {host_schema:1,job_id:'renderer-fixture',status:'completed',plan_id:'renderer-fixture-not-native',total_rows:2,blockers:['PHYSICAL_APPLY_UNAVAILABLE'],warnings:[]};
 if(command==='get_plan_page')return window.__realHostFixture;
 throw new Error('Unexpected mock command '+command);
}};
"""
class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *_):
        pass
server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(QuietHandler, directory=str(ROOT / 'dist')))
threading.Thread(target=server.serve_forever, daemon=True).start()
checks = []
try:
    with sync_playwright() as p:
        options = {'headless': True}
        if os.environ.get('SINGLES_CHROMIUM'):
            options['executable_path'] = os.environ['SINGLES_CHROMIUM']
        browser = p.chromium.launch(**options)
        page = browser.new_page(viewport={'width': 1000, 'height': 800})
        errors = []
        page.on('pageerror', lambda error: errors.append(str(error)))
        page.add_init_script('window.__realHostFixture=' + json.dumps(REAL_PAGE, ensure_ascii=False) + ';' + MOCK)
        page.goto(f'http://127.0.0.1:{server.server_port}/')
        expect(page.get_by_role('heading', name='מסדר הסינגלים', exact=True)).to_be_visible()
        expect(page.locator('#sort')).to_be_enabled()
        page.evaluate("document.title='Renderer test • mocked IPC'")
        expect(page.locator('dialog[open]')).to_have_count(0)
        measurements=[]
        for width,height in [(1000,650),(1024,768)]:
            page.set_viewport_size({'width':width,'height':height})
            page.wait_for_timeout(100)
            metrics=page.evaluate('() => {'+METRICS+'}')
            assert_layout(metrics)
            measurements.append(metrics)
            page.screenshot(path=str(OUTPUT / f'renderer-home-{width}x{height}.png'),full_page=True)
        badge=page.locator('[aria-label="עזרה: תיקית מקור"]')
        badge.hover()
        expect(page.locator('#context-help')).to_be_visible()
        page.locator('#context-help').hover()
        expect(page.locator('#context-help')).to_be_visible()
        page.keyboard.press('Escape')
        expect(page.locator('#context-help')).to_be_hidden()
        badge.focus()
        page.keyboard.press('Enter')
        expect(page.locator('#context-help')).to_be_visible()
        page.locator('h1').click()
        expect(page.locator('#context-help')).to_be_hidden()
        checks.append('Help hover, hoverable content, keyboard pin, Escape and outside dismissal')
        (OUTPUT / 'layout-metrics.json').write_text(json.dumps(measurements,ensure_ascii=False,indent=2),encoding='utf-8')
        page.screenshot(path=str(OUTPUT / 'renderer-mock-desktop.png'), full_page=True)
        checks.append('Desktop page identity and nonblank renderer')
        page.locator('#source-picker').click()
        expect(page.locator('#source')).to_have_value('C:\\Music\\שירים')
        page.evaluate('window.__pickerCancel=true')
        page.locator('#source-picker').click()
        expect(page.locator('#source')).to_have_value('C:\\Music\\שירים')
        checks.append('Cancelled picker preserves previous selection')
        page.locator('#sort').click()
        expect(page.locator('#error')).to_contain_text('יעד')
        page.locator('#target').fill('D:\\מוזיקה')
        page.locator('#singles_folder').uncheck()
        page.locator('#sort').click()
        expect(page.locator('#rows tr')).to_have_count(2)
        expect(page.locator('#rows img')).to_have_count(0)
        expect(page.locator('#rows')).to_contain_text('אברהם פריד - בדיקה.mp3')
        expect(page.locator('#rows')).to_contain_text('בדיקת אלבום')
        expect(page.locator('#rows')).to_contain_text('בדיקת שיר')
        assert page.evaluate("window.__calls.find(c=>c.command==='start_preview').args.request.settings.singles_folder") is False
        checks.append('Sort sends explicit false and renders actual serialized Rust host fixture in Hebrew')
        page.evaluate("window.__realHostFixture.rows[1].source='<img src=x onerror=alert(1)>.mp3'")
        page.locator('#sort').click()
        expect(page.locator('#rows')).to_contain_text('<img src=x onerror=alert(1)>.mp3')
        expect(page.locator('#rows img')).to_have_count(0)
        checks.append('Additional hostile filename remains literal text without HTML execution')
        page.evaluate("document.querySelector('#warnings').textContent='אזהרה ארוכה '.repeat(500); document.querySelector('#details').open=true")
        page.wait_for_timeout(100)
        assert_layout(page.evaluate('() => {'+METRICS+'}'))
        assert page.evaluate("document.querySelector('#results-body').scrollHeight > document.querySelector('#results-body').clientHeight")
        checks.append('Long warnings and expanded details stay in the single results scroller')
        page.screenshot(path=str(OUTPUT / 'renderer-mock-results.png'), full_page=True)
        page.locator('#advanced').click()
        expect(page.locator('#advanced-dialog')).to_be_visible()
        page.locator('#duet-all').check()
        expect(page.locator('#duet-all')).to_be_checked()
        assert page.evaluate('window.__storedSettings.duet_mode') is True
        assert page.evaluate('window.__storedSettings.singles_folder') is True
        page.locator('#edit-list').click()
        expect(page.locator('#list-dialog')).to_be_visible()
        page.locator('#list-alias-0').fill('אמן ראשון חדש')
        page.locator('#list-cancel').click()
        assert page.evaluate('window.__personalList.rows[0][0]') == 'אמן ראשון'
        page.locator('#edit-list').click()
        page.locator('#list-alias-0').fill('אמן ראשון חדש')
        page.locator('#list-save').click()
        expect(page.locator('#list-notice')).to_contain_text('הרשימה נשמרה')
        page.locator('#list-cancel').click()
        page.locator('#import-list').click()
        expect(page.locator('#confirm-import')).to_be_disabled()
        page.locator('#import-encoding').select_option('Windows1255')
        page.locator('#decode-import').click()
        expect(page.locator('#import-summary')).to_contain_text('1 שורות')
        page.locator('#confirm-import').click()
        expect(page.locator('#advanced-dialog')).to_be_visible()
        assert page.evaluate('window.__personalList.rows.length') == 2
        checks.append('Draft cancellation, explicit list save, encoding preview and confirmed append')
        page.locator('#ai-profile').select_option('Configured')
        page.keyboard.press('Escape')
        expect(page.locator('#advanced-dialog')).not_to_be_visible()
        expect(page.locator('#advanced')).to_be_focused()
        page.locator('#save-settings').click()
        expect(page.locator('#settings-notice')).to_contain_text('נשמרו')
        assert page.evaluate('window.__storedSettings.singles_folder') is False
        checks.append('Main settings save separately from immediate duet persistence')
        page.locator('#fix').click()
        expect(page.locator('#error')).to_contain_text('RECOGNITION_UNAVAILABLE')
        checks.append('Dialog escape/focus return, sixth setting, configured AI fail closed')
        page.locator('#menu').click()
        page.get_by_role('button', name='איך משתמשים?').click()
        expect(page.locator('#help-content')).to_contain_text('בחר תיקית מקור')
        page.keyboard.press('Escape')
        expect(page.locator('#menu')).to_be_focused()
        checks.append('Help menu keyboard dismissal and focus return')
        page.set_viewport_size({'width':1000,'height':650})
        page.evaluate("document.documentElement.style.fontSize='200%'")
        page.wait_for_timeout(100)
        large=page.evaluate('() => {'+METRICS+'}')
        assert large['reflow']
        assert_layout(large,require_bounded=False)
        for selector in ['#source','#target','#copy_mode','#exist_only','#advanced','#save-settings','#sort','#fix']:
            page.locator(selector).scroll_into_view_if_needed()
            expect(page.locator(selector)).to_be_in_viewport()
        page.evaluate("document.documentElement.style.fontSize=''")
        checks.append('200% desktop text intentionally reflows and retains every primary command')
        page.set_viewport_size({'width': 360, 'height': 800})
        page.screenshot(path=str(OUTPUT / 'renderer-mock-mobile.png'), full_page=True)
        assert page.evaluate('document.documentElement.scrollWidth <= window.innerWidth')
        expect(page.locator('#copy_mode')).to_be_visible()
        checks.append('360px layout retains basic controls without page horizontal overflow')
        page.evaluate("document.documentElement.style.fontSize='200%'")
        assert page.evaluate('document.documentElement.scrollWidth <= window.innerWidth')
        page.screenshot(path=str(OUTPUT / 'renderer-mock-mobile-large-text.png'), full_page=True)
        checks.append('200% base text no horizontal page overflow')
        assert not errors, errors
        checks.append('No JavaScript page errors')
        for scale in [1.25,1.5]:
            context=browser.new_context(viewport={'width':1000,'height':650},device_scale_factor=scale)
            scaled=context.new_page()
            scaled.add_init_script('window.__realHostFixture='+json.dumps(REAL_PAGE,ensure_ascii=False)+';'+MOCK)
            scaled.goto(f'http://127.0.0.1:{server.server_port}/')
            expect(scaled.locator('#sort')).to_be_enabled()
            scaled.wait_for_timeout(100)
            metrics=scaled.evaluate('() => {'+METRICS+'}')
            assert metrics['dpr']==scale
            assert_layout(metrics)
            scaled.screenshot(path=str(OUTPUT / f'renderer-dpr-{scale}.png'),full_page=True)
            (OUTPUT / f'renderer-dpr-{scale}.json').write_text(json.dumps(metrics,ensure_ascii=False,indent=2),encoding='utf-8')
            context.close()
        checks.append('Renderer DPR 1.25/1.5 at measured 1000x650 CSS viewport; not OS scaling proof')
        touch=browser.new_context(viewport={'width':360,'height':800},has_touch=True,is_mobile=True)
        touch_page=touch.new_page()
        touch_page.add_init_script('window.__realHostFixture='+json.dumps(REAL_PAGE,ensure_ascii=False)+';'+MOCK)
        touch_page.goto(f'http://127.0.0.1:{server.server_port}/')
        expect(touch_page.locator('#sort')).to_be_enabled()
        touch_badge=touch_page.locator('[aria-label="עזרה: תיקית מקור"]')
        touch_badge.tap()
        expect(touch_page.locator('#context-help')).to_be_visible()
        touch_badge.tap()
        expect(touch_page.locator('#context-help')).to_be_hidden()
        assert touch_badge.bounding_box()['height'] >= 44
        touch.close()
        checks.append('Touch badge toggles and retains a 44px target')
        browser.close()
finally:
    server.shutdown()
(OUTPUT / 'renderer-checks.json').write_text(json.dumps({'kind':'renderer_only_mocked_ipc','checks':checks,'native_windows_verified':False,'android_verified':False},ensure_ascii=False,indent=2),encoding='utf-8')
print(f'{len(checks)} renderer checks passed. IPC was mocked; native runtime remains unverified.')
