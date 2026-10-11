import {describe,it,expect,vi} from 'vitest';
import {PreviewController} from '../src/state';
import {defaults} from '../src/contracts';
import type {Host,RootSelection,Snapshot} from '../src/contracts';
const root=(token='source',display_name='C:\\Music'):RootSelection=>({token,display_name,capabilities:{read:true,list:true},platform:'windows'});
const snapshot:Snapshot={host_schema:1,job_id:'j1',status:'completed',plan_id:'p1',total_rows:1,blockers:['PHYSICAL_APPLY_UNAVAILABLE'],warnings:[]};
function host():Host{return{
 selectRoot:vi.fn(async()=>root()),resolveRoot:vi.fn(async(role,text)=>root(role,text)),
 getSettings:vi.fn(async()=>({...defaults})),getModelStatus:vi.fn(async()=>({available:false,message:'No native model',platform:'windows' as const})),
 cancelPreview:vi.fn(async()=>{}),startPreview:vi.fn(async()=>({job_id:'j1'})),snapshot:vi.fn(async()=>snapshot),
 page:vi.fn(async()=>({total:1,rows:[{id:'o0',source:'אמן - שיר.mp3',kind:'העברה מתוכננת',destination:'אמן/סינגלים/שיר.mp3',reason:'filename_alias',blockers:[]}]}))
};}
const deferred=<T>()=>{let resolve!:(value:T)=>void;const promise=new Promise<T>(r=>resolve=r);return{promise,resolve};};
async function ready(h=host()){const c=new PreviewController(h);await c.initialize();return c;}
describe('read-only preview controller',()=>{
 it('uses all six release defaults including explicit false',async()=>{const c=await ready();expect(c.state.settings).toEqual(defaults);expect(c.state.settings.singles_folder).toBe(true);});
 it('keeps the explicit Disabled default even when a model is available',async()=>{const h=host();h.getModelStatus=async()=>({available:true,message:'loaded',platform:'windows'});const c=await ready(h);expect(c.state.model?.available).toBe(true);expect(c.state.ai).toBe('Disabled');});
 it('preserves explicit false preference values',async()=>{const h=host();h.getSettings=async()=>({...defaults,singles_folder:false});const c=await ready(h);expect(c.state.settings.singles_folder).toBe(false);});
 it('rejects malformed settings rather than silently coercing false',async()=>{const h=host();h.getSettings=async()=>({...defaults,copy_mode:'false'} as never);const c=await ready(h);expect(c.state.hostAvailable).toBe(false);});
 it('does not fabricate a host in a browser',async()=>{const h=host();h.getSettings=async()=>{throw new Error('offline');};const c=await ready(h);await c.preview('Sort');expect(h.startPreview).not.toHaveBeenCalled();expect(c.state.error).toBe('offline');});
 it('requires a source before asking the native planner',async()=>{const h=host();const c=await ready(h);await c.preview('FixNames');expect(h.startPreview).not.toHaveBeenCalled();expect(c.state.error).toContain('מקור');});
 it('requires target only for sorting',async()=>{const h=host();const c=await ready(h);c.editRoot('Source','C:\\source');await c.preview('Sort');expect(h.startPreview).not.toHaveBeenCalled();expect(c.state.error).toContain('יעד');await c.preview('FixNames');expect(h.startPreview).toHaveBeenCalledWith(expect.objectContaining({mode:'FixNames',target_token:null}));});
 it('passes untouched path and exact settings to the host',async()=>{const h=host();const c=await ready(h);c.editRoot('Source','C:\\אבג  ');c.editRoot('Target','D:\\מוזיקה');c.changeSetting('singles_folder',false);c.changeSetting('duet_mode',true);await c.preview('Sort');expect(h.resolveRoot).toHaveBeenCalledWith('Source','C:\\אבג  ');expect(h.startPreview).toHaveBeenCalledWith({mode:'Sort',source_token:'Source',target_token:'Target',settings:{...defaults,singles_folder:false,duet_mode:true},ai_profile:'Disabled'});expect(c.state.page?.rows[0].source).toBe('אמן - שיר.mp3');});
 it('preserves selection when picker is cancelled',async()=>{const h=host();const c=await ready(h);await c.selectRoot('Source');h.selectRoot=async()=>null;await c.selectRoot('Source');expect(c.state.source.selection?.token).toBe('source');expect(c.state.source.text).toBe('C:\\Music');});
 it('ignores a picker reply after the user edits the path',async()=>{const h=host(),d=deferred<RootSelection|null>();h.selectRoot=()=>d.promise;const c=await ready(h);const pending=c.selectRoot('Source');c.editRoot('Source','D:\\new');d.resolve(root('old'));await pending;expect(c.state.source.selection).toBeNull();expect(c.state.source.text).toBe('D:\\new');});
 it('does not open duplicate pickers or start during an open picker',async()=>{const h=host(),d=deferred<RootSelection|null>();h.selectRoot=vi.fn(()=>d.promise);const c=await ready(h);const pending=c.selectRoot('Source');await c.selectRoot('Source');await c.preview('Sort');expect(h.selectRoot).toHaveBeenCalledTimes(1);expect(h.startPreview).not.toHaveBeenCalled();d.resolve(null);await pending;});
 it('keeps one scan owner and freezes settings during it',async()=>{const h=host(),d=deferred<{job_id:string}>();h.startPreview=vi.fn(()=>d.promise);const c=await ready(h);await c.selectRoot('Source');const run=c.preview('FixNames');await Promise.resolve();await c.preview('FixNames');c.changeSetting('copy_mode',true);c.editRoot('Source','D:\\new');expect(c.state.settings.copy_mode).toBe(false);expect(c.state.source.text).toBe('C:\\Music');d.resolve({job_id:'j1'});await run;expect(h.startPreview).toHaveBeenCalledTimes(1);});
 it('invalidates inspected results when a setting or path changes',async()=>{const c=await ready();c.editRoot('Source','C:\\source');await c.preview('FixNames');expect(c.state.page).not.toBeNull();c.changeSetting('exist_only',true);expect(c.state.page).toBeNull();expect(c.state.snapshot).toBeNull();});
 it('does not silently disable configured but unavailable AI',async()=>{const h=host();h.startPreview=vi.fn(async()=>{throw new Error('RECOGNITION_UNAVAILABLE');});const c=await ready(h);c.editRoot('Source','C:\\source');c.changeAi('Configured');await c.preview('FixNames');expect(h.startPreview).toHaveBeenCalledWith(expect.objectContaining({ai_profile:'Configured'}));expect(c.state.ai).toBe('Configured');expect(c.state.error).toBe('RECOGNITION_UNAVAILABLE');expect(h.page).not.toHaveBeenCalled();});

 it('keeps native failure distinct from empty success',async()=>{const h=host();h.snapshot=async()=>({...snapshot,status:'failed',error:'SOURCE_CHANGED'});const c=await ready(h);c.editRoot('Source','C:\\source');await c.preview('FixNames');expect(c.state.error).toBe('SOURCE_CHANGED');expect(c.state.page).toBeNull();});
 it('renders a truly empty plan without invented rows',async()=>{const h=host();h.snapshot=async()=>({...snapshot,total_rows:0});h.page=async()=>({total:0,rows:[]});const c=await ready(h);c.editRoot('Source','C:\\source');await c.preview('FixNames');expect(c.state.page).toEqual({total:0,rows:[]});expect(c.state.error).toBeNull();});
 it('bounds paging and preserves order from the host',async()=>{const h=host();h.snapshot=async()=>({...snapshot,total_rows:101});h.page=vi.fn(async()=>({total:101,rows:[]}));const c=await ready(h);c.editRoot('Source','C:\\source');await c.preview('FixNames');await c.changePage(-1);await c.changePage(1.5);await c.changePage(101);expect(h.page).toHaveBeenCalledTimes(1);await c.changePage(50);expect(h.page).toHaveBeenLastCalledWith('j1',50,50);expect(c.state.offset).toBe(50);});
});

