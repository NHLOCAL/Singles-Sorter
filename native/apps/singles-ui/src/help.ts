/** One non-interactive explanation per badge, usable with pointer, keyboard or touch. */
export function installHelpBadges(root:HTMLElement){
 let active:HTMLButtonElement|null=null,pinned=false,timer:ReturnType<typeof setTimeout>|undefined;
 const bubble=document.createElement('div');bubble.id='context-help';bubble.className='help-popover';bubble.role='tooltip';bubble.hidden=true;
 function cancelClose(){clearTimeout(timer);}
 function close(){cancelClose();active?.removeAttribute('aria-describedby');active?.setAttribute('aria-expanded','false');active=null;pinned=false;bubble.hidden=true;}
 function position(){if(!active)return;const box=active.getBoundingClientRect();const width=bubble.getBoundingClientRect().width;
  bubble.style.left=`${Math.max(8,Math.min(window.innerWidth-width-8,box.right-width))}px`;
  const height=bubble.getBoundingClientRect().height;
  bubble.style.top=`${Math.max(8,box.bottom+6+height<window.innerHeight?box.bottom+6:box.top-height-6)}px`;
 }
 function show(button:HTMLButtonElement){cancelClose();if(active!==button){close();active=button;}bubble.textContent=button.dataset.helpText??'';
  // A dialog is in the top layer, so its explanation must belong to that dialog.
  (button.closest('dialog')??document.body).append(bubble);bubble.hidden=false;button.setAttribute('aria-describedby',bubble.id);button.setAttribute('aria-expanded','true');position();
 }
 function scheduleClose(){cancelClose();if(!pinned)timer=setTimeout(close,160);}
 for(const button of root.querySelectorAll<HTMLButtonElement>('.help-badge')){
  button.type='button';button.setAttribute('aria-expanded','false');
  button.addEventListener('pointerenter',event=>{if(event.pointerType!=='touch')show(button);});
  button.addEventListener('pointerleave',scheduleClose);
  button.addEventListener('focus',()=>show(button));
  button.addEventListener('blur',()=>{if(!pinned)scheduleClose();});
  button.addEventListener('click',()=>{if(active===button&&pinned)close();else{show(button);pinned=true;}});
 }
 bubble.addEventListener('pointerenter',cancelClose);bubble.addEventListener('pointerleave',scheduleClose);
 document.addEventListener('pointerdown',event=>{if(active&&!active.contains(event.target as Node)&&!bubble.contains(event.target as Node))close();});
 document.addEventListener('keydown',event=>{if(event.key==='Escape'&&active){event.preventDefault();event.stopPropagation();close();}},true);
 document.addEventListener('scroll',position,true);window.addEventListener('resize',position);
 root.querySelectorAll('dialog').forEach(dialog=>dialog.addEventListener('close',close));
}
