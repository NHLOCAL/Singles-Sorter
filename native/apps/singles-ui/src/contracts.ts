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
export interface Snapshot {host_schema:1;job_id:string;status:'running'|'completed'|'failed'|'cancelled';plan_id:string|null;total_rows:number;blockers:string[];warnings:string[];error?:string|null;apply?:ApplyReadiness|null}
export interface PlanRow {id:string;source:string;kind:string;destination:string|null;reason:string;blockers:string[];operation_ids?:number[];artist?:string|null;developer_only?:boolean}
export interface PlanPage {rows:PlanRow[];total:number;skipped?:number}
export interface Host {
  selectRoot(role:Role):Promise<RootSelection|null>;
  resolveRoot(role:Role,text:string):Promise<RootSelection>;
  getSettings():Promise<Settings>;
  getModelStatus():Promise<ModelStatus>;
  startPreview(request:PreviewRequest):Promise<{job_id:string}>;
  cancelPreview(jobId:string):Promise<void>;
  snapshot(jobId:string):Promise<Snapshot>;
  page(jobId:string,offset:number,limit:number,developerMode?:boolean):Promise<PlanPage>;
  startApply?(jobId:string,planId:string):Promise<{job_id:string}>;
  executionSnapshot?(jobId:string):Promise<ExecutionSnapshot>;
  cancelApply?(jobId:string):Promise<void>;
  latestExecution?():Promise<{preview:Snapshot;execution:ExecutionSnapshot}|null>;
}

export type MainSettings = Omit<Settings,'duet_mode'>;
export interface ListDiagnostic {index:number;code:string;width:number}
export interface PersonalList {revision:string;rows:string[][];diagnostics:ListDiagnostic[]}
export type ImportEncoding = 'Utf8'|'Utf8Bom'|'Windows1255'|'Windows1252'|'Utf16Le'|'Utf16Be';
export type ImportEdit={kind:'set';index:number;column:0|1;value:string}|{kind:'delete';index:number};
export interface ImportPreview {token:string;encoding:ImportEncoding;rows:string[][];diagnostics:ListDiagnostic[]}
export interface ImportSelection {token:string;filename:string;encoding_required:boolean;preview:ImportPreview|null}
export interface ProfileHost {
 saveMainSettings(settings:MainSettings):Promise<Settings>;
 setDuet(enabled:boolean):Promise<Settings>;
 getPersonalList():Promise<PersonalList>;
 savePersonalList(revision:string,rows:string[][]):Promise<PersonalList>;
 selectPersonalImport():Promise<ImportSelection|null>;
 previewPersonalImport(token:string,encoding:ImportEncoding):Promise<ImportPreview>;
 confirmPersonalImport(token:string,encoding:ImportEncoding,revision:string,edits?:ImportEdit[]):Promise<PersonalList>;
 exportPersonalList():Promise<{filename:string;rows:number}|null>;
 getFirstRunStatus():Promise<{version:string;seen:boolean}>;
 markFirstRunSeen():Promise<{version:string;seen:boolean}>;
}

/** Apply eligibility is granted by retained native evidence, never inferred from rows. */
export interface ApplyReadiness {supported:boolean;reason:string|null;operation_count:number;copy_mode:boolean;source:string;target:string}
export type ExecutionStatus='preparing'|'running'|'complete'|'partial'|'cancelled'|'failed'|'needs_recovery';
export type ItemStatus='complete'|'source_retained'|'failed'|'cancelled'|'blocked'|'needs_recovery';
export interface ExecutionItem {operation:number;status:ItemStatus;detail:string}
export interface ExecutionResult {source_items:number;committed_destinations:number;moved_sources:number;failed_operations:number;blocked_operations:number;recovery_operations:number;cancelled_operations:number;items:ExecutionItem[]}
export interface ExecutionSnapshot {host_schema:1;job_id:string;plan_id:string;status:ExecutionStatus;operation_count:number;operation:number|null;bytes_written:number;source_length:number;journal_path:string|null;error:string|null;result:ExecutionResult|null;recovery?:{observed_directories:number;pending_directories:number;rejected_directories:number;incomplete_tail:boolean;transfers:{operation:number;status:string}[]}|null}
