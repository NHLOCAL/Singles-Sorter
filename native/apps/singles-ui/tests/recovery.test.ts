import {describe,it,expect,vi} from 'vitest';
import type {RecoveryHost,RecoveryPage,RecoveryReport,RecoverySnapshot,RecoveryProgress} from '../src/contracts';
import {RecoveryController,displayRecoveryPath} from '../src/recovery';
const page:RecoveryPage={offset:0,total:1,truncated:false,records:[{token:'record',job_id:'persisted',journal_name:'sorter.directories.jsonl',family:'single',source:'C:\\source',target:'C:\\target',state:'inspection_needed',detail:'incomplete',operation_count:2}]};
const report:RecoveryReport={job_id:'persisted',state:'needs_attention',observed_directories:1,pending_directories:0,rejected_directories:0,incomplete_tail:false,unverified_directories:0,observation_limit_reached:false,operations:[{operation:0,source:'a',destination:'b',status:'Committed'},{operation:1,source:'c',destination:'d',status:'ReadyForExplicitResume'}]};
type TestHost=RecoveryHost&{listRecovery:(offset:number,limit:number)=>Promise<RecoveryPage>;inspectRecovery:(token:string,source:string,target:string|null)=>Promise<RecoveryReport>};
const progress:RecoveryProgress={stage:'files',operation:null,total_operations:0,completed_operations:0,records_checked:0,observations:0,bytes_read:0,elapsed_ms:0};
function host():TestHost{
 let serial=0;
 const pending=new Map<string,{kind:'list'|'inspect';result:Promise<RecoveryPage|RecoveryReport>}>();
 const h:TestHost={
  listRecovery:vi.fn(async()=>structuredClone(page)),
  inspectRecovery:vi.fn(async()=>structuredClone(report)),
  selectRecoveryRoot:vi.fn(async role=>({token:role,display_name:role,platform:'windows' as const,capabilities:{read:true,list:true}})),
  startRecoveryList:vi.fn(async(offset,limit)=>{const id=`run-${serial++}`;pending.set(id,{kind:'list',result:Promise.resolve().then(()=>h.listRecovery(offset,limit))});return {run_id:id};}),
  startRecoveryInspection:vi.fn(async(token,source,target)=>{const id=`run-${serial++}`;pending.set(id,{kind:'inspect',result:Promise.resolve().then(()=>h.inspectRecovery(token,source,target))});return {run_id:id};}),
  recoverySnapshot:vi.fn(async (id:string):Promise<RecoverySnapshot>=>{const run=pending.get(id)!;const result=await run.result;return {run_id:id,kind:run.kind,status:'completed',progress,page:run.kind==='list'?result as RecoveryPage:null,report:run.kind==='inspect'?result as RecoveryReport:null,error:null};}),
  cancelRecovery:vi.fn(async()=>{}),
 };
 return h;
}
async function chosen(h=host()){const c=new RecoveryController(h);await c.load();c.choose('record');return c;}
describe('read-only restart recovery flow',()=>{
 it('requires separate fresh root choices; recorded display paths never become tokens',async()=>{const h=host(),c=await chosen(h);await c.inspect();expect(h.inspectRecovery).not.toHaveBeenCalled();await c.selectRoot('Source');await c.inspect();expect(h.inspectRecovery).not.toHaveBeenCalled();await c.selectRoot('Target');await c.inspect();expect(h.inspectRecovery).toHaveBeenCalledExactlyOnceWith('record','Source','Target');expect(c.report?.state).toBe('needs_attention');});
 it('preserves selection when the native folder picker is cancelled',async()=>{const h=host(),c=await chosen(h);await c.selectRoot('Source');h.selectRecoveryRoot=async()=>null;await c.selectRoot('Source');expect(c.source?.token).toBe('Source');});
 it('invalid records cannot acquire roots or start reconciliation',async()=>{const h=host();h.listRecovery=async()=>({...page,records:[{...page.records[0],state:'invalid'}]});const c=await chosen(h);await c.selectRoot('Source');await c.inspect();expect(h.selectRecoveryRoot).not.toHaveBeenCalled();expect(h.inspectRecovery).not.toHaveBeenCalled();});
 it('new history pages revoke previous selections and inspection result',async()=>{const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');await c.inspect();await c.load();expect(c.selected).toBeNull();expect(c.source).toBeNull();expect(c.target).toBeNull();expect(c.report).toBeNull();});
 it('never converts wrong-job or failed reconciliation into an all-clear',async()=>{const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');h.inspectRecovery=async()=>({...report,job_id:'other',state:'consistent'});await c.inspect();expect(c.report).toBeNull();expect(c.error).toBeTruthy();});
 it('clears revoked journal rows when refresh fails',async()=>{const h=host(),c=await chosen(h);h.listRecovery=async()=>{throw new Error('read failed');};await c.load();expect(c.page).toBeNull();c.choose('record');expect(c.selected).toBeNull();});
 it('holds selection and prevents duplicate inspection while native work is pending',async()=>{let done!:(r:RecoveryReport)=>void;const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');h.inspectRecovery=vi.fn(()=>new Promise<RecoveryReport>(resolve=>{done=resolve;}));const run=c.inspect();c.choose('record');await c.inspect();expect(h.inspectRecovery).toHaveBeenCalledTimes(1);expect(c.busy).toBe(true);done(report);await run;expect(c.busy).toBe(false);});
});

it('shows path formatting characters explicitly without changing Hebrew text',()=>{expect(displayRecoveryPath('C:\\מקור\u202eabc\n')).toBe('C:\\מקור\\u{202e}abc\\u{000a}');});

it('bounds rendered operation pages and rejects invalid offsets',async()=>{const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');h.inspectRecovery=async()=>({...report,operations:Array.from({length:125},(_,operation)=>({...report.operations[0],operation}))});await c.inspect();expect(c.visibleOperations).toHaveLength(50);c.setOperationPage(100);expect(c.visibleOperations).toHaveLength(25);c.setOperationPage(125);expect(c.operationOffset).toBe(100);await c.inspect();expect(c.operationOffset).toBe(0);});

it('unsupported families stay visible but cannot acquire root or inspection authority',async()=>{for(const family of ['album','repair','orphan'] as const){const h=host();h.listRecovery=async()=>({...page,records:[{...page.records[0],family,state:'unsupported'}]});const c=await chosen(h);expect(c.selected?.family).toBe(family);await c.selectRoot('Source');await c.inspect();expect(h.selectRecoveryRoot).not.toHaveBeenCalled();expect(h.inspectRecovery).not.toHaveBeenCalled();}});

it('rejects an all-clear with unresolved directories or an exhausted observation budget',async()=>{for(const fields of [{unverified_directories:1},{observation_limit_reached:true}]){const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');h.inspectRecovery=async()=>({...report,...fields,state:'consistent'});await c.inspect();expect(c.report).toBeNull();expect(c.error).toContain('לא הושלמה');}});

function pending<T>(){let resolve!:(value:T)=>void;const promise=new Promise<T>(done=>{resolve=done;});return {promise,resolve};}
function stoppedSnapshot(id:string,status:RecoverySnapshot['status']='cancelled'):RecoverySnapshot{return {run_id:id,kind:'inspect',status,progress:{...progress,operation:1,total_operations:2,completed_operations:1,bytes_read:80*1024*1024,elapsed_ms:1200},page:null,report:null,error:status==='cancelled'?{code:'CANCELLED',message:'הבדיקה בוטלה'}:null};}

it('queues cancellation while start acknowledgement is pending and keeps incomplete evidence unresolved',async()=>{
 const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');
 const started=pending<{run_id:string}>();h.startRecoveryInspection=vi.fn(()=>started.promise);h.recoverySnapshot=vi.fn(async()=>stoppedSnapshot('slow'));
 const run=c.inspect();await c.cancel();expect(h.cancelRecovery).not.toHaveBeenCalled();expect(c.busy).toBe(true);
 started.resolve({run_id:'slow'});await run;expect(h.cancelRecovery).toHaveBeenCalledExactlyOnceWith('slow');expect(c.report).toBeNull();expect(c.canResume).toBe(true);expect(c.progress?.bytes_read).toBe(80*1024*1024);
});

it('closing during pending start cancels the eventual token and ignores late terminal data',async()=>{
 const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');
 const started=pending<{run_id:string}>();h.startRecoveryInspection=vi.fn(()=>started.promise);h.recoverySnapshot=vi.fn(async()=>({...stoppedSnapshot('late','completed'),report}));
 const run=c.inspect();c.close();started.resolve({run_id:'late'});await run;
 expect(h.cancelRecovery).toHaveBeenCalledExactlyOnceWith('late');expect(c.report).toBeNull();expect(c.selected).toBeNull();expect(c.source).toBeNull();expect(c.error).toBeNull();expect(c.busy).toBe(false);
});

it('resumes using only the retained stopped token and replaces progress with new observations',async()=>{
 const h=host(),c=await chosen(h);await c.selectRoot('Source');await c.selectRoot('Target');
 h.startRecoveryInspection=vi.fn(async()=>({run_id:'stopped'}));h.recoverySnapshot=vi.fn(async()=>stoppedSnapshot('stopped'));await c.inspect();
 expect(c.canResume).toBe(true);h.startRecoveryInspection=vi.fn(async()=>({run_id:'fresh'}));h.recoverySnapshot=vi.fn(async()=>({...stoppedSnapshot('fresh','completed'),report,progress:{...progress,total_operations:2,completed_operations:2}}));
 await c.inspect(true);expect(h.startRecoveryInspection).toHaveBeenCalledExactlyOnceWith('record','Source','Target','stopped');expect(c.progress?.bytes_read).toBe(0);expect(c.canResume).toBe(false);expect(c.report).toEqual(report);
});

it('polls measurable running progress and refuses a wrong run reply',async()=>{
 const h=host();const c=new RecoveryController(h,async()=>{});await c.load();c.choose('record');await c.selectRoot('Source');await c.selectRoot('Target');
 h.startRecoveryInspection=vi.fn(async()=>({run_id:'observing'}));let count=0;
 h.recoverySnapshot=vi.fn(async()=>++count===1?stoppedSnapshot('observing','running'):stoppedSnapshot('other','completed'));
 await c.inspect();expect(count).toBe(2);expect(c.progress?.completed_operations).toBe(1);expect(c.report).toBeNull();expect(c.error).toBeTruthy();expect(h.cancelRecovery).toHaveBeenCalledExactlyOnceWith('observing');
});

it('cannot resurrect a root picker result after its view closes',async()=>{
 const h=host(),c=await chosen(h);const picked=pending<Awaited<ReturnType<RecoveryHost['selectRecoveryRoot']>>>();h.selectRecoveryRoot=()=>picked.promise;
 const selecting=c.selectRoot('Source');c.close();picked.resolve({token:'late-root',display_name:'C:\\source',platform:'windows',capabilities:{read:true,list:true}});await selecting;
 expect(c.source).toBeNull();expect(c.selected).toBeNull();expect(c.busy).toBe(false);
});

it('cancels discovery too, while a late page cannot restore closed selections',async()=>{
 const h=host(),c=new RecoveryController(h);const started=pending<{run_id:string}>();h.startRecoveryList=()=>started.promise;
 h.recoverySnapshot=async()=>({...stoppedSnapshot('list','completed'),kind:'list',page});
 const loading=c.load();c.close();started.resolve({run_id:'list'});await loading;
 expect(h.cancelRecovery).toHaveBeenCalledExactlyOnceWith('list');expect(c.page).toBeNull();expect(c.selected).toBeNull();
});


it('inspects Fix from a fresh source-only choice and does not acquire target authority',async()=>{
 const h=host();h.listRecovery=async()=>({...page,records:[{...page.records[0],family:'repair',target:null}]});const c=await chosen(h);
 await c.inspect();expect(h.startRecoveryInspection).not.toHaveBeenCalled();await c.selectRoot('Target');expect(h.selectRecoveryRoot).not.toHaveBeenCalled();
 await c.selectRoot('Source');expect(c.canInspect).toBe(true);await c.inspect();expect(h.startRecoveryInspection).toHaveBeenCalledExactlyOnceWith('record','Source',null,null);expect(c.report?.state).toBe('needs_attention');
});

it('Fix consistency requires verified current and retained original evidence',async()=>{
 for(const evidence of [undefined,[],[{role:'backup',path:'C:\\source\\original.backup',status:'Missing'}],[{role:'current',path:'C:\\source\\song.mp3',status:'Verified'},{role:'backup',path:'C:\\source\\original.backup',status:'Changed'}]]){
  const h=host();h.listRecovery=async()=>({...page,records:[{...page.records[0],family:'repair',target:null}]});h.inspectRecovery=async()=>({...report,state:'consistent',operations:[{...report.operations[0],evidence}]});
  const c=await chosen(h);await c.selectRoot('Source');await c.inspect();expect(c.report).toBeNull();expect(c.error).toContain('לא הושלמה');
 }
 const h=host();h.listRecovery=async()=>({...page,records:[{...page.records[0],family:'repair',target:null}]});
 const evidence=[{role:'current',path:'C:\\source\\song.mp3',status:'Verified'},{role:'backup',path:'C:\\source\\original.backup',status:'Verified'}];
 h.inspectRecovery=async()=>({...report,state:'consistent',operations:[{...report.operations[0],evidence}]});const c=await chosen(h);await c.selectRoot('Source');await c.inspect();expect(c.report?.operations[0].evidence).toEqual(evidence);
});
