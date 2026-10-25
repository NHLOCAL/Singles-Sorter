/** Host display contract v1. A display path never becomes an authority token. */
export type Role = 'Source' | 'Target';
export type Mode = 'Sort' | 'FixNames';
export type Platform = 'windows' | 'android' | 'linux_test';
export interface Settings {
  copy_mode: boolean; main_folder_only: boolean; singles_folder: boolean;
  exist_only: boolean; abc_sort: boolean; duet_mode: boolean;
}
export const defaults: Readonly<Settings> = Object.freeze({copy_mode:false,main_folder_only:false,singles_folder:false,exist_only:false,abc_sort:false,duet_mode:false});
export interface RootSelection {token:string;display_name:string;capabilities:{read:boolean;list:boolean};platform:Platform}
export interface PreviewRequest {mode:Mode;source_token:string;target_token:string|null;settings:Settings;ai_profile:'Disabled'|'Configured'}
export interface ModelStatus {execution_available?:boolean;available:boolean;message:string;platform:Platform}
export interface Snapshot {host_schema:1;job_id:string;status:'running'|'completed'|'failed'|'cancelled';plan_id:string|null;total_rows:number;blockers:string[];warnings:string[];error?:string|null;apply?:ApplyReadiness|null}
export interface PlanRow {id:string;source:string;kind:string;destination:string|null;reason:string;blockers:string[];effective_blockers?:string[];operation_ids?:number[];artist?:string|null;developer_only?:boolean;no_action?:boolean;included_in_album?:number}
export interface PlanPage {rows:PlanRow[];total:number;skipped?:number}
export interface RecoveryNotice {status:'attention'|'unreadable';journal_count:number;acknowledged_operations:number|null;unresolved_operations:number|null}
export interface Host {
  selectRoot(role:Role):Promise<RootSelection|null>;
  resolveRoot(role:Role,text:string):Promise<RootSelection>;
  getSettings():Promise<Settings>;
  getModelStatus():Promise<ModelStatus>;
  startPreview(request:PreviewRequest):Promise<{job_id:string}>;
  cancelPreview(jobId:string):Promise<void>;
  snapshot(jobId:string):Promise<Snapshot>;
  page(jobId:string,offset:number,limit:number,developerMode?:boolean,showInactive?:boolean):Promise<PlanPage>;
  /** Explicit Android picker for a retained Album's exact source parent; never a Source/Target role. */
  selectAlbumSourceParent?(jobId:string,planId:string):Promise<RootSelection|null>;
  startApply?(jobId:string,planId:string):Promise<{job_id:string}>;
  executionSnapshot?(jobId:string):Promise<ExecutionSnapshot>;
  cancelApply?(jobId:string):Promise<void>;
  latestExecution?():Promise<{preview:Snapshot;execution:ExecutionSnapshot}|{recovery_notice:RecoveryNotice}|null>;
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
export interface MetadataDiagnostics {failed_observations:number;affected_sources:number;truncated:boolean;issues:{observation_index:number;after_operations:number;relative_source:string;label_truncated:boolean;error_kind:string;storage_kind?:string}[]}
export interface ApplyReadiness {metadata_diagnostics?:MetadataDiagnostics;duet_groups?:number;supported:boolean;recovery_scope_admitted?:boolean;source_parent_selection_required?:boolean;reason:string|null;operation_count:number;copy_mode:boolean;source:string;target:string|null;mode?:Mode;notice?:string|null}
export type ExecutionStatus='preparing'|'running'|'complete'|'partial'|'cancelled'|'failed'|'needs_recovery';
export type ItemStatus='partial'|'complete'|'source_retained'|'failed'|'cancelled'|'blocked'|'needs_recovery'|'unsupported_tags'|'metadata_failure';
/** Optional display-only observed leaf; older/Android hosts may omit it. */
export interface RepairOutcome {renamed:boolean;repaired:boolean;unchanged:boolean;observed_filename?:string|null}
export interface RepairSummary {total_files:number;processed_files:number;renamed_files:number;repaired_files:number;unchanged_files:number;unsupported_files:number;failed_files:number;retained_backup_bytes:number}
export interface ExecutionItem {duet_retirement_status?:'pending'|'retired'|'needs_recovery';operation:number;status:ItemStatus;detail:string;repair?:RepairOutcome|null}
export interface LegacySummary {songs_sorted:number;artist_folders_created:number;albums_processed:number;top_artists:[string,number][]}
export interface ExecutionResult {duets?:{retired_sources:number;no_action_sources:number;unfinished_retirements:number}|null;no_op?:boolean;legacy_summary?:LegacySummary|null;source_items:number;committed_destinations:number;moved_sources:number;failed_operations:number;blocked_operations:number;recovery_operations:number;cancelled_operations:number;items:ExecutionItem[];repair?:RepairSummary|null;albums?:{completed:number;child_names_changed:number;created_directories:number;removed_directories:number;retained_directories:number}|null}
export interface VerificationProgress {completed_operations:number;total_operations:number;bytes_read:number;elapsed_ms:number}
export interface ExecutionSnapshot {verification?:VerificationProgress|null;host_schema:1;job_id:string;plan_id:string;status:ExecutionStatus;operation_count:number;operation:number|null;bytes_written:number;source_length:number;journal_path:string|null;error:string|null;result:ExecutionResult|null;recovery_error?:string|null;recovery?:{observed_directories:number;pending_directories:number;rejected_directories:number;incomplete_tail:boolean;transfers:{operation:number;status:string}[];pending_operations?:number[];directory_evidence?:{reference:unknown;role:string;status:string}[];metadata_issues?:number|null;terminal_records_complete?:boolean|null;append_blocked?:boolean|null}|null}

/** Recorded journals are evidence for a new read-only inspection, never replay authority. */
export type RecoveryVerificationScope='recorded_main_hash_identity_membership';
export interface RecoveryRecord {verification_scope?:RecoveryVerificationScope;token:string;job_id:string|null;journal_name:string;family:'single'|'album'|'repair'|'orphan';source:string|null;target:string|null;original_parent?:string|null;state:'recorded_complete'|'inspection_needed'|'invalid'|'unsupported';detail:string;operation_count:number}
export interface RecoveryPage {records:RecoveryRecord[];total:number;offset:number;truncated:boolean}
export interface RecoveryEvidence {role:string;path:string;status:string}
export interface RecoveryOperation {operation:number;source:string;destination:string;status:string;evidence?:RecoveryEvidence[]}
export interface RecoveryReport {verification_scope?:RecoveryVerificationScope;job_id:string;state:'consistent'|'needs_attention';observed_directories:number;pending_directories:number;rejected_directories:number;incomplete_tail:boolean;unverified_directories:number;observation_limit_reached:boolean;operations:RecoveryOperation[];evidence?:RecoveryEvidence[]}
export interface RecoveryProgress {stage:string;operation:number|null;total_operations:number;completed_operations:number;records_checked:number;observations:number;bytes_read:number;elapsed_ms:number}
export interface RecoveryRunId {run_id:string}
export interface RecoverySnapshot {run_id:string;kind:'list'|'inspect';status:'running'|'cancelling'|'completed'|'cancelled'|'failed';progress:RecoveryProgress;page:RecoveryPage|null;report:RecoveryReport|null;error:{code:string;message:string}|null}
export interface RecoveryHost {
  startRecoveryList(offset:number,limit:number):Promise<RecoveryRunId>;
  selectRecoveryRoot(role:Role):Promise<RootSelection|null>;
  selectRecoveryParent?(recordToken:string):Promise<RootSelection|null>;
  startRecoveryInspection(token:string,sourceToken:string,targetToken:string|null,resumeRunId:string|null):Promise<RecoveryRunId>;
  recoverySnapshot(runId:string):Promise<RecoverySnapshot>;
  cancelRecovery(runId:string):Promise<void>;
}
