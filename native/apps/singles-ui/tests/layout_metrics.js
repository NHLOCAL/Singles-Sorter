return (() => {
 const rect = el => {const r=el.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom,right:r.right};};
 const setupIds=['source','target','source-picker','target-picker','copy_mode','main_folder_only','singles_folder','abc_sort','exist_only','advanced','save-settings','sort','fix'];
 const app=document.getElementById('app');const ids=app.dataset.view==='results'?['back-setup',...(!document.getElementById('pagination').hidden?['previous','next']:[])]:setupIds;
 const controls=Object.fromEntries(ids.map(id=>[id,rect(document.getElementById(id))]));
 const body=document.getElementById('results-body');
 return {view:app.dataset.view,setupVisible:!document.getElementById('setup-screen').hidden,resultsVisible:!document.getElementById('results').hidden,innerWidth,innerHeight,dpr:devicePixelRatio,screenWidth:screen.width,screenHeight:screen.height,
  reflow:app.classList.contains('reflow')||innerWidth<=740||innerHeight<=560,
  pageWidth:document.documentElement.scrollWidth,pageHeight:document.documentElement.scrollHeight,
  results:rect(document.getElementById('results')),resultsBody:rect(body),resultsScrollHeight:body.scrollHeight,
  rowHeights:Array.from(document.querySelectorAll('#rows tr')).map(el=>el.getBoundingClientRect().height),
  resultsOverflow:getComputedStyle(body).overflowY,controls,
  dialogs:Array.from(document.querySelectorAll('dialog[open]')).map(dialog=>({rect:rect(dialog),scrollWidth:dialog.scrollWidth,clientWidth:dialog.clientWidth,scrollHeight:dialog.scrollHeight,scrollTop:dialog.scrollTop,controls:Array.from(dialog.querySelectorAll('button,input,select,summary,a')).filter(el=>el.getClientRects().length).map(el=>({id:el.id,tag:el.tagName,type:el.getAttribute('type'),rect:rect(el)}))})),
  fonts:Object.fromEntries(['body','h1','h2','.option'].map(selector=>{const el=document.querySelector(selector),style=getComputedStyle(el);return [selector,{family:style.fontFamily,size:style.fontSize,lineHeight:style.lineHeight,weight:style.fontWeight,rect:rect(el)}];})),
  segoeFontCheckNotActualGlyphProof:document.fonts.check('14px "Segoe UI"'),openDialogs:document.querySelectorAll('dialog[open]').length};
})();
