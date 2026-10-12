import type {RecoveryHost,RecoveryPage,RecoveryRecord,RecoveryReport,RecoveryProgress,RecoveryRunId,RecoverySnapshot,Role,RootSelection} from './contracts';
import {errorText} from './state';
export const RECOVERY_PAGE_SIZE=10;
export const RECOVERY_OPERATION_PAGE_SIZE=50;
type Run={epoch:number;id:string|null;cancelRequested:boolean;cancelSent:boolean};
const stoppedMessage='הבדיקה נעצרה. חלק מהקבצים עדיין לא אומתו. אפשר להמשיך בבדיקה; כל הקבצים ייבדקו מחדש והיומנים נשמרים.';
/** Every run observes current files again. Resume tokens only order fresh reads. */
export class RecoveryController {
  page:RecoveryPage|null=null;
  selected:RecoveryRecord|null=null;
  source:RootSelection|null=null;
  target:RootSelection|null=null;
  report:RecoveryReport|null=null;
  progress:RecoveryProgress|null=null;
  operationOffset=0;
  busy=false;
  cancelling=false;
  error:string|null=null;
  private epoch=0;
  private active:Run|null=null;
  private stoppedRunId:string|null=null;
  private listeners=new Set<()=>void>();
  constructor(private host:RecoveryHost,private wait=(ms:number)=>new Promise<void>(resolve=>setTimeout(resolve,ms))){}
  subscribe(listener:()=>void){this.listeners.add(listener);return()=>this.listeners.delete(listener);}
  private emit(){for(const listener of this.listeners)listener();}
  get inspectable(){return !!this.selected&&['recorded_complete','inspection_needed'].includes(this.selected.state);}
  get requiresTarget(){return this.selected?.family!=='repair';}
  get canInspect(){return !this.busy&&this.inspectable&&!!this.selected&&!!this.source&&(!this.requiresTarget||!!this.target);}
  get canResume(){return this.canInspect&&this.stoppedRunId!==null;}
  private async sendCancel(run:Run){
    if(!run.id||run.cancelSent)return;
    run.cancelSent=true;
    try{await this.host.cancelRecovery(run.id);}
    catch(error){if(this.active===run&&this.epoch===run.epoch){this.error=errorText(error);this.emit();}}
  }
  async cancel(){
    const run=this.active;if(!run)return;
    run.cancelRequested=true;this.cancelling=true;this.emit();await this.sendCancel(run);
  }
  close(){
    this.epoch++;this.page=null;this.selected=null;this.source=null;this.target=null;this.report=null;this.error=null;this.stoppedRunId=null;
    void this.cancel();this.emit();
  }
  private async run(kind:'list'|'inspect',start:()=>Promise<RecoveryRunId>,accept:(snapshot:RecoverySnapshot)=>void){
    const run:Run={epoch:++this.epoch,id:null,cancelRequested:false,cancelSent:false};
    this.active=run;this.busy=true;this.cancelling=false;this.error=null;this.progress=null;this.emit();
    try{
      const started=await start();
      if(!started.run_id)throw new Error('לא התקבל מזהה בדיקה תקין.');
      run.id=started.run_id;
      if(run.cancelRequested)await this.sendCancel(run);
      while(true){
        const snapshot=await this.host.recoverySnapshot(run.id);
        if(snapshot.run_id!==run.id||snapshot.kind!==kind||!validProgress(snapshot.progress))throw new Error('התקבל מצב שאינו שייך לבדיקה הנוכחית.');
        if(this.epoch===run.epoch){this.progress=snapshot.progress;this.cancelling=run.cancelRequested||snapshot.status==='cancelling';this.emit();}
        if(snapshot.status==='running'||snapshot.status==='cancelling'){await this.wait(250);continue;}
        if(!['completed','cancelled','failed'].includes(snapshot.status))throw new Error('מצב הבדיקה לא אומת.');
        if(this.epoch===run.epoch){
          if(kind==='inspect'&&['cancelled','failed'].includes(snapshot.status))this.stoppedRunId=run.id;
          if(snapshot.status==='completed'&&!run.cancelRequested){accept(snapshot);this.stoppedRunId=null;}
          else this.error=snapshot.error?.message??stoppedMessage;
        }
        break;
      }
    }catch(error){
      run.cancelRequested=true;await this.sendCancel(run);
      if(this.epoch===run.epoch)this.error=errorText(error);
    }finally{
      if(this.active===run){this.active=null;this.busy=false;this.cancelling=false;this.emit();}
    }
  }
  async load(offset=0){
    if(this.busy||!Number.isSafeInteger(offset)||offset<0)return;
    this.page=null;this.selected=null;this.report=null;this.operationOffset=0;this.source=null;this.target=null;this.stoppedRunId=null;
    await this.run('list',()=>this.host.startRecoveryList(offset,RECOVERY_PAGE_SIZE),snapshot=>{
      const page=snapshot.page;
      if(!page||page.offset!==offset||!Number.isSafeInteger(page.total)||page.total<0||page.records.length>RECOVERY_PAGE_SIZE)throw new Error('רשימת היומנים לא אומתה.');
      this.page=page;
    });
  }
  choose(token:string){
    if(this.busy)return;
    const record=this.page?.records.find(row=>row.token===token);
    if(!record)return;
    this.selected=record;this.source=null;this.target=null;this.report=null;this.progress=null;this.stoppedRunId=null;this.operationOffset=0;this.error=null;this.emit();
  }
  async selectRoot(role:Role){
    if(this.busy||!this.selected||!this.inspectable||(role==='Target'&&!this.requiresTarget))return;
    const epoch=this.epoch;
    this.busy=true;this.error=null;this.emit();
    try{
      const root=await this.host.selectRecoveryRoot(role);
      if(root&&this.epoch===epoch){if(role==='Source')this.source=root;else this.target=root;this.report=null;this.stoppedRunId=null;this.operationOffset=0;}
    }catch(error){if(this.epoch===epoch)this.error=errorText(error);}
    finally{this.busy=false;this.emit();}
  }
  get visibleOperations(){return this.report?.operations.slice(this.operationOffset,this.operationOffset+RECOVERY_OPERATION_PAGE_SIZE)??[];}
  setOperationPage(offset:number){
    if(this.busy||!this.report||!Number.isSafeInteger(offset)||offset<0||offset>=this.report.operations.length)return;
    this.operationOffset=offset;this.emit();
  }
  async inspect(resume=false){
    if(!this.canInspect||(resume&&!this.canResume))return;
    const record=this.selected!,source=this.source!.token,target=this.requiresTarget?this.target!.token:null,resumeId=resume?this.stoppedRunId:null;
    this.report=null;this.operationOffset=0;
    await this.run('inspect',()=>this.host.startRecoveryInspection(record.token,source,target,resumeId),snapshot=>{
      const report=snapshot.report;
      if(!report||report.job_id!==record.job_id||!['consistent','needs_attention'].includes(report.state))throw new Error('התקבלה תשובה שאינה שייכת ליומן שנבחר.');
      if(report.state==='consistent'&&(report.unverified_directories>0||report.observation_limit_reached||report.pending_directories>0||report.rejected_directories>0||report.incomplete_tail||!report.operations.length||report.operations.some(operation=>operation.status!=='Committed'||(record.family==='repair'&&(!operation.evidence?.length||operation.evidence.some(evidence=>evidence.status!=='Verified'))))))throw new Error('הבדיקה לא הושלמה; יש לבדוק את הקבצים והתיקיות לפני ביצוע נוסף.');
      this.report=report;
    });
  }
}
function validProgress(progress:RecoveryProgress):boolean{
  return !!progress&&typeof progress.stage==='string'&&(progress.operation===null||(Number.isSafeInteger(progress.operation)&&progress.operation>=0))
    &&[progress.total_operations,progress.completed_operations,progress.records_checked,progress.observations,progress.bytes_read,progress.elapsed_ms].every(value=>Number.isSafeInteger(value)&&value>=0)
    &&progress.completed_operations<=progress.total_operations;
}
export function recoveryProgressText(progress:RecoveryProgress|null):string{
  if(!progress)return 'מתחיל בדיקה לקריאה בלבד…';
  const stage=({queued:'ממתין לבדיקה',discovery:'מחפש יומנים',journals:'מאמת יומנים',directories:'בודק תיקיות',files:'בודק קבצים'} as Record<string,string>)[progress.stage]??'בודק';
  return `${stage} · ${progress.completed_operations} מתוך ${progress.total_operations} פעולות · ${(progress.bytes_read/(1024*1024)).toFixed(1)} MiB נקראו · ${(progress.elapsed_ms/1000).toFixed(1)} שניות`;
}
export function recoveryStateLabel(state:RecoveryRecord['state']):string {
 return {recorded_complete:'היומן מציין ביצוע שהושלם',inspection_needed:'נדרשת בדיקת הקבצים',invalid:'לא ניתן לאמת את היומן',unsupported:'הבדיקה עדיין אינה נתמכת לפעולה זו'}[state];
}
export function recoveryOperationLabel(status:string):string {
 return ({Committed:'הפעולה רשומה ומאומתת',ReadyForExplicitResume:'לא נמצאה פעולה שהושלמה',EffectProvenAwaitingJournal:'נמצא שינוי ללא סיום מאומת ביומן',NeedsRecovery:'נדרשת בדיקה נוספת'} as Record<string,string>)[status]??'מצב לא מוכר; נדרשת בדיקה';
}

