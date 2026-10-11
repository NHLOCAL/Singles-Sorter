import { invoke } from '@tauri-apps/api/core';
import type { Host,ProfileHost } from './contracts';
/** No generic shell, filesystem or arbitrary URL command is exposed here. Apply accepts only a retained job and its inspected plan digest. */
export const nativeHost: Host & ProfileHost = {
  saveMainSettings: settings => invoke('save_main_settings',{settings}),
  setDuet: enabled => invoke('set_duet',{enabled}),
  getPersonalList: () => invoke('get_personal_list'),
  savePersonalList: (revision,rows) => invoke('save_personal_list',{revision,rows}),
  selectPersonalImport: () => invoke('select_personal_import'),
  previewPersonalImport: (token,encoding) => invoke('preview_personal_import',{token,encoding}),
  confirmPersonalImport: (token,encoding,revision,edits=[]) => invoke('confirm_personal_import',{token,encoding,revision,edits}),
  exportPersonalList: () => invoke('export_personal_list'),
  getFirstRunStatus: () => invoke('get_first_run_status'),
  markFirstRunSeen: () => invoke('mark_first_run_seen'),
  selectRoot: role => invoke('select_root',{role}),
  resolveRoot: (role,text) => invoke('resolve_typed_root',{role,text}),
  getSettings: () => invoke('get_settings'),
  getModelStatus: () => invoke('get_model_status'),
  startPreview: request => invoke('start_preview',{request}),
  cancelPreview: jobId => invoke('cancel_preview',{jobId}),
  snapshot: jobId => invoke('job_snapshot',{jobId}),
  page: (jobId,offset,limit) => invoke('get_plan_page',{jobId,offset,limit}),
  startApply: (jobId,planId) => invoke('start_apply',{jobId,planId}),
  executionSnapshot: jobId => invoke('execution_snapshot',{jobId}),
  cancelApply: jobId => invoke('cancel_apply',{jobId}),
  latestExecution: () => invoke('latest_execution'),
};

/** The host owns the single allowlisted destination; no URL arrives from JS. */
export const openFooterLink = ():Promise<void> => invoke('open_footer_link');
