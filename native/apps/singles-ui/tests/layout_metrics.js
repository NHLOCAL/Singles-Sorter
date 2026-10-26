return (() => {
 const rect = el => {const r=el.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom,right:r.right};};
 const renderedVisible=el=>{if(!el.getClientRects().length)return false;for(let node=el;node;node=node.parentElement){const style=getComputedStyle(node);if(style.visibility!=='visible'||!(parseFloat(style.opacity)>0))return false;}return true;};
 const scrollParent=el=>{for(let p=el.parentElement;p&&p.tagName!=='DIALOG';p=p.parentElement)if(['auto','scroll'].includes(getComputedStyle(p).overflowY))return {rect:rect(p),scrollTop:p.scrollTop,scrollHeight:p.scrollHeight};return null;};
 const setupIds=['source','target','source-picker','target-picker','copy_mode','main_folder_only','singles_folder','abc_sort','exist_only','advanced','save-settings','sort','fix'];
 const app=document.getElementById('app');const ids=app.dataset.view==='results'?['back-setup','review-apply','cancel-apply',...(!document.getElementById('pagination').hidden?['previous','next']:[])]:setupIds;
 const controls=Object.fromEntries([...ids,'tab-mode-sort','tab-mode-fix'].map(id=>document.getElementById(id)).filter(el=>el&&el.getClientRects().length).map(el=>[el.id,rect(el)]));
 const body=document.getElementById('results-body'),panel=document.getElementById('execution-panel');
 const panelVisible=panel.getClientRects().length>0;
 const panelStyle=getComputedStyle(panel);
 const executionPanel=panelVisible?{rect:rect(panel),visible:renderedVisible(panel),parentIsResults:panel.parentElement===document.getElementById('results'),marginTop:parseFloat(panelStyle.marginTop),marginBottom:parseFloat(panelStyle.marginBottom),expectedActions:Array.from(panel.querySelectorAll('button')).filter(el=>!el.hidden).map(el=>el.id),actions:Array.from(panel.querySelectorAll('button')).filter(el=>el.getClientRects().length).map(el=>({id:el.id,rect:rect(el),visible:renderedVisible(el)}))}:null;
 return {view:app.dataset.view,setupVisible:!document.getElementById('setup-screen').hidden,resultsVisible:!document.getElementById('results').hidden,innerWidth,innerHeight,dpr:devicePixelRatio,screenWidth:screen.width,screenHeight:screen.height,
  reflow:app.classList.contains('reflow')||innerWidth<=740||innerHeight<=560,
  pageWidth:document.documentElement.scrollWidth,pageHeight:document.documentElement.scrollHeight,
  chrome:{header:rect(app.querySelector('header')),footer:rect(app.querySelector('footer'))},
  results:rect(document.getElementById('results')),resultsBody:rect(body),resultsScrollHeight:body.scrollHeight,executionPanel,
  rowHeights:Array.from(document.querySelectorAll('#rows tr')).map(el=>el.getBoundingClientRect().height),
  resultsOverflow:getComputedStyle(body).overflowY,controls,
  backgroundLocked:getComputedStyle(document.documentElement).overflowY==='hidden'&&getComputedStyle(document.body).overflowY==='hidden',
  dialogs:Array.from(document.querySelectorAll('dialog[open]')).map(dialog=>({id:dialog.id,editor:dialog.classList.contains('editor-dialog'),absoluteRowLabels:Array.from(dialog.querySelectorAll('.list-row>button>span')).filter(el=>getComputedStyle(el).position==='absolute').map(el=>({containedByButton:el.offsetParent===el.parentElement,rect:rect(el),button:rect(el.parentElement)})),actions:Array.from(dialog.querySelectorAll('.editor-footer button')).map(el=>({id:el.id,rect:rect(el)})),rect:rect(dialog),scrollWidth:dialog.scrollWidth,clientWidth:dialog.clientWidth,scrollHeight:dialog.scrollHeight,scrollTop:dialog.scrollTop,controls:Array.from(dialog.querySelectorAll('button,input,select,summary,a')).filter(el=>el.getClientRects().length).map(el=>({id:el.id,tag:el.tagName,type:el.getAttribute('type'),rect:rect(el),scrollParent:scrollParent(el)}))})),
  fonts:Object.fromEntries(['body','h1','h2','.option'].map(selector=>{const el=document.querySelector(selector),style=getComputedStyle(el);return [selector,{family:style.fontFamily,size:style.fontSize,lineHeight:style.lineHeight,weight:style.fontWeight,rect:rect(el)}];})),
  segoeFontCheckNotActualGlyphProof:document.fonts.check('14px "Segoe UI"'),openDialogs:document.querySelectorAll('dialog[open]').length};
})();
