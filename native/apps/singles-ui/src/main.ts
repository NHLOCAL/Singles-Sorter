import './styles.css';
import {nativeHost,openFooterLink,openProjectLink} from './host';
import {installAbout} from './about';
import {readDeveloperMode,saveDeveloperMode,simpleStatus,simpleReason,simpleExecutionSummary,simpleError,simpleRepairReason} from './result-presentation';
import {kindLabel,reasonLabel,blockerLabel,warningPresentation,noticeLabel,sourceLabel} from './presentation';
import {PAGE_SIZE,PreviewController} from './state';
import type {Settings,ImportEncoding,Mode,PlanRow,ExecutionItem} from './contracts';
import {PersonalListController} from './lists';
import {installHelpBadges} from './help';
import {WorkspaceNavigation} from './navigation';
import {installIcons,iconElement,setButtonLabel} from './icons';
const controller=new PreviewController(nativeHost,nativeHost);
try{controller.state.developerMode=readDeveloperMode(localStorage);}catch{/* Fresh or unavailable storage keeps the simple view. */}
const personal=new PersonalListController(nativeHost,()=>controller.resourcesChanged());
let listOffset=0,importOffset=0;
const navigation=new WorkspaceNavigation();
let detailTrigger:HTMLButtonElement|null=null;
let resultsScroll=0;

