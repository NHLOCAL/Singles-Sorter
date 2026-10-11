import {defaults} from './contracts';
import type {Host,ProfileHost,Mode,ModelStatus,PlanPage,Role,RootSelection,Settings,Snapshot} from './contracts';
export const PAGE_SIZE = 50;
const rootKeys = {Source:'source',Target:'target'} as const;
export interface RootInput {text:string;selection:RootSelection|null;pending:boolean;revision:number}
export interface State {
  source:RootInput;target:RootInput;settings:Settings;model:ModelStatus|null;
  initialized:boolean;hostAvailable:boolean;busy:boolean;activity:'preview'|'settings'|'page'|null;error:string|null;
  snapshot:Snapshot|null;page:PlanPage|null;offset:number;ai:'Disabled'|'Configured';
}
const newRoot = ():RootInput => ({text:'',selection:null,pending:false,revision:0});
export function errorText(error:unknown):string {
  if(typeof error==='string')return error;
  if(error instanceof Error)return error.message;
  if(error && typeof error==='object' && 'message' in error){
    const code='code' in error?String(error.code):'';
    const labels:Record<string,string>={
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
    return labels[code]?`${labels[code]} (${code})`:String(error.message);
  }
  return 'הפעולה לא הושלמה. נסה שוב לאחר בדיקת החיבור לתוכנה.';
}
export class PreviewController {
  readonly state:State={source:newRoot(),target:newRoot(),settings:{...defaults},model:null,initialized:false,hostAvailable:false,busy:false,activity:null,error:null,snapshot:null,page:null,offset:0,ai:'Disabled'};
  private listeners=new Set<()=>void>();
  savedNotice:string|null=null;
  settingsDirty=false;
  constructor(private host:Host,private profile?:ProfileHost){}
  subscribe(listener:()=>void):()=>void{this.listeners.add(listener);return()=>this.listeners.delete(listener);}
  private emit(){for(const listener of this.listeners)listener();}
  private invalidate(){this.state.snapshot=null;this.state.page=null;this.state.offset=0;this.state.error=null;}
  async initialize(){
    try{
      const [settings,model]=await Promise.all([this.host.getSettings(),this.host.getModelStatus()]);
      for(const key of Object.keys(defaults) as (keyof Settings)[]){if(typeof settings[key]!=='boolean')throw new Error('הגדרות התוכנה אינן תקינות.');}
      this.state.settings={...settings};this.state.model=model;this.state.hostAvailable=true;
    }catch(error){this.state.error=errorText(error);this.state.hostAvailable=false;}
    this.state.initialized=true;this.emit();
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
  async preview(mode:Mode){
    if(this.state.busy||this.state.source.pending||this.state.target.pending||!this.state.hostAvailable)return;
    this.invalidate();this.state.busy=true;this.state.activity='preview';this.emit();
    try{
      const source=await this.resolve('Source');
      const target=mode==='Sort'?await this.resolve('Target'):null;
      const started=await this.host.startPreview({mode,source_token:source.token,target_token:target?.token??null,settings:{...this.state.settings},ai_profile:this.state.ai});
      const snapshot=await this.host.snapshot(started.job_id);
      if(snapshot.host_schema!==1||snapshot.job_id!==started.job_id)throw new Error('התקבלה תשובה שאינה שייכת לבדיקה הנוכחית.');
      this.state.snapshot=snapshot;
      if(snapshot.status==='failed')throw new Error(snapshot.error||'התצוגה המקדימה נכשלה.');
      this.state.page=await this.host.page(started.job_id,0,PAGE_SIZE);
    }catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
  async changePage(offset:number){
    const snapshot=this.state.snapshot;
    if(this.state.busy||!snapshot||snapshot.status!=='completed'||!Number.isSafeInteger(offset)||offset<0||offset>=snapshot.total_rows)return;
    this.state.busy=true;this.state.activity='page';this.state.error=null;this.emit();
    try{const page=await this.host.page(snapshot.job_id,offset,PAGE_SIZE);this.state.page=page;this.state.offset=offset;}
    catch(error){this.state.error=errorText(error);}
    finally{this.state.busy=false;this.state.activity=null;this.emit();}
  }
}