describe('asynchronous preview ownership',()=>{
 const tick=()=>vi.advanceTimersByTimeAsync(250);
 async function selected(h=host()){
  const c=await ready(h);await c.selectRoot('Source');return c;
 }
 it('polls running jobs, keeps controls frozen, and fetches rows only after completion',async()=>{
  vi.useFakeTimers();
  try{
   const h=host();h.snapshot=vi.fn().mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValue(snapshot);
   const c=await selected(h),run=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);
   expect(c.state.busy).toBe(true);expect(c.state.snapshot?.status).toBe('running');expect(h.page).not.toHaveBeenCalled();
   await c.preview('FixNames');c.editRoot('Source','new');c.changeSetting('copy_mode',true);c.changeAi('Configured');
   expect(c.state.source.text).toBe('C:\\Music');expect(c.state.settings.copy_mode).toBe(false);expect(c.state.ai).toBe('Disabled');
   await tick();await run;expect(h.startPreview).toHaveBeenCalledTimes(1);expect(h.page).toHaveBeenCalledTimes(1);expect(c.state.busy).toBe(false);
  }finally{vi.useRealTimers();}
 });
 it('surfaces failure after running without requesting result rows',async()=>{
  vi.useFakeTimers();
  try{
   const h=host();h.snapshot=vi.fn().mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValue({...snapshot,status:'failed',error:'MODEL_LOAD_FAILED'});
   const c=await selected(h),run=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);expect(c.state.busy).toBe(true);
   await tick();await run;expect(c.state.error).toBe('MODEL_LOAD_FAILED');expect(c.state.busy).toBe(false);expect(h.page).not.toHaveBeenCalled();
  }finally{vi.useRealTimers();}
 });
 it('remembers cancellation before start returns and waits for actual terminal state',async()=>{
  vi.useFakeTimers();
  try{
   const h=host(),start=deferred<{job_id:string}>();h.startPreview=vi.fn(()=>start.promise);
   h.snapshot=vi.fn().mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValue({...snapshot,status:'cancelled'});
   const c=await selected(h),run=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);
   c.cancelPreview();c.cancelPreview();expect(h.cancelPreview).not.toHaveBeenCalled();expect(c.state.busy).toBe(true);
   start.resolve({job_id:'j1'});await vi.advanceTimersByTimeAsync(0);
   expect(h.cancelPreview).toHaveBeenCalledExactlyOnceWith('j1');expect(c.state.busy).toBe(true);
   await c.preview('FixNames');c.cancelPreview();expect(h.startPreview).toHaveBeenCalledTimes(1);expect(h.cancelPreview).toHaveBeenCalledTimes(1);
   await tick();await run;expect(c.state.snapshot?.status).toBe('cancelled');expect(c.state.busy).toBe(false);expect(h.page).not.toHaveBeenCalled();
  }finally{vi.useRealTimers();}
 });
 it('does not launch work if cancelled while resolving a root',async()=>{
  const h=host(),selection=deferred<RootSelection>();h.resolveRoot=()=>selection.promise;
  const c=await ready(h);c.editRoot('Source','path');const run=c.preview('FixNames');c.cancelPreview();selection.resolve(root());await run;
  expect(h.startPreview).not.toHaveBeenCalled();expect(h.cancelPreview).not.toHaveBeenCalled();expect(c.state.busy).toBe(false);
 });
 it('keeps ownership after a snapshot error and rejects a stale job reply',async()=>{
  vi.useFakeTimers();
  try{
   const h=host();h.snapshot=vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce({...snapshot,job_id:'other'}).mockResolvedValue(snapshot);
   const c=await selected(h),run=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);
   expect(c.state.error).toBe('offline');expect(c.state.busy).toBe(true);
   await tick();expect(c.state.snapshot).toBeNull();expect(c.state.busy).toBe(true);expect(h.page).not.toHaveBeenCalled();
   await tick();await run;expect(c.state.snapshot?.job_id).toBe('j1');expect(c.state.error).toBeNull();
  }finally{vi.useRealTimers();}
 });
 it('retries failed cancellation without releasing the worker or duplicating in-flight requests',async()=>{
  vi.useFakeTimers();
  try{
   const h=host();h.snapshot=vi.fn().mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValue({...snapshot,status:'cancelled'});
   h.cancelPreview=vi.fn().mockRejectedValueOnce(new Error('temporary')).mockResolvedValue(undefined);
   const c=await selected(h),run=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);c.cancelPreview();c.cancelPreview();await vi.advanceTimersByTimeAsync(0);
   expect(h.cancelPreview).toHaveBeenCalledTimes(1);expect(c.state.error).toBe('temporary');expect(c.state.busy).toBe(true);
   await tick();expect(h.cancelPreview).toHaveBeenCalledTimes(2);expect(c.state.busy).toBe(true);
   await tick();await run;expect(c.state.busy).toBe(false);expect(h.page).not.toHaveBeenCalled();
  }finally{vi.useRealTimers();}
 });
 it('suppresses completed rows if cancellation races with worker completion',async()=>{
  const h=host(),reply=deferred<Snapshot>();h.snapshot=()=>reply.promise;
  const c=await selected(h),run=c.preview('FixNames');await Promise.resolve();await Promise.resolve();
  c.cancelPreview();reply.resolve(snapshot);await run;expect(c.state.busy).toBe(false);expect(c.state.page).toBeNull();expect(h.page).not.toHaveBeenCalled();
 });
 it('ignores late cancellation failures after a newer preview starts',async()=>{
  vi.useFakeTimers();
  try{
   const h=host();let reject!:(error:Error)=>void;
   h.cancelPreview=vi.fn(()=>new Promise<void>((_,r)=>{reject=r;}));
   h.snapshot=vi.fn().mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValueOnce({...snapshot,status:'cancelled'}).mockResolvedValueOnce({...snapshot,status:'running'}).mockResolvedValue(snapshot);
   const c=await selected(h),first=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);c.cancelPreview();await tick();await first;
   const second=c.preview('FixNames');await vi.advanceTimersByTimeAsync(0);reject(new Error('old cancellation'));await vi.advanceTimersByTimeAsync(0);
   expect(c.state.error).toBeNull();expect(c.state.busy).toBe(true);expect(c.state.cancelRequested).toBe(false);
   await tick();await second;expect(c.state.page).not.toBeNull();
  }finally{vi.useRealTimers();}
 });
 it('honors cancellation while the completed page is still loading',async()=>{
  const h=host(),page=deferred<{total:number;rows:[]}>();h.page=vi.fn(()=>page.promise);
  const c=await selected(h),run=c.preview('FixNames');await Promise.resolve();await Promise.resolve();await Promise.resolve();
  expect(h.page).toHaveBeenCalled();c.cancelPreview();page.resolve({total:0,rows:[]});await run;
  expect(c.state.page).toBeNull();expect(h.cancelPreview).not.toHaveBeenCalled();expect(c.state.busy).toBe(false);
 });
});

