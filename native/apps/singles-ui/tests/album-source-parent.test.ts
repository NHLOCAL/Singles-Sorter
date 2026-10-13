import {describe,it,expect,vi} from 'vitest';
import {PreviewController} from '../src/state';
import {defaults} from '../src/contracts';
import type {Host,RootSelection,Snapshot} from '../src/contracts';

const root=(token:string):RootSelection=>({token,display_name:token,capabilities:{read:true,list:true},platform:'android'});
const plan:Snapshot={host_schema:1,job_id:'album-job',status:'completed',plan_id:'album-plan',total_rows:1,blockers:[],warnings:[],apply:{supported:false,source_parent_selection_required:true,reason:'Select the direct parent',operation_count:2,copy_mode:false,source:'source',target:'target',mode:'Sort'}};
const deferred=<T>()=>{let resolve!:(value:T)=>void;let reject!:(reason:unknown)=>void;const promise=new Promise<T>((yes,no)=>{resolve=yes;reject=no;});return{promise,resolve,reject};};
function host():Host{return{
 getSettings:async()=>({...defaults}),getModelStatus:async()=>({available:true,message:'fixture',platform:'android'}),
 selectRoot:vi.fn(async role=>root(role)),resolveRoot:vi.fn(async role=>root(role)),
 startPreview:vi.fn(async()=>({job_id:'album-job'})),cancelPreview:vi.fn(async()=>{}),snapshot:vi.fn(async()=>structuredClone(plan)),page:vi.fn(async()=>({total:1,rows:[]})),
 selectAlbumSourceParent:vi.fn(async()=>root('parent-only')),startApply:vi.fn(async()=>({job_id:'album-job'})),executionSnapshot:vi.fn(),cancelApply:vi.fn(async()=>{})
};}
async function selected(h=host()){
 const c=new PreviewController(h);await c.initialize();await c.selectRoot('Source');await c.selectRoot('Target');await c.preview('Sort');return c;
}
describe('explicit selected-root Album parent permission',()=>{
 it('requires the explicit Android method and native flag on a completed original Sort Move preview',async()=>{
  const h=host(),c=await selected(h);expect(c.needsAlbumSourceParent).toBe(true);expect(c.canSelectAlbumSourceParent).toBe(true);expect(c.canApply).toBe(false);
  for(const platform of ['windows','linux_test'] as const){c.state.model!.platform=platform;expect(c.needsAlbumSourceParent).toBe(false);}
  c.state.model!.platform='android';
  c.state.snapshot!.apply!.source_parent_selection_required=false;expect(c.needsAlbumSourceParent).toBe(false);
  c.state.snapshot!.apply!.source_parent_selection_required=true;
  c.state.snapshot!.apply!.copy_mode=true;expect(c.needsAlbumSourceParent).toBe(false);c.state.snapshot!.apply!.copy_mode=false;
  c.state.snapshot!.status='running';expect(c.needsAlbumSourceParent).toBe(false);c.state.snapshot!.status='completed';
  c.state.snapshot!.apply!.mode='FixNames';expect(c.needsAlbumSourceParent).toBe(false);c.state.snapshot!.apply!.mode='Sort';
  h.selectAlbumSourceParent=undefined;expect(c.needsAlbumSourceParent).toBe(false);await c.selectAlbumSourceParent();
 });
 it('does not infer permission from displayed Album rows, a supported plan, or stale snapshots',async()=>{
  const h=host(),c=await selected(h);c.state.snapshot!.apply!.supported=true;
  expect(c.canApply).toBe(false);
  c.state.snapshot!.apply!.source_parent_selection_required=undefined;expect(c.canApply).toBe(true);expect(c.needsAlbumSourceParent).toBe(false);
  c.state.snapshot=structuredClone(plan);expect(c.needsAlbumSourceParent).toBe(false);
  const fix=await selected(h);await fix.preview('FixNames');expect(fix.needsAlbumSourceParent).toBe(false);
 });
 it('sends only the exact retained job and plan, preserving the entire preview on cancel',async()=>{
  const h=host();h.selectAlbumSourceParent=vi.fn(async()=>null);const c=await selected(h),snapshot=c.state.snapshot,page=c.state.page;
  c.state.offset=50;expect(await c.selectAlbumSourceParent()).toBe(false);
  expect(h.selectAlbumSourceParent).toHaveBeenCalledExactlyOnceWith('album-job','album-plan');
  expect(c.state.snapshot).toBe(snapshot);expect(c.state.page).toBe(page);expect(c.state.offset).toBe(50);expect(c.state.busy).toBe(false);expect(c.canApply).toBe(false);expect(h.startPreview).toHaveBeenCalledTimes(1);
 });
 it('freezes competing work and duplicate selection until the picker responds',async()=>{
  const h=host(),choice=deferred<RootSelection|null>();h.selectAlbumSourceParent=vi.fn(()=>choice.promise);const c=await selected(h),pending=c.selectAlbumSourceParent();
  expect(c.state.busy).toBe(true);expect(c.state.activity).toBe('album-parent');expect(c.canSelectAlbumSourceParent).toBe(false);
  c.editRoot('Source','other');c.changeSetting('copy_mode',true);c.changeAi('Disabled');c.resourcesChanged();await c.selectRoot('Target');await c.setDeveloperMode(true);await c.setShowInactive(true);await c.preview('FixNames');await c.apply();await c.selectAlbumSourceParent();
  expect(c.state.source.selection?.token).toBe('Source');expect(c.state.settings.copy_mode).toBe(false);expect(c.state.developerMode).toBe(false);expect(c.state.showInactive).toBe(false);expect(h.selectRoot).toHaveBeenCalledTimes(2);expect(h.startPreview).toHaveBeenCalledTimes(1);expect(h.startApply).not.toHaveBeenCalled();expect(h.selectAlbumSourceParent).toHaveBeenCalledTimes(1);
  choice.resolve(null);await pending;expect(c.state.busy).toBe(false);
 });
 it('invalidates on success and requires a fresh original Sort preview before enabling Apply',async()=>{
  const h=host(),c=await selected(h);c.changeSetting('singles_folder',false);c.changeSetting('duet_mode',true);c.changeAi('Configured');await c.preview('Sort');
  const original=structuredClone(vi.mocked(h.startPreview).mock.calls[1][0]);
  expect(await c.selectAlbumSourceParent()).toBe(true);expect(c.state.snapshot).toBeNull();expect(c.state.page).toBeNull();expect(c.canApply).toBe(false);
  expect(c.state.source.selection?.token).toBe('Source');expect(c.state.target.selection?.token).toBe('Target');
  h.startPreview=vi.fn(async()=>({job_id:'fresh-job'}));h.snapshot=async()=>({...plan,job_id:'fresh-job',plan_id:'fresh-plan',apply:{...plan.apply!,supported:true,source_parent_selection_required:false}});
  await c.preview('Sort');expect(h.startPreview).toHaveBeenCalledExactlyOnceWith(original);expect(c.state.snapshot?.plan_id).toBe('fresh-plan');expect(c.canApply).toBe(true);expect(c.needsAlbumSourceParent).toBe(false);
 });
 it.each(['PARENT_MISMATCH','PARENT_PERMISSION'])('shows %s without enabling Apply or replacing Source/Target',async code=>{
  const h=host();h.selectAlbumSourceParent=vi.fn(async()=>{throw{code,message:'Parent permission was not verified'};});const c=await selected(h),snapshot=c.state.snapshot;
  expect(await c.selectAlbumSourceParent()).toBe(false);expect(c.state.error).toBe('Parent permission was not verified');expect(c.state.snapshot).toBe(snapshot);expect(c.canApply).toBe(false);expect(c.state.source.selection?.token).toBe('Source');expect(c.state.target.selection?.token).toBe('Target');
 });
 it('ignores a stale picker success after the retained preview changes',async()=>{
  const h=host(),choice=deferred<RootSelection|null>();h.selectAlbumSourceParent=()=>choice.promise;const c=await selected(h),pending=c.selectAlbumSourceParent();
  const newer={...plan,job_id:'newer',plan_id:'newer-plan'};c.state.snapshot=newer;choice.resolve(root('old-parent'));
  expect(await pending).toBe(false);expect(c.state.snapshot).toBe(newer);expect(c.state.error).toBeNull();expect(h.startPreview).toHaveBeenCalledTimes(1);
 });
 it('does not replace a newer operation with an old picker failure',async()=>{
  const h=host(),choice=deferred<RootSelection|null>();h.selectAlbumSourceParent=()=>choice.promise;const c=await selected(h),pending=c.selectAlbumSourceParent();
  c.state.snapshot={...plan,job_id:'newer'};c.state.activity='preview';c.state.error='Newer operation notice';choice.reject(new Error('Old picker failure'));
  expect(await pending).toBe(false);expect(c.state.activity).toBe('preview');expect(c.state.busy).toBe(true);expect(c.state.error).toBe('Newer operation notice');
 });
 it('rejects changed plan bindings and does not claim a parent token as ordinary root authority',async()=>{
  const h=host(),choice=deferred<RootSelection|null>();h.selectAlbumSourceParent=()=>choice.promise;const c=await selected(h),pending=c.selectAlbumSourceParent();
  c.state.snapshot!.plan_id='changed-plan';choice.resolve(root('old-parent'));
  expect(await pending).toBe(false);expect(c.state.snapshot?.plan_id).toBe('changed-plan');expect(c.state.source.selection?.token).toBe('Source');
 });
 it('requires an actual valid native selection result, rather than a truthy reply',async()=>{
  const h=host();h.selectAlbumSourceParent=async()=>({token:'parent-only'} as RootSelection);const c=await selected(h);
  expect(await c.selectAlbumSourceParent()).toBe(false);expect(c.state.error).toContain('לא אומתה');expect(c.canApply).toBe(false);
 });
 it('keeps Apply disabled when the required fresh preview fails',async()=>{
  const h=host(),c=await selected(h);expect(await c.selectAlbumSourceParent()).toBe(true);
  h.startPreview=vi.fn(async()=>{throw new Error('Fresh preview failed');});await c.preview('Sort');
  expect(c.state.error).toBe('Fresh preview failed');expect(c.state.snapshot).toBeNull();expect(c.canApply).toBe(false);expect(h.startApply).not.toHaveBeenCalled();
 });
 it('does not open the parent picker while an ordinary root picker is pending',async()=>{
  const h=host(),choice=deferred<RootSelection|null>();h.selectRoot=vi.fn(async role=>root(role));const c=await selected(h);
  h.selectRoot=()=>choice.promise;const pending=c.selectRoot('Source');expect(c.canSelectAlbumSourceParent).toBe(false);await c.selectAlbumSourceParent();expect(h.selectAlbumSourceParent).not.toHaveBeenCalled();choice.resolve(null);await pending;
 });
 it.each(['success','failure'] as const)('discards a %s reply after the view closes',async outcome=>{
  const h=host(),choice=deferred<RootSelection|null>();h.selectAlbumSourceParent=()=>choice.promise;const c=await selected(h),pending=c.selectAlbumSourceParent();
  c.discardAlbumSourceParentSelection();if(outcome==='success')choice.resolve(root('late-parent'));else choice.reject(new Error('View closed'));
  expect(await pending).toBe(false);expect(c.state.snapshot).toBeNull();expect(c.state.page).toBeNull();expect(c.state.error).toBeNull();expect(c.state.busy).toBe(false);expect(h.startPreview).toHaveBeenCalledTimes(1);
 });
});
