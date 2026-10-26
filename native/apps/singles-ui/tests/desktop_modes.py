"""Navigate the real mode tabs before editing mode-specific setup controls."""

SORT_TAB_READY = """return document.querySelector('#app').dataset.view==='setup'
 && !document.querySelector('dialog[open]') && !document.querySelector('#tab-mode-sort').disabled"""
SORT_CONTROLS_READY = """return (() => {
 const app=document.querySelector('#app'),tab=document.querySelector('#tab-mode-sort');
 const visible=element=>{
   if(!element||element.getClientRects().length===0||element.disabled)return false;
   for(let node=element;node;node=node.parentElement){
     const style=getComputedStyle(node);
     if(style.visibility!=='visible'||Number(style.opacity)===0)return false;
   }
   return true;
 };
 return app.dataset.view==='setup'&&app.dataset.workMode==='sort'
   &&!document.querySelector('dialog[open]')&&tab.getAttribute('aria-pressed')==='true'
   &&visible(tab)&&['source','source-picker','target','target-picker','copy_mode',
     'main_folder_only','singles_folder','abc_sort','exist_only','sort']
       .every(id=>visible(document.getElementById(id)));
})();"""


def select_sort_setup(script, click, wait):
    # Back preserves the chosen mode. Fix correctly hides target and Sort-only
    # settings, so use the actual tab rather than editing invisible controls.
    # An open modal is not silently dismissed or bypassed.
    wait(lambda: script(SORT_TAB_READY))
    click('#tab-mode-sort')
    wait(lambda: script(SORT_CONTROLS_READY))
