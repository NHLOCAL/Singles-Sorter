import {afterEach,beforeEach,describe,expect,it,vi} from 'vitest';
vi.mock('@tauri-apps/api/core',()=>({invoke:vi.fn()}));
let windowMock: Record<string,unknown>, close:()=>void;
beforeEach(()=>{vi.resetModules();windowMock={addEventListener:(_event:string,callback:()=>void)=>{close=callback;}};vi.stubGlobal('window',windowMock);});
afterEach(()=>vi.unstubAllGlobals());
describe('owned Android transport',()=>{
 it('rejects missing native host',async()=>{const transport=await import('../src/android-transport');expect(transport.hasAndroidHost()).toBe(false);await expect(transport.androidInvoke('get_settings')).rejects.toThrow('unavailable');});
 it('matches response IDs and ignores replayed replies',async()=>{let sent:any;windowMock.SinglesAndroid={invoke:(request:string)=>{sent=JSON.parse(request);}};const t=await import('../src/android-transport');const result=t.androidInvoke('get_settings');expect(sent).toEqual({id:1,command:'get_settings',args:{}});const reply=windowMock.__singlesAndroidReply as Function;reply(999,true,'wrong');reply(1,true,{copy_mode:true});reply(1,false,'replay');expect(await result).toEqual({copy_mode:true});});
 it('surfaces native errors without treating them as success',async()=>{windowMock.SinglesAndroid={invoke:()=>{}};const t=await import('../src/android-transport');const result=t.androidInvoke('get_settings');(windowMock.__singlesAndroidReply as Function)(1,false,{code:'BUSY'});await expect(result).rejects.toEqual({code:'BUSY'});});
 it('bounds pending work and rejects all owned requests on view close',async()=>{windowMock.SinglesAndroid={invoke:()=>{}};const t=await import('../src/android-transport');const pending=Array.from({length:64},()=>t.androidInvoke('get_settings').catch(e=>e));await expect(t.androidInvoke('get_settings')).rejects.toThrow('Too many');close();expect((await Promise.all(pending)).every(e=>e instanceof Error&&e.message==='View closed')).toBe(true);});
 it('releases pending slot when native invocation throws',async()=>{windowMock.SinglesAndroid={invoke:()=>{throw new Error('closed');}};const t=await import('../src/android-transport');for(let i=0;i<70;i++)await expect(t.androidInvoke('get_settings')).rejects.toThrow('closed');});
});

describe('Android-only Album source-parent bridge',()=>{
 it('exposes no parent selection method to non-Android hosts',async()=>{
  const {nativeHost}=await import('../src/host');expect(nativeHost.selectAlbumSourceParent).toBeUndefined();
 });
 it('sends the exact retained IDs only, never a renderer URI or desktop IPC',async()=>{
  let sent:any;windowMock.SinglesAndroid={invoke:(request:string)=>{sent=JSON.parse(request);}};
  const {nativeHost}=await import('../src/host');const {invoke}=await import('@tauri-apps/api/core');
  const pending=nativeHost.selectAlbumSourceParent!('retained-job','retained-plan');
  expect(sent).toEqual({id:1,command:'select_album_source_parent',args:{jobId:'retained-job',planId:'retained-plan'}});
  (windowMock.__singlesAndroidReply as Function)(sent.id,true,null);expect(await pending).toBeNull();expect(invoke).not.toHaveBeenCalled();
 });
 it('does not fall back to desktop IPC if the Android bridge disappears',async()=>{
  windowMock.SinglesAndroid={invoke:()=>{}};const {nativeHost}=await import('../src/host');const {invoke}=await import('@tauri-apps/api/core');
  delete windowMock.SinglesAndroid;await expect(nativeHost.selectAlbumSourceParent!('job','plan')).rejects.toThrow('Android host unavailable');expect(invoke).not.toHaveBeenCalled();
 });
});

describe('Android WebView 83 compatibility',()=>{
 it('replaces, moves and clears children without losing node identity',async()=>{
  class OwnedElement {
   children:unknown[]=['old'];
   ownerDocument={createDocumentFragment:()=>({nodes:[] as unknown[],append(...nodes:unknown[]){this.nodes.push(...nodes);}})};
   set textContent(_value:string){this.children=[];}
   appendChild(fragment:{nodes:unknown[]}){this.children.push(...fragment.nodes);return fragment;}
  }
  vi.stubGlobal('Element',OwnedElement);windowMock.SinglesAndroid={invoke:()=>{}};
  await import('../src/android-transport');
  const element=new OwnedElement() as OwnedElement&{replaceChildren(...nodes:unknown[]):void};
  const node={owned:true};element.replaceChildren(node,'text');
  expect(element.children).toEqual([node,'text']);expect(element.children[0]).toBe(node);
  element.replaceChildren();expect(element.children).toEqual([]);
 });
 it('does not alter a supported native method',async()=>{
  const native=vi.fn();class ModernElement {replaceChildren=native;}
  ModernElement.prototype.replaceChildren=native;
  vi.stubGlobal('Element',ModernElement);windowMock.SinglesAndroid={invoke:()=>{}};
  await import('../src/android-transport');expect(ModernElement.prototype.replaceChildren).toBe(native);
 });
 it('does not patch non-Android hosts',async()=>{
  class OtherElement {}
  vi.stubGlobal('Element',OtherElement);await import('../src/android-transport');
  expect('replaceChildren' in OtherElement.prototype).toBe(false);
 });
});

describe('Android modal scroll ownership',()=>{
 it('locks only while at least one dialog is open and restores on close/dispose',async()=>{
  const {watchAndroidDialogs}=await import('../src/android-transport');
  let opened=0,notify=()=>{};const classes=new Set<string>();let disconnected=false;
  const root={classList:{toggle:(key:string,on:boolean)=>on?classes.add(key):classes.delete(key),remove:(key:string)=>classes.delete(key)}};
  const doc={documentElement:root,querySelector:(selector:string)=>{expect(selector).toBe('dialog[open]');return opened?{}:null;}};
  class Observer {
   constructor(callback:()=>void){notify=callback;}
   observe(target:unknown,options:unknown){expect(target).toBe(root);expect(options).toEqual({subtree:true,attributes:true,attributeFilter:['open']});}
   disconnect(){disconnected=true;}
  }
  const stop=watchAndroidDialogs(doc as unknown as Document,Observer as unknown as typeof MutationObserver);
  expect(classes.size).toBe(0);opened=2;notify();expect(classes.has('android-modal-open')).toBe(true);
  opened=1;notify();expect(classes.has('android-modal-open')).toBe(true);
  opened=0;notify();expect(classes.size).toBe(0);
  opened=1;notify();stop();expect(classes.size).toBe(0);expect(disconnected).toBe(true);
 });
});
