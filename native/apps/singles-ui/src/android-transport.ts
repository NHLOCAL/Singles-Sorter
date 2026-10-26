import './android.css';
// CSS.supports('gap') also succeeds on engines that only support Grid gap.
// Measure a real flex layout so WebView 83 receives spacing fallbacks.
const flexGapProbe=document.createElement('div');
flexGapProbe.style.cssText='display:flex;flex-direction:column;row-gap:1px;position:absolute;visibility:hidden';
flexGapProbe.append(document.createElement('div'),document.createElement('div'));
document.documentElement.append(flexGapProbe);
document.documentElement.classList.toggle('no-flex-gap',flexGapProbe.scrollHeight===0);
flexGapProbe.remove();
/** Only the packaged Android host injects this narrow command transport.
 * No paths, URLs or generic filesystem operations are accepted by this layer.
 */
interface AndroidBridge { invoke(request: string): void; }
declare global {
  interface Window {
    SinglesAndroid?: AndroidBridge;
    __singlesAndroidReply?: (id: number, ok: boolean, value: unknown) => void;
  }
}
const pending = new Map<number,{resolve:(value:unknown)=>void;reject:(error:unknown)=>void}>();
let next = 0;
export function hasAndroidHost(): boolean { return typeof window.SinglesAndroid?.invoke === 'function'; }
export function androidInvoke<T>(command:string,args:Record<string,unknown>={}):Promise<T> {
  if (!hasAndroidHost()) return Promise.reject(new Error('Android host unavailable'));
  if (pending.size >= 64) return Promise.reject(new Error('Too many pending requests'));
  const id = ++next;
  return new Promise<T>((resolve,reject) => {
    pending.set(id,{resolve: value => resolve(value as T),reject});
    try { window.SinglesAndroid!.invoke(JSON.stringify({id,command,args})); }
    catch (error) { pending.delete(id); reject(error); }
  });
}
window.__singlesAndroidReply = (id,ok,value) => {
  const callback = pending.get(id);
  if (!callback) return;
  pending.delete(id);
  if (ok) callback.resolve(value); else callback.reject(value);
};
window.addEventListener('pagehide',()=>{
  for (const value of pending.values()) value.reject(new Error('View closed'));
  pending.clear();
});

/** Attribute-driven fallback for WebView versions without CSS :has(). */
export function watchAndroidDialogs(doc:Document,Observer:typeof MutationObserver):()=>void {
  const update=()=>doc.documentElement.classList.toggle('android-modal-open',doc.querySelector('dialog[open]')!==null);
  const observer=new Observer(update);
  observer.observe(doc.documentElement,{subtree:true,attributes:true,attributeFilter:['open']});
  update();
  return ()=>{observer.disconnect();doc.documentElement.classList.remove('android-modal-open');};
}

// Vite lowers syntax, not missing built-ins. Android 11's original WebView
// predates Object.hasOwn; preserve the exact own-property semantics in the UI.
if(hasAndroidHost()) {
  if(typeof Object.hasOwn!=='function')Object.defineProperty(Object,'hasOwn',{configurable:true,writable:true,value:(value:object,key:PropertyKey)=>Object.prototype.hasOwnProperty.call(value,key)});
  // WebView 83 also predates Element.replaceChildren (Chrome 86). Build the
  // replacement fragment first so moving existing children preserves identity.
  if(typeof Element!=='undefined'&&typeof Element.prototype.replaceChildren!=='function') {
    Object.defineProperty(Element.prototype,'replaceChildren',{configurable:true,writable:true,value:function(this:Element,...nodes:(Node|string)[]) {
      const fragment=this.ownerDocument.createDocumentFragment();
      fragment.append(...nodes);
      this.textContent='';
      this.appendChild(fragment);
    }});
  }
  if(typeof document!=='undefined') {
    document.documentElement.dataset.androidHost='true';
    if(typeof MutationObserver!=='undefined'){const stop=watchAndroidDialogs(document,MutationObserver);window.addEventListener('pagehide',stop);}
    queueMicrotask(()=>{for(const id of ['source','target']){const input=document.getElementById(id) as HTMLInputElement|null;if(input){input.readOnly=true;input.placeholder='בחר תיקיה';input.addEventListener('click',()=>document.getElementById(`${id}-picker`)?.click());}}});
  }
}