describe('reload startup recovery',()=>{
 it('waits through BUSY until the old cancelled worker releases the host',async()=>{
  vi.useFakeTimers();try{
   const h=host();h.getModelStatus=vi.fn().mockRejectedValueOnce({code:'BUSY',message:'old worker'}).mockRejectedValueOnce({code:'BUSY',message:'draining'}).mockResolvedValue({available:true,message:'verified',platform:'windows'});
   const c=new PreviewController(h),pending=c.initialize();await vi.advanceTimersByTimeAsync(0);
   expect(c.state.initialized).toBe(false);expect(c.state.startupWaiting).toBe(true);expect(c.state.hostAvailable).toBe(false);expect(c.state.error).toBeNull();
   await vi.advanceTimersByTimeAsync(500);await pending;
   expect(h.getModelStatus).toHaveBeenCalledTimes(3);expect(c.state.initialized).toBe(true);expect(c.state.startupWaiting).toBe(false);expect(c.state.hostAvailable).toBe(true);expect(c.state.ai).toBe('Disabled');
  }finally{vi.useRealTimers();}
 });
 it('does not retry a model status transport failure as BUSY',async()=>{
  const h=host();h.getModelStatus=vi.fn().mockRejectedValue({code:'WORKER_FAILED',message:'probe failed'});const c=await ready(h);
  expect(h.getModelStatus).toHaveBeenCalledTimes(1);expect(c.state.initialized).toBe(true);expect(c.state.hostAvailable).toBe(false);expect(c.state.error).toBe('probe failed');
 });
});

describe('authoritative lost job response',()=>{
 it('leaves processing without inventing a terminal result or retrying UNKNOWN_JOB',async()=>{
  vi.useFakeTimers();try{
   const h=host();h.snapshot=vi.fn().mockResolvedValueOnce({...snapshot,status:'running'}).mockRejectedValue({code:'UNKNOWN_JOB',message:'not retained'});
   const c=await ready(h);await c.selectRoot('Source');const pending=c.preview('FixNames');await vi.advanceTimersByTimeAsync(250);await pending;
   expect(h.snapshot).toHaveBeenCalledTimes(2);expect(c.state.busy).toBe(false);expect(c.state.snapshot).toBeNull();expect(c.state.page).toBeNull();expect(c.state.error).toContain('אינה זמינה');expect(h.page).not.toHaveBeenCalled();
  }finally{vi.useRealTimers();}
 });
});
