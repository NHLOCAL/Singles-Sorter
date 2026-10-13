import {hasAndroidHost} from './android-transport';
import './styles.css';
import './studio.css';
import {appTemplate} from './template';
import {installStudio} from './studio';
import {installViewport} from './viewport';
import {nativeHost,nativeRecoveryHost,openFooterLink,openProjectLink,openLocalLicenses,openExecutionDiagnostics,openRecovery} from './host';
import {installAbout} from './about';
import {readDeveloperMode,saveDeveloperMode,simpleStatus,simpleReason,simpleExecutionSummary,simpleError,simpleRepairReason,verificationWarning,developerRowSummary,recoveryNoticeText,verificationProgressText,resultFilename,executionReadinessText,executionBytesText,REPAIR_REVIEW_TEXT} from './result-presentation';
import {kindLabel,reasonLabel,blockerLabel,warningPresentation,noticeLabel} from './presentation';
import {PAGE_SIZE,PreviewController} from './state';
import type {Settings,ImportEncoding,Mode,PlanRow,ExecutionItem} from './contracts';
import {PersonalListController} from './lists';
import {installHelpBadges} from './help';
import {RecoveryController,RECOVERY_PAGE_SIZE,RECOVERY_OPERATION_PAGE_SIZE,recoveryStateLabel,recoveryFamilyLabel,recoveryNextStep,recoveryOperationLabel,displayRecoveryPath,recoveryProgressText,recoveryEvidenceRole,recoveryEvidenceStatus} from './recovery';
import {WorkspaceNavigation} from './navigation';
import {installIcons,iconElement,setButtonLabel} from './icons';
const controller=new PreviewController(nativeHost,nativeHost);
try{controller.state.developerMode=readDeveloperMode(localStorage);}catch{/* Fresh or unavailable storage keeps the simple view. */}
const personal=new PersonalListController(nativeHost,()=>controller.resourcesChanged());
let listOffset=0,importOffset=0;
const navigation=new WorkspaceNavigation();
const recovery=new RecoveryController(nativeRecoveryHost);
let detailTrigger:HTMLButtonElement|null=null;
let resultsScroll=0;
let viewRevision=0;
window.addEventListener('pagehide',()=>{viewRevision++;controller.discardAlbumSourceParentSelection();});

