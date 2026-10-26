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
from desktop_layout import METRICS, assert_layout, assert_editor_actions

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
 if(command==='latest_execution')return null;
 if(command==='get_settings')return {...window.__storedSettings};
 if(command==='save_main_settings'){window.__storedSettings={...window.__storedSettings,...args.settings};return {...window.__storedSettings};}
 if(command==='set_duet'){window.__storedSettings.duet_mode=args.enabled;return {...window.__storedSettings};}
 if(command==='get_first_run_status'||command==='mark_first_run_seen')return {version:'0.1.0',seen:false};
 if(command==='get_personal_list')return structuredClone(window.__personalList);
 if(command==='save_personal_list'){window.__personalList={revision:'r2',rows:args.rows,diagnostics:[]};return structuredClone(window.__personalList);}
 if(command==='export_personal_list')return null;
 if(command==='select_personal_import')return {token:'i1',filename:'sample.csv',encoding_required:true,preview:null};
 if(command==='preview_personal_import')return {token:'i1',encoding:args.encoding,rows:structuredClone(window.__importRows??[['זמר נוסף','תיקייה']]),diagnostics:[]};
 if(command==='confirm_personal_import'){const rows=structuredClone(window.__importRows??[['זמר נוסף','תיקייה']]);const removed=new Set();for(const edit of args.edits??[]){if(edit.kind==='delete')removed.add(edit.index);else rows[edit.index][edit.column]=edit.value;}window.__personalList.rows.push(...rows.filter((_,index)=>!removed.has(index)));window.__personalList.revision='r3';return structuredClone(window.__personalList);}
 if(command==='open_footer_link')return null;
 if(command==='open_project_link'){if(window.__linkFailure)throw 'OPEN_LINK_FAILED';return null;}
 if(command==='get_model_status')return {available:false,message:'בדיקת ממשק בלבד עם IPC מדומה. לא בוצעה הסקה.',platform:'linux_test'};
 if(command==='select_root')return window.__pickerCancel?null:{token:args.role,display_name:args.role==='Source'?'C:\\Music\\שירים':'D:\\מוזיקה',capabilities:{read:true,list:true},platform:'linux_test'};
 if(command==='resolve_typed_root'){if(args.text==='bad')throw 'INVALID_ROOT';return {token:args.role,display_name:args.text,capabilities:{read:true,list:true},platform:'linux_test'};}
 if(command==='start_preview'){
  if(args.request.ai_profile==='Configured')throw 'RECOGNITION_UNAVAILABLE';
  await new Promise(resolve=>setTimeout(resolve,window.__startDelay??80));return {job_id:'renderer-fixture'};
 }
 if(command==='cancel_preview')return null;
 if(command==='job_snapshot')return {host_schema:1,job_id:'renderer-fixture',status:window.__jobStatus??'completed',plan_id:'renderer-fixture-not-native',total_rows:2,blockers:['PHYSICAL_APPLY_UNAVAILABLE'],warnings:['Effects and counters are predicted; no files or tags were changed','AI intentionally disabled; no native inference bundle is used','Unknown actionable fixture warning']};
 if(command==='get_plan_page'){const projected=window.__realHostFixture.rows.filter(row=>args.developerMode||!row.developer_only);const rows=projected.filter(row=>args.showInactive||!row.no_action);return {rows:rows.slice(args.offset,args.offset+args.limit),total:rows.length,skipped:projected.filter(row=>row.no_action).length};}
 throw new Error('Unexpected mock command '+command);
}};
"""
def developer_mode(page, enabled):
    page.locator('#open-settings').click()
    if enabled:
        page.locator('#developer-mode').check()
    else:
        page.locator('#developer-mode').uncheck()
    expect(page.locator('#app')).to_have_attribute('data-developer-mode',str(enabled).lower())
    page.locator('#advanced-dialog .close-dialog.primary').click()

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
        page.locator('#source').fill('C:\\Music')
        page.locator('#target').fill('D:\\Music')
        page.locator('#tab-mode-fix').click()
        expect(page.locator('#tab-mode-fix')).to_have_attribute('aria-pressed','true')
        expect(page.locator('#target-station')).to_be_hidden()
        expect(page.locator('#destination-options-group')).to_be_hidden()
        expect(page.locator('#copy_mode')).to_be_hidden()
        expect(page.locator('#fix')).to_be_visible()
        page.screenshot(path=str(OUTPUT / 'renderer-studio-fix.png'),full_page=True)
        page.locator('#tab-mode-sort').click()
        expect(page.locator('#source')).to_have_value('C:\\Music')
        expect(page.locator('#target')).to_have_value('D:\\Music')
        expect(page.locator('#target-station')).to_be_visible()
        assert not page.evaluate("window.__calls.some(c=>c.command==='start_preview')")
        page.locator('#source').fill('')
        page.locator('#target').fill('')
        icon_centers=page.evaluate("""()=>['menu','open-settings'].map(id=>{const b=document.getElementById(id).getBoundingClientRect(),i=document.querySelector('#'+id+' svg').getBoundingClientRect();return Math.abs((b.x+b.width/2)-(i.x+i.width/2))<=1&&Math.abs((b.y+b.height/2)-(i.y+i.height/2))<=1})""")
        assert all(icon_centers), icon_centers
        assert page.locator('#open-settings').bounding_box()['x'] < 150
        checks.append('Work modes preserve paths without starting work; both toolbar icons centered on the left')
        badge=page.locator('[aria-label="עזרה: תיקית מקור"]')
        badge.hover()
        expect(page.locator('#context-help')).to_be_visible()
        # Check within the same event turn: no timeout, transition or pinned click.
        assert badge.evaluate("el=>{el.dispatchEvent(new PointerEvent('pointerleave',{pointerType:'mouse'}));return document.querySelector('#context-help').hidden}")
        page.locator('h1').hover()
        badge.hover()
        badge.click(click_count=2)
        page.locator('h1').hover()
        expect(page.locator('#context-help')).to_be_hidden()
        page.locator('#source').focus()
        badge.focus()
        expect(page.locator('#context-help')).to_be_visible()
        page.keyboard.press('Escape')
        expect(page.locator('#context-help')).to_be_hidden()
        expect(badge).to_be_focused()
        page.keyboard.press('Enter')
        expect(page.locator('#context-help')).to_be_visible()
        page.keyboard.press('Tab')
        expect(page.locator('#context-help')).to_be_hidden()
        checks.append('Help synchronous pointer leave, repeated mouse clicks, keyboard focus/blur, Escape and reopen')
        (OUTPUT / 'layout-metrics.json').write_text(json.dumps(measurements,ensure_ascii=False,indent=2),encoding='utf-8')
        page.screenshot(path=str(OUTPUT / 'renderer-mock-desktop.png'), full_page=True)
        checks.append('Desktop page identity and nonblank renderer')
        cancel_page=browser.new_page(viewport={'width':1000,'height':650})
        cancel_page.add_init_script('window.__realHostFixture='+json.dumps(REAL_PAGE,ensure_ascii=False)+';'+MOCK)
        cancel_page.goto(f'http://127.0.0.1:{server.server_port}/')
        expect(cancel_page.locator('#sort')).to_be_enabled()
        cancel_page.locator('#source').fill('C:\\Music')
        cancel_page.evaluate("window.__jobStatus='running';window.__startDelay=400")
        cancel_page.locator('#tab-mode-fix').click()
        cancel_page.locator('#fix').click()
        expect(cancel_page.locator('#processing-screen')).to_be_visible()
        expect(cancel_page.locator('#tab-mode-sort')).to_be_disabled()
        expect(cancel_page.locator('#tab-mode-fix')).to_be_disabled()
        cancel_page.locator('#cancel-preview').click()
        expect(cancel_page.locator('#cancel-preview')).to_be_disabled()
        expect(cancel_page.locator('#source')).to_be_disabled()
        cancel_page.wait_for_function("window.__calls.some(c=>c.command==='cancel_preview')")
        expect(cancel_page.locator('#processing-screen')).to_be_visible()
        assert cancel_page.evaluate("window.__calls.filter(c=>c.command==='start_preview').length")==1
        assert cancel_page.evaluate("window.__calls.filter(c=>c.command==='cancel_preview').length")==1
        assert not cancel_page.evaluate("window.__calls.some(c=>c.command==='get_plan_page')")
        cancel_page.screenshot(path=str(OUTPUT / 'renderer-cancellation-pending.png'),full_page=True)
        cancel_page.evaluate("window.__jobStatus='cancelled'")
        expect(cancel_page.locator('#setup-screen')).to_be_visible()
        expect(cancel_page.locator('#source')).to_be_enabled()
        expect(cancel_page.locator('#source')).to_be_focused()
        # Browser Back requests cancellation, stays blocked until native terminal.
        cancel_page.evaluate("window.__jobStatus='running';window.__startDelay=0")
        cancel_page.locator('#tab-mode-fix').click()
        cancel_page.locator('#fix').click()
        expect(cancel_page.locator('#processing-screen')).to_be_visible()
        cancel_page.go_back()
        cancel_page.wait_for_function("window.__calls.filter(c=>c.command==='cancel_preview').length===2")
        expect(cancel_page.locator('#processing-screen')).to_be_visible()
        expect(cancel_page.locator('#source')).to_be_disabled()
        cancel_page.evaluate("window.__jobStatus='cancelled'")
        expect(cancel_page.locator('#setup-screen')).to_be_visible()
        cancel_page.go_forward()
        expect(cancel_page.locator('#setup-screen')).to_be_visible()
        assert cancel_page.evaluate("window.__calls.filter(c=>c.command==='start_preview').length")==2
        cancel_page.close()
        checks.append('Cancellation before job ID and Browser Back both wait for terminal native state; Forward never restarts work')

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
        expect(page.locator('#rows tr')).to_have_count(1)
        expect(page.locator('#rows img')).to_have_count(0)
        expect(page.locator('#rows')).to_contain_text('אברהם פריד - בדיקה.mp3')
        expect(page.locator('#rows')).to_contain_text('להעברה')
        expect(page.locator('#rows')).not_to_contain_text('בדיקת אלבום')
        expect(page.locator('#rows')).not_to_contain_text('לפי כללי')
        expect(page.locator('#details')).to_be_hidden()
        assert page.evaluate("window.__calls.find(c=>c.command==='start_preview').args.request.settings.singles_folder") is False
        expect(page.locator('#warnings .warning')).to_have_count(1)
        expect(page.locator('#warnings')).to_contain_text('דורשים בדיקה')
        expect(page.locator('#warnings')).not_to_contain_text('Unknown actionable')
        page.screenshot(path=str(OUTPUT/'renderer-results-simple.png'),full_page=True)
        page.locator('#filter-chip-attention').click()
        expect(page.locator('#filter-chip-attention')).to_have_attribute('aria-pressed','true')
        expect(page.locator('#filter-empty')).to_be_visible()
        expect(page.locator('#filter-summary')).to_have_text('0 מתוך 1 בעמוד')
        page.locator('#filter-chip-ready').click()
        expect(page.locator('#rows tr:not([hidden])')).to_have_count(1)
        expect(page.locator('#filter-empty')).to_be_hidden()
        page.locator('#filter-chip-inactive').click()
        expect(page.locator('#show-inactive')).to_be_checked()
        page.locator('#filter-chip-all').click()
        page.locator('#show-inactive').uncheck()
        expect(page.locator('#rows tr')).to_have_count(1)
        assert page.evaluate("window.__calls.filter(c=>c.command==='start_preview').length")==1
        checks.append('Page filters expose selected state, match counts and empty matches without restarting or changing the plan')
        # Ordinary rows already show everything they know; details are a developer tool.
        expect(page.locator('#rows .detail-button')).to_have_count(0)
        developer_mode(page,True)
        expect(page.locator('#rows tr')).to_have_count(1)
        page.locator('#show-inactive').check()
        expect(page.locator('#rows tr')).to_have_count(2)
        expect(page.locator('#rows')).to_contain_text('בדיקת אלבום')
        expect(page.locator('#warnings')).to_have_text('Unknown actionable fixture warning')
        expect(page.locator('#technical-notices')).to_contain_text('Effects and counters are predicted')
        expect(page.locator('#technical-notices')).to_contain_text('AI intentionally disabled')
        expect(page.locator('#details')).to_be_visible()
        page.screenshot(path=str(OUTPUT/'renderer-results-developer.png'),full_page=True)
        developer_mode(page,False)
        page.locator('#show-inactive').uncheck()
        expect(page.locator('#rows tr')).to_have_count(1)
        checks.append('Ordinary results hide routine folder diagnostics and technical text; developer mode restores complete evidence with correct pagination')
        page.locator('#back-setup').click()
        expect(page.locator('#setup-screen')).to_be_visible()
        page.evaluate("window.__realHostFixture.rows[1].source='<img src=x onerror=alert(1)>.mp3'")
        page.locator('#sort').click()
        expect(page.locator('#rows')).to_contain_text('<img src=x onerror=alert(1)>.mp3')
        expect(page.locator('#rows img')).to_have_count(0)
        checks.append('Additional hostile filename remains literal text without HTML execution')
        developer_mode(page,True)
        page.evaluate("document.querySelector('#warnings').textContent='אזהרה ארוכה '.repeat(500); document.querySelector('#details').open=true")
        page.wait_for_timeout(100)
        assert_layout(page.evaluate('() => {'+METRICS+'}'))
        assert page.evaluate("document.querySelector('#results-body').scrollHeight > document.querySelector('#results-body').clientHeight")
        checks.append('Long warnings and expanded details stay in the single results scroller')
        windows_page=browser.new_page(viewport={'width':1000,'height':650})
        windows_fixture=json.loads(json.dumps(REAL_PAGE))
        long_source='C:\\Users\\runneradmin\\AppData\\Local\\Temp\\' + ('owned-fixture-' * 10) + '\\מקור\\אברהם פריד - בדיקה.mp3'
        windows_fixture['rows'][1]['source']=long_source
        windows_page.add_init_script('window.__realHostFixture='+json.dumps(windows_fixture,ensure_ascii=False)+';'+MOCK.replace("platform:'linux_test'","platform:'windows'"))
        windows_page.goto(f'http://127.0.0.1:{server.server_port}/')
        expect(windows_page.locator('#sort')).to_be_enabled()
        windows_page.locator('#source').fill('C:\\Music')
        windows_page.locator('#target').fill('D:\\Music')
        windows_page.locator('#sort').click()
        expect(windows_page.locator('#rows tr')).to_have_count(1)
        expect(windows_page.locator('#rows .source-name')).not_to_have_attribute('title',long_source)
        developer_mode(windows_page,True)
        source_summary=windows_page.locator('#rows tr').nth(0).locator('.source-name')
        expect(source_summary).to_have_text('אברהם פריד - בדיקה.mp3')
        expect(source_summary).to_have_attribute('title',long_source)
        detail_button=windows_page.locator('#rows tr').nth(0).locator('.detail-button')
        detail_button.focus()
        windows_page.keyboard.press('Enter')
        full_source=windows_page.locator('#row-dialog [aria-label="מקור"]')
        expect(full_source).to_have_value(long_source)
        full_source.focus()
        assert full_source.evaluate('(el)=>el.value.substring(el.selectionStart,el.selectionEnd)')==long_source
        windows_page.keyboard.press('Escape')
        expect(windows_page.locator('#row-dialog')).not_to_be_visible()
        expect(detail_button).to_be_focused()
        windows_page.go_forward()
        expect(windows_page.locator('#row-dialog')).to_be_visible()
        expect(windows_page.locator('#row-dialog [aria-label="מקור"]')).to_have_value(long_source)
        windows_page.keyboard.press('Escape')
        expect(windows_page.locator('#row-dialog')).not_to_be_visible()
        windows_page.wait_for_timeout(100)
        assert_layout(windows_page.evaluate('() => {'+METRICS+'}'))
        # Regression: large-text reflow must not keep stretched grid chrome after reset.
        windows_page.evaluate("document.documentElement.style.fontSize='200%';window.dispatchEvent(new Event('resize'))")
        windows_page.wait_for_timeout(150)
        assert windows_page.evaluate("document.getElementById('app').classList.contains('reflow')")
        windows_page.evaluate("document.documentElement.style.fontSize='';window.dispatchEvent(new Event('resize'))")
        windows_page.wait_for_timeout(150)
        assert_layout(windows_page.evaluate('() => {'+METRICS+'}'))
        windows_page.close()
        checks.append('Windows source filename first; unmodified full path available by hover, keyboard disclosure and copy selection')
        page.screenshot(path=str(OUTPUT / 'renderer-mock-results.png'), full_page=True)
        scans=page.evaluate("window.__calls.filter(call=>call.command==='start_preview').length")
        page.locator('#back-setup').click()
        expect(page.locator('#setup-screen')).to_be_visible()
        page.locator('#reopen-results').click()
        expect(page.locator('#results')).to_be_visible()
        assert page.evaluate("window.__calls.filter(call=>call.command==='start_preview').length")==scans
        page.locator('#back-setup').click()
        expect(page.locator('#setup-screen')).to_be_visible()
        page.locator('#open-settings').click()
        expect(page.locator('#advanced-dialog')).to_be_visible()
        expect(page.locator('#model-status')).to_have_text('המודל לא מותקן')
        expect(page.locator('#model-help')).to_have_attribute('data-help-text', 'בדיקת ממשק בלבד עם IPC מדומה. לא בוצעה הסקה.')
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
        expect(page.locator('#list-notice')).to_have_text('נשמר')
        expect(page.locator('#profile-notice')).to_be_hidden()
        page.locator('#list-cancel').click()
        # Five stacked personal rows reproduce the native narrow editor overflow.
        # Hidden delete labels must stay anchored inside the scrolling row buttons.
        original_personal_rows=page.evaluate('structuredClone(window.__personalList.rows)')
        page.evaluate("window.__personalList.rows=Array.from({length:5},(_,i)=>['אמן '+i,'תיקייה '+i])")
        page.locator('#edit-list').click()
        for width in [360,390,1000]:
            page.set_viewport_size({'width':width,'height':800 if width<740 else 650})
            page.wait_for_timeout(150)
            metrics=page.evaluate('() => {'+METRICS+'}')
            (OUTPUT/f'personal-five-{width}px-metrics.json').write_text(json.dumps(metrics,ensure_ascii=False,indent=2),encoding='utf-8')
            page.screenshot(path=str(OUTPUT/f'personal-five-{width}px.png'),full_page=True)
            assert_editor_actions(metrics)
            if width<740:
                labels=metrics['dialogs'][0]['absoluteRowLabels']
                assert len(labels)==5 and all(label['containedByButton'] for label in labels),labels
            for selector in ['#list-alias-0','#list-alias-4']:
                page.locator(selector).focus()
                assert_editor_actions(page.evaluate('() => {'+METRICS+'}'))
                assert page.locator(selector).evaluate('(el)=>{const r=el.getBoundingClientRect(),footer=el.closest("dialog").querySelector(".editor-footer").getBoundingClientRect();return r.top>=0&&r.bottom<=footer.top}')
        page.locator('#list-cancel').click()
        page.evaluate('(rows)=>{window.__personalList.rows=rows}',original_personal_rows)
        checks.append('Five personal rows keep narrow hidden labels inside scroll owner and footer reachable')
        before_import=page.evaluate('JSON.stringify(window.__personalList.rows)')
        page.evaluate("window.__importRows=Array.from({length:53},(_,i)=>['זמר '+i,'תיקייה '+i]);window.__importRows[0].push('עמודה נוספת')")
        page.locator('#import-list').click()
        page.locator('#import-encoding').select_option('Windows1255')
        page.locator('#decode-import').click()
        expect(page.locator('#import-rows .list-row')).to_have_count(50)
        for width in [1000,360,390]:
            page.set_viewport_size({'width':width,'height':650 if width==1000 else 800})
            page.wait_for_timeout(150)
            assert_editor_actions(page.evaluate('() => {'+METRICS+'}'))
            background_y=page.evaluate('window.scrollY')
            page.mouse.move(1,400)
            page.mouse.wheel(0,600)
            page.wait_for_timeout(100)
            assert page.evaluate('window.scrollY')==background_y
            page.locator('#import-alias-0').focus()
            page.locator('#import-alias-49').focus()
            assert_editor_actions(page.evaluate('() => {'+METRICS+'}'))
            assert page.locator('#import-alias-49').evaluate('(el)=>{const r=el.getBoundingClientRect();const footer=el.closest("dialog").querySelector(".editor-footer").getBoundingClientRect();return r.top>=0&&r.bottom<=footer.top}')
        page.set_viewport_size({'width':1000,'height':650})
        page.evaluate("document.documentElement.style.fontSize='200%';window.dispatchEvent(new Event('resize'))")
        page.wait_for_timeout(150)
        assert_editor_actions(page.evaluate('() => {'+METRICS+'}'))
        page.locator('#import-alias-0').focus()
        page.locator('#import-alias-49').focus()
        assert page.locator('#import-alias-49').evaluate('(el)=>{const r=el.getBoundingClientRect();return r.top>=0&&r.bottom<=el.closest("dialog").querySelector(".editor-footer").getBoundingClientRect().top}')
        page.evaluate("document.documentElement.style.fontSize='';window.dispatchEvent(new Event('resize'))")
        page.wait_for_timeout(150)

        page.locator('#import-alias-0').fill('עריכה שנשמרת בטיוטה')
        page.locator('[data-import-delete="1"]').click()
        expect(page.locator('#import-alias-2')).to_be_focused()
        expect(page.locator('#import-alias-1')).to_have_count(0)
        page.locator('#import-next').click()
        expect(page.locator('#import-alias-51')).to_be_visible()
        page.locator('#import-alias-51').fill('עריכה בעמוד שני')
        page.locator('#import-prev').click()
        expect(page.locator('#import-alias-0')).to_have_value('עריכה שנשמרת בטיוטה')
        expect(page.locator('#import-rows')).to_contain_text('עמודה נוספת')
        page.locator('#cancel-import').click()
        assert page.evaluate('JSON.stringify(window.__personalList.rows)')==before_import
        page.evaluate('delete window.__importRows')
        page.locator('#import-list').click()
        page.locator('#import-encoding').select_option('Windows1255')
        page.locator('#decode-import').click()
        page.locator('[data-import-delete="0"]').click()
        expect(page.locator('#confirm-import')).to_be_disabled()
        expect(page.locator('#cancel-import')).to_be_focused()
        page.locator('#cancel-import').click()
        assert page.evaluate('JSON.stringify(window.__personalList.rows)')==before_import
        checks.append('Import draft pagination, stable indices, delete focus, raw extra fields, cancel and empty draft guard')
        page.locator('#import-list').click()
        expect(page.locator('#confirm-import')).to_be_disabled()
        page.locator('#import-encoding').select_option('Windows1255')
        page.locator('#decode-import').click()
        expect(page.locator('#import-summary')).to_contain_text('1 שורות')
        page.locator('#import-alias-0').fill('זמר שנערך לפני אישור')
        page.locator('#confirm-import').click()
        expect(page.locator('#advanced-dialog')).to_be_visible()
        assert page.evaluate('window.__personalList.rows.length') == 2
        assert page.evaluate('window.__personalList.rows[1][0]') == 'זמר שנערך לפני אישור'
        checks.append('Draft cancellation, explicit list save, encoding preview and confirmed append')
        page.locator('#ai-profile').select_option('Configured')
        page.keyboard.press('Escape')
        expect(page.locator('#advanced-dialog')).not_to_be_visible()
        expect(page.locator('#open-settings')).to_be_focused()
        page.locator('#save-settings').click()
        expect(page.locator('#settings-notice')).to_have_text('נשמר')
        assert page.evaluate('window.__storedSettings.singles_folder') is False
        checks.append('Main settings save separately from immediate duet persistence')
        page.locator('#tab-mode-fix').click()
        page.locator('#fix').click()
        expect(page.locator('#error')).to_contain_text('RECOGNITION_UNAVAILABLE')
        checks.append('Dialog escape/focus return, sixth setting, configured AI fail closed')
        page.locator('#menu').click()
        page.get_by_role('button', name='איך משתמשים?').click()
        expect(page.locator('#help-content')).to_contain_text('בחר תיקית מקור')
        page.locator('[data-help="about"]').click()
        expect(page.locator('#usage-content')).to_be_hidden()
        expect(page.locator('#about-content')).to_be_visible()
        expect(page.locator('#about-version')).to_have_text(json.loads((ROOT/'package.json').read_text())['version'])
        expect(page.locator('[data-help="about"]')).to_have_attribute('aria-pressed','true')
        page.screenshot(path=str(OUTPUT/'renderer-about.png'),full_page=True)
        for target in ['Source','License']:
            page.locator(f'[data-project-link="{target}"]').click()
            assert page.evaluate("window.__calls.at(-1)")=={'command':'open_project_link','args':{'target':target}}
        page.evaluate('window.__linkFailure=true')
        page.locator('[data-project-link="Source"]').click()
        expect(page.locator('#about-link-error')).to_contain_text('https://github.com/NHLOCAL/Singles-Sorter')
        expect(page.locator('[data-project-link="Source"]')).to_be_enabled()
        page.evaluate('window.__linkFailure=false')
        page.locator('.about-credits summary').click()
        expect(page.locator('.about-credits')).to_have_attribute('open','')
        for width,height in [(360,800),(1000,650)]:
            page.set_viewport_size({'width':width,'height':height})
            page.locator('.about-credits').scroll_into_view_if_needed()
            assert page.locator('#menu-dialog').evaluate('el=>el.scrollWidth<=el.clientWidth+1')
            page.screenshot(path=str(OUTPUT/f'renderer-about-{width}.png'),full_page=True)
        page.keyboard.press('Escape')
        expect(page.locator('#menu')).to_be_focused()
        checks.append('About version, credits, narrow layout, named links/failure fallback, menu keyboard dismissal and focus return')
        page.set_viewport_size({'width':1000,'height':650})
        page.evaluate("document.documentElement.style.fontSize='200%'")
        page.wait_for_timeout(100)
        large=page.evaluate('() => {'+METRICS+'}')
        assert large['reflow']
        assert_layout(large,require_bounded=False)
        page.locator('#tab-mode-sort').click()
        for selector in ['#source','#target','#copy_mode','#exist_only','#open-settings','#save-settings','#sort']:
            page.locator(selector).scroll_into_view_if_needed()
            expect(page.locator(selector)).to_be_in_viewport()
        page.evaluate("document.documentElement.style.fontSize=''")
        checks.append('200% desktop text intentionally reflows and retains every primary command')
        page.set_viewport_size({'width': 360, 'height': 800})
        page.wait_for_function("!document.getElementById('app').classList.contains('reflow')")
        page.evaluate("document.querySelector('main').scrollTop=0")
        page.wait_for_timeout(100)
        mobile_image=page.screenshot(path=str(OUTPUT / 'renderer-mock-mobile.png'), full_page=True)
        assert int.from_bytes(mobile_image[16:20],'big') == 360
        assert page.evaluate('document.documentElement.scrollWidth <= window.innerWidth')
        expect(page.locator('#copy_mode')).to_be_visible()
        checks.append('360px layout retains basic controls without page horizontal overflow')
        page.evaluate("document.documentElement.style.fontSize='200%'")
        assert page.evaluate('document.documentElement.scrollWidth <= window.innerWidth')
        for selector in ['#source','#source-picker','#target','#target-picker','#sort','#open-settings','#save-settings']:
            control=page.locator(selector)
            control.scroll_into_view_if_needed()
            box=control.bounding_box()
            assert box and box['width'] > 0 and box['x'] >= -1 and box['x'] + box['width'] <= 361, (selector,box)
            assert control.evaluate('(el)=>el.scrollWidth <= el.clientWidth + 1'), selector
            expect(control).to_be_in_viewport()
        page.screenshot(path=str(OUTPUT / 'renderer-mock-mobile-large-text.png'), full_page=True)
        checks.append('200% mobile text retains complete folder pickers and primary commands without clipping')
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
        expect(touch_page.locator('#app')).to_have_attribute('data-developer-mode','false')
        touch_badge=touch_page.locator('[aria-label="עזרה: תיקית מקור"]')
        touch_badge.tap()
        expect(touch_page.locator('#context-help')).to_be_visible()
        touch_badge.tap()
        expect(touch_page.locator('#context-help')).to_be_hidden()
        touch_badge.tap()
        touch_page.locator('h1').tap()
        expect(touch_page.locator('#context-help')).to_be_hidden()
        touch_page.locator('#menu').tap()
        touch_page.locator('[data-help="about"]').tap()
        expect(touch_page.locator('#about-content')).to_be_visible()
        touch_page.screenshot(path=str(OUTPUT/'renderer-about-touch.png'),full_page=True)
        assert touch_badge.bounding_box()['height'] >= 48
        touch.close()
        checks.append('Touch badge toggles and retains a 48px target')
        recovery_page=browser.new_page(viewport={'width':1000,'height':650})
        recovery_page.on('pageerror', lambda error: errors.append(str(error)))
        recovery_mock=MOCK.replace("platform:'linux_test'", "platform:'windows'")
        recovery_page.add_init_script('window.__realHostFixture='+json.dumps(REAL_PAGE,ensure_ascii=False)+';'+recovery_mock+r"""
        const original=window.__TAURI_INTERNALS__.invoke;
        let serial=0;const runs=new Map();
        const progress={stage:'files',operation:0,total_operations:51,completed_operations:1,records_checked:30,observations:10,bytes_read:80*1024*1024,elapsed_ms:1200};
        window.__TAURI_INTERNALS__.invoke=async(command,args={})=>{
          if(!['start_recovery_list','select_recovery_root','start_recovery_inspection','recovery_snapshot','cancel_recovery'].includes(command))return original(command,args);
          window.__calls.push({command,args});
          if(command==='start_recovery_list'){
            const run_id='recovery-'+serial++;runs.set(run_id,{run_id,kind:'list',status:'completed',progress,page:{offset:args.offset,total:3,truncated:false,records:[
              {token:'single',job_id:'sorter-1-2-3',journal_name:'sorter-1-2-3.directories.jsonl',family:'single',source:'C:\\מקור',target:'C:\\יעד',state:'recorded_complete',detail:'היומן מתעד סיום; יש לבדוק את מצב הקבצים.',operation_count:51},
              {token:'album',job_id:'album-1-2-3',journal_name:'album.jsonl',family:'album',source:null,target:null,state:'unsupported',detail:'בדיקה לאחר פתיחה מחדש עדיין אינה נתמכת. שמור את היומן.',operation_count:0},
              {token:'repair',job_id:'repair-1-2-3',journal_name:'repair.jsonl',family:'repair',source:'C:\\תיקון',target:null,state:'inspection_needed',detail:'יש לבחור תיקיית מקור ולבדוק את העותקים המקוריים.',operation_count:1}]},report:null,error:null});return {run_id};
          }
          if(command==='select_recovery_root')return window.__recoveryPickerCancelled?null:{token:args.role,display_name:args.role,capabilities:{read:true,list:true},platform:'windows'};
          if(command==='start_recovery_inspection'){
            const run_id='recovery-'+serial++,run={run_id,kind:'inspect',status:'running',progress,page:null,report:null,error:null};runs.set(run_id,run);
            if(args.token==='repair'){run.status='completed';run.progress={...progress,total_operations:1,completed_operations:1};run.report={job_id:'repair-1-2-3',state:'consistent',observed_directories:0,pending_directories:0,rejected_directories:0,incomplete_tail:false,unverified_directories:0,observation_limit_reached:false,operations:[{operation:0,source:'C:\\תיקון\\song__demo.mp3',destination:'C:\\תיקון\\song demo.mp3',status:'Committed',evidence:[{role:'current',path:'C:\\תיקון\\song demo.mp3',status:'Verified'},{role:'backup',path:'C:\\תיקון\\.sorter-original.backup',status:'Verified'}]}]};return {run_id};}
            window.__finishRecovery=()=>{run.status='completed';run.progress={...progress,completed_operations:51};run.report={job_id:'sorter-1-2-3',state:'needs_attention',observed_directories:1,pending_directories:0,rejected_directories:0,incomplete_tail:false,unverified_directories:0,observation_limit_reached:false,operations:Array.from({length:51},(_,operation)=>({operation,source:'C:\\מקור\\שיר.wav',destination:'C:\\יעד\\שיר.wav',status:operation===0?'NeedsRecovery':'Committed'}))};};return {run_id};
          }
          if(command==='cancel_recovery'){const run=runs.get(args.runId);run.status='cancelled';run.error={code:'CANCELLED',message:'הבדיקה בוטלה'};return;}
          if(command==='recovery_snapshot'){const run=runs.get(args.runId);if(run.status==='cancelled')window.__cancelObserved=true;return structuredClone(run);}
        };
        """)
        recovery_page.goto(f'http://127.0.0.1:{server.server_port}/')
        expect(recovery_page.locator('#sort')).to_be_enabled()
        recovery_page.locator('#menu').click();recovery_page.locator('#recovery-menu').click()
        expect(recovery_page.locator('#recovery-list button')).to_have_count(3)
        recovery_page.locator('[data-recovery-family="album"]').click()
        expect(recovery_page.locator('#recovery-selection')).to_be_hidden()
        expect(recovery_page.locator('#recovery-error')).to_contain_text('אינה נתמכת')
        recovery_page.locator('[data-recovery-family="single"]').click()
        expect(recovery_page.locator('#recovery-inspect')).to_be_disabled()
        recovery_page.locator('#recovery-source-picker').click()
        expect(recovery_page.locator('#recovery-inspect')).to_be_disabled()
        recovery_page.evaluate('window.__recoveryPickerCancelled=true')
        recovery_page.locator('#recovery-source-picker').click()
        expect(recovery_page.locator('#recovery-selected-source')).to_have_text('Source')
        recovery_page.evaluate('window.__recoveryPickerCancelled=false')
        recovery_page.locator('#recovery-target-picker').click()
        recovery_page.locator('#recovery-inspect').click()
        expect(recovery_page.locator('#recovery-inspect')).to_be_disabled()
        expect(recovery_page.locator('#recovery-close')).to_be_enabled()
        expect(recovery_page.locator('#recovery-busy')).to_contain_text('80.0 MiB')
        recovery_page.keyboard.press('Escape')
        expect(recovery_page.locator('#recovery-dialog')).to_be_hidden()
        recovery_page.wait_for_function('window.__cancelObserved===true')
        assert len(recovery_page.evaluate("window.__calls.filter(call=>call.command==='cancel_recovery')"))==1
        recovery_page.locator('#menu').click();recovery_page.locator('#recovery-menu').click()
        expect(recovery_page.locator('#recovery-list button')).to_have_count(3)
        recovery_page.locator('[data-recovery-family="single"]').click()
        expect(recovery_page.locator('#recovery-inspect')).to_be_disabled()
        recovery_page.locator('#recovery-source-picker').click();recovery_page.locator('#recovery-target-picker').click()
        recovery_page.locator('#recovery-inspect').click()
        recovery_page.wait_for_function("window.__calls.filter(call=>call.command==='start_recovery_inspection').length===2")
        recovery_page.evaluate('window.__finishRecovery()')
        expect(recovery_page.locator('#recovery-result-title')).to_have_text('נדרשת בדיקה נוספת')
        expect(recovery_page.locator('#recovery-next-step')).to_contain_text('לפני ביצוע נוסף')
        expect(recovery_page.locator('#recovery-operation-list article')).to_have_count(50)
        recovery_page.locator('#recovery-operation-next').click()
        expect(recovery_page.locator('#recovery-operation-list article')).to_have_count(1)
        for width,height in [(1000,650),(360,800)]:
            recovery_page.set_viewport_size({'width':width,'height':height})
            assert recovery_page.locator('#recovery-dialog').evaluate('el=>el.scrollWidth<=el.clientWidth+1')
            recovery_page.screenshot(path=str(OUTPUT/f'renderer-recovery-{width}.png'),full_page=True)
        recovery_page.keyboard.press('Escape')
        expect(recovery_page.locator('#recovery-dialog')).to_be_hidden()
        expect(recovery_page.locator('#menu')).to_be_focused()
        recovery_page.locator('#menu').click();recovery_page.locator('#recovery-menu').click()
        expect(recovery_page.locator('#recovery-list button')).to_have_count(3)
        recovery_page.locator('[data-recovery-family="repair"]').click()
        expect(recovery_page.locator('#recovery-target-picker')).to_be_hidden()
        expect(recovery_page.locator('#recovery-inspect')).to_be_disabled()
        recovery_page.locator('#recovery-source-picker').click();recovery_page.locator('#recovery-inspect').click()
        expect(recovery_page.locator('#recovery-result-title')).to_have_text('הקבצים שנבדקו תואמים ליומן')
        expect(recovery_page.locator('.recovery-evidence')).to_have_count(2)
        expect(recovery_page.locator('[data-recovery-evidence-role="backup"]')).to_contain_text('עותק מקורי שנשמר: אומת')
        assert recovery_page.evaluate("window.__calls.filter(call=>call.command==='start_recovery_inspection').at(-1).args.targetToken") is None
        recovery_page.screenshot(path=str(OUTPUT/'renderer-recovery-fix-source-only.png'),full_page=True)
        recovery_page.keyboard.press('Escape')
        assert len(recovery_page.evaluate("window.__calls.filter(call=>call.command==='start_recovery_inspection')"))==3
        assert not errors, errors
        recovery_page.close()
        checks.append('Mocked restart flow: unsupported Album, source-only Fix backup evidence, fresh roles, picker cancel, measured progress, active cancel/close and fresh-reopen guards, bounded results and narrow layout; no native runtime claim')
        browser.close()
finally:
    server.shutdown()
(OUTPUT / 'renderer-checks.json').write_text(json.dumps({'kind':'renderer_only_mocked_ipc','checks':checks,'native_windows_verified':False,'android_verified':False},ensure_ascii=False,indent=2),encoding='utf-8')
print(f'{len(checks)} renderer checks passed. IPC was mocked; native runtime remains unverified.')
