"""Renderer-only parent-picker acceptance with explicit mocked IPC, never device evidence.

Run after npm run build. Uses installed Python Playwright/Chromium; no downloads.
"""
import functools
import http.server
import json
import os
from pathlib import Path
import threading

from playwright.sync_api import expect, sync_playwright

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = Path(os.environ.get('SINGLES_UI_EVIDENCE', str(ROOT / 'test-results')))
OUTPUT.mkdir(parents=True, exist_ok=True)
MOCK = r"""
window.__calls=[];
window.__job=0;
window.__parentGranted=false;
window.__replyParent=(ok,value)=>{const id=window.__parentRequest.id;delete window.__parentRequest;window.__singlesAndroidReply(id,ok,value);};
const parentResult={token:'parent-only-token',display_name:'parent',capabilities:{read:true,list:true},platform:'android'};
window.__acceptParent=()=>{window.__parentGranted=true;window.__replyParent(true,parentResult);};
const dispatch=async(command,args={})=>{
 window.__calls.push({command,args});
 if(command==='get_settings')return {copy_mode:false,main_folder_only:false,singles_folder:true,exist_only:false,abc_sort:false,duet_mode:false};
 if(command==='get_model_status')return {available:false,message:'Mocked renderer only',platform:window.__platform??'android'};
 if(command==='get_first_run_status'||command==='mark_first_run_seen')return {version:'fixture',seen:true};
 if(command==='latest_execution')return null;
 if(command==='select_root')return {token:args.role+'-original',display_name:args.role==='Source'?'אלבום מקור':'יעד',capabilities:{read:true,list:true},platform:window.__platform??'android'};
 if(command==='start_preview'){
  window.__job++;
  if(window.__job>1&&window.__holdFresh)await new Promise(resolve=>{window.__releaseFresh=resolve;});
  return {job_id:'job-'+window.__job};
 }
 if(command==='job_snapshot')return {host_schema:1,job_id:'job-'+window.__job,status:'completed',plan_id:'plan-'+window.__job,total_rows:1,blockers:[],warnings:[],apply:{supported:window.__parentGranted||window.__flag===false,source_parent_selection_required:window.__flag!==false&&!window.__parentGranted,reason:'Parent permission required',operation_count:2,copy_mode:false,source:'אלבום מקור',target:'יעד',mode:'Sort'}};
 if(command==='get_plan_page')return {total:1,rows:[{id:'album',source:'אלבום מקור',destination:'יעד/אלבום',kind:'Album',reason:'album',blockers:[],artist:'אמן'}]};
 throw new Error('Unexpected mock command '+command);
};
if(window.__desktop)window.__TAURI_INTERNALS__={invoke:dispatch};
else window.SinglesAndroid={invoke:raw=>{
 const request=JSON.parse(raw);
 if(request.command==='select_album_source_parent'){
  window.__calls.push({command:request.command,args:request.args});window.__parentRequest=request;return;
 }
 dispatch(request.command,request.args).then(value=>window.__singlesAndroidReply(request.id,true,value),error=>window.__singlesAndroidReply(request.id,false,{message:String(error)}));
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
        browser = p.chromium.launch(headless=True, **({'executable_path': os.environ['SINGLES_CHROMIUM']} if os.environ.get('SINGLES_CHROMIUM') else {}))
        errors = []

        def open_plan(prefix=''):
            page = browser.new_page(viewport={'width': 393, 'height': 851})
            page.on('pageerror', lambda error: errors.append(str(error)))
            page.add_init_script(prefix + MOCK)
            page.goto(f'http://127.0.0.1:{server.server_port}/')
            expect(page.locator('#sort')).to_be_enabled()
            page.locator('#source-picker').click()
            page.locator('#target-picker').click()
            page.locator('#sort').click()
            expect(page.locator('#results')).to_be_visible()
            return page

        for prefix in ["window.__desktop=true;window.__platform='windows';", "window.__flag=false;"]:
            page = open_plan(prefix)
            expect(page.locator('#select-album-source-parent')).to_be_hidden()
            expect(page.locator('#album-source-parent-help')).to_be_hidden()
            if '__flag' in prefix:
                expect(page.locator('#review-apply')).to_be_enabled()
            assert not page.evaluate("window.__calls.some(c=>c.command==='select_album_source_parent')")
            page.close()
        checks.append('Desktop and ordinary unflagged Android plans have no parent-picker action')

        page = open_plan()
        button = page.get_by_role('button', name='בחירת תיקיית האב ובדיקה מחדש', exact=True)
        expect(button).to_be_visible()
        expect(button).to_be_enabled()
        expect(button).to_have_attribute('aria-describedby', 'album-source-parent-help')
        expect(page.locator('#album-source-parent-help')).to_contain_text('אם תתרוקן לאחר ההעברה המתוכננת')
        expect(page.locator('#album-source-parent-help')).to_contain_text('שום יישום או משתמש אחר')
        expect(page.locator('#review-apply')).to_be_hidden()
        for width, height in [(393, 851), (320, 568)]:
            page.set_viewport_size({'width': width, 'height': height})
            button.scroll_into_view_if_needed()
            expect(button).to_be_in_viewport()
            assert page.evaluate('document.documentElement.scrollWidth<=window.innerWidth+1')
            page.screenshot(path=str(OUTPUT / f'renderer-parent-required-{width}x{height}.png'), full_page=True)
        page.set_viewport_size({'width': 393, 'height': 851})
        before = page.evaluate("window.__calls.find(c=>c.command==='start_preview').args.request")
        button.click()
        page.wait_for_function('!!window.__parentRequest')
        for selector in ['#select-album-source-parent', '#back-setup', '#source-picker', '#target-picker', '#tab-mode-fix', '#show-inactive', '#filter-chip-all']:
            expect(page.locator(selector)).to_be_disabled()
        page.evaluate("document.getElementById('select-album-source-parent').click()")
        page.go_back()
        expect(page.locator('#results')).to_be_visible()
        assert page.evaluate("window.__calls.filter(c=>c.command==='select_album_source_parent').length") == 1
        assert page.evaluate('window.__parentRequest.args') == {'jobId': 'job-1', 'planId': 'plan-1'}
        page.evaluate('window.__replyParent(true,null)')
        expect(button).to_be_enabled()
        expect(button).to_be_focused()
        expect(page.locator('#review-apply')).to_be_hidden()
        assert page.evaluate("window.__calls.filter(c=>c.command==='start_preview').length") == 1
        checks.append('Picker owns controls and Browser Back; cancellation retains the exact completed preview')

        button.click()
        page.wait_for_function('!!window.__parentRequest')
        page.evaluate("window.__replyParent(false,{code:'PARENT_MISMATCH',message:'בחר את תיקיית האב הישירה של המקור.'})")
        expect(page.locator('#error')).to_be_visible()
        expect(page.locator('#review-apply')).to_be_hidden()
        expect(button).to_be_enabled()
        assert page.evaluate("window.__calls.filter(c=>c.command==='start_preview').length") == 1
        checks.append('Wrong-parent failure is visible and never enables Apply')

        page.evaluate('window.__holdFresh=true')
        button.click()
        page.wait_for_function('!!window.__parentRequest')
        page.evaluate('window.__acceptParent()')
        expect(page.locator('#processing-screen')).to_be_visible()
        expect(page.locator('#cancel-preview')).to_be_enabled()
        expect(page.locator('#review-apply')).to_be_hidden()
        page.wait_for_function('!!window.__releaseFresh')
        assert page.evaluate("window.__calls.filter(c=>c.command==='start_preview')[1].args.request") == before
        page.evaluate('window.__releaseFresh()')
        expect(page.locator('#results')).to_be_visible()
        expect(page.locator('#review-apply')).to_be_enabled()
        expect(button).to_be_hidden()
        assert page.evaluate('history.state.jobId') == 'job-2'
        expect(page.locator('#source')).to_have_value('אלבום מקור')
        expect(page.locator('#target')).to_have_value('יעד')
        assert not page.evaluate("window.__calls.some(c=>c.command==='start_apply')")
        checks.append('Success discards old plan and runs a fresh original Sort request before exposing Apply')
        page.close()

        page = open_plan()
        page.locator('#select-album-source-parent').click()
        page.wait_for_function('!!window.__parentRequest')
        page.evaluate("window.dispatchEvent(new Event('pagehide'));window.__acceptParent()")
        expect(page.locator('#select-album-source-parent')).to_be_hidden()
        expect(page.locator('#review-apply')).to_be_hidden()
        expect(page.locator('#error')).to_be_hidden()
        assert page.evaluate("window.__calls.filter(c=>c.command==='start_preview').length") == 1
        checks.append('Pagehide discards late picker success and cannot restart a preview')
        assert not errors, errors
        browser.close()
finally:
    server.shutdown()
    server.server_close()

(OUTPUT / 'renderer-parent-checks.json').write_text(json.dumps({'kind': 'renderer_only_mocked_android_ipc', 'checks': checks, 'native_android_verified': False}, ensure_ascii=False, indent=2), encoding='utf-8')
print(f'{len(checks)} parent-selection renderer checks passed. IPC was mocked; native Android remains unverified.')
