import {afterEach,describe,expect,it,vi} from 'vitest';
import {PreviewController} from '../src/state';
import {defaults} from '../src/contracts';
import type {ExecutionSnapshot,Host,Snapshot} from '../src/contracts';

const plan:Snapshot={host_schema:1,job_id:'job',plan_id:'plan',status:'completed',total_rows:2,blockers:[],warnings:[],apply:{supported:true,reason:null,operation_count:2,copy_mode:false,source:'source',target:'target'}};
const running:ExecutionSnapshot={host_schema:1,job_id:'job',plan_id:'plan',status:'running',operation_count:2,operation:0,bytes_written:0,source_length:100,journal_path:null,error:null,result:null};
const complete:ExecutionSnapshot={...running,status:'complete',operation:null,result:{source_items:2,committed_destinations:2,moved_sources:2,failed_operations:0,blocked_operations:0,recovery_operations:0,cancelled_operations:0,items:[]}};
const verification={completed_operations:0,total_operations:2,bytes_read:0,elapsed_ms:0};
const recovery={observed_directories:0,pending_directories:0,rejected_directories:0,incomplete_tail:false,transfers:[]};
function makeHost():Host{return {
 getSettings:async()=>({...defaults}),getModelStatus:async()=>({available:false,message:'fixture',platform:'windows'}),selectRoot:async()=>null,
 resolveRoot:async(role,text)=>({token:role,display_name:text,capabilities:{read:true,list:true},platform:'windows'}),
 startPreview:vi.fn(async()=>({job_id:'job'})),cancelPreview:async()=>{},snapshot:vi.fn(async()=>structuredClone(plan)),
 page:vi.fn(async()=>({rows:[],total:2})),startApply:vi.fn(async()=>({job_id:'job'})),executionSnapshot:vi.fn(async()=>structuredClone(complete)),cancelApply:vi.fn(async()=>{})
};}
async function selected(h:Host){const c=new PreviewController(h);await c.initialize();c.editRoot('Source','source');c.editRoot('Target','target');await c.preview('Sort');return c;}
function observe(c:PreviewController){const events:{execution:ExecutionSnapshot|null;error:string|null;busy:boolean;cancelRequested:boolean}[]=[];c.subscribe(()=>events.push(structuredClone({execution:c.state.execution,error:c.state.error,busy:c.state.busy,cancelRequested:c.state.cancelRequested})));return events;}
function snapshots(h:Host,values:ExecutionSnapshot[]){h.executionSnapshot=vi.fn();for(const value of values)vi.mocked(h.executionSnapshot!).mockResolvedValueOnce(structuredClone(value));}
afterEach(()=>vi.useRealTimers());

