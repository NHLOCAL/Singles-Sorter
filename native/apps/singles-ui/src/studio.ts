import type {Mode,PlanRow} from './contracts';
import {iconElement} from './icons';

type ResultFilter='all'|'ready'|'attention'|'inactive';

export function installStudio(app:HTMLElement,isBusy:()=>boolean,onModeChange:()=>void){
  const get=<T extends HTMLElement=HTMLElement>(id:string)=>app.querySelector<T>(`#${id}`)!;
  const modeButtons=[get<HTMLButtonElement>('tab-mode-sort'),get<HTMLButtonElement>('tab-mode-fix')];
  const filters=[...app.querySelectorAll<HTMLButtonElement>('.filter-chip')];
  get('station-flow').replaceChildren(iconElement('next'));
  get('fix-inplace-station').querySelector('.station-badge')!.replaceChildren(iconElement('fix'));
  let mode:Mode='Sort';
  let filter:ResultFilter='all';

  function selectMode(next:Mode){
    mode=next;
    const sorting=mode==='Sort';
    app.dataset.workMode=sorting?'sort':'fix';
    modeButtons.forEach((button,index)=>{
      const selected=sorting?index===0:index===1;
      button.classList.toggle('active',selected);
      button.setAttribute('aria-pressed',String(selected));
    });
    for(const id of ['target-station','station-flow','destination-options-group','sort-command-wrap'])get(id).hidden=!sorting;
    for(const id of ['fix-inplace-station','fix-command-wrap'])get(id).hidden=sorting;
    get('copy_mode').closest<HTMLElement>('.option-row')!.hidden=!sorting;
    get('options-heading').textContent=sorting?'כללי המיון':'אפשרויות בדיקה';
    get('mode-deck-description').textContent=sorting
      ?'בחר תיקיית מקור לבדיקה ותיקיית יעד לארגון השירים לפי אמנים'
      :'בדיקת שמות ותגיות בתיקיית המקור, ללא צורך בתיקיית יעד';
  }

  function filterRows(){
    const rows=[...app.querySelectorAll<HTMLTableRowElement>('#rows tr')];
    let visible=0;
    for(const row of rows){
      row.hidden=filter!=='all'&&row.dataset.filterCategory!==filter;
      if(!row.hidden)visible++;
    }
    filters.forEach(button=>{
      const selected=button.dataset.filter===filter;
      button.classList.toggle('active',selected);
      button.setAttribute('aria-pressed',String(selected));
    });
    get('filter-summary').textContent=rows.length?`${visible} מתוך ${rows.length} בעמוד`:'';
    get('filter-empty').hidden=!rows.length||visible>0;
  }

  function resetFilter(){filter='all';filterRows();}
  function update(){
    modeButtons.forEach(button=>{button.disabled=isBusy();});
    filters.forEach(button=>{button.disabled=isBusy();});
  }

  modeButtons.forEach((button,index)=>button.addEventListener('click',()=>{
    if(isBusy())return;
    selectMode(index===0?'Sort':'FixNames');
    onModeChange();
  }));
  filters.forEach(button=>button.addEventListener('click',()=>{
    if(isBusy())return;
    filter=button.dataset.filter as ResultFilter;
    const inactive=get<HTMLInputElement>('show-inactive');
    if(filter==='inactive'&&!inactive.checked)inactive.click();
    filterRows();
  }));
  get('show-inactive').addEventListener('change',()=>{
    if(filter==='inactive'&&!(get<HTMLInputElement>('show-inactive').checked))resetFilter();
  });

  function category(row:PlanRow,status:string):ResultFilter{
    if(['דולג','ללא פעולה','ללא שינוי','כלול באלבום'].includes(status)||row.no_action)return 'inactive';
    if(['להעברה','להעתקה','לתיקון','הועבר','הועתק','תוקן'].includes(status))return 'ready';
    return 'attention';
  }

  selectMode(mode);update();
  return {selectMode,resetFilter,filterRows,update,category};
}
