import {describe,it,expect} from 'vitest';
import {executionRefusalSummary,executionRefusalDetails} from '../src/result-presentation';
import type {ApplyReadiness} from '../src/contracts';
const refused:ApplyReadiness={supported:false,reason:'התוכנית כוללת פעולות שאינן זמינות לביצוע בטוח במארח זה (Unsupported)',operation_count:63,copy_mode:true,source:'C:\\private\\source',target:'D:\\private\\target',mode:'Sort'};
describe('ordinary refused-plan explanation',()=>{
 it('explains the exact observed metadata blocker without exposing paths or claiming completion',()=>{
  const value=executionRefusalSummary(refused,['METADATA_ADAPTER_LIMITATION','PHYSICAL_APPLY_UNAVAILABLE']);
  expect(value).toContain('קריאת פרטי הקבצים');expect(value).toContain('פרטי החסימה');expect(value).toContain('לא בוצעו שינויים');
  expect(value).not.toContain('Unsupported');expect(value).not.toContain('63');expect(value).not.toContain('private');
 });
 it('uses exact known blocker categories, not substrings in raw diagnostic text',()=>{
  const value=executionRefusalSummary({...refused,reason:'private METADATA_ADAPTER_LIMITATION trace'},['unknown_METADATA_ADAPTER_LIMITATION']);
  expect(value).toContain('התוכנית לא אושרה לביצוע');expect(value).not.toContain('קריאת פרטי הקבצים');expect(value).not.toContain('private');
  expect(executionRefusalSummary(refused,['SOURCE_TARGET_OVERLAP'])).toContain('בחר תיקיות נפרדות');
 });
 it('keeps exact host reason, full-plan counts and all codes available only as diagnostic text',()=>{
  const value=executionRefusalDetails(refused,['METADATA_ADAPTER_LIMITATION','PHYSICAL_APPLY_UNAVAILABLE']);
  expect(value).toContain(refused.reason);expect(value).toContain('63');expect(value).toContain('METADATA_ADAPTER_LIMITATION');
  expect(value).not.toContain(refused.source);expect(value).not.toContain(refused.target);
 });
 it('handles missing reasons and zero operations without inventing a cause or executed files',()=>{
  const value=executionRefusalDetails({...refused,reason:null,operation_count:0},[]);
  expect(value).toContain('לא נמסרה סיבה מפורטת');expect(value).toContain('0');
  expect(executionRefusalSummary({...refused,reason:null},[])).toContain('התוכנית לא אושרה לביצוע');
 });
 it('shows truthful full-observation and distinct-source counts with relative typed details',()=>{
  const readiness={...refused,metadata_diagnostics:{failed_observations:27,affected_sources:14,truncated:true,issues:Array.from({length:20},(_,n)=>({observation_index:n+3,after_operations:n,relative_source:`אלבום/שיר${n}.mp3`,label_truncated:false,error_kind:n===0?'Storage':'UnsupportedClass',...(n===0?{storage_kind:'Changed'}:{})}))}};
  const summary=executionRefusalSummary(readiness,['METADATA_ADAPTER_LIMITATION']);
  expect(summary).toContain('27');expect(summary).toContain('14');expect(summary).not.toContain('שיר');
  const detail=executionRefusalDetails(readiness,['METADATA_ADAPTER_LIMITATION']);
  expect(detail).toContain('Storage / Changed');expect(detail).toContain('אותו קובץ עשוי להיבדק יותר מפעם אחת');
  expect(detail).toContain('20 התצפיות הראשונות בלבד מתוך 27');
  expect(detail).toContain('\u2068אלבום/שיר0.mp3\u2069');expect(detail).not.toContain('private');
 });
 it('marks shortened labels without claiming all files were listed',()=>{
  const readiness={...refused,metadata_diagnostics:{failed_observations:1,affected_sources:1,truncated:false,issues:[{observation_index:7,after_operations:0,relative_source:'x'.repeat(512),label_truncated:true,error_kind:'Parser'}]}};
  const detail=executionRefusalDetails(readiness,[]);expect(detail).toContain('[השם קוצר]');expect(detail).not.toContain('התצפיות הראשונות בלבד');
 });
 it('does not clutter supported or absent readiness even when the planning-only marker remains',()=>{
  for(const value of [undefined,null,{...refused,supported:true}]){
   expect(executionRefusalSummary(value,['METADATA_ADAPTER_LIMITATION'])).toBe('');
   expect(executionRefusalDetails(value,['PHYSICAL_APPLY_UNAVAILABLE'])).toBe('');
  }
 });
});
