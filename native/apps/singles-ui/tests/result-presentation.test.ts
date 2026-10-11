import {describe,it,expect} from 'vitest';
import {readDeveloperMode,saveDeveloperMode,simpleStatus,simpleReason,simpleExecutionSummary,simpleError,DEVELOPER_PREFERENCE} from '../src/result-presentation';
import type {ExecutionSnapshot,PlanRow,ExecutionItem} from '../src/contracts';
const row:PlanRow={id:'file',source:'song.mp3',artist:'אמן',kind:'Single',destination:'אמן/song.mp3',reason:'raw_trace(hash)',blockers:[],operation_ids:[0]};
const item=(status:ExecutionItem['status']):ExecutionItem=>({operation:0,status,detail:'raw filesystem evidence'});
describe('ordinary result presentation',()=>{
 it('defaults off for absent, invalid or unavailable stored preferences',()=>{
  for(const value of [null,'false','TRUE','1','{}'])expect(readDeveloperMode({getItem:()=>value})).toBe(false);
  expect(readDeveloperMode({getItem:()=>{throw new Error('unavailable');}})).toBe(false);
 });
 it('remembers only the explicitly chosen developer preference',()=>{
  const store=new Map<string,string>();const storage={getItem:(key:string)=>store.get(key)??null,setItem:(key:string,value:string)=>{store.set(key,value);}};
  saveDeveloperMode(storage,true);expect(store.get(DEVELOPER_PREFERENCE)).toBe('true');expect(readDeveloperMode(storage)).toBe(true);
  saveDeveloperMode(storage,false);expect(readDeveloperMode(storage)).toBe(false);
 });
 it('shows short proposed actions, never technical traces',()=>{
  expect(simpleStatus(row,[],true)).toBe('להעתקה');expect(simpleStatus(row,[],false)).toBe('להעברה');
  expect(simpleStatus({...row,operation_ids:[]},[],false)).toBe('דולג');
  expect(simpleReason({...row,artist:null,operation_ids:[]})).toBe('לא זוהה אמן.');
  expect(simpleReason(row)).not.toContain(row.reason);
 });
 it('keeps failure, partial, cancellation and retained sources visible',()=>{
  expect(simpleStatus(row,[item('complete')],true)).toBe('הועתק');
  expect(simpleStatus(row,[item('complete'),item('failed')],true)).toBe('בוצע חלקית');
  expect(simpleStatus(row,[item('complete'),item('cancelled')],true)).toBe('בוצע חלקית');
  expect(simpleStatus(row,[item('cancelled')],true)).toBe('נעצר');
  expect(simpleStatus(row,[item('source_retained')],false)).toBe('הועתק; המקור נשמר');
  expect(simpleStatus(row,[item('needs_recovery')],false)).toBe('נדרשת בדיקה');
  expect(simpleStatus({...row,kind:'ItemError'},[],false)).toBe('לא ניתן לבדוק');
  expect(simpleStatus({...row,kind:'Vanished'},[],false)).toBe('הקובץ לא נמצא');
 });
 it('counts copies rather than inventing distinct completed source files',()=>{
  const execution:Pick<ExecutionSnapshot,'status'|'result'>={status:'partial',result:{source_items:1,committed_destinations:2,moved_sources:0,failed_operations:1,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[]}};
  expect(simpleExecutionSummary(execution,true)).toBe('נוצרו 2 עותקים · 1 לא הושלמו');
  expect(simpleExecutionSummary(execution,false)).toContain('0 קבצים הועברו · חלק מקובצי המקור נשמרו');
 });
 it('does not claim zero effects from missing final counters or expose raw errors',()=>{
  expect(simpleExecutionSummary({status:'failed',result:null},false)).toContain('לא התקבל אישור');
  expect(simpleError('Stale { expected_hash: "123" }')).not.toContain('123');
  expect(simpleError('בחר תיקיית מקור. (INVALID_ROOT)')).toBe('בחר תיקיית מקור.');
  expect(simpleError('RECOGNITION_UNAVAILABLE')).toContain('מודל הזיהוי');
 });
});