/** Display only: expose control/bidi formatting characters instead of letting
 * an untrusted journal disguise a path. Authority always remains native tokens. */
export function displayRecoveryPath(path:string):string {
 return path.replace(/[\u0000-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/g,char=>`\\u{${char.codePointAt(0)!.toString(16).padStart(4,'0')}}`);
}

export function recoveryFamilyLabel(family:RecoveryRecord['family']):string {
 return {single:'מיון שירים',album:'מיון אלבומים',repair:'תיקון שמות ותגיות',orphan:'יומן פעולה לא שלם'}[family];
}
export function recoveryNextStep(report:RecoveryReport):string {
 if(report.observation_limit_reached)return 'הבדיקה הגיעה למגבלת הקריאה הבטוחה ונעצרה. חלק מהקבצים לא אומתו. יש לבדוק אותם לפני ביצוע נוסף ולשמור את היומנים והעותקים המקוריים.';
 return report.state==='consistent'?'הקבצים תואמים לתיעוד בזמן שכל אחד מהם נבדק. הבדיקה נערכה לאורך פרק זמן, ולא מהווה תמונה בו־זמנית של כל הספרייה. לא בוצעו שינויים.':'יש לבדוק את הקבצים המפורטים לפני ביצוע נוסף. שמור את היומנים ואת העותקים המקוריים. לא בוצעו שינויים ולא הופעלה פעולה מחדש.';
}

export function recoveryEvidenceRole(role:string):string{
 return ({current:'קובץ נוכחי לפי התיעוד',backup:'עותק מקורי שנשמר',staged:'קובץ ביניים',original:'קובץ מקור',planned_name:'שם מתוכנן לבדיקה',backup_candidate:'עותק מקורי מתוכנן לבדיקה',published_candidate:'יעד פרסום לבדיקה'} as Record<string,string>)[role]??'ראיית קובץ לבדיקה';
}
export function recoveryEvidenceStatus(status:string):string{
 return ({Verified:'אומת',Missing:'לא נמצא',Changed:'השתנה',Unverified:'לא אומת'} as Record<string,string>)[status]??'מצב לא מוכר';
}