const app=document.querySelector<HTMLDivElement>('#app')!;
// Static trusted markup only. Every host/file value is rendered with textContent.
app.innerHTML=appTemplate;
// Keep the historical footer destination, verified from the released source.
const $=<T extends HTMLElement=HTMLElement>(id:string)=>document.getElementById(id) as T;
const source=$<HTMLInputElement>('source'),target=$<HTMLInputElement>('target');
const studio=installStudio(app,()=>controller.state.busy||personal.busy,()=>{if(navigation.screen==='results')backToSetup();});
$('recovery-menu').addEventListener('click',async()=>{
 if(controller.state.busy)return;
 $<HTMLDialogElement>('menu-dialog').close();openDialog('recovery-dialog');await recovery.load();
});
$('recovery-close').addEventListener('click',()=>$<HTMLDialogElement>('recovery-dialog').close());
$('recovery-dialog').addEventListener('close',()=>{recovery.close();$('menu').focus();});
$('recovery-cancel').addEventListener('click',()=>void recovery.cancel());
$('recovery-refresh').addEventListener('click',()=>void recovery.load());
$('recovery-resume').addEventListener('click',()=>void recovery.inspect(true));
$('recovery-source-picker').addEventListener('click',()=>void recovery.selectRoot('Source'));
$('recovery-source-location').addEventListener('change',()=>recovery.setSourceMode($<HTMLSelectElement>('recovery-source-location').value==='removed'?'removed':'present'));
$('recovery-target-picker').addEventListener('click',()=>void recovery.selectRoot('Target'));
$('recovery-inspect').addEventListener('click',async()=>{await recovery.inspect();if(recovery.report)$('recovery-result-title').focus();});
$('recovery-prev').addEventListener('click',()=>void recovery.load(Math.max(0,(recovery.page?.offset??0)-RECOVERY_PAGE_SIZE)));
$('recovery-next').addEventListener('click',()=>void recovery.load((recovery.page?.offset??0)+RECOVERY_PAGE_SIZE));
$('recovery-operation-prev').addEventListener('click',()=>recovery.setOperationPage(Math.max(0,recovery.operationOffset-RECOVERY_OPERATION_PAGE_SIZE)));
$('recovery-operation-next').addEventListener('click',()=>recovery.setOperationPage(recovery.operationOffset+RECOVERY_OPERATION_PAGE_SIZE));
$('recovery-evidence-prev').addEventListener('click',()=>recovery.setEvidencePage(Math.max(0,recovery.evidenceOffset-RECOVERY_OPERATION_PAGE_SIZE)));
$('recovery-evidence-next').addEventListener('click',()=>recovery.setEvidencePage(recovery.evidenceOffset+RECOVERY_OPERATION_PAGE_SIZE));
function renderRecovery(){
 const page=recovery.page,record=recovery.selected,report=recovery.report;
 $('recovery-list').replaceChildren();
 if(page&&!page.records.length)$('recovery-list').append(text('p','לא נמצאו יומני פעולות קודמות.','hint'));
 for(const item of page?.records??[]){
  const button=document.createElement('button');button.className='recovery-record secondary';button.disabled=recovery.busy;
  button.setAttribute('aria-pressed',String(record?.token===item.token));button.dataset.recoveryJob=item.job_id??'';button.dataset.recoveryFamily=item.family;button.dataset.recoveryScope=item.verification_scope??'';button.dataset.recoveryState=item.state;
  button.append(text('bdi',controller.state.developerMode?item.job_id??item.journal_name:recoveryFamilyLabel(item.family)),text('span',recoveryStateLabel(item.state),'hint'));
  button.addEventListener('click',()=>recovery.choose(item.token));$('recovery-list').append(button);
 }
 $('recovery-error').hidden=!recovery.error;$('recovery-error').textContent=recovery.error??'';
 $('recovery-busy').hidden=!recovery.busy;
 $('recovery-busy').textContent=recovery.cancelling?'עוצר את הבדיקה ומשחרר את הקבצים…':recoveryProgressText(recovery.progress);
 $('recovery-cancel').hidden=!recovery.busy;$<HTMLButtonElement>('recovery-cancel').disabled=recovery.cancelling;
 $<HTMLButtonElement>('recovery-refresh').disabled=recovery.busy;
 $('recovery-resume').hidden=!recovery.canResume;
 $('recovery-selection').hidden=!recovery.inspectable;$('recovery-record-detail').textContent=record?.detail??'';
 $('recovery-source-mode').hidden=!recovery.canSelectParent;
 $<HTMLSelectElement>('recovery-source-location').value=recovery.sourceMode;$<HTMLSelectElement>('recovery-source-location').disabled=recovery.busy;
 $<HTMLInputElement>('recovery-recorded-parent').value=displayRecoveryPath(record?.original_parent??'');
 $('recovery-source-picker').textContent=recovery.sourceMode==='removed'?'בחר את התיקייה שהכילה את המקור':'בחר תיקית מקור';
 $('recovery-recorded-target').parentElement!.hidden=!recovery.requiresTarget;
 $('recovery-target-picker').hidden=!recovery.requiresTarget;$('recovery-selected-target').hidden=!recovery.requiresTarget;
 $<HTMLInputElement>('recovery-recorded-source').value=displayRecoveryPath(record?.source??'');$<HTMLInputElement>('recovery-recorded-target').value=displayRecoveryPath(record?.target??'');
 $('recovery-selected-source').textContent=displayRecoveryPath(recovery.source?.display_name??'לא נבחרה תיקייה');$('recovery-selected-target').textContent=displayRecoveryPath(recovery.target?.display_name??'לא נבחרה תיקייה');
 for(const id of ['recovery-source-picker','recovery-target-picker'])$<HTMLButtonElement>(id).disabled=recovery.busy;
 $<HTMLButtonElement>('recovery-inspect').disabled=!recovery.canInspect;
 $('recovery-pagination').hidden=!page||page.total<=RECOVERY_PAGE_SIZE;
 $('recovery-range').textContent=page&&page.total?`${page.offset+1}–${Math.min(page.offset+page.records.length,page.total)} מתוך ${page.total}`:'';
 $<HTMLButtonElement>('recovery-prev').disabled=recovery.busy||!page||page.offset===0;$<HTMLButtonElement>('recovery-next').disabled=recovery.busy||!page||page.offset+RECOVERY_PAGE_SIZE>=page.total;
 $('recovery-truncated').hidden=!page?.truncated;$('recovery-report').hidden=!report;
 $('recovery-result-title').textContent=report?.state==='consistent'?'הקבצים שנבדקו תואמים ליומן':'נדרשת בדיקה נוספת';
 $('recovery-result-summary').textContent=report?(record?.family==='repair'?`${report.operations.length} פעולות תיקון נבדקו · הבדיקה כוללת עותקים מקוריים כשאלה תועדו${report.incomplete_tail?' · היומן מסתיים ברשומה חלקית':''}`:`${report.operations.length} ${report.verification_scope?'קבצים':'פעולות'} נבדקו · ${report.pending_directories} בדיקות תיקייה ממתינות לבירור · ${report.unverified_directories} תיקיות לא אומתו${report.incomplete_tail?' · היומן מסתיים ברשומה חלקית':''}`):'';
 if(report?.evidence?.some(entry=>!['Verified','AbsentExpected'].includes(entry.status)))$('recovery-result-summary').append(' · יש אי־התאמה בתיקיות או בתיעוד. פרטים בבדיקות הנוספות.');
 $('recovery-next-step').textContent=report?recoveryNextStep(report):record?.detail??'';
 $('recovery-evidence').hidden=!report?.evidence?.length;
 $('recovery-evidence-list').replaceChildren();
 for(const evidence of recovery.visibleEvidence){const block=text('label',`${recoveryEvidenceRole(evidence.role)}: ${recoveryEvidenceStatus(evidence.status)}`,'detail-path-label recovery-evidence');block.dataset.recoveryEvidenceRole=evidence.role;block.dataset.recoveryEvidenceStatus=evidence.status;const input=document.createElement('input');input.className='source-path';input.readOnly=true;input.dir='ltr';input.value=displayRecoveryPath(evidence.path);block.append(input);$('recovery-evidence-list').append(block);}
 const evidenceCount=report?.evidence?.length??0;
 $('recovery-evidence-pagination').hidden=evidenceCount<=RECOVERY_OPERATION_PAGE_SIZE;
 $('recovery-evidence-range').textContent=evidenceCount?`${recovery.evidenceOffset+1}–${Math.min(recovery.evidenceOffset+RECOVERY_OPERATION_PAGE_SIZE,evidenceCount)} מתוך ${evidenceCount}`:'';
 $<HTMLButtonElement>('recovery-evidence-prev').disabled=recovery.busy||recovery.evidenceOffset===0;
 $<HTMLButtonElement>('recovery-evidence-next').disabled=recovery.busy||recovery.evidenceOffset+RECOVERY_OPERATION_PAGE_SIZE>=evidenceCount;
 $('recovery-operation-list').replaceChildren();
 for(const operation of recovery.visibleOperations){const item=text('article','', 'recovery-operation');item.dataset.recoveryOperationStatus=operation.status;item.append(text('h4',`${report?.verification_scope?'קובץ':'פעולה'} ${operation.operation+1}: ${recoveryOperationLabel(operation.status)}`));for(const [label,path] of [['מקור',operation.source],['יעד',operation.destination]]){const block=text('label',label,'detail-path-label');const input=document.createElement('input');input.className='source-path';input.readOnly=true;input.dir='ltr';input.value=displayRecoveryPath(path);block.append(input);item.append(block);}for(const evidence of operation.evidence??[]){const block=text('label',`${recoveryEvidenceRole(evidence.role)}: ${recoveryEvidenceStatus(evidence.status)}`,'detail-path-label recovery-evidence');block.dataset.recoveryEvidenceRole=evidence.role;block.dataset.recoveryEvidenceStatus=evidence.status;const input=document.createElement('input');input.className='source-path';input.readOnly=true;input.dir='ltr';input.value=displayRecoveryPath(evidence.path);block.append(input);item.append(block);}$('recovery-operation-list').append(item);}
 $('recovery-operation-pagination').hidden=!report||report.operations.length<=RECOVERY_OPERATION_PAGE_SIZE;
 $('recovery-operation-range').textContent=report?.operations.length?`${recovery.operationOffset+1}–${Math.min(recovery.operationOffset+RECOVERY_OPERATION_PAGE_SIZE,report.operations.length)} מתוך ${report.operations.length}`:'';
 $<HTMLButtonElement>('recovery-operation-prev').disabled=recovery.busy||recovery.operationOffset===0;
 $<HTMLButtonElement>('recovery-operation-next').disabled=recovery.busy||!report||recovery.operationOffset+RECOVERY_OPERATION_PAGE_SIZE>=report.operations.length;
 if(record&&!recovery.inspectable){$('recovery-error').hidden=false;$('recovery-error').textContent=record.detail;}
}
recovery.subscribe(renderRecovery);
$('footer-link').addEventListener('click',async()=>{try{await openFooterLink();}catch{ $('error').textContent='לא ניתן לפתוח כעת את הקישור בדפדפן. כתובת האתר: https://nhlocal.github.io/shir-bot';$('error').hidden=false; }});
source.addEventListener('input',()=>controller.editRoot('Source',source.value));
target.addEventListener('input',()=>controller.editRoot('Target',target.value));
$('source-picker').addEventListener('click',()=>void controller.selectRoot('Source'));
$('target-picker').addEventListener('click',()=>void controller.selectRoot('Target'));
function hasResults(){return controller.state.snapshot?.status==='completed'&&!!controller.state.page;}
function showSetup(){if(controller.state.activity==='apply'||controller.state.activity==='album-parent')return;if(controller.state.activity==='preview'){controller.cancelPreview();return;}if(navigation.screen==='results')resultsScroll=$('results-body').scrollTop;navigation.setup();render();source.focus();}
function showResults(push=true){navigation.results(hasResults());if(push)history.pushState({singlesView:'results',jobId:controller.state.snapshot?.job_id},'');render();$('results-heading').focus();requestAnimationFrame(()=>{$('results-body').scrollTop=resultsScroll;});}
function backToSetup(){if(controller.state.activity==='album-parent')return;if(history.state?.singlesView==='results')history.back();else showSetup();}
history.replaceState({singlesView:'setup'},'');
window.addEventListener('popstate',()=>{
 if(controller.state.activity==='apply'||controller.state.activity==='album-parent'){history.replaceState({singlesView:'results',jobId:controller.state.snapshot?.job_id},'');return;}
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
 $('apply-description').textContent=repairing?REPAIR_REVIEW_TEXT:readiness.operation_count===0?'לא בוצעו שינויים. קובצי המקור יישארו במקומם; הבדיקה החוזרת תאמת שאין פעולות לביצוע.':controller.state.developerMode?`${readiness.copy_mode?'העתקת':'העברת'} ${readiness.operation_count.toLocaleString('he-IL')} פעולות לפי התוכנית שהוצגה.`:readiness.copy_mode?'העתקת הקבצים שברשימה. קובצי המקור יישארו במקומם.':'העברת הקבצים שברשימה אל תיקיית היעד.';
 if(readiness.duet_groups&&readiness.operation_count>0)$('apply-description').append(document.createTextNode(' בדואטים, המקור יימחק רק לאחר אימות כל העותקים הנדרשים בתוכנית. מקור ללא יעד מתאים יישמר.'));
 if(readiness.notice)$('apply-description').append(document.createTextNode(' '+readiness.notice));
 if(controller.state.model?.platform==='android'&&readiness.reason)$('apply-description').append(document.createTextNode(' '+readiness.reason));
 $<HTMLInputElement>('apply-source').value=readiness.source;$<HTMLInputElement>('apply-target').value=readiness.target??'';
 openDialog('apply-dialog');$('apply-back').focus();
});
for(const id of ['apply-dismiss','apply-back'])$(id).addEventListener('click',()=>{$<HTMLDialogElement>('apply-dialog').close();$('review-apply').focus();});
$('confirm-apply').addEventListener('click',async()=>{$<HTMLDialogElement>('apply-dialog').close();await controller.apply();$('execution-title').tabIndex=-1;$('execution-title').focus();});
$('cancel-apply').addEventListener('click',()=>controller.cancelApply());
$('select-album-source-parent').addEventListener('click',async()=>{
 if(personal.busy||!controller.canSelectAlbumSourceParent)return;
 const revision=viewRevision;
 const selected=await controller.selectAlbumSourceParent();
 if(revision!==viewRevision)return;
 if(selected)await runPreview('Sort');
 else if(controller.state.error)$('error').focus();
 else if(controller.canSelectAlbumSourceParent)$('select-album-source-parent').focus();
});
$('reopen-results').addEventListener('click',()=>showResults());
async function runPreview(mode:Mode){
 if(controller.state.busy||personal.busy||!navigation.start(mode))return;
 studio.selectMode(mode);studio.resetFilter();
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
 detailTrigger=trigger;$('row-title').textContent=resultFilename(row,executionItems(row),controller.state.model?.platform);$('row-content').replaceChildren();
 if(!controller.state.developerMode){
  const status=rowStatus(row);
  if(row.artist)$('row-content').append(text('p',row.artist,'detail-artist'));
  $('row-content').append(text('p',status,'detail-status'));
  for(const item of executionItems(row)){const repairReason=simpleRepairReason(item);if(repairReason)$('row-content').append(text('p',repairReason,'hint'));}
  const reason=simpleReason(row,controller.state.snapshot?.apply?.supported);if(reason&&!executionItems(row).length)$('row-content').append(text('p',reason,'hint'));
  if(executionItems(row).some(item=>['partial','failed','needs_recovery','source_retained'].includes(item.status)))$('row-content').append(text('p',row.kind==='FixNames'?'בדוק את הקובץ ואת הגיבוי בתיקיית המקור לפני ניסיון נוסף.':'בדוק את הקובץ בתיקיית המקור ובתיקיית היעד לפני ניסיון נוסף.','hint'));
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
let settingsTrigger:HTMLElement=$('advanced');
for(const id of ['advanced','open-settings'])$(id).addEventListener('click',()=>{settingsTrigger=$(id);openDialog('advanced-dialog');});
$('advanced-dialog').addEventListener('close',()=>{if(!document.querySelector('dialog[open]'))settingsTrigger.focus();});
$('menu-dialog').addEventListener('close',()=>{if(!document.querySelector('dialog[open]'))$('menu').focus();});
for(const button of document.querySelectorAll<HTMLButtonElement>('.close-dialog'))button.addEventListener('click',()=>button.closest('dialog')?.close());
installIcons(app);
installHelpBadges(app);
// Alerts live in the scrollable middle track and never change the shell geometry.
installViewport(app);
installAbout(app,openProjectLink,hasAndroidHost()?openLocalLicenses:undefined);
$('open-recovery').addEventListener('click',()=>{if(controller.state.busy)return;controller.resourcesChanged();void openRecovery().catch(()=>{$('recovery-notice-text').textContent+=' לא ניתן לפתוח את הבדיקה כעת.';});});
$('execution-diagnostics').addEventListener('click',()=>{void openExecutionDiagnostics().catch(()=>{$('settings-error').textContent='לא ניתן לפתוח את יומני הפעולה.';$('settings-error').hidden=false;});});
$('developer-mode').addEventListener('change',async()=>{
 const enabled=$<HTMLInputElement>('developer-mode').checked;
 await controller.setDeveloperMode(enabled);
 if(controller.state.developerMode===enabled)try{saveDeveloperMode(localStorage,enabled);}catch{/* Session preference remains usable. */}
});
$('show-inactive').addEventListener('change',()=>void controller.setShowInactive($<HTMLInputElement>('show-inactive').checked));
$('previous').addEventListener('click',()=>void controller.changePage(Math.max(0,controller.state.offset-PAGE_SIZE)));
$('next').addEventListener('click',()=>void controller.changePage(controller.state.offset+PAGE_SIZE));
function text(tag:string,value:string,className?:string){const node=document.createElement(tag);node.textContent=value;if(className)node.className=className;return node;}
const executionTitles:Record<string,string>={preparing:'מאמת את הקבצים לפני ביצוע…',running:'מבצע את התוכנית…',complete:'הביצוע הושלם',partial:'הביצוע הושלם חלקית',cancelled:'הביצוע נעצר',failed:'הביצוע לא הושלם',needs_recovery:'נדרשת בדיקת התאוששות'};
function itemLabel(item:ExecutionItem){return ({partial:'בוצע חלקית',complete:'בוצע',source_retained:'היעד נכתב; המקור נשמר',failed:'נכשל',cancelled:'בוטל',blocked:'לא בוצע',needs_recovery:'נדרשת בדיקה',unsupported_tags:'תגיות לא נתמכות',metadata_failure:'קריאת התגיות נכשלה'})[item.status];}
function executionItems(row:PlanRow):ExecutionItem[]{return controller.state.execution?.result?.items.filter(item=>row.operation_ids?.includes(item.operation))??[];}
function rowStatus(row:PlanRow){
 const s=controller.state;
 return simpleStatus(row,executionItems(row),s.snapshot?.apply?.copy_mode??s.settings.copy_mode,{repairEligible:s.snapshot?.apply?.supported,terminalResult:!!s.execution?.result&&!['preparing','running'].includes(s.execution.status),uncertain:!!s.execution&&!s.execution.result&&!['preparing','running'].includes(s.execution.status)});
}
function renderExecution(){
 const s=controller.state,e=s.execution,readiness=s.snapshot?.apply;
 $('execution-panel').hidden=!readiness&&!e;$('results').dataset.executionVisible=String(!!readiness||!!e);
 $('review-apply').hidden=!!e||!readiness?.supported||readiness.source_parent_selection_required===true;$<HTMLButtonElement>('review-apply').disabled=!controller.canApply;
 const parentNeeded=controller.needsAlbumSourceParent,parentPending=s.activity==='album-parent';
 $('select-album-source-parent').hidden=!parentNeeded;$<HTMLButtonElement>('select-album-source-parent').disabled=!controller.canSelectAlbumSourceParent;
 $('album-source-parent-help').hidden=!parentNeeded;
 const running=s.activity==='apply';$('cancel-apply').hidden=!running;$<HTMLButtonElement>('cancel-apply').disabled=s.cancelRequested;
 $<HTMLButtonElement>('back-setup').disabled=running||parentPending;
 $('execution-title').textContent=e?e.status==='complete'&&e.recovery_error?'הקבצים טופלו; נדרשת בדיקה':executionTitles[e.status]:running?'מאמת את הקבצים לפני ביצוע…':readiness?.supported?'התוכנית מוכנה לביצוע':'התוכנית זמינה לתצוגה בלבד';
 const r=e?.result;
 const developer=s.developerMode;
 $('execution-summary').textContent=developer?(r?r.repair?`${r.repair.renamed_files} שמות תוקנו · ${r.repair.repaired_files} תגיות תוקנו · ${r.repair.unchanged_files} ללא שינוי בתגיות · ${r.repair.unsupported_files} לא נתמכים · ${r.repair.retained_backup_bytes.toLocaleString('he-IL')} בתים נשמרו בגיבוי`:r.duets?simpleExecutionSummary(e!,readiness?.copy_mode??s.settings.copy_mode):`${r.committed_destinations.toLocaleString('he-IL')} יעדים נכתבו · ${r.moved_sources.toLocaleString('he-IL')} קבצי מקור הועברו · ${r.failed_operations.toLocaleString('he-IL')} נכשלו · ${r.blocked_operations.toLocaleString('he-IL')} לא בוצעו`:s.cancelRequested&&running?'ממתין לעצירה בטוחה…':e?.status==='running'&&e.operation!==null?`פעולה ${e.operation+1} מתוך ${e.operation_count} · ${executionBytesText(e.bytes_written,e.source_length)}`:e&&!['preparing','running'].includes(e.status)?(e.error??'לא התקבלו מוני ביצוע מאומתים. יש לבדוק את היומן.'):readiness?.supported?executionReadinessText(readiness,true):readiness?.reason??''):(e?simpleExecutionSummary(e,readiness?.copy_mode??s.settings.copy_mode):readiness?.supported?executionReadinessText(readiness,false):'לא בוצעו שינויים בקבצים.');
 if(!developer&&running)$('execution-summary').textContent=s.cancelRequested?'ממתין לעצירה בטוחה…':e?.operation!==null&&e?.operation!==undefined?`התקדמות: ${e.operation+1} מתוך ${e.operation_count}`:'בודק שהקבצים מוכנים…';
 if(parentNeeded){$('execution-title').textContent='נדרשת בחירה מפורשת של תיקיית האב';$('execution-summary').textContent=parentPending?'ממתין לבחירת תיקיית האב…':'לאחר הבחירה תופעל בדיקת מיון חדשה. עדיין לא יבוצעו שינויים בקבצים.';}
 if(running&&e?.verification){
  $('execution-title').textContent='בודק את תוצאות הביצוע';
  $('execution-summary').textContent=s.cancelRequested?'ממתין לעצירה בטוחה…':verificationProgressText(e.verification,developer);
 }
 const progress=$<HTMLProgressElement>('execution-progress');progress.hidden=!running;
 if(running&&e?.verification){
  if(e.verification.total_operations>0){progress.max=e.verification.total_operations;progress.value=Math.min(e.verification.completed_operations,e.verification.total_operations);}else progress.removeAttribute('value');
 }else if(e?.status==='running'&&e.source_length>0){progress.max=e.source_length;progress.value=Math.min(e.bytes_written,e.source_length);}else progress.removeAttribute('value');
 const verification=verificationWarning(e,developer);
 $('execution-recovery').hidden=!verification&&(!e||!['needs_recovery','partial','cancelled','failed'].includes(e.status));
 $('execution-recovery').textContent=verification||(developer?(e?.status==='needs_recovery'?'היומן נשמר. אין להפעיל שוב את התוכנית או למחוק את היומן לפני בדיקת הקבצים. התאוששות אוטומטית אינה מופעלת.':'פעולות שהושלמו נשארות בתוקף. עיין בפרטי הקבצים וביומן לפני בדיקה חדשה.'):readiness?.mode==='FixNames'?'בדוק את הקבצים והגיבויים בתיקיית המקור לפני ניסיון נוסף.':'קבצים שכבר טופלו נשארים במקומם. בדוק את המקור והיעד לפני ניסיון נוסף.');
 $('recovery-inspection').hidden=!developer||!e?.recovery;$('recovery-inspection').textContent=e?.recovery?`נבדק היומן מול הקבצים: ${e.recovery.transfers.length} פעולות · ${readiness?.mode==='FixNames'?'תיקון שמות ותגיות':`${e.recovery.pending_directories} בדיקות תיקייה ממתינות לבירור`}${e.recovery.incomplete_tail?' · רשומה אחרונה לא שלמה':''}`:'';
 $('journal-label').hidden=!developer||!e?.journal_path;$<HTMLInputElement>('journal-path').value=e?.journal_path??'';
}
function render(){
 $('recovery-menu').hidden=controller.state.model?.platform!=='windows';$<HTMLButtonElement>('recovery-menu').disabled=controller.state.busy;
 const s=controller.state;const blocked=s.busy||!s.hostAvailable;
 const developer=s.developerMode;app.dataset.developerMode=String(developer);app.dataset.showInactive=String(s.showInactive);
 $('execution-diagnostics').hidden=!developer||!hasAndroidHost();$<HTMLButtonElement>('execution-diagnostics').disabled=s.busy;
 $<HTMLInputElement>('developer-mode').checked=developer;$<HTMLInputElement>('developer-mode').disabled=s.busy;
 $<HTMLInputElement>('show-inactive').checked=s.showInactive;$<HTMLInputElement>('show-inactive').disabled=s.busy;
 $('inactive-count').textContent=s.page?.skipped?`(${s.page.skipped.toLocaleString('he-IL')})`:'';
 $('result-second-heading').textContent=developer?'פעולה':'אמן';$('result-third-heading').textContent=developer?'סיבה / חסימה':'מצב';
 studio.update();
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
 $('recovery-notice').hidden=!s.recoveryNotice;
 $('recovery-notice-text').textContent=recoveryNoticeText(s.recoveryNotice);
 $('open-recovery').hidden=!hasAndroidHost()||!s.recoveryNotice;$<HTMLButtonElement>('open-recovery').disabled=s.busy;
 $('settings-label').textContent=controller.settingsDirty?'שינויים שלא נשמרו':'ההגדרות שלך';$('settings-notice').textContent=noticeLabel(controller.savedNotice);$<HTMLButtonElement>('save-settings').disabled=blocked;$('settings-error').hidden=!s.error;$('settings-error').textContent=s.error?(developer?s.error:simpleError(s.error)):'';
 $<HTMLButtonElement>('cancel-preview').disabled=!s.busy||s.cancelRequested;
 $('busy').textContent=s.cancelRequested&&s.activity==='preview'?'מבטל את הבדיקה…':s.activity==='settings'?'שומר הגדרות…':s.activity==='page'?'טוען עמוד תוצאות…':s.activity==='album-parent'?'ממתין לבחירת תיקיית האב…':'קורא ומנתח את הקבצים…';
 $('busy').hidden=!s.busy;$('results').setAttribute('aria-busy',String(s.busy));
 $('error').hidden=!s.error;$('error').textContent=s.error?(developer?s.error:simpleError(s.error)):'';$('error').tabIndex=-1;
 const ready=s.snapshot?.status==='completed'&&!!s.page;
 $('empty').hidden=s.busy||!!s.error||ready&&s.page!.total>0;
 $('empty').textContent=ready?(s.snapshot?.apply?.mode??navigation.mode)==='FixNames'?'לא נמצאו קבצים לתיקון.':'לא נמצאו קבצים למיון.':'התוצאה תופיע כאן לאחר בחירת תיקיות והפעלת בדיקה.';
 $('table-wrap').hidden=!ready||!s.page!.rows.length;
 $('count').textContent=ready?`${s.page!.total.toLocaleString('he-IL')} ${developer?'רשומות':s.page!.total===1?'פריט':'פריטים'}`:'';
 $('rows').replaceChildren();
 if(ready)for(const row of s.page!.rows){
  const tr=document.createElement('tr'),items=executionItems(row),filename=resultFilename(row,items,s.model?.platform);
  tr.dataset.filterCategory=studio.category(row,rowStatus(row));
  const origin=document.createElement('td');origin.className='file-name';const name=text('bdi',filename,'source-name');name.dir='auto';name.title=developer?row.source:filename;origin.append(name);tr.append(origin);
  const action=text('td',developer?(items.length?items.map(itemLabel).join(' · '):kindLabel(row.kind)):(row.artist??''),'row-action');action.dir='auto';tr.append(action);
  const reason=document.createElement('td');reason.className='row-reason';reason.append(text('span',developer?`${kindLabel(row.kind)} · `:row.artist?`${row.artist} · `:'','mobile-action'),text('span',developer?developerRowSummary(row,rowStatus(row),items.length>0||!!s.execution&&!['preparing','running'].includes(s.execution.status)):rowStatus(row)));tr.append(reason);
  const detail=document.createElement('td');detail.className='row-detail';const button=document.createElement('button');button.className='icon-button detail-button';button.disabled=s.activity==='album-parent';button.dataset.rowId=row.id;button.setAttribute('aria-label',`פרטים: ${filename}`);button.append(iconElement('info'));button.addEventListener('click',()=>openRow(row,button));detail.append(button);tr.append(detail);$('rows').append(tr);
 }
 studio.filterRows();
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
 studio.update();
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
