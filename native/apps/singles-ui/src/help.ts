import {HelpState} from './help-state';
/** Hover closes synchronously on badge leave; keyboard and touch retain their
 * own dismissal rules. Explanations contain text only and never receive focus. */
export function installHelpBadges(root:HTMLElement){
 const state=new HelpState<HTMLButtonElement>();
 let described:HTMLButtonElement|null=null,pointerFocus:HTMLButtonElement|null=null,pointerType='';
 const bubble=document.createElement('div');bubble.id='context-help';bubble.className='help-popover';bubble.setAttribute('role','tooltip');bubble.hidden=true;
 const disposers:Array<()=>void>=[];
 function listen(target:EventTarget,type:string,listener:EventListener,capture=false){target.addEventListener(type,listener,capture);disposers.push(()=>target.removeEventListener(type,listener,capture));}
 function position(){const active=state.active;if(!active)return;const box=active.getBoundingClientRect();const width=bubble.getBoundingClientRect().width;
  bubble.style.left=`${Math.max(8,Math.min(window.innerWidth-width-8,box.right-width))}px`;
  const height=bubble.getBoundingClientRect().height;
  bubble.style.top=`${Math.max(8,box.bottom+6+height<window.innerHeight?box.bottom+6:box.top-height-6)}px`;
 }
 function render(){
  if(described&&described!==state.active){described.removeAttribute('aria-describedby');described.setAttribute('aria-expanded','false');}
  described=state.active;
  if(!described){bubble.hidden=true;return;}
  bubble.textContent=described.dataset.helpText??'';
  (described.closest('dialog')??document.body).append(bubble);
  bubble.hidden=false;described.setAttribute('aria-describedby',bubble.id);described.setAttribute('aria-expanded','true');position();
 }
 function close(){state.close();render();}
 for(const button of root.querySelectorAll<HTMLButtonElement>('.help-badge')){
  button.type='button';button.setAttribute('aria-expanded','false');
  listen(button,'pointerenter',event=>{state.enter(button,(event as PointerEvent).pointerType);render();});
  listen(button,'pointerleave',event=>{state.leave(button,(event as PointerEvent).pointerType);render();});
  listen(button,'focus',()=>{state.focus(button,pointerFocus===button);render();});
  listen(button,'blur',()=>{if(pointerFocus===button)pointerFocus=null;state.blur(button);render();});
  listen(button,'click',event=>{
   const origin=pointerFocus===button?(pointerType==='touch'?'touch':'hover'):(event as MouseEvent).detail===0?'keyboard':'hover';
   pointerFocus=null;state.activate(button,origin);render();
  });
 }
 listen(document,'pointerdown',event=>{
  const target=event.target instanceof Element?event.target:null;
  const badge=target?.closest<HTMLButtonElement>('.help-badge')??null;
  pointerFocus=badge&&root.contains(badge)?badge:null;pointerType=(event as PointerEvent).pointerType;
  if(state.active&&!state.active.contains(target))close();
 },true);
 listen(document,'keydown',event=>{
  pointerFocus=null;
  if((event as KeyboardEvent).key==='Escape'&&state.active){event.preventDefault();event.stopPropagation();close();}
 },true);
 listen(document,'scroll',()=>{if(state.origin==='hover')close();else position();},true);
 listen(window,'resize',position);
 listen(window,'blur',close);
 listen(document,'visibilitychange',()=>{if(document.hidden)close();});
 root.querySelectorAll('dialog').forEach(dialog=>listen(dialog,'close',close));
 return()=>{close();for(const dispose of disposers)dispose();bubble.remove();};
}
