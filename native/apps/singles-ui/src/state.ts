import {defaults} from './contracts';
import type {Host,ProfileHost,Mode,ModelStatus,PlanPage,Role,RootSelection,Settings,Snapshot,ExecutionSnapshot,RecoveryNotice,PreviewRequest} from './contracts';
export const PAGE_SIZE = 50;
const rootKeys = {Source:'source',Target:'target'} as const;
export interface RootInput {text:string;selection:RootSelection|null;pending:boolean;revision:number}
export interface State {
  source:RootInput;target:RootInput;settings:Settings;model:ModelStatus|null;
  initialized:boolean;startupWaiting:boolean;hostAvailable:boolean;busy:boolean;cancelRequested:boolean;activity:'preview'|'apply'|'settings'|'page'|'album-parent'|null;error:string|null;
  recoveryNotice:RecoveryNotice|null;snapshot:Snapshot|null;execution:ExecutionSnapshot|null;page:PlanPage|null;offset:number;ai:'Disabled'|'Configured';developerMode:boolean;showInactive:boolean;
}
const newRoot = ():RootInput => ({text:'',selection:null,pending:false,revision:0});
export const userErrorLabels:Readonly<Record<string,string>>={
  LIST_CHANGED:'הרשימה השתנתה מאז פתיחתה. בטל ופתח מחדש לפני שמירה או ייבוא.',
  PROFILE_BUSY:'הפרופיל נמצא בשימוש. המתן לסיום הפעולה ונסה שוב.',
  PROFILE_IO:'לא ניתן לקרוא או לשמור את הפרופיל. בדוק מקום פנוי והרשאות.',
  PROFILE_INVALID:'קובץ הפרופיל אינו תקין. הוא לא אופס או הוחלף.',
  PROFILE_SCHEMA:'גרסת הפרופיל אינה נתמכת. לא בוצע שינוי בקובץ.',
  ENCODING_INVALID:'הקידוד שבחרת אינו מתאים לקובץ. בחר קידוד אחר ובדוק את התצוגה.',
  CSV_TOO_LARGE:'קובץ הרשימה חורג ממגבלת 16 MiB.',
  EXPORT_FAILED:'הייצוא לא הושלם. בחר שם CSV חדש ובדוק מקום פנוי והרשאות.',
  RECOGNITION_UNAVAILABLE:'מודל הזיהוי הדרוש אינו זמין. הבדיקה לא הופעלה.',
  IMPORT_NOT_PREVIEWED:'יש לבדוק את קידוד הייבוא והתצוגה המקדימה לפני אישור.',
};
export function errorText(error:unknown):string {
  if(typeof error==='string')return error;
  if(error instanceof Error)return error.message;
  if(error && typeof error==='object' && 'message' in error){
    const code='code' in error?String(error.code):'';

    return userErrorLabels[code]?`${userErrorLabels[code]} (${code})`:String(error.message);
  }
  return 'הפעולה לא הושלמה. נסה שוב לאחר בדיקת החיבור לתוכנה.';
}
export class PreviewController {
  readonly state:State={source:newRoot(),target:newRoot(),settings:{...defaults},model:null,initialized:false,startupWaiting:false,hostAvailable:false,busy:false,cancelRequested:false,activity:null,error:null,recoveryNotice:null,snapshot:null,execution:null,page:null,offset:0,ai:'Disabled',developerMode:false,showInactive:false};
  private activeApply:{jobId:string;started:boolean;cancelSent:boolean;terminal:boolean}|null=null;
  private activePreview:{jobId:string|null;cancelSent:boolean;terminal:boolean}|null=null;
  private retainedPreview:{snapshot:Snapshot;request:PreviewRequest}|null=null;
  private activeAlbumParent:object|null=null;
  private listeners=new Set<()=>void>();
  savedNotice:string|null=null;
  settingsDirty=false;
  constructor(private host:Host,private profile?:ProfileHost){}
  subscribe(listener:()=>void):()=>void{this.listeners.add(listener);return()=>this.listeners.delete(listener);}
  private emit(){for(const listener of this.listeners)listener();}
  private invalidate(){this.retainedPreview=null;this.state.execution=null;this.state.snapshot=null;this.state.page=null;this.state.offset=0;this.state.error=null;}
  async initialize(){
    try{
      const settings=await this.host.getSettings();
      for(const key of Object.keys(defaults) as (keyof Settings)[]){if(typeof settings[key]!=='boolean')throw new Error('הגדרות התוכנה אינן תקינות.');}
      let model:ModelStatus;
      for(;;){
        try{model=await this.host.getModelStatus();break;}
        catch(error){
          if(!error||typeof error!=='object'||!('code' in error)||error.code!=='BUSY')throw error;
          // Reload cancels the previous worker; wait for its actual guard release.
          this.state.startupWaiting=true;this.emit();
          await new Promise(resolve=>setTimeout(resolve,250));
        }
      }
      this.state.settings={...settings};this.state.model=model;this.state.hostAvailable=true;
      if((model.platform==='windows'||model.execution_available===true)&&this.host.latestExecution){
        const retained=await this.host.latestExecution();
        if(retained&&'recovery_notice' in retained){
          const notice=retained.recovery_notice;
          if((notice.status==='attention'&&(notice.acknowledged_operations===null||notice.unresolved_operations===null))||!['attention','unreadable'].includes(notice.status)||!Number.isSafeInteger(notice.journal_count)||notice.journal_count<1||notice.journal_count>1000||[notice.acknowledged_operations,notice.unresolved_operations].some(value=>value!==null&&(!Number.isSafeInteger(value)||value<0||value>10000000)))throw new Error('מידע הבדיקה של הביצוע הקודם אינו תקין.');
          this.state.recoveryNotice=notice;
        }else if(retained){
          if(retained.preview.status!=='completed'||retained.preview.job_id!==retained.execution.job_id||retained.preview.plan_id!==retained.execution.plan_id||['running','preparing'].includes(retained.execution.status))throw new Error('תוצאת הביצוע הקודמת טרם אומתה.');
          this.state.snapshot=retained.preview;this.state.execution=retained.execution;
          this.state.page=await this.host.page(retained.preview.job_id,0,PAGE_SIZE,this.state.developerMode,this.state.showInactive);
        }
      }
    }catch(error){this.state.error=errorText(error);this.state.hostAvailable=false;}
    this.state.startupWaiting=false;this.state.initialized=true;this.emit();
  }
  editRoot(role:Role,text:string){
    if(this.state.busy)return;
    const root=this.state[rootKeys[role]];root.text=text;root.selection=null;root.pending=false;root.revision++;
    this.invalidate();this.emit();
  }
  changeSetting(key:keyof Settings,value:boolean){
    if(this.state.busy)return;this.state.settings={...this.state.settings,[key]:value};this.settingsDirty=true;this.savedNotice=null;this.invalidate();this.emit();
  }
  async saveMainSettings(){
    if(this.state.busy||!this.profile||!this.state.hostAvailable)return;
    const {duet_mode:_,...main}=this.state.settings;
    this.state.busy=true;this.state.activity='settings';this.state.error=null;this.savedNotice=null;this.emit();
    try{
      const stored=await this.profile.saveMainSettings(main);
      if((Object.keys(main) as (keyof typeof main)[]).some(key=>stored[key]!==main[key]))throw new Error('שמירת ההגדרות לא אומתה.');
      this.settingsDirty=false;this.savedNotice='הגדרות המיון נשמרו להפעלות הבאות.';
    }catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
  async setDuet(enabled:boolean){
    if(this.state.busy||!this.profile||!this.state.hostAvailable)return;
    this.state.busy=true;this.state.activity='settings';this.state.error=null;this.savedNotice=null;this.emit();
    try{
      const stored=await this.profile.setDuet(enabled);
      if(stored.duet_mode!==enabled)throw new Error('שמירת מצב הדואטים לא אומתה.');
      // Other five values are the unsaved draft, not the host's older stored values.
      this.state.settings={...this.state.settings,duet_mode:stored.duet_mode};
      this.invalidate();this.savedNotice='מצב הדואטים נשמר מיד.';
    }catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
  resourcesChanged(){if(this.state.busy)return;this.invalidate();this.emit();}
  changeAi(value:'Disabled'|'Configured'){
    if(this.state.busy)return;this.state.ai=value;this.invalidate();this.emit();
  }
  async selectRoot(role:Role){
    if(this.state.busy||!this.state.hostAvailable)return;
    const root=this.state[rootKeys[role]];
    if(root.pending)return;
    const revision=++root.revision;root.pending=true;this.state.error=null;this.emit();
    try{
      const selected=await this.host.selectRoot(role);
      if(root.revision!==revision)return;
      // A cancelled picker leaves both text and previous authority untouched.
      if(selected){root.selection=selected;root.text=selected.display_name;this.invalidate();}
    }catch(error){if(root.revision===revision)this.state.error=errorText(error);}
    finally{if(root.revision===revision)root.pending=false;this.emit();}
  }
  private async resolve(role:Role){
    const root=this.state[rootKeys[role]];
    if(root.selection)return root.selection;
    if(!root.text.trim())throw new Error(role==='Source'?'בחר תיקיית מקור.':'בחר תיקיית יעד למיון.');
    // Never trim or normalize an actual path supplied by the user.
    const selected=await this.host.resolveRoot(role,root.text);
    root.selection=selected;root.text=selected.display_name;return selected;
  }
  cancelPreview(){
    const run=this.activePreview;
    if(!run)return;
    this.state.cancelRequested=true;this.emit();
    this.sendCancellation(run);
  }
  private sendCancellation(run:NonNullable<PreviewController['activePreview']>){
    if(!run.jobId||run.cancelSent||run.terminal)return;
    run.cancelSent=true;
    // Cancellation is a request, not evidence that the native worker stopped.
    void this.host.cancelPreview(run.jobId).catch(error=>{
      if(this.activePreview!==run||run.terminal)return;
      run.cancelSent=false;this.state.error=errorText(error);this.emit();
    });
  }
  async preview(mode:Mode){
    if(this.state.busy||this.state.source.pending||this.state.target.pending||!this.state.hostAvailable)return;
    const run={jobId:null as string|null,cancelSent:false,terminal:false};
    this.activePreview=run;
    this.invalidate();this.state.busy=true;this.state.cancelRequested=false;this.state.activity='preview';this.emit();
    try{
      const source=await this.resolve('Source');
      if(this.state.cancelRequested)return;
      const target=mode==='Sort'?await this.resolve('Target'):null;
      if(this.state.cancelRequested)return;
      const request:PreviewRequest={mode,source_token:source.token,target_token:target?.token??null,settings:{...this.state.settings},ai_profile:this.state.ai};
      const started=await this.host.startPreview(request);
      run.jobId=started.job_id;
      if(this.state.cancelRequested)this.sendCancellation(run);
      for(;;){
        if(this.state.cancelRequested)this.sendCancellation(run);
        let snapshot:Snapshot;
        try{
          snapshot=await this.host.snapshot(started.job_id);
          if(snapshot.host_schema!==1||snapshot.job_id!==started.job_id||!['running','completed','failed','cancelled'].includes(snapshot.status))throw new Error('התקבלה תשובה שאינה שייכת לבדיקה הנוכחית.');
        }catch(error){
          if(error&&typeof error==='object'&&'code' in error&&error.code==='UNKNOWN_JOB'){
            // This explicit host response proves this job is no longer retained.
            // Do not invent completion/cancellation or expose stale results.
            run.terminal=true;this.state.snapshot=null;this.state.page=null;
            throw new Error('הבדיקה אינה זמינה עוד במארח. אפשר להפעיל בדיקה חדשה.');
          }
          // A lost or stale reply cannot prove that the host released its worker.
          this.state.error=errorText(error);this.emit();
          await new Promise(resolve=>setTimeout(resolve,250));
          continue;
        }
        this.state.snapshot=snapshot;this.state.error=null;
        if(snapshot.status!=='running'){
          run.terminal=true;
          if(snapshot.status==='failed')throw new Error(snapshot.error||'התצוגה המקדימה נכשלה.');
          if(snapshot.status==='completed'&&!this.state.cancelRequested){
            const page=await this.host.page(started.job_id,0,PAGE_SIZE,this.state.developerMode,this.state.showInactive);
            if(!this.state.cancelRequested){this.state.page=page;this.retainedPreview={snapshot,request};}
          }
          break;
        }
        this.emit();
        await new Promise(resolve=>setTimeout(resolve,250));
      }
    }catch(error){this.state.error=errorText(error);}
    finally{
      run.terminal=true;this.activePreview=null;
      this.state.busy=false;this.state.activity=null;this.emit();
    }
  }
  get needsAlbumSourceParent():boolean {
    const snapshot=this.state.snapshot,retained=this.retainedPreview;
    return this.state.model?.platform==='android'&&!!this.host.selectAlbumSourceParent&&snapshot?.status==='completed'&&!!snapshot.plan_id&&snapshot.apply?.source_parent_selection_required===true&&snapshot.apply.copy_mode===false&&snapshot.apply.mode!=='FixNames'&&!this.state.execution&&retained?.snapshot===snapshot&&retained.request.mode==='Sort';
  }
  get canSelectAlbumSourceParent():boolean {
    return this.needsAlbumSourceParent&&this.state.hostAvailable&&!this.state.busy&&!this.state.source.pending&&!this.state.target.pending;
  }
  /** A disappearing view cannot use a late picker reply to grant or restart work. */
  discardAlbumSourceParentSelection(){
    if(!this.activeAlbumParent)return;
    this.activeAlbumParent=null;this.invalidate();this.state.busy=false;this.state.activity=null;this.emit();
  }
  /** True requests a fresh Sort preview, never Apply. The returned parent token is not stored in either root. */
  async selectAlbumSourceParent():Promise<boolean>{
    if(!this.canSelectAlbumSourceParent)return false;
    const retained=this.retainedPreview!,snapshot=retained.snapshot,jobId=snapshot.job_id,planId=snapshot.plan_id!;
    const sourceRevision=this.state.source.revision,targetRevision=this.state.target.revision,run={};
    const current=()=>this.activeAlbumParent===run&&this.state.activity==='album-parent'&&this.needsAlbumSourceParent&&this.retainedPreview===retained&&this.state.snapshot===snapshot&&snapshot.job_id===jobId&&snapshot.plan_id===planId&&this.state.source.revision===sourceRevision&&this.state.target.revision===targetRevision&&this.state.source.selection?.token===retained.request.source_token&&this.state.target.selection?.token===retained.request.target_token&&this.state.ai===retained.request.ai_profile&&(Object.keys(defaults) as (keyof Settings)[]).every(key=>this.state.settings[key]===retained.request.settings[key]);
    this.activeAlbumParent=run;this.state.busy=true;this.state.activity='album-parent';this.state.error=null;this.emit();
    try{
      const selected=await this.host.selectAlbumSourceParent!(jobId,planId);
      if(!current()||selected===null)return false;
      if(!selected||selected.platform!=='android'||typeof selected.token!=='string'||!selected.token||typeof selected.display_name!=='string'||selected.capabilities?.read!==true||selected.capabilities.list!==true)throw new Error('בחירת תיקיית האב לא אומתה. יש לבצע בדיקה חדשה.');
      // The native host invalidated this exact preview. Keep both original roots,
      // settings and AI choice; the existing UI starts a new Sort preview next.
      this.invalidate();return true;
    }catch(error){if(current())this.state.error=errorText(error);return false;}
    finally{if(this.activeAlbumParent===run){this.activeAlbumParent=null;if(this.state.activity==='album-parent'){this.state.busy=false;this.state.activity=null;}this.emit();}}
  }
  get canApply():boolean {
    const snapshot=this.state.snapshot;
    return !this.state.busy&&(!this.state.recoveryNotice||(this.state.recoveryNotice.status==='attention'&&this.state.model?.platform==='android'&&snapshot?.apply?.recovery_scope_admitted===true))&&this.state.hostAvailable&&snapshot?.status==='completed'&&!!snapshot.plan_id&&snapshot.apply?.supported===true&&snapshot.apply.source_parent_selection_required!==true&&!this.state.execution&&!!this.host.startApply&&!!this.host.executionSnapshot&&!!this.host.cancelApply;
  }
  cancelApply(){
    const run=this.activeApply;if(!run||run.terminal)return;
    this.state.cancelRequested=true;this.emit();this.sendApplyCancellation(run);
  }
  private sendApplyCancellation(run:NonNullable<PreviewController['activeApply']>){
    if(!run.started||run.cancelSent||run.terminal||!this.host.cancelApply)return;
    run.cancelSent=true;
    void this.host.cancelApply(run.jobId).catch(error=>{
      if(this.activeApply!==run||run.terminal)return;
      run.cancelSent=false;this.state.error=errorText(error);this.emit();
    });
  }
  async apply(){
    if(!this.canApply)return;
    const preview=this.state.snapshot!,planId=preview.plan_id!;
    const run={jobId:preview.job_id,started:false,cancelSent:false,terminal:false};
    this.activeApply=run;this.state.busy=true;this.state.activity='apply';this.state.cancelRequested=false;this.state.error=null;this.emit();
    let lastSnapshotJson:string|undefined;
    try{
      try{
        const started=await this.host.startApply!(run.jobId,planId);
        if(started.job_id!==run.jobId)throw new Error('התקבלה תשובה שאינה שייכת לפעולה הנוכחית.');
        run.started=true;
      }catch(error){
        // Explicit native refusal is terminal. A lost IPC response is not proof
        // that mutation never started: retain ownership and ask the known job.
        if(error&&typeof error==='object'&&'code' in error)throw error;
        this.state.error=errorText(error);this.emit();
      }
      for(;;){
        if(this.state.cancelRequested)this.sendApplyCancellation(run);
        let snapshot:ExecutionSnapshot;
        try{
          snapshot=await this.host.executionSnapshot!(run.jobId);
          if(snapshot.host_schema!==1||snapshot.job_id!==run.jobId||snapshot.plan_id!==planId||!['preparing','running','complete','partial','cancelled','failed','needs_recovery'].includes(snapshot.status))throw new Error('מצב הפעולה לא אומת. ממתין לתשובה מהמארח.');
        }catch(error){
          if(error&&typeof error==='object'&&'code' in error&&['APPLY_NOT_STARTED','UNKNOWN_JOB'].includes(String(error.code))){
            throw new Error('מצב הביצוע אינו זמין. יש לבדוק את יומן הפעולה לפני בדיקה או ביצוע נוספים.');
          }
          this.state.error=errorText(error);this.emit();await new Promise(resolve=>setTimeout(resolve,250));continue;
        }
        run.started=true;
        if(this.state.cancelRequested&&['running','preparing'].includes(snapshot.status))this.sendApplyCancellation(run);
        const active=snapshot.status==='running'||snapshot.status==='preparing';
        // Compare the entire validated IPC JSON, including clocks and future fields.
        // Polling and cancellation still run every time; terminal delivery and error
        // clearing must notify even when the execution data itself is unchanged.
        const snapshotJson=JSON.stringify(snapshot);
        const notify=!active||snapshotJson!==lastSnapshotJson||this.state.error!==null;
        lastSnapshotJson=snapshotJson;
        this.state.execution=snapshot;this.state.error=null;if(notify)this.emit();
        if(!active){
          run.terminal=true;
          if(snapshot.error)this.state.error=snapshot.error;
          // Reconcile display filtering against the actual terminal outcomes.
          // This does not alter the retained plan, execution or its counters.
          try{
            this.state.page=await this.host.page(run.jobId,0,PAGE_SIZE,this.state.developerMode,this.state.showInactive);
            this.state.offset=0;
          }catch(error){this.state.error=errorText(error);}
          break;
        }
        await new Promise(resolve=>setTimeout(resolve,250));
      }
    }catch(error){
      this.state.error=errorText(error);
      // A refused/unknown attempt is never automatically retryable from the old plan.
      if(this.state.snapshot?.apply)this.state.snapshot={...this.state.snapshot,apply:{...this.state.snapshot.apply,supported:false,reason:'יש לבצע בדיקה חדשה לפני ניסיון נוסף.'}};
    }finally{run.terminal=true;this.activeApply=null;this.state.busy=false;this.state.activity=null;this.emit();}
  }
  async setDeveloperMode(enabled:boolean){
    if(this.state.busy||this.state.developerMode===enabled)return;
    const snapshot=this.state.snapshot;
    if(!snapshot||snapshot.status!=='completed'){this.state.developerMode=enabled;this.emit();return;}
    this.state.busy=true;this.state.activity='page';this.state.error=null;this.emit();
    try{
      const page=await this.host.page(snapshot.job_id,0,PAGE_SIZE,enabled,this.state.showInactive);
      this.state.page=page;this.state.offset=0;this.state.developerMode=enabled;
    }catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
  async setShowInactive(enabled:boolean){
    if(this.state.busy||this.state.showInactive===enabled)return;
    const snapshot=this.state.snapshot;
    if(!snapshot||snapshot.status!=='completed'){this.state.showInactive=enabled;this.emit();return;}
    this.state.busy=true;this.state.activity='page';this.state.error=null;this.emit();
    try{
      const page=await this.host.page(snapshot.job_id,0,PAGE_SIZE,this.state.developerMode,enabled);
      this.state.page=page;this.state.offset=0;this.state.showInactive=enabled;
    }catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
  async changePage(offset:number){
    const snapshot=this.state.snapshot;
    if(this.state.busy||!snapshot||snapshot.status!=='completed'||!Number.isSafeInteger(offset)||offset<0||offset>=(this.state.page?.total??0))return;
    this.state.busy=true;this.state.activity='page';this.state.error=null;this.emit();
    try{const page=await this.host.page(snapshot.job_id,offset,PAGE_SIZE,this.state.developerMode,this.state.showInactive);this.state.page=page;this.state.offset=offset;}
    catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
}