const app=document.querySelector<HTMLDivElement>('#app')!;
// Static trusted markup only. Every host/file value is rendered with textContent.
app.innerHTML=`
<header class="topbar"><div class="brand"><img src="/icon.png" alt="" width="32" height="32"><h1>מסדר הסינגלים</h1></div><button id="menu" class="icon-button" aria-label="עזרה ומידע" aria-haspopup="dialog">⋮</button></header>
<main>
<div id="connection" class="notice" role="status" hidden></div>
<div id="error" class="error" role="alert" hidden></div>
<div class="workspace" id="setup-screen">
<section class="source-panel" aria-labelledby="folders-heading"><h2 id="folders-heading">תיקיות</h2>
<div class="folder"><div class="field-label"><label for="source">תיקית מקור</label><button class="help-badge" aria-label="עזרה: תיקית מקור" data-help-text="בחירת תיקיה המכילה את המוזיקה שברצונך לסדר.">?</button></div><div class="folder-row"><input id="source" dir="ltr" autocomplete="off" spellcheck="false" placeholder="C:\\Music\\סינגלים"><button id="source-picker" class="secondary">בחר תיקיה</button></div></div>
<div class="folder"><div class="field-label"><label for="target">תיקית יעד</label><button class="help-badge" aria-label="עזרה: תיקית יעד" data-help-text="בחירת תיקית יעד אליה יוכנסו תיקיות המוזיקה שיווצרו. נדרשת למיון בלבד.">?</button></div><div class="folder-row"><input id="target" dir="ltr" autocomplete="off" spellcheck="false" placeholder="D:\\המוזיקה שלי"><button id="target-picker" class="secondary">בחר תיקיה</button></div></div>
<div class="actions"><div class="action-command"><button id="sort" class="primary">מיין שירים</button><button class="help-badge" aria-label="עזרה: מיין שירים" data-help-text="מיון מתקדם של הסינגלים שלך בתיקיות לפי אמנים.">?</button></div><div class="action-command"><button id="fix" class="secondary">תקן שמות</button><button class="help-badge" aria-label="עזרה: תקן שמות" data-help-text="תיקון ג׳יבריש במאפייני הקובץ והסרת תוכן מיותר בשמות הקבצים.">?</button></div></div>
<button id="reopen-results" class="text-button" hidden>הצג תוצאות</button>
</section>
<aside class="options card" aria-labelledby="options-heading"><div class="section-title"><div class="field-label"><h2 id="options-heading">אפשרויות מיון</h2><button class="help-badge" aria-label="עזרה: אפשרויות מיון" data-help-text="חמש האפשרויות נשמרות בלחיצה על שמור הגדרות; מצב דואטים נשמר מיד בהגדרות מתקדמות.">?</button></div><span id="settings-label" class="quiet-label">ההגדרות שלך</span></div>
<fieldset><legend class="visually-hidden">בסיסי</legend><div class="option-row"><label class="option"><input id="copy_mode" type="checkbox" role="switch"><span id="copy_mode-label">העתק קבצים</span></label><button class="help-badge" aria-label="עזרה: העתק קבצים" data-help-text="קבע אם להעתיק או להעביר את הקבצים לתיקיית היעד">?</button></div><div class="option-row"><label class="option"><input id="main_folder_only" type="checkbox" role="switch"><span id="main_folder_only-label">סרוק רק תיקייה ראשית</span></label><button class="help-badge" aria-label="עזרה: סרוק רק תיקייה ראשית" data-help-text="בחר אם לסרוק את כל תיקיות המשנה או רק את תיקיית המקור הראשית">?</button></div></fieldset>
<fieldset><legend class="visually-hidden">מתקדם</legend><div class="option-row"><label class="option"><input id="singles_folder" type="checkbox"><span id="singles_folder-label">צור תיקיות סינגלים</span></label><button class="help-badge" aria-label="עזרה: צור תיקיות סינגלים" data-help-text="יצירת תיקייה ייעודית בשם ״סינגלים״ בתוך כל תיקיית אמן">?</button></div><div class="option-row"><label class="option"><input id="abc_sort" type="checkbox"><span id="abc_sort-label">מיון אלפביתי</span></label><button class="help-badge" aria-label="עזרה: מיון אלפביתי" data-help-text="יצירת תיקיות ראשיות לפי אותיות הא׳-ב׳ וארגון תיקיות האמנים בתוכן">?</button></div><div class="option-row"><label class="option"><input id="exist_only" type="checkbox"><span id="exist_only-label">שימוש בתיקיות קיימות</span></label><button class="help-badge" aria-label="עזרה: שימוש בתיקיות קיימות" data-help-text="העברת קבצים רק לתיקיות אמנים הקיימות כבר בתיקית היעד">?</button></div></fieldset>
<div class="options-bottom"><button id="advanced" class="text-button">הגדרות מתקדמות</button><button id="save-settings" class="text-button">שמור הגדרות</button></div><p id="settings-notice" role="status" class="hint"></p>
</aside></div>
<section id="processing-screen" hidden aria-labelledby="processing-title"><progress aria-label="מתבצעת בדיקה"></progress><h2 id="processing-title" tabindex="-1">קורא ומנתח את הקבצים…</h2><p id="busy" role="status" aria-live="polite"></p><button id="cancel-preview" class="text-button">ביטול</button></section>
<section id="results" class="results card" aria-labelledby="results-heading"><div class="results-toolbar"><button id="back-setup" class="text-button">חזרה להגדרות</button><div class="section-title"><h2 id="results-heading" tabindex="-1">תוכנית מיון</h2><span id="count" class="quiet-label" role="status"></span></div></div>
<section id="execution-panel" class="execution-panel" aria-label="ביצוע התוכנית" hidden><div class="execution-heading"><div><h3 id="execution-title">התוכנית מוכנה לבדיקה</h3><p id="execution-summary" role="status" aria-live="polite"></p></div><button id="review-apply" class="primary">בצע את התוכנית</button><button id="cancel-apply" class="secondary" hidden>עצור ביצוע</button></div><progress id="execution-progress" aria-label="התקדמות הקובץ הנוכחי" hidden></progress><p id="execution-recovery" class="warning" hidden></p><p id="recovery-inspection" class="hint" hidden></p><label id="journal-label" class="detail-path-label" hidden>יומן הפעולה<input id="journal-path" class="source-path" dir="ltr" readonly></label></section>
<div class="results-body" id="results-body" tabindex="0" aria-label="תוכן התוצאות">

<div id="empty" class="empty">התוצאה תופיע כאן לאחר בחירת תיקיות והפעלת בדיקה.</div>

<div id="warnings"></div>
<div id="table-wrap" hidden><table><thead><tr><th scope="col">שם הקובץ</th><th scope="col" id="result-second-heading">אמן</th><th scope="col" id="result-third-heading">מצב</th><th scope="col"><span class="visually-hidden">פרטים</span></th></tr></thead><tbody id="rows"></tbody></table></div>

<details id="details" hidden><summary>פרטי התכנית והבדיקה</summary><dl><dt>מזהה תכנית</dt><dd id="plan-id" dir="ltr"></dd><dt>חסימות ביצוע</dt><dd id="blockers"></dd></dl><div id="technical-notices"></div></details>
</div><nav id="pagination" aria-label="עמודי תוצאות" hidden><button id="previous" class="secondary">הקודם</button><span id="page-label"></span><button id="next" class="secondary">הבא</button></nav>
</section>
</main>
<footer>להורדת סינגלים חינם · <button id="footer-link" class="footer-link">שיר בוט</button></footer>
<dialog id="apply-dialog" aria-labelledby="apply-title"><div class="dialog-heading"><h2 id="apply-title">ביצוע התוכנית</h2><button id="apply-dismiss" class="icon-button" aria-label="חזור לתוכנית">×</button></div><p id="apply-description"></p><label class="detail-path-label">תיקית מקור<input id="apply-source" class="source-path" dir="ltr" readonly></label><label id="apply-target-label" class="detail-path-label">תיקית יעד<input id="apply-target" class="source-path" dir="ltr" readonly></label><p class="hint">הקבצים והתוכנית ייבדקו מחדש לפני הביצוע. עצירה עשויה להשאיר פעולות שכבר הושלמו; התוצאה תציג מה בוצע בפועל.</p><div class="dialog-actions"><button id="confirm-apply" class="primary">אשר ביצוע</button><button id="apply-back" class="secondary">חזור לתוכנית</button></div></dialog>
<dialog id="row-dialog" class="wide-dialog" aria-labelledby="row-title"><div class="dialog-heading"><h2 id="row-title"></h2><button id="row-close" class="icon-button" aria-label="סגור פרטים">×</button></div><div id="row-content"></div></dialog>
<dialog id="menu-dialog" aria-labelledby="menu-title"><div class="dialog-heading"><h2 id="menu-title">עזרה ומידע</h2><button class="close-dialog icon-button" aria-label="סגור">×</button></div><div class="menu-items" role="group" aria-label="נושאי מידע"><button data-help="help" aria-pressed="true" aria-controls="usage-content">איך משתמשים?</button><button data-help="about" aria-pressed="false" aria-controls="about-content">אודות התוכנה</button></div>
<div id="help-content" class="help-content">
<section id="usage-content" data-info-page="help" aria-labelledby="usage-title"><h3 id="usage-title">מהתיקייה לתוכנית המיון</h3><ol class="usage-steps"><li>בחר תיקית מקור, ובמיון גם תיקית יעד.</li><li>התאם את האפשרויות ולחץ על <strong>מיין שירים</strong> או <strong>תקן שמות</strong>. במצב תיקון אין צורך ביעד.</li><li>עיין בהצעות ובסיבות לחסימה. כשאפשר לבצע את התוכנית, יופיע כפתור ביצוע ואישור נוסף לפני שינוי הקבצים.</li></ol><p class="hint">רשימת האמנים האישית ומיון דואטים נמצאים בהגדרות מתקדמות. ההסבר ליד כל אפשרות זמין גם במקלדת ובמגע.</p></section>
<section id="about-content" data-info-page="about" aria-labelledby="about-name" hidden><div class="about-brand"><img src="/icon.png" width="44" height="44" alt=""><div><h3 id="about-name">מסדר הסינגלים</h3><p class="hint">סידור המוזיקה שלך בתיקיות לפי אמנים</p></div></div><dl class="about-facts"><div><dt>גרסת התוכנה</dt><dd id="about-version" dir="ltr"></dd></div><div><dt>כללי המיון</dt><dd>מבוססים על גרסה 14.0</dd></div><div><dt>פרויקט המקור</dt><dd><bdi>NHLOCAL / Singles-Sorter</bdi></dd></div></dl><p>מיון לפי רשימת אמנים, אפשרויות המיון והרשימה האישית שלך. זיהוי בעזרת המודלים המקוריים פועל מקומית כאשר הם זמינים.</p><p class="hint about-capability">הזמינות לביצוע מוצגת בכל תוכנית לפי המכשיר והפעולות הנדרשות. פעולות שאינן נתמכות נשארות לתצוגה בלבד.</p><div class="about-links"><button class="secondary" data-project-link="Source">קוד המקור ב־GitHub</button><button class="secondary" data-project-link="License">רישיון MIT</button></div><p class="hint about-browser-note">הקישורים נפתחים בדפדפן שלך.</p><p id="about-link-error" class="error" role="alert" hidden></p><details class="about-credits"><summary>רכיבים ותודות</summary><p>הפרויקט מבוסס על מסדר הסינגלים המקורי של NHLOCAL, רשימת האמנים והמודלים המקוריים לזיהוי שמות אמנים.</p><p><bdi>Rust</bdi> למנוע המיון, <bdi>Tauri</bdi> למארח שולחן העבודה, <bdi>TagLib</bdi> לקריאת מאפייני קבצים, וסמלים מתוך <bdi>Microsoft Fluent UI System Icons</bdi>.</p><p class="hint">קוד הפרויקט מופץ ברישיון MIT. לרכיבי צד שלישי ולמודלים יש תנאי רישוי נפרדים; הודעות הרישוי הנלוות לחבילה הן המקור המלא.</p></details></section></div><details class="developer-options"><summary>אפשרויות תצוגה</summary><label class="option"><input id="developer-mode" type="checkbox" role="switch"><span>מצב מפתחים</span></label><p class="hint">מציג את כל פרטי הבדיקה והיומנים. אינו משנה את המיון או את הקבצים.</p></details></dialog>
<dialog id="advanced-dialog" aria-labelledby="advanced-title"><div class="dialog-heading"><h2 id="advanced-title">הגדרות מתקדמות</h2><button class="close-dialog icon-button" aria-label="סגור">×</button></div>
<fieldset><legend>מיון דואטים <button class="help-badge" aria-label="עזרה: דואטים" data-help-text="בחר להיכן יועתקו שירי דואט. הבחירה נשמרת מיד, בלי לשמור שינויים באפשרויות האחרות.">?</button></legend><label class="option"><input type="radio" name="duet" id="duet-first" value="first" checked><span>לזמר הראשון בשם השיר</span></label><label class="option"><input type="radio" name="duet" id="duet-all" value="all"><span>לכל הזמרים המופיעים בשיר</span></label></fieldset>
<fieldset><legend class="ai-heading"><span>זיהוי בעזרת AI</span><span id="model-status" class="quiet-label"></span><button id="model-help" class="help-badge" aria-label="עזרה: זמינות AI" data-help-text="">?</button></legend><div class="field-label"><label for="ai-profile">פרופיל הבדיקה</label><button class="help-badge" aria-label="עזרה: פרופיל הבדיקה" data-help-text="אם נדרש מודל ואינו זמין, הבדיקה תיחסם. אין מעבר אוטומטי לפרופיל אחר.">?</button></div><select id="ai-profile"><option value="Disabled">ללא AI</option><option value="Configured">דרוש מודל AI זמין</option></select></fieldset>
<section><div class="field-label"><h3>הוספת זמרים</h3><button class="help-badge" aria-label="עזרה: רשימת אמנים" data-help-text="השורות האישיות מתווספות אחרי הרשימה המובנית. סדר ההתאמה נשמר.">?</button></div><div class="list-actions"><button id="edit-list" class="secondary">ערוך רשימה</button><button id="import-list" class="secondary">יבא קובץ CSV</button><button id="export-list" class="secondary">יצא לקובץ CSV</button></div><p id="profile-notice" class="hint" role="status"></p><p id="profile-error" class="error" role="alert" hidden></p></section><p id="settings-error" class="error" role="alert" hidden></p><button class="close-dialog primary">סגור</button></dialog>
<dialog id="list-dialog" class="wide-dialog editor-dialog" aria-labelledby="list-title"><div class="dialog-heading"><h2 id="list-title">הוספת זמרים</h2><button id="list-cancel-x" class="icon-button" aria-label="סגור בלי לשמור">×</button><button class="help-badge" aria-label="עזרה: עריכת רשימה" data-help-text="שם האמן הוא הטקסט לחיפוש; שם התיקייה הוא היעד. אין שינוי סדר או הסרת כפילויות אוטומטיים.">?</button></div><div class="editor-content"><div class="editor-config"><p id="list-error" class="error" role="alert" hidden></p><p id="list-notice" role="status"></p></div><div class="editor-row-scroll" tabindex="0" aria-label="שורות לעריכה"><div class="editor-columns" aria-hidden="true"><span></span><span>שם תיקיה</span><span>שם זמר</span><span></span></div><div id="list-rows"></div></div></div><div class="editor-footer"><nav class="list-pagination" aria-label="עמודי רשימת אמנים"><button id="list-prev" class="secondary">הקודם</button><span id="list-range"></span><button id="list-next" class="secondary">הבא</button></nav><div class="dialog-actions"><button id="list-add" class="secondary">הוסף שורה</button><button id="list-save" class="primary">שמור רשימה</button><button id="list-cancel" class="secondary">ביטול</button></div></div></dialog>
<dialog id="import-dialog" class="wide-dialog editor-dialog" aria-labelledby="import-title"><div class="dialog-heading"><h2 id="import-title">בדיקת קובץ הייבוא</h2><button id="import-cancel-x" class="icon-button" aria-label="בטל ייבוא">×</button><button class="help-badge" aria-label="עזרה: ייבוא רשימה" data-help-text="האישור מוסיף את כל השורות בסוף הרשימה. שורות לא תקינות וכפילויות אינן מוסרות אוטומטית. אפשר לערוך או למחוק שורות לפני האישור.">?</button></div><div class="editor-content"><div class="editor-config"><p id="import-filename" dir="auto"></p><label for="import-encoding">קידוד הקובץ</label><select id="import-encoding"><option value="Utf8">UTF-8</option><option value="Utf8Bom">UTF-8 עם BOM</option><option value="Windows1255">Windows-1255 (עברית)</option><option value="Windows1252">Windows-1252</option><option value="Utf16Le">UTF-16 LE</option><option value="Utf16Be">UTF-16 BE</option></select><button id="decode-import" class="secondary">הצג לפי הקידוד שבחרתי</button><p id="import-error" class="error" role="alert" hidden></p><p id="import-summary"></p></div><div class="editor-row-scroll" tabindex="0" aria-label="שורות לעריכה"><div class="editor-columns" aria-hidden="true"><span></span><span>שם תיקיה</span><span>שם זמר</span><span></span></div><div id="import-rows"></div></div></div><div class="editor-footer"><nav class="list-pagination" aria-label="עמודי הייבוא"><button id="import-prev" class="secondary">הקודם</button><span id="import-range"></span><button id="import-next" class="secondary">הבא</button></nav><div class="dialog-actions"><button id="confirm-import" class="primary">אשר הוספה לרשימה</button><button id="cancel-import" class="secondary">ביטול</button></div></div></dialog>`;
// Keep the historical footer destination, verified from the released source.
const $=<T extends HTMLElement=HTMLElement>(id:string)=>document.getElementById(id) as T;
const source=$<HTMLInputElement>('source'),target=$<HTMLInputElement>('target');
$('footer-link').addEventListener('click',async()=>{try{await openFooterLink();}catch{ $('error').textContent='לא ניתן לפתוח כעת את הקישור בדפדפן. כתובת האתר: https://nhlocal.github.io/shir-bot';$('error').hidden=false; }});
source.addEventListener('input',()=>controller.editRoot('Source',source.value));
target.addEventListener('input',()=>controller.editRoot('Target',target.value));
$('source-picker').addEventListener('click',()=>void controller.selectRoot('Source'));
$('target-picker').addEventListener('click',()=>void controller.selectRoot('Target'));
function hasResults(){return controller.state.snapshot?.status==='completed'&&!!controller.state.page;}
function showSetup(){if(controller.state.activity==='apply')return;if(controller.state.activity==='preview'){controller.cancelPreview();return;}if(navigation.screen==='results')resultsScroll=$('results-body').scrollTop;navigation.setup();render();source.focus();}
function showResults(push=true){navigation.results(hasResults());if(push)history.pushState({singlesView:'results',jobId:controller.state.snapshot?.job_id},'');render();$('results-heading').focus();requestAnimationFrame(()=>{$('results-body').scrollTop=resultsScroll;});}
function backToSetup(){if(history.state?.singlesView==='results')history.back();else showSetup();}
history.replaceState({singlesView:'setup'},'');
window.addEventListener('popstate',()=>{
 if(controller.state.activity==='apply'){history.replaceState({singlesView:'results',jobId:controller.state.snapshot?.job_id},'');return;}
 if(controller.state.activity==='preview'){controller.cancelPreview();history.replaceState({singlesView:'setup'},'');return;}
 const valid=hasResults()&&history.state?.jobId===controller.state.snapshot?.job_id;
 if($<HTMLDialogElement>('row-dialog').open){
  $<HTMLDialogElement>('row-dialog').close();detailTrigger?.focus();
  if(history.state?.singlesView==='results'&&valid)return;
 }
 if(history.state?.singlesView==='detail'&&valid){
  const row=controller.state.page?.rows.find(row=>row.id===history.state.rowId);
  if(row){showResults(false);const trigger=[...document.querySelectorAll<HTMLButtonElement>('.detail-button')].find(button=>button.dataset.rowId===row.id);if(trigger){openRow(row,trigger,false);return;}}
 }
 if(history.state?.singlesView==='results'&&valid)showResults(false);
 else{history.replaceState({singlesView:'setup'},'');showSetup();}
});
$('cancel-preview').addEventListener('click',()=>controller.cancelPreview());
$('back-setup').addEventListener('click',backToSetup);
$('review-apply').addEventListener('click',()=>{
 if(!controller.canApply)return;
 const readiness=controller.state.snapshot!.apply!;
 const repairing=readiness.mode==='FixNames';
 $('apply-title').textContent=repairing?'תיקון שמות ותגיות':'ביצוע התוכנית';
 $('apply-target-label').hidden=repairing;
 $('apply-description').textContent=repairing?'תיקון שמות ותגיות בקבצים שברשימה. לפני החלפת תגיות נשמר עותק מקורי לצד הקובץ.':controller.state.developerMode?`${readiness.copy_mode?'העתקת':'העברת'} ${readiness.operation_count.toLocaleString('he-IL')} פעולות לפי התוכנית שהוצגה.`:readiness.copy_mode?'העתקת הקבצים שברשימה. קובצי המקור יישארו במקומם.':'העברת הקבצים שברשימה אל תיקיית היעד.';
 $<HTMLInputElement>('apply-source').value=readiness.source;$<HTMLInputElement>('apply-target').value=readiness.target??'';
 openDialog('apply-dialog');$('apply-back').focus();
});
for(const id of ['apply-dismiss','apply-back'])$(id).addEventListener('click',()=>{$<HTMLDialogElement>('apply-dialog').close();$('review-apply').focus();});
$('confirm-apply').addEventListener('click',async()=>{$<HTMLDialogElement>('apply-dialog').close();await controller.apply();$('execution-title').tabIndex=-1;$('execution-title').focus();});
$('cancel-apply').addEventListener('click',()=>controller.cancelApply());
$('reopen-results').addEventListener('click',()=>showResults());
async function runPreview(mode:Mode){
 if(controller.state.busy||personal.busy||!navigation.start(mode))return;
 history.pushState({singlesView:'processing'},'');
 resultsScroll=0;render();$('processing-title').focus();
 await controller.preview(mode);navigation.finish(hasResults());
 history.replaceState({singlesView:'setup'},'');
 if(hasResults())showResults();else{render();if(controller.state.error)$('error').focus();else source.focus();}
}
function closeRow(){if(history.state?.singlesView==='detail')history.back();else{$<HTMLDialogElement>('row-dialog').close();detailTrigger?.focus();}}
$('row-close').addEventListener('click',closeRow);
$('row-dialog').addEventListener('cancel',event=>{event.preventDefault();closeRow();});
function openRow(row:PlanRow,trigger:HTMLButtonElement,push=true){
 detailTrigger=trigger;$('row-title').textContent=sourceLabel(row.source,controller.state.model?.platform);$('row-content').replaceChildren();
 if(!controller.state.developerMode){
  const status=rowStatus(row);
  if(row.artist)$('row-content').append(text('p',row.artist,'detail-artist'));
  $('row-content').append(text('p',status,'detail-status'));
  for(const item of executionItems(row)){const repairReason=simpleRepairReason(item);if(repairReason)$('row-content').append(text('p',repairReason,'hint'));}
  const reason=simpleReason(row,controller.state.snapshot?.apply?.supported);if(reason&&!executionItems(row).length)$('row-content').append(text('p',reason,'hint'));
  if(executionItems(row).some(item=>['failed','needs_recovery','source_retained'].includes(item.status)))$('row-content').append(text('p',row.kind==='FixNames'?'בדוק את הקובץ ואת הגיבוי בתיקיית המקור לפני ניסיון נוסף.':'בדוק את הקובץ בתיקיית המקור ובתיקיית היעד לפני ניסיון נוסף.','hint'));
  if(push)history.pushState({singlesView:'detail',jobId:controller.state.snapshot?.job_id,rowId:row.id},'');openDialog('row-dialog');return;
 }

 for(const [label,value]of [['מקור',row.source],['יעד מוצע',row.destination]]as const){if(value===null)continue;const wrapper=text('label',label,'detail-path-label');const input=document.createElement('input');input.className='source-path';input.readOnly=true;input.dir='ltr';input.value=value;input.setAttribute('aria-label',label);input.addEventListener('focus',()=>input.select());wrapper.append(input);$('row-content').append(wrapper);}
 const description=text('p',reasonLabel(row),'detail-reason');$('row-content').append(description);
 for(const item of executionItems(row))$('row-content').append(text('p',`${itemLabel(item)} · ${item.detail}`,'execution-item'));
 if(row.blockers.length)$('row-content').append(text('p',row.blockers.map(blockerLabel).join(' · '),'warning'));
 const technical=document.createElement('details');technical.append(text('summary','מידע טכני'),text('p',row.reason),text('p',row.blockers.join(' · ')));$('row-content').append(technical);
 if(push)history.pushState({singlesView:'detail',jobId:controller.state.snapshot?.job_id,rowId:row.id},'');openDialog('row-dialog');
}
$('sort').addEventListener('click',()=>void runPreview('Sort'));
$('fix').addEventListener('click',()=>void runPreview('FixNames'));
const keys:['copy_mode','main_folder_only','singles_folder','abc_sort','exist_only']=['copy_mode','main_folder_only','singles_folder','abc_sort','exist_only'];
for(const key of keys)$<HTMLInputElement>(key).addEventListener('change',event=>controller.changeSetting(key,(event.target as HTMLInputElement).checked));
$('duet-first').addEventListener('change',()=>void controller.setDuet(false));
$('duet-all').addEventListener('change',()=>void controller.setDuet(true));
$('save-settings').addEventListener('click',()=>void controller.saveMainSettings());
$<HTMLSelectElement>('ai-profile').addEventListener('change',event=>controller.changeAi((event.target as HTMLSelectElement).value as 'Disabled'|'Configured'));
function syncPersonalNotices(){
 const editing=$<HTMLDialogElement>('list-dialog').open||$<HTMLDialogElement>('import-dialog').open;
 $('profile-notice').hidden=editing;
}
function openDialog(id:string){$<HTMLDialogElement>(id).showModal();syncPersonalNotices();}
for(const id of ['list-dialog','import-dialog'])$(id).addEventListener('close',syncPersonalNotices);
$('menu').addEventListener('click',()=>openDialog('menu-dialog'));
$('advanced').addEventListener('click',()=>openDialog('advanced-dialog'));
$('advanced-dialog').addEventListener('close',()=>{if(!document.querySelector('dialog[open]')&&navigation.screen==='setup')$('advanced').focus();});
$('menu-dialog').addEventListener('close',()=>{if(!document.querySelector('dialog[open]'))$('menu').focus();});
for(const button of document.querySelectorAll<HTMLButtonElement>('.close-dialog'))button.addEventListener('click',()=>button.closest('dialog')?.close());
installIcons(app);
installHelpBadges(app);
// Respect enlarged text without hiding controls or trapping the page in a fixed height.
const main=app.querySelector('main')!,workspace=app.querySelector<HTMLElement>('.workspace')!,connection=$('connection'),errorBox=$('error');
function fitWorkspace(){
 const styles=getComputedStyle(main);
 const gap=parseFloat(styles.rowGap==='normal'?'0':styles.rowGap);
 const noticeHeight=[connection,errorBox].reduce((height,element)=>height+(element.hidden?0:element.getBoundingClientRect().height+gap),0);
 const screenHeight=navigation.screen==='setup'?workspace.getBoundingClientRect().height:220;
 const required=noticeHeight+screenHeight+app.querySelector('header')!.getBoundingClientRect().height+app.querySelector('footer')!.getBoundingClientRect().height+parseFloat(styles.paddingTop)+parseFloat(styles.paddingBottom)+parseFloat(styles.rowGap==='normal'?'0':styles.rowGap)+160;
 app.classList.toggle('reflow',required>window.innerHeight||parseFloat(getComputedStyle(document.documentElement).fontSize)>21);
}
const layoutObserver=new ResizeObserver(fitWorkspace);
for(const element of [workspace,connection,errorBox,app.querySelector('header')!,app.querySelector('footer')!])layoutObserver.observe(element);
window.addEventListener('resize',fitWorkspace);fitWorkspace();
installAbout(app,openProjectLink);
$('developer-mode').addEventListener('change',async()=>{
 const enabled=$<HTMLInputElement>('developer-mode').checked;
 await controller.setDeveloperMode(enabled);
 if(controller.state.developerMode===enabled)try{saveDeveloperMode(localStorage,enabled);}catch{/* Session preference remains usable. */}
});
$('previous').addEventListener('click',()=>void controller.changePage(Math.max(0,controller.state.offset-PAGE_SIZE)));
$('next').addEventListener('click',()=>void controller.changePage(controller.state.offset+PAGE_SIZE));
function text(tag:string,value:string,className?:string){const node=document.createElement(tag);node.textContent=value;if(className)node.className=className;return node;}
const executionTitles:Record<string,string>={preparing:'מאמת את הקבצים לפני ביצוע…',running:'מבצע את התוכנית…',complete:'הביצוע הושלם',partial:'הביצוע הושלם חלקית',cancelled:'הביצוע נעצר',failed:'הביצוע לא הושלם',needs_recovery:'נדרשת בדיקת התאוששות'};
function itemLabel(item:ExecutionItem){return ({complete:'בוצע',source_retained:'היעד נכתב; המקור נשמר',failed:'נכשל',cancelled:'בוטל',blocked:'לא בוצע',needs_recovery:'נדרשת בדיקה',unsupported_tags:'תגיות לא נתמכות',metadata_failure:'קריאת התגיות נכשלה'})[item.status];}
function executionItems(row:PlanRow):ExecutionItem[]{return controller.state.execution?.result?.items.filter(item=>row.operation_ids?.includes(item.operation))??[];}
function rowStatus(row:PlanRow){
 const s=controller.state;
 return simpleStatus(row,executionItems(row),s.snapshot?.apply?.copy_mode??s.settings.copy_mode,{repairEligible:s.snapshot?.apply?.supported,terminalResult:!!s.execution?.result&&!['preparing','running'].includes(s.execution.status),uncertain:!!s.execution&&!s.execution.result&&!['preparing','running'].includes(s.execution.status)});
}
function renderExecution(){
 const s=controller.state,e=s.execution,readiness=s.snapshot?.apply;
 $('execution-panel').hidden=!readiness&&!e;
 $('review-apply').hidden=!!e||!readiness?.supported;$<HTMLButtonElement>('review-apply').disabled=!controller.canApply;
 const running=s.activity==='apply';$('cancel-apply').hidden=!running;$<HTMLButtonElement>('cancel-apply').disabled=s.cancelRequested;
 $<HTMLButtonElement>('back-setup').disabled=running;
 $('execution-title').textContent=e?executionTitles[e.status]:running?'מאמת את הקבצים לפני ביצוע…':readiness?.supported?'התוכנית מוכנה לביצוע':'התוכנית זמינה לתצוגה בלבד';
 const r=e?.result;
 const developer=s.developerMode;
 $('execution-summary').textContent=developer?(r?r.repair?`${r.repair.renamed_files} שמות תוקנו · ${r.repair.repaired_files} תגיות תוקנו · ${r.repair.unchanged_files} ללא שינוי בתגיות · ${r.repair.unsupported_files} לא נתמכים · ${r.repair.retained_backup_bytes.toLocaleString('he-IL')} בתים נשמרו בגיבוי`:`${r.committed_destinations.toLocaleString('he-IL')} יעדים נכתבו · ${r.moved_sources.toLocaleString('he-IL')} קבצי מקור הועברו · ${r.failed_operations.toLocaleString('he-IL')} נכשלו · ${r.blocked_operations.toLocaleString('he-IL')} לא בוצעו`:s.cancelRequested&&running?'ממתין לעצירה בטוחה…':e?.status==='running'&&e.operation!==null?`פעולה ${e.operation+1} מתוך ${e.operation_count} · ${e.bytes_written.toLocaleString('he-IL')} מתוך ${e.source_length.toLocaleString('he-IL')} בתים`:e&&!['preparing','running'].includes(e.status)?(e.error??'לא התקבלו מוני ביצוע מאומתים. יש לבדוק את היומן.'):readiness?.supported?`${readiness.operation_count.toLocaleString('he-IL')} פעולות ${readiness.mode==='FixNames'?'תיקון':readiness.copy_mode?'העתקה':'העברה'} ממתינות לאישור`:readiness?.reason??''):(e?simpleExecutionSummary(e,readiness?.copy_mode??s.settings.copy_mode):readiness?.supported?'הקבצים מוכנים. אפשר לבדוק ולאשר את הביצוע.':'לא בוצעו שינויים בקבצים.');
 if(!developer&&running)$('execution-summary').textContent=s.cancelRequested?'ממתין לעצירה בטוחה…':e?.operation!==null&&e?.operation!==undefined?`התקדמות: ${e.operation+1} מתוך ${e.operation_count}`:'בודק שהקבצים מוכנים…';
 const progress=$<HTMLProgressElement>('execution-progress');progress.hidden=!running;
 if(e?.status==='running'&&e.source_length>0){progress.max=e.source_length;progress.value=Math.min(e.bytes_written,e.source_length);}else progress.removeAttribute('value');
 $('execution-recovery').hidden=!e||!['needs_recovery','partial','cancelled','failed'].includes(e.status);
 $('execution-recovery').textContent=developer?(e?.status==='needs_recovery'?'היומן נשמר. אין להפעיל שוב את התוכנית או למחוק את היומן לפני בדיקת הקבצים. התאוששות אוטומטית אינה מופעלת.':'פעולות שהושלמו נשארות בתוקף. עיין בפרטי הקבצים וביומן לפני בדיקה חדשה.'):readiness?.mode==='FixNames'?'בדוק את הקבצים והגיבויים בתיקיית המקור לפני ניסיון נוסף.':'קבצים שכבר טופלו נשארים במקומם. בדוק את המקור והיעד לפני ניסיון נוסף.';
 $('recovery-inspection').hidden=!developer||!e?.recovery;$('recovery-inspection').textContent=e?.recovery?`נבדק היומן מול הקבצים: ${e.recovery.transfers.length} פעולות · ${readiness?.mode==='FixNames'?'תיקון שמות ותגיות':`${e.recovery.pending_directories} יצירות תיקייה ממתינות לבירור`}${e.recovery.incomplete_tail?' · רשומה אחרונה לא שלמה':''}`:'';
 $('journal-label').hidden=!developer||!e?.journal_path;$<HTMLInputElement>('journal-path').value=e?.journal_path??'';
}
function render(){
 const s=controller.state;const blocked=s.busy||!s.hostAvailable;
 const developer=s.developerMode;app.dataset.developerMode=String(developer);
 $<HTMLInputElement>('developer-mode').checked=developer;$<HTMLInputElement>('developer-mode').disabled=s.busy;
 $('result-second-heading').textContent=developer?'פעולה':'אמן';$('result-third-heading').textContent=developer?'סיבה / חסימה':'מצב';
 app.dataset.view=navigation.screen;
 $('setup-screen').hidden=navigation.screen!=='setup';$('processing-screen').hidden=navigation.screen!=='processing';$('results').hidden=navigation.screen!=='results';$('reopen-results').hidden=!hasResults();
 $('results-heading').textContent=(s.snapshot?.apply?.mode??navigation.mode)==='Sort'?'תוכנית מיון':'תוכנית תיקון שמות';
 if(source.value!==s.source.text)source.value=s.source.text;if(target.value!==s.target.text)target.value=s.target.text;
 source.disabled=blocked;target.disabled=blocked;source.readOnly=s.model?.platform==='android';target.readOnly=s.model?.platform==='android';
 for(const [id,pending]of [['source-picker',s.source.pending],['target-picker',s.target.pending]]as const){$<HTMLButtonElement>(id).disabled=blocked||pending;setButtonLabel($(id),pending?'ממתין לבחירה…':'בחר תיקיה',id==='source-picker'?'source':'folder');}
 for(const id of ['sort','fix'])$<HTMLButtonElement>(id).disabled=blocked||s.source.pending||s.target.pending;
 for(const key of keys){const input=$<HTMLInputElement>(key);input.checked=s.settings[key as keyof Settings];input.disabled=blocked;}
 $('copy_mode-label').textContent=s.settings.copy_mode?'העתק קבצים':'העבר קבצים';
 $('main_folder_only-label').textContent=s.settings.main_folder_only?'סרוק תיקיה ראשית בלבד':'סרוק עץ תיקיות';
 $<HTMLInputElement>('duet-first').checked=!s.settings.duet_mode;$<HTMLInputElement>('duet-all').checked=s.settings.duet_mode;
 for(const id of ['duet-first','duet-all','ai-profile'])$(id).toggleAttribute('disabled',blocked);
 $<HTMLSelectElement>('ai-profile').value=s.ai;
 $('model-status').textContent=s.model?.available?'זמין':'לא זמין';
 $('model-help').dataset.helpText=s.model?.message??'לא התקבל מצב המודל מהמארח.';
 const previewCancelled=s.snapshot?.status==='cancelled'&&!s.busy&&!s.execution;
 $('connection').hidden=s.hostAvailable&&!previewCancelled;
 $('connection').textContent=previewCancelled?'הבדיקה בוטלה. לא בוצעו שינויים בקבצים.':!s.initialized?(s.startupWaiting?'ממתין לסיום הבדיקה הקודמת…':'טוען את המארח המקומי…'):'המארח המקומי אינו מחובר. יש לפתוח את התוכנה כדי לבדוק תיקיות.';
 $('settings-label').textContent=controller.settingsDirty?'שינויים שלא נשמרו':'ההגדרות שלך';$('settings-notice').textContent=noticeLabel(controller.savedNotice);$<HTMLButtonElement>('save-settings').disabled=blocked;$('settings-error').hidden=!s.error;$('settings-error').textContent=s.error?(developer?s.error:simpleError(s.error)):'';
 $<HTMLButtonElement>('cancel-preview').disabled=!s.busy||s.cancelRequested;
 $('busy').textContent=s.cancelRequested&&s.activity==='preview'?'מבטל את הבדיקה…':s.activity==='settings'?'שומר הגדרות…':s.activity==='page'?'טוען עמוד תוצאות…':'קורא ומנתח את הקבצים…';
 $('busy').hidden=!s.busy;$('results').setAttribute('aria-busy',String(s.busy));
 $('error').hidden=!s.error;$('error').textContent=s.error?(developer?s.error:simpleError(s.error)):'';$('error').tabIndex=-1;
 const ready=s.snapshot?.status==='completed'&&!!s.page;
 $('empty').hidden=s.busy||!!s.error||ready&&s.page!.total>0;
 $('empty').textContent=ready?(s.snapshot?.apply?.mode??navigation.mode)==='FixNames'?'לא נמצאו קבצים לתיקון.':'לא נמצאו קבצים למיון.':'התוצאה תופיע כאן לאחר בחירת תיקיות והפעלת בדיקה.';
 $('table-wrap').hidden=!ready||!s.page!.rows.length;
 $('count').textContent=ready?`${s.page!.total.toLocaleString('he-IL')} ${developer?'רשומות':s.page!.total===1?'פריט':'פריטים'}${s.page!.skipped?` · ${s.page!.skipped.toLocaleString('he-IL')} דולגו`:''}`:'';
 $('rows').replaceChildren();
 if(ready)for(const row of s.page!.rows){
  const tr=document.createElement('tr');
  const origin=document.createElement('td');origin.className='file-name';const name=text('bdi',sourceLabel(row.source,s.model?.platform),'source-name');name.dir='auto';if(developer)name.title=row.source;origin.append(name);tr.append(origin);
  const items=executionItems(row);const action=text('td',developer?(items.length?items.map(itemLabel).join(' · '):kindLabel(row.kind)):(row.artist??'—'),'row-action');action.dir='auto';tr.append(action);
  const reason=document.createElement('td');reason.className='row-reason';reason.append(text('span',developer?`${kindLabel(row.kind)} · `:row.artist?`${row.artist} · `:'','mobile-action'),text('span',developer?(row.blockers.length?row.blockers.map(blockerLabel).join(' · '):reasonLabel(row)):rowStatus(row)));tr.append(reason);
  const detail=document.createElement('td');detail.className='row-detail';const button=document.createElement('button');button.className='icon-button detail-button';button.dataset.rowId=row.id;button.setAttribute('aria-label',`פרטים: ${sourceLabel(row.source,s.model?.platform)}`);button.append(iconElement('info'));button.addEventListener('click',()=>openRow(row,button));detail.append(button);tr.append(detail);$('rows').append(tr);
 }
 $('warnings').replaceChildren();$('technical-notices').replaceChildren();
 if(ready)for(const warning of s.snapshot!.warnings){
  const display=warningPresentation(warning);
  if(display.technical)$('technical-notices').append(text('p',warning));
  else if(developer)$('warnings').append(text('p',display.label,'warning'));
  else if(!$('warnings').childElementCount)$('warnings').append(text('p','יש פריטים שדורשים בדיקה. עיין במצב של כל קובץ.','warning'));
 }
 $('details').hidden=!ready||!developer;$('plan-id').textContent=s.snapshot?.plan_id??'';$('blockers').textContent=s.snapshot?.blockers.join(' · ')||'ללא חסימות נוספות שדווחו.';
 $('pagination').hidden=!ready||s.page!.total<=PAGE_SIZE;
 $('page-label').textContent=ready?`${s.offset+1}–${Math.min(s.offset+PAGE_SIZE,s.page!.total)} מתוך ${s.page!.total}`:'';
 renderExecution();
 $<HTMLButtonElement>('previous').disabled=s.busy||s.offset===0;$<HTMLButtonElement>('next').disabled=s.busy||!ready||s.offset+PAGE_SIZE>=s.page!.total;
}
document.documentElement.dataset.startupState='loading';
controller.subscribe(render);render();void controller.initialize().then(()=>{
 document.documentElement.dataset.startupState=controller.state.hostAvailable?'ready':'failed';
 if(controller.state.execution&&hasResults())showResults();
});
// Personal-list dialogs operate on ordered rows; all persistent writes are host commands.
function returnAdvanced(){if(!$<HTMLDialogElement>('advanced-dialog').open)openDialog('advanced-dialog');}
function closeList(){if(personal.busy)return;personal.cancel();$<HTMLDialogElement>('list-dialog').close();returnAdvanced();}
function closeImport(){if(personal.busy)return;personal.cancel();$<HTMLDialogElement>('import-dialog').close();returnAdvanced();}
$('edit-list').addEventListener('click',async()=>{if(controller.state.busy||personal.busy)return;await personal.open();if(personal.error)return;listOffset=0;renderPersonal();$<HTMLDialogElement>('advanced-dialog').close();openDialog('list-dialog');});
$('import-list').addEventListener('click',async()=>{if(controller.state.busy||personal.busy)return;await personal.selectImport();if(!personal.selection)return;importOffset=0;renderPersonal();$<HTMLDialogElement>('advanced-dialog').close();openDialog('import-dialog');});
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
$('decode-import').addEventListener('click',()=>{importOffset=0;void personal.decode(personal.selectedEncoding);});
$('import-prev').addEventListener('click',()=>{importOffset=Math.max(0,importOffset-50);renderPersonal();});
$('import-next').addEventListener('click',()=>{importOffset+=50;renderPersonal();});
$('confirm-import').addEventListener('click',async()=>{await personal.confirmImport();if(!personal.error&&!personal.selection){$<HTMLDialogElement>('import-dialog').close();returnAdvanced();}});
function renderPersonal(){
 for(const button of document.querySelectorAll<HTMLButtonElement>('#advanced-dialog .close-dialog'))button.disabled=personal.busy||controller.state.busy;
 $<HTMLSelectElement>('import-encoding').disabled=personal.busy;
 for(const id of ['profile-error','list-error','import-error']){$(id).hidden=!personal.error;$(id).textContent=personal.error??'';}
 $('profile-notice').textContent=personal.busy?'ממתין להשלמת הפעולה…':noticeLabel(personal.notice);
 $('list-notice').textContent=personal.busy?'ממתין להשלמת הפעולה…':noticeLabel(personal.notice);
 for(const id of ['edit-list','import-list','export-list','list-save','list-add','list-cancel','list-cancel-x','decode-import','cancel-import','import-cancel-x'])$<HTMLButtonElement>(id).disabled=personal.busy||controller.state.busy;
 $('list-rows').replaceChildren();
 listOffset=Math.min(listOffset,Math.floor(Math.max(0,personal.draft.length-1)/50)*50);
 for(let index=listOffset;index<Math.min(listOffset+50,personal.draft.length);index++){
  const row=personal.draft[index],entry=document.createElement('div');entry.className='list-row';
  entry.append(text('span',String(index+1),'list-index'));
  for(const [column,label] of [[1,'שם תיקיה'],[0,'שם זמר']] as const){const wrapper=document.createElement('label');wrapper.append(text('span',label,'editor-cell-label'));const input=document.createElement('input');input.id=`list-${column===0?'alias':'folder'}-${index}`;input.value=row[column]??'';input.dir='auto';input.disabled=personal.busy||controller.state.busy;input.setAttribute('aria-label',`${label}, שורה ${index+1}`);input.addEventListener('input',()=>personal.edit(index,column,input.value));wrapper.append(input);entry.append(wrapper);}
  const remove=document.createElement('button');setButtonLabel(remove,'מחק','delete');remove.className='text-button';remove.disabled=personal.busy||controller.state.busy;remove.setAttribute('aria-label',`מחק שורה ${index+1}`);remove.addEventListener('click',()=>{personal.remove(index);const next=Math.min(index,personal.draft.length-1);if(next>=0)$(`list-alias-${next}`).focus();else $('list-add').focus();});entry.append(remove);
  if(row.length!==2){entry.append(text('p',`מבנה לא תקין (${row.length} שדות): ${JSON.stringify(row)}. יש לתקן או למחוק במפורש.`,'list-row-warning'));}
  $('list-rows').append(entry);
 }
 $('list-range').textContent=personal.draft.length?`${listOffset+1}–${Math.min(listOffset+50,personal.draft.length)} מתוך ${personal.draft.length}`:'הרשימה ריקה';
 $<HTMLButtonElement>('list-prev').disabled=personal.busy||listOffset===0;$<HTMLButtonElement>('list-next').disabled=personal.busy||listOffset+50>=personal.draft.length;
 $('import-filename').textContent=personal.selection?.filename??'';
 $<HTMLSelectElement>('import-encoding').value=personal.selectedEncoding;
 $('import-summary').textContent=personal.preview?`${personal.importRows.length} שורות לייבוא`:personal.selection?.encoding_required?'בחר קידוד ובדוק את הטקסט לפני אישור.':'';
 $('import-rows').replaceChildren();
 const imports=personal.importRows;importOffset=Math.min(importOffset,Math.floor(Math.max(0,imports.length-1)/50)*50);
 for(const {row,index}of imports.slice(importOffset,importOffset+50)){
  const entry=document.createElement('div');entry.className='list-row';entry.append(text('span',String(index+1),'list-index'));
  for(const [column,label]of [[1,'שם תיקיה'],[0,'שם זמר']]as const){const wrapper=document.createElement('label');wrapper.append(text('span',label,'editor-cell-label'));const input=document.createElement('input');input.id=`import-${column===0?'alias':'folder'}-${index}`;input.value=row[column]??'';input.dir='auto';input.disabled=personal.busy||controller.state.busy;input.setAttribute('aria-label',`${label}, שורה ${index+1}`);input.addEventListener('input',()=>personal.editImport(index,column,input.value));wrapper.append(input);entry.append(wrapper);}
  const remove=document.createElement('button');setButtonLabel(remove,'מחק','delete');remove.className='text-button';remove.disabled=personal.busy||controller.state.busy;remove.dataset.importDelete=String(index);remove.setAttribute('aria-label',`מחק שורה ${index+1} מהייבוא`);remove.addEventListener('click',()=>{const position=personal.importRows.findIndex(item=>item.index===index);personal.deleteImport(index);const remaining=personal.importRows;const next=remaining[Math.min(position,remaining.length-1)];if(next)$(`import-alias-${next.index}`).focus();else $('cancel-import').focus();});entry.append(remove);
  const original=personal.preview!.rows[index];if(original.length!==2||original.some(field=>!field))entry.append(text('p',`מבנה מקורי: ${JSON.stringify(original)}`,'list-row-warning'));
  $('import-rows').append(entry);
 }
 $('import-range').textContent=imports.length?`${importOffset+1}–${Math.min(importOffset+50,imports.length)} מתוך ${imports.length}`:'הרשימה ריקה';
 $<HTMLButtonElement>('import-prev').disabled=personal.busy||importOffset===0;$<HTMLButtonElement>('import-next').disabled=personal.busy||importOffset+50>=imports.length;
 $<HTMLButtonElement>('confirm-import').disabled=personal.busy||!personal.preview||!personal.importRows.length||personal.preview.encoding!==personal.selectedEncoding;
}
personal.subscribe(renderPersonal);controller.subscribe(renderPersonal);renderPersonal();
