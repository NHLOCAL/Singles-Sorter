import './styles.css';
import {nativeHost,openFooterLink} from './host';
import {kindLabel,reasonLabel,blockerLabel} from './presentation';
import {PAGE_SIZE,PreviewController} from './state';
import type {Settings,ImportEncoding,Mode} from './contracts';
import {PersonalListController} from './lists';
import {FirstRunGate} from './first-run';
const controller=new PreviewController(nativeHost,nativeHost);
const personal=new PersonalListController(nativeHost,()=>controller.resourcesChanged());
let listOffset=0;
const firstRun=new FirstRunGate(nativeHost);
const app=document.querySelector<HTMLDivElement>('#app')!;
// Static trusted markup only. Every host/file value is rendered with textContent.
app.innerHTML=`
<header class="topbar"><div class="brand"><img src="/icon.png" alt="" width="42" height="42"><div><h1>מסדר הסינגלים</h1><span>תצוגה מקדימה · גרסת פיתוח</span></div></div><button id="menu" class="icon-button" aria-label="עזרה ומידע" aria-haspopup="dialog">⋮</button></header>
<main>
<div id="connection" class="notice" role="status">מתחבר לליבת התוכנה…</div>
<div class="workspace">
<section class="source-panel" aria-labelledby="folders-heading"><h2 id="folders-heading">התיקיות שלך</h2><p class="muted lead">בחר מקור ויעד, ובדוק את התוצאה לפני שינוי קבצים.</p>
<div class="folder"><label for="source">תיקיית מקור</label><div class="folder-row"><input id="source" dir="ltr" autocomplete="off" spellcheck="false" placeholder="C:\\Music\\סינגלים" aria-describedby="source-hint"><button id="source-picker" class="secondary">בחר תיקייה</button></div><p id="source-hint" class="hint">כאן נמצאים השירים לבדיקה</p></div>
<div class="folder"><label for="target">תיקיית יעד</label><div class="folder-row"><input id="target" dir="ltr" autocomplete="off" spellcheck="false" placeholder="D:\\המוזיקה שלי" aria-describedby="target-hint"><button id="target-picker" class="secondary">בחר תיקייה</button></div><p id="target-hint" class="hint">נדרשת למיון בלבד; תיקון נבדק בתיקיית המקור</p></div>
<div class="actions"><button id="sort" class="primary">תצוגה מקדימה למיון</button><button id="fix" class="secondary">תצוגה מקדימה לתיקון</button></div>
<p class="safety">הבדיקה קוראת קבצים בלבד. אין העברה, העתקה או תיקון בפועל.</p>
<aside class="tip"><strong>כדאי לדעת</strong><p>אפשר לבדוק גם תיקון שמות ותגיות. התוצאה מציגה הצעות, חסימות וסיבות, לפני ביצוע כלשהו.</p></aside>
</section>
<aside class="options card" aria-labelledby="options-heading"><div class="section-title"><h2 id="options-heading">אפשרויות מיון</h2><span id="settings-label" class="quiet-label">ההגדרות שלך</span></div>
<fieldset><legend>בסיסי</legend><label class="option"><input id="copy_mode" type="checkbox"><span>העתק קבצים<small>כבוי: תכנון העברה</small></span></label><label class="option"><input id="main_folder_only" type="checkbox"><span>סרוק רק תיקייה ראשית<small>כבוי: סריקה גם בתיקיות משנה</small></span></label></fieldset>
<fieldset><legend>מתקדם</legend><label class="option"><input id="singles_folder" type="checkbox"><span>צור תיקיית ״סינגלים״</span></label><label class="option"><input id="abc_sort" type="checkbox"><span>מיון אלפביתי</span></label><label class="option"><input id="exist_only" type="checkbox"><span>שימוש בתיקיות קיימות בלבד</span></label></fieldset>
<div class="options-bottom"><button id="advanced" class="text-button">הגדרות נוספות</button><button id="save-settings" class="text-button" aria-describedby="save-note">שמור הגדרות</button></div><p id="settings-notice" role="status" class="hint"></p><p id="save-note" class="hint">חמש האפשרויות נשמרות בלחיצה על שמירה. מצב דואטים נשמר מיד.</p>
</aside></div>
<section id="results" class="results card" aria-labelledby="results-heading"><div class="section-title"><h2 id="results-heading" tabindex="-1">תוצאות התצוגה המקדימה</h2><span id="count" class="quiet-label" role="status"></span></div>
<div id="busy" hidden role="status" aria-live="polite"><span class="busy-mark" aria-hidden="true">◷</span> קורא ומנתח את הקבצים… <small>ממתין לתוצאת הליבה. אין ביצוע שינויים.</small></div>
<div id="error" class="error" role="alert" hidden></div>
<div id="empty" class="empty">התוצאה תופיע כאן לאחר בחירת תיקיות והפעלת בדיקה.</div>
<p id="result-note" class="muted" hidden>אלו פעולות מתוכננות בלבד. לא בוצעו שינויים בקבצים.</p>
<div id="warnings"></div>
<div id="table-wrap" hidden><table><thead><tr><th scope="col">מקור</th><th scope="col">פעולה ויעד מוצע</th><th scope="col">סיבה / חסימה</th></tr></thead><tbody id="rows"></tbody></table></div>
<nav id="pagination" aria-label="עמודי תוצאות" hidden><button id="previous" class="secondary">הקודם</button><span id="page-label"></span><button id="next" class="secondary">הבא</button></nav>
<details id="details" hidden><summary>פרטי התכנית והבדיקה</summary><dl><dt>מזהה תכנית</dt><dd id="plan-id" dir="ltr"></dd><dt>חסימות ביצוע</dt><dd id="blockers"></dd></dl></details>
</section>
</main>
<footer>להורדת סינגלים חינם · <button id="footer-link" class="footer-link">שיר בוט ↗</button><span>הקבצים נשארים במחשב שלך</span></footer>
<dialog id="menu-dialog" aria-labelledby="menu-title"><div class="dialog-heading"><h2 id="menu-title">עזרה ומידע</h2><button class="close-dialog icon-button" aria-label="סגור">×</button></div><div class="menu-items"><button data-help="help">איך משתמשים?</button><button data-help="about">אודות התוכנה</button><button data-help="new">מה חדש בגרסת התצוגה?</button></div><div id="help-content" class="help-content"></div></dialog>
<dialog id="advanced-dialog" aria-labelledby="advanced-title"><div class="dialog-heading"><h2 id="advanced-title">הגדרות נוספות</h2><button class="close-dialog icon-button" aria-label="סגור">×</button></div>
<fieldset><legend>שירים עם כמה אמנים</legend><label class="option"><input type="radio" name="duet" id="duet-first" value="first" checked><span>מיון לפי האמן הראשון</span></label><label class="option"><input type="radio" name="duet" id="duet-all" value="all"><span>מיון לכל האמנים</span></label></fieldset><p class="hint">בחירת מצב דואטים נשמרת מיד, בלי לשמור שינויים באפשרויות האחרות.</p>
<fieldset><legend>זיהוי בעזרת AI</legend><p id="model-status" class="muted"></p><label for="ai-profile">פרופיל הבדיקה</label><select id="ai-profile"><option value="Disabled">בדיקה עם AI מושבת במפורש</option><option value="Configured">דרוש מודל AI זמין</option></select><p class="hint">אם נדרש מודל ואינו זמין, הבדיקה תיחסם. אין מעבר אוטומטי לפרופיל אחר.</p></fieldset>
<section><h3>רשימת אמנים אישית</h3><p class="muted">השורות האישיות מתווספות אחרי הרשימה המובנית. סדר ההתאמה נשמר.</p><div class="list-actions"><button id="edit-list" class="secondary">עריכת הרשימה</button><button id="import-list" class="secondary">ייבוא CSV</button><button id="export-list" class="secondary">ייצוא CSV</button></div><p id="profile-notice" class="hint" role="status"></p><p id="profile-error" class="error" role="alert" hidden></p></section><p id="settings-error" class="error" role="alert" hidden></p><button class="close-dialog primary">סגור</button></dialog>
<dialog id="list-dialog" class="wide-dialog" aria-labelledby="list-title"><div class="dialog-heading"><h2 id="list-title">רשימת האמנים האישית</h2><button id="list-cancel-x" class="icon-button" aria-label="סגור בלי לשמור">×</button></div><p class="hint">שם האמן הוא הטקסט לחיפוש; שם התיקייה הוא היעד. אין שינוי סדר או הסרת כפילויות אוטומטיים.</p><p id="list-error" class="error" role="alert" hidden></p><p id="list-notice" role="status"></p><div id="list-rows"></div><nav class="list-pagination" aria-label="עמודי רשימת אמנים"><button id="list-prev" class="secondary">הקודם</button><span id="list-range"></span><button id="list-next" class="secondary">הבא</button></nav><div class="dialog-actions"><button id="list-add" class="secondary">הוסף שורה</button><button id="list-save" class="primary">שמור רשימה</button><button id="list-cancel" class="secondary">ביטול</button></div></dialog>
<dialog id="import-dialog" class="wide-dialog" aria-labelledby="import-title"><div class="dialog-heading"><h2 id="import-title">בדיקת קובץ הייבוא</h2><button id="import-cancel-x" class="icon-button" aria-label="בטל ייבוא">×</button></div><p id="import-filename" dir="auto"></p><label for="import-encoding">קידוד הקובץ</label><select id="import-encoding"><option value="Utf8">UTF-8</option><option value="Utf8Bom">UTF-8 עם BOM</option><option value="Windows1255">Windows-1255 (עברית)</option><option value="Windows1252">Windows-1252</option><option value="Utf16Le">UTF-16 LE</option><option value="Utf16Be">UTF-16 BE</option></select><button id="decode-import" class="secondary">הצג לפי הקידוד שבחרתי</button><p id="import-error" class="error" role="alert" hidden></p><p id="import-summary"></p><ol id="import-rows"></ol><p class="hint">האישור מוסיף את כל השורות בסוף הרשימה. שורות לא תקינות וכפילויות אינן מוסרות אוטומטית. התצוגה מציגה עד 20 שורות ראשונות.</p><div class="dialog-actions"><button id="confirm-import" class="primary">אשר הוספה לרשימה</button><button id="cancel-import" class="secondary">ביטול</button></div></dialog>`;
// Keep the historical footer destination, verified from the released source.
const $=<T extends HTMLElement=HTMLElement>(id:string)=>document.getElementById(id) as T;
const source=$<HTMLInputElement>('source'),target=$<HTMLInputElement>('target');
$('footer-link').addEventListener('click',async()=>{try{await openFooterLink();}catch{ $('error').textContent='לא ניתן לפתוח כעת את הקישור בדפדפן. כתובת האתר: https://nhlocal.github.io/shir-bot';$('error').hidden=false; }});
source.addEventListener('input',()=>controller.editRoot('Source',source.value));
target.addEventListener('input',()=>controller.editRoot('Target',target.value));
$('source-picker').addEventListener('click',()=>void controller.selectRoot('Source'));
$('target-picker').addEventListener('click',()=>void controller.selectRoot('Target'));
async function runPreview(mode:Mode){await controller.preview(mode);if(controller.state.snapshot?.status==='completed'&&!document.querySelector('dialog[open]'))$('results-heading').focus();}
$('sort').addEventListener('click',()=>void runPreview('Sort'));
$('fix').addEventListener('click',()=>void runPreview('FixNames'));
const keys:['copy_mode','main_folder_only','singles_folder','abc_sort','exist_only']=['copy_mode','main_folder_only','singles_folder','abc_sort','exist_only'];
for(const key of keys)$<HTMLInputElement>(key).addEventListener('change',event=>controller.changeSetting(key,(event.target as HTMLInputElement).checked));
$('duet-first').addEventListener('change',()=>void controller.setDuet(false));
$('duet-all').addEventListener('change',()=>void controller.setDuet(true));
$('save-settings').addEventListener('click',()=>void controller.saveMainSettings());
$<HTMLSelectElement>('ai-profile').addEventListener('change',event=>controller.changeAi((event.target as HTMLSelectElement).value as 'Disabled'|'Configured'));
function openDialog(id:string){$<HTMLDialogElement>(id).showModal();}
$('menu').addEventListener('click',()=>openDialog('menu-dialog'));
$('advanced').addEventListener('click',()=>openDialog('advanced-dialog'));
let menuClosing=false;
async function closeMenu(){
 if(menuClosing)return;menuClosing=true;
 try{await firstRun.acknowledge();}
 catch{$('settings-notice').textContent='אישור ההודעה לא נשמר. היא עשויה להופיע שוב בהפעלה הבאה.';}
 finally{$<HTMLDialogElement>('menu-dialog').close();menuClosing=false;}
}
for(const button of document.querySelectorAll<HTMLButtonElement>('.close-dialog'))button.addEventListener('click',()=>{const dialog=button.closest('dialog');if(dialog?.id==='menu-dialog')void closeMenu();else dialog?.close();});
$<HTMLDialogElement>('menu-dialog').addEventListener('cancel',event=>{event.preventDefault();void closeMenu();});
const help={help:'בחר תיקיית מקור, ובמיון גם תיקיית יעד. התאם את האפשרויות ולחץ על תצוגה מקדימה. במצב תיקון אין צורך ביעד. עיין בכל הצעה ובסיבות לחסימה. גרסה זו אינה מבצעת שינויים.',about:'מסדר הסינגלים · גרסת פיתוח מקומית. הממשק החדש מחובר לחוזה התכנון של ליבת Rust. כללי המיון נשענים על הגרסה היציבה 14.0. חיבור AI וביצוע שינויים במוזיקה עדיין אינם זמינים בגרסה זו. ההגדרות והרשימה האישית נשמרות בפרופיל פיתוח נפרד.',new:'תצוגה מקדימה למיון ולתיקון, תוצאות מסודרות עם סיבות וחסימות, ופריסה מותאמת למסכים צרים. זוהי גרסת פיתוח; היא אינה מחליפה את התוכנה היציבה.'};
for(const button of document.querySelectorAll<HTMLButtonElement>('[data-help]'))button.addEventListener('click',()=>{$('help-content').textContent=help[button.dataset.help as keyof typeof help];});
$('previous').addEventListener('click',()=>void controller.changePage(Math.max(0,controller.state.offset-PAGE_SIZE)));
$('next').addEventListener('click',()=>void controller.changePage(controller.state.offset+PAGE_SIZE));
function text(tag:string,value:string,className?:string){const node=document.createElement(tag);node.textContent=value;if(className)node.className=className;return node;}
function render(){
 const s=controller.state;const blocked=s.busy||!s.hostAvailable;
 if(source.value!==s.source.text)source.value=s.source.text;if(target.value!==s.target.text)target.value=s.target.text;
 source.disabled=blocked;target.disabled=blocked;source.readOnly=s.model?.platform==='android';target.readOnly=s.model?.platform==='android';
 for(const [id,pending]of [['source-picker',s.source.pending],['target-picker',s.target.pending]]as const){$<HTMLButtonElement>(id).disabled=blocked||pending;$(id).textContent=pending?'ממתין לבחירה…':'בחר תיקייה';}
 for(const id of ['sort','fix'])$<HTMLButtonElement>(id).disabled=blocked||s.source.pending||s.target.pending;
 for(const key of keys){const input=$<HTMLInputElement>(key);input.checked=s.settings[key as keyof Settings];input.disabled=blocked;}
 $<HTMLInputElement>('duet-first').checked=!s.settings.duet_mode;$<HTMLInputElement>('duet-all').checked=s.settings.duet_mode;
 for(const id of ['duet-first','duet-all','ai-profile'])$(id).toggleAttribute('disabled',blocked);
 $<HTMLSelectElement>('ai-profile').value=s.ai;
 $('model-status').textContent=s.model?.message??'לא התקבל מצב המודל מהמארח.';
 $('connection').textContent=!s.initialized?'מתחבר לליבת התוכנה…':!s.hostAvailable?'המארח המקומי אינו מחובר. יש לפתוח את גרסת התוכנה כדי לבדוק תיקיות.':s.model?.platform==='linux_test'?'סביבת בדיקה ב־Linux · תצוגה מקדימה בלבד; אינה אימות של Windows או Android.':'תצוגה מקדימה בלבד · AI מושבת במפורש כברירת מחדל · ללא שינוי קבצים';
 $('settings-label').textContent=controller.settingsDirty?'שינויים שלא נשמרו':'ההגדרות שלך';$('settings-notice').textContent=controller.savedNotice??'';$<HTMLButtonElement>('save-settings').disabled=blocked;$('settings-error').hidden=!s.error;$('settings-error').textContent=s.error??'';
 $('busy').textContent=s.activity==='settings'?'שומר הגדרות…':s.activity==='page'?'טוען עמוד תוצאות…':'קורא ומנתח את הקבצים… אין ביצוע שינויים במוזיקה.';
 $('busy').hidden=!s.busy;$('results').setAttribute('aria-busy',String(s.busy));
 $('error').hidden=!s.error;$('error').textContent=s.error??'';
 const ready=s.snapshot?.status==='completed'&&!!s.page;
 $('empty').hidden=s.busy||!!s.error||ready&&s.page!.total>0;
 $('empty').textContent=ready?'לא נמצאו החלטות או פעולות להצגה בתכנית זו.':'התוצאה תופיע כאן לאחר בחירת תיקיות והפעלת בדיקה.';
 $('table-wrap').hidden=!ready||!s.page!.rows.length;$('result-note').hidden=!ready;
 $('count').textContent=ready?`${s.page!.total.toLocaleString('he-IL')} רשומות בתכנית`:'';
 $('rows').replaceChildren();
 if(ready)for(const row of s.page!.rows){const tr=document.createElement('tr');const origin=text('td',row.source,'file-name');origin.setAttribute('dir','auto');tr.append(origin);const action=document.createElement('td');action.append(text('strong',kindLabel(row.kind)));if(row.destination){const path=text('bdi',row.destination,'destination');path.dir='auto';action.append(path);}tr.append(action);const reason=document.createElement('td');reason.append(text('span',reasonLabel(row)));if(row.blockers.length)reason.append(text('small',row.blockers.map(blockerLabel).join(' · '),'row-blockers'));const technical=document.createElement('details');technical.append(text('summary','מידע טכני'),text('p',row.reason),text('p',row.blockers.join(' · ')));reason.append(technical);tr.append(reason);$('rows').append(tr);}
 $('warnings').replaceChildren();if(ready)for(const warning of s.snapshot!.warnings)$('warnings').append(text('p',warning,'warning'));
 $('details').hidden=!ready;$('plan-id').textContent=s.snapshot?.plan_id??'';$('blockers').textContent=s.snapshot?.blockers.join(' · ')||'ללא חסימות נוספות שדווחו; ביצוע עדיין אינו זמין בגרסה זו.';
 $('pagination').hidden=!ready||s.page!.total<=PAGE_SIZE;
 $('page-label').textContent=ready?`${s.offset+1}–${Math.min(s.offset+PAGE_SIZE,s.page!.total)} מתוך ${s.page!.total}`:'';
 $<HTMLButtonElement>('previous').disabled=s.busy||s.offset===0;$<HTMLButtonElement>('next').disabled=s.busy||!ready||s.offset+PAGE_SIZE>=s.page!.total;
}
document.documentElement.dataset.startupState='loading';
controller.subscribe(render);render();void controller.initialize().then(async()=>{
 if(!controller.state.hostAvailable){document.documentElement.dataset.startupState='failed';return;}
 try{
  if(await firstRun.load()){$('help-content').textContent=help.new;if(!$<HTMLDialogElement>('menu-dialog').open)openDialog('menu-dialog');}
  document.documentElement.dataset.startupState='ready';
 }catch{
  $('settings-notice').textContent='מצב ההפעלה הראשונה לא נשמר. ההודעה עשויה להופיע שוב.';
  document.documentElement.dataset.startupState='failed';
 }
});
// Personal-list dialogs operate on ordered rows; all persistent writes are host commands.
function returnAdvanced(){if(!$<HTMLDialogElement>('advanced-dialog').open)openDialog('advanced-dialog');}
function closeList(){if(personal.busy)return;personal.cancel();$<HTMLDialogElement>('list-dialog').close();returnAdvanced();}
function closeImport(){if(personal.busy)return;personal.cancel();$<HTMLDialogElement>('import-dialog').close();returnAdvanced();}
$('edit-list').addEventListener('click',async()=>{if(controller.state.busy||personal.busy)return;await personal.open();if(personal.error)return;listOffset=0;renderPersonal();openDialog('list-dialog');});
$('import-list').addEventListener('click',async()=>{if(controller.state.busy||personal.busy)return;await personal.selectImport();if(!personal.selection)return;renderPersonal();openDialog('import-dialog');});
$('export-list').addEventListener('click',()=>{if(!controller.state.busy)void personal.export();});
for(const id of ['list-cancel','list-cancel-x'])$(id).addEventListener('click',closeList);
for(const id of ['cancel-import','import-cancel-x'])$(id).addEventListener('click',closeImport);
$<HTMLDialogElement>('list-dialog').addEventListener('cancel',event=>{event.preventDefault();closeList();});
$<HTMLDialogElement>('import-dialog').addEventListener('cancel',event=>{event.preventDefault();closeImport();});
$<HTMLDialogElement>('advanced-dialog').addEventListener('cancel',event=>{if(controller.state.busy||personal.busy)event.preventDefault();});
$('list-add').addEventListener('click',()=>{personal.add();listOffset=Math.floor(Math.max(0,personal.draft.length-1)/50)*50;renderPersonal();$<HTMLInputElement>(`list-alias-${personal.draft.length-1}`).focus();});
$('list-save').addEventListener('click',()=>void personal.save());
$('list-prev').addEventListener('click',()=>{listOffset=Math.max(0,listOffset-50);renderPersonal();});
$('list-next').addEventListener('click',()=>{listOffset+=50;renderPersonal();});
$<HTMLSelectElement>('import-encoding').addEventListener('change',()=>personal.chooseEncoding($<HTMLSelectElement>('import-encoding').value as ImportEncoding));
$('decode-import').addEventListener('click',()=>void personal.decode(personal.selectedEncoding));
$('confirm-import').addEventListener('click',async()=>{await personal.confirmImport();if(!personal.error&&!personal.selection){$<HTMLDialogElement>('import-dialog').close();returnAdvanced();}});
function renderPersonal(){
 for(const button of document.querySelectorAll<HTMLButtonElement>('#advanced-dialog .close-dialog'))button.disabled=personal.busy||controller.state.busy;
 $<HTMLSelectElement>('import-encoding').disabled=personal.busy;
 for(const id of ['profile-error','list-error','import-error']){$(id).hidden=!personal.error;$(id).textContent=personal.error??'';}
 $('profile-notice').textContent=personal.busy?'ממתין להשלמת הפעולה…':personal.notice??'';
 $('list-notice').textContent=personal.busy?'ממתין להשלמת הפעולה…':personal.notice??'';
 for(const id of ['edit-list','import-list','export-list','list-save','list-add','list-cancel','list-cancel-x','decode-import','cancel-import','import-cancel-x'])$<HTMLButtonElement>(id).disabled=personal.busy||controller.state.busy;
 $('list-rows').replaceChildren();
 listOffset=Math.min(listOffset,Math.floor(Math.max(0,personal.draft.length-1)/50)*50);
 for(let index=listOffset;index<Math.min(listOffset+50,personal.draft.length);index++){
  const row=personal.draft[index],entry=document.createElement('div');entry.className='list-row';
  entry.append(text('span',String(index+1),'list-index'));
  for(const [column,label] of [[1,'שם תיקייה'],[0,'שם אמן']] as const){const wrapper=document.createElement('label');wrapper.textContent=label;const input=document.createElement('input');input.id=`list-${column===0?'alias':'folder'}-${index}`;input.value=row[column]??'';input.dir='auto';input.disabled=personal.busy;input.setAttribute('aria-label',`${label}, שורה ${index+1}`);input.addEventListener('input',()=>personal.edit(index,column,input.value));wrapper.append(input);entry.append(wrapper);}
  const remove=document.createElement('button');remove.textContent='מחק';remove.className='text-button';remove.disabled=personal.busy;remove.setAttribute('aria-label',`מחק שורה ${index+1}`);remove.addEventListener('click',()=>{personal.remove(index);$('list-add').focus();});entry.append(remove);
  if(row.length!==2){entry.append(text('p',`מבנה לא תקין (${row.length} שדות): ${JSON.stringify(row)}. יש לתקן או למחוק במפורש.`,'list-row-warning'));}
  $('list-rows').append(entry);
 }
 $('list-range').textContent=personal.draft.length?`${listOffset+1}–${Math.min(listOffset+50,personal.draft.length)} מתוך ${personal.draft.length}`:'הרשימה ריקה';
 $<HTMLButtonElement>('list-prev').disabled=personal.busy||listOffset===0;$<HTMLButtonElement>('list-next').disabled=personal.busy||listOffset+50>=personal.draft.length;
 $('import-filename').textContent=personal.selection?.filename??'';
 $<HTMLSelectElement>('import-encoding').value=personal.selectedEncoding;
 $('import-summary').textContent=personal.preview?`${personal.preview.rows.length} שורות לייבוא; ${personal.preview.diagnostics.length} אזהרות מבנה.`:personal.selection?.encoding_required?'הקידוד לא זוהה בוודאות. בחר קידוד ובדוק את הטקסט לפני אישור.':'לא התקבלה עדיין תצוגה מקדימה.';
 $('import-rows').replaceChildren();
 for(const row of personal.preview?.rows.slice(0,20)??[])$('import-rows').append(text('li',row.length===2?`שם אמן: ${row[0]} · תיקייה: ${row[1]}`:`מבנה לא תקין (${row.length} שדות): ${JSON.stringify(row)}`));
 $<HTMLButtonElement>('confirm-import').disabled=personal.busy||!personal.preview||personal.preview.encoding!==personal.selectedEncoding;
}
personal.subscribe(renderPersonal);controller.subscribe(renderPersonal);renderPersonal();
