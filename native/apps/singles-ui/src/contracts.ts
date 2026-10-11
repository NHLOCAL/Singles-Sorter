/** Host display contract v1. A display path never becomes an authority token. */
export type Role = 'Source' | 'Target';
export type Mode = 'Sort' | 'FixNames';
export type Platform = 'windows' | 'android' | 'linux_test';
export interface Settings {
  copy_mode: boolean; main_folder_only: boolean; singles_folder: boolean;
  exist_only: boolean; abc_sort: boolean; duet_mode: boolean;
}
export const defaults: Readonly<Settings> = Object.freeze({copy_mode:false,main_folder_only:false,singles_folder:true,exist_only:false,abc_sort:false,duet_mode:false});
export interface RootSelection {token:string;display_name:string;capabilities:{read:boolean;list:boolean};platform:Platform}
export interface PreviewRequest {mode:Mode;source_token:string;target_token:string|null;settings:Settings;ai_profile:'Disabled'|'Configured'}
export interface ModelStatus {available:boolean;message:string;platform:Platform}
export interface Snapshot {host_schema:1;job_id:string;status:'completed'|'failed';plan_id:string|null;total_rows:number;blockers:string[];warnings:string[];error?:string|null}
export interface PlanRow {id:string;source:string;kind:string;destination:string|null;reason:string;blockers:string[]}
export interface PlanPage {rows:PlanRow[];total:number}
export interface Host {
  selectRoot(role:Role):Promise<RootSelection|null>;
  resolveRoot(role:Role,text:string):Promise<RootSelection>;
  getSettings():Promise<Settings>;
  getModelStatus():Promise<ModelStatus>;
  startPreview(request:PreviewRequest):Promise<{job_id:string}>;
  snapshot(jobId:string):Promise<Snapshot>;
  page(jobId:string,offset:number,limit:number):Promise<PlanPage>;
}

export type MainSettings = Omit<Settings,'duet_mode'>;
export interface ListDiagnostic {index:number;code:string;width:number}
export interface PersonalList {revision:string;rows:string[][];diagnostics:ListDiagnostic[]}
export type ImportEncoding = 'Utf8'|'Utf8Bom'|'Windows1255'|'Windows1252'|'Utf16Le'|'Utf16Be';
export interface ImportPreview {token:string;encoding:ImportEncoding;rows:string[][];diagnostics:ListDiagnostic[]}
export interface ImportSelection {token:string;filename:string;encoding_required:boolean;preview:ImportPreview|null}
export interface ProfileHost {
 saveMainSettings(settings:MainSettings):Promise<Settings>;
 setDuet(enabled:boolean):Promise<Settings>;
 getPersonalList():Promise<PersonalList>;
 savePersonalList(revision:string,rows:string[][]):Promise<PersonalList>;
 selectPersonalImport():Promise<ImportSelection|null>;
 previewPersonalImport(token:string,encoding:ImportEncoding):Promise<ImportPreview>;
 confirmPersonalImport(token:string,encoding:ImportEncoding,revision:string):Promise<PersonalList>;
 exportPersonalList():Promise<{filename:string;rows:number}|null>;
 getFirstRunStatus():Promise<{version:string;seen:boolean}>;
 markFirstRunSeen():Promise<{version:string;seen:boolean}>;
}
