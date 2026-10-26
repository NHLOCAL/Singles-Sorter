"""Refused-plan presentation with mocked IPC; no native or file-mutation claim.
Run after npm run build with installed Playwright/Chromium. Browser plugin absent.
"""
import functools
import http.server
import json
import os
from pathlib import Path
import threading
from playwright.sync_api import sync_playwright, expect

if not __debug__:
    raise RuntimeError('Refusal acceptance requires enabled Python assertions')
ROOT = Path(__file__).resolve().parents[1]
OUTPUT = Path(os.environ.get('SINGLES_UI_EVIDENCE', ROOT / 'test-results/refusal'))
OUTPUT.mkdir(parents=True, exist_ok=True)
MOCK = r"""
window.__calls=[];window.__job=0;window.__supported=false;
window.__reason='התוכנית כוללת פעולות שאינן זמינות לביצוע בטוח במארח זה (Unsupported)';
window.__blockers=['METADATA_ADAPTER_LIMITATION','PHYSICAL_APPLY_UNAVAILABLE'];
window.__metadata={failed_observations:2,affected_sources:1,truncated:false,issues:[{observation_index:3,after_operations:0,relative_source:'אלבום/שיר.mp3',label_truncated:false,error_kind:'UnsupportedClass'},{observation_index:7,after_operations:2,relative_source:'אלבום/שיר.mp3',label_truncated:false,error_kind:'Storage',storage_kind:'Changed'}]};
window.__settings={copy_mode:true,main_folder_only:false,singles_folder:true,exist_only:false,abc_sort:false,duet_mode:false};
window.__TAURI_INTERNALS__={invoke:async(command,args={})=>{
 window.__calls.push({command,args});
 if(command==='get_settings')return {...window.__settings};
 if(command==='save_main_settings'){window.__settings={...window.__settings,...args.settings};return {...window.__settings};}
 if(command==='get_model_status')return {available:false,message:'Mocked IPC only',platform:'windows',execution_available:true};
 if(command==='get_first_run_status'||command==='mark_first_run_seen')return {version:'fixture',seen:true};
 if(command==='latest_execution')return null;
 if(command==='resolve_typed_root')return {token:args.role,display_name:args.text,capabilities:{read:true,list:true},platform:'windows'};
 if(command==='start_preview')return {job_id:'job-'+(++window.__job)};
 if(command==='job_snapshot')return {host_schema:1,job_id:'job-'+window.__job,status:'completed',plan_id:'plan-'+window.__job,total_rows:103,blockers:window.__blockers,warnings:[],apply:{supported:window.__supported,reason:window.__reason,operation_count:63,copy_mode:window.__settings.copy_mode,source:'C:\\owned-source',target:'D:\\owned-target',mode:'Sort',...(window.__metadata?{metadata_diagnostics:window.__metadata}:{})}};
 if(command==='get_plan_page')return {total:103,rows:Array.from({length:Math.min(args.limit,103-args.offset)},(_,n)=>({id:'row-'+(args.offset+n),source:'fixture-'+(args.offset+n)+'.mp3',kind:'Single',artist:'אמן',reason:'authored',destination:'אמן/fixture.mp3',blockers:[],operation_ids:[args.offset+n]}))};
 throw new Error('Unexpected command '+command);
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
        page = browser.new_page(viewport={'width': 1000, 'height': 800})
        errors = []
        page.on('pageerror', lambda error: errors.append(str(error)))
        page.add_init_script(MOCK)
        page.goto(f'http://127.0.0.1:{server.server_port}/')
        expect(page.locator('#app')).to_have_attribute('data-developer-mode', 'false')
        page.locator('#source').fill('C:\\owned-source')
        page.locator('#target').fill('D:\\owned-target')
        page.locator('#copy_mode').uncheck()
        page.locator('#save-settings').click()
        expect(page.locator('#save-settings')).to_be_enabled()
        page.locator('#sort').click()
        expect(page.locator('#execution-summary')).to_contain_text('קריאת פרטי הקבצים')
        expect(page.locator('#execution-summary')).to_contain_text('לא בוצעו שינויים')
        expect(page.locator('#review-apply')).to_be_hidden()
        expect(page.locator('#execution-refusal > summary')).to_be_visible()
        expect(page.locator('#execution-refusal-details')).to_be_hidden()
        page.screenshot(path=str(OUTPUT / 'ordinary-refusal-1000.png'))
        page.locator('#execution-refusal > summary').click()
        expect(page.locator('#execution-refusal-details')).to_contain_text('Unsupported')
        expect(page.locator('#execution-refusal-details')).to_contain_text('METADATA_ADAPTER_LIMITATION')
        expect(page.locator('#execution-refusal-details')).to_contain_text('63')
        expect(page.locator('#execution-refusal-details')).to_contain_text('אלבום/שיר.mp3')
        expect(page.locator('#execution-refusal-details')).to_contain_text('Storage / Changed')
        page.locator('#next').click()
        expect(page.locator('#page-label')).to_have_text('51–100 מתוך 103')
        expect(page.locator('#execution-refusal-details')).to_be_visible()
        for enabled in (True, False):
            page.locator('#open-settings').click()
            page.locator('#developer-mode').set_checked(enabled)
            expect(page.locator('#app')).to_have_attribute('data-developer-mode',str(enabled).lower())
            page.locator('#advanced-dialog .close-dialog.primary').click()
        expect(page.locator('#review-apply')).to_be_hidden()
        expect(page.locator('#execution-summary')).to_contain_text('קריאת פרטי הקבצים')
        assert page.evaluate("window.__calls.filter(c=>c.command==='save_main_settings').length") == 1
        assert page.evaluate("window.__settings.copy_mode") is False
        checks.append('Default refused plan explains metadata failure; opt-in raw details and63 full-plan count survive paging/developer display without granting Apply or saving settings')
        page.locator('#back-setup').click()
        page.evaluate("window.__metadata=null;window.__blockers=['UNKNOWN_CATEGORY'];window.__reason='private C:\\\\owned <script>diagnostic</script>'; ")
        page.locator('#sort').click()
        expect(page.locator('#execution-summary')).to_contain_text('התוכנית לא אושרה לביצוע')
        expect(page.locator('#execution-summary')).not_to_contain_text('private')
        expect(page.locator('#execution-refusal-details')).to_be_hidden()
        page.locator('#execution-refusal > summary').click()
        expect(page.locator('#execution-refusal-details')).to_contain_text('<script>diagnostic</script>')
        expect(page.locator('#execution-refusal script')).to_have_count(0)
        for width in (360, 1000):
            page.set_viewport_size({'width': width, 'height': 800})
            page.locator('#execution-refusal > summary').scroll_into_view_if_needed()
            assert page.evaluate('document.documentElement.scrollWidth<=innerWidth+1')
            page.screenshot(path=str(OUTPUT / f'unknown-refusal-{width}.png'))
        checks.append('New plan collapses raw details; unknown failure is explicit and escaped; narrow/desktop body has no horizontal overflow')
        page.locator('#back-setup').click()
        page.evaluate("window.__supported=true;window.__reason=null;window.__blockers=['PHYSICAL_APPLY_UNAVAILABLE'];")
        page.locator('#sort').click()
        expect(page.locator('#execution-refusal')).to_be_hidden()
        expect(page.locator('#review-apply')).to_be_visible()
        expect(page.locator('#review-apply')).to_be_enabled()
        page.locator('#review-apply').click()
        expect(page.locator('#apply-dialog')).to_be_visible()
        page.locator('#apply-back').click()
        assert not page.evaluate("window.__calls.some(c=>c.command==='start_apply')")
        checks.append('Supported ordinary plan keeps visible Apply and real confirmation; cancelled review invokes no mutation')
        page.reload()
        expect(page.locator('#app')).to_have_attribute('data-developer-mode','false')
        expect(page.locator('#execution-refusal')).to_be_hidden()
        assert not errors, errors
        browser.close()
    (OUTPUT/'refusal-checks.json').write_text(json.dumps({'status':'passed','ipc_mocked':True,'native_execution':False,'checks':checks},ensure_ascii=False,indent=2),encoding='utf-8')
finally:
    server.shutdown()
