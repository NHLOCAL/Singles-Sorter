import { invoke } from '@tauri-apps/api/core';
import { androidInvoke,hasAndroidHost } from './android-transport';
const invokeHost = <T>(command:string,args:Record<string,unknown>={}):Promise<T> => hasAndroidHost()?androidInvoke<T>(command,args):invoke<T>(command,args);
import type { Host,ProfileHost } from './contracts';
/** No generic shell, filesystem or arbitrary URL command is exposed here.
 * Apply accepts only a retained job and its inspected plan digest. */
export const nativeHost: Host & ProfileHost = {
  saveMainSettings: settings => invokeHost('save_main_settings',{settings}),
  setDuet: enabled => invokeHost('set_duet',{enabled}),
  getPersonalList: () => invokeHost('get_personal_list'),
  savePersonalList: (revision,rows) => invokeHost('save_personal_list',{revision,rows}),
  selectPersonalImport: () => invokeHost('select_personal_import'),
  previewPersonalImport: (token,encoding) => invokeHost('preview_personal_import',{token,encoding}),
  confirmPersonalImport: (token,encoding,revision,edits=[]) => invokeHost('confirm_personal_import',{token,encoding,revision,edits}),
  exportPersonalList: () => invokeHost('export_personal_list'),
  getFirstRunStatus: () => invokeHost('get_first_run_status'),
  markFirstRunSeen: () => invokeHost('mark_first_run_seen'),
  selectRoot: role => invokeHost('select_root',{role}),
  resolveRoot: (role,text) => invokeHost('resolve_typed_root',{role,text}),
  getSettings: () => invokeHost('get_settings'),
  getModelStatus: () => invokeHost('get_model_status'),
  startPreview: request => invokeHost('start_preview',{request}),
  cancelPreview: jobId => invokeHost('cancel_preview',{jobId}),
  snapshot: jobId => invokeHost('job_snapshot',{jobId}),
  page: (jobId,offset,limit) => invokeHost('get_plan_page',{jobId,offset,limit}),
  startApply: (jobId,planId) => invokeHost('start_apply',{jobId,planId}),
  executionSnapshot: jobId => invokeHost('execution_snapshot',{jobId}),
  cancelApply: jobId => invokeHost('cancel_apply',{jobId}),
  latestExecution: () => invokeHost('latest_execution'),
};

/** The host owns the single allowlisted destination; no URL arrives from JS. */
export const openFooterLink = ():Promise<void> => invokeHost('open_footer_link');