describe('validated execution notifications',()=>{
 it('keeps the 250 ms native polling cadence but emits identical preparing/running data once',async()=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[{...running,status:'preparing'},{...running,status:'preparing'},running,running,complete]);
  const c=await selected(h),events=observe(c),run=c.apply();await vi.advanceTimersByTimeAsync(0);
  expect(h.executionSnapshot).toHaveBeenCalledTimes(1);await vi.advanceTimersByTimeAsync(249);expect(h.executionSnapshot).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(751);await run;expect(h.executionSnapshot).toHaveBeenCalledTimes(5);
  expect(events.filter(e=>e.execution?.status==='preparing')).toHaveLength(1);
  expect(events.filter(e=>e.execution?.status==='running')).toHaveLength(1);
  expect(events.filter(e=>e.execution?.status==='complete').map(e=>e.busy)).toEqual([true,false]);
 });
 const changes:[string,ExecutionSnapshot,ExecutionSnapshot][]=[
  ['status',running,{...running,status:'preparing'}],
  ['operation count',running,{...running,operation_count:3}],
  ['operation',running,{...running,operation:1}],
  ['bytes written',running,{...running,bytes_written:1}],
  ['source length',running,{...running,source_length:101}],
  ['journal path',running,{...running,journal_path:'journal'}],
  ['native error',running,{...running,error:'retained warning'}],
  ['result',running,{...running,result:complete.result}],
  ['optional verification null',running,{...running,verification:null}],
  ['verification starts',running,{...running,verification}],
  ...(['completed_operations','total_operations','bytes_read','elapsed_ms'] as const).map(key=>[key,{...running,verification},{...running,verification:{...verification,[key]:verification[key]+1}}] as [string,ExecutionSnapshot,ExecutionSnapshot]),
  ['recovery error',running,{...running,recovery_error:'verification failed'}],
  ['recovery starts',running,{...running,recovery}],
  ['nested recovery change',{...running,recovery},{...running,recovery:{...recovery,pending_operations:[1]}}],
  ['future JSON field',running,Object.assign({},running,{future_progress:{value:1}})],
 ];
 it.each(changes)('publishes %s changes without a lossy selected-field comparison',async(_name,first,changed)=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[first,changed,changed,complete]);const c=await selected(h),events=observe(c),run=c.apply();
  await vi.runAllTimersAsync();await run;
  const progress=events.filter(e=>e.execution&&['running','preparing'].includes(e.execution.status));
  expect(progress.map(e=>e.execution)).toEqual([first,changed]);
 });
 it('clears a transport error visibly even when the next valid execution snapshot is identical',async()=>{
  vi.useFakeTimers();const h=makeHost();h.executionSnapshot=vi.fn().mockResolvedValueOnce(running).mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce(structuredClone(running)).mockResolvedValueOnce(complete);
  const c=await selected(h),events=observe(c),run=c.apply();await vi.runAllTimersAsync();await run;
  expect(events.filter(e=>e.execution?.status==='running').map(e=>e.error)).toEqual([null,'offline',null]);
 });
 it.each([['host_schema',2],['job_id','other'],['plan_id','other'],['status','unknown']] as const)('validates %s before equality and never replaces the accepted snapshot with foreign data',async(key,value)=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[running,{...running,[key]:value} as ExecutionSnapshot,running,complete]);
  const c=await selected(h),events=observe(c),run=c.apply();await vi.advanceTimersByTimeAsync(250);
  expect(c.state.execution).toEqual(running);expect(c.state.error).not.toBeNull();expect(c.state.busy).toBe(true);
  await vi.runAllTimersAsync();await run;expect(events.some(e=>e.execution?.job_id==='other'||e.execution?.plan_id==='other')).toBe(false);
  expect(events.filter(e=>e.execution?.status==='running').map(e=>e.error===null)).toEqual([true,false,true]);
 });
 it('delivers cancellation immediately during unchanged progress and waits for the native terminal state',async()=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[running,running,running,{...complete,status:'cancelled'}]);
  const c=await selected(h),events=observe(c),run=c.apply();await vi.advanceTimersByTimeAsync(250);c.cancelApply();
  expect(events.at(-1)?.cancelRequested).toBe(true);expect(h.cancelApply).toHaveBeenCalledExactlyOnceWith('job');
  await vi.advanceTimersByTimeAsync(250);expect(c.state.busy).toBe(true);expect(c.state.execution?.status).toBe('running');
  await vi.runAllTimersAsync();await run;expect(c.state.execution?.status).toBe('cancelled');expect(c.state.busy).toBe(false);
 });
 it('keeps cancellation failures visible and retries once a later unchanged poll permits it',async()=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[running,running,running,{...complete,status:'cancelled'}]);
  h.cancelApply=vi.fn().mockRejectedValueOnce(new Error('cancel offline')).mockResolvedValue(undefined);
  const c=await selected(h),events=observe(c),run=c.apply();await vi.advanceTimersByTimeAsync(0);c.cancelApply();await vi.advanceTimersByTimeAsync(0);
  expect(c.state.error).toBe('cancel offline');expect(c.state.busy).toBe(true);
  await vi.advanceTimersByTimeAsync(250);expect(h.cancelApply).toHaveBeenCalledTimes(2);expect(events.at(-1)?.error).toBeNull();
  await vi.runAllTimersAsync();await run;expect(c.state.execution?.status).toBe('cancelled');
 });
 it.each(['complete','partial','cancelled','failed','needs_recovery'] as const)('always delivers %s and reconciles the current page after repeated progress',async status=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[running,running,{...complete,status}]);const c=await selected(h);
  c.state.offset=50;c.state.developerMode=true;c.state.showInactive=true;
  vi.mocked(h.page).mockResolvedValueOnce({rows:[],total:1,skipped:1});const events=observe(c),run=c.apply();await vi.runAllTimersAsync();await run;
  expect(h.page).toHaveBeenLastCalledWith('job',0,50,true,true);expect(c.state.offset).toBe(0);expect(c.state.page?.skipped).toBe(1);
  expect(events.filter(e=>e.execution?.status===status).map(e=>e.busy)).toEqual([true,false]);
 });
 it('publishes terminal page errors and releases Busy without losing the outcome',async()=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[running,running,complete]);const c=await selected(h);
  vi.mocked(h.page).mockRejectedValueOnce(new Error('page offline'));const events=observe(c),run=c.apply();await vi.runAllTimersAsync();await run;
  expect(events.at(-1)).toMatchObject({execution:complete,error:'page offline',busy:false});
 });
 it('starts a fresh comparison scope for a newly inspected job and plan',async()=>{
  vi.useFakeTimers();const h=makeHost();snapshots(h,[running,running,complete]);const c=await selected(h),events=observe(c);let run=c.apply();await vi.runAllTimersAsync();await run;
  vi.mocked(h.startPreview).mockResolvedValue({job_id:'second'});vi.mocked(h.snapshot).mockResolvedValue({...plan,job_id:'second',plan_id:'second-plan'});
  vi.mocked(h.startApply!).mockResolvedValue({job_id:'second'});await c.preview('Sort');
  const next={...running,job_id:'second',plan_id:'second-plan'};snapshots(h,[next,next,{...complete,job_id:'second',plan_id:'second-plan'}]);
  run=c.apply();await vi.runAllTimersAsync();await run;
  expect(events.filter(e=>e.execution?.status==='running').map(e=>e.execution?.job_id)).toEqual(['job','second']);
  expect(h.startApply).toHaveBeenLastCalledWith('second','second-plan');expect(h.page).toHaveBeenLastCalledWith('second',0,50,false,false);
 });
});
