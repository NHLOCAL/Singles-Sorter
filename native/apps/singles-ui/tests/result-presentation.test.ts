import {describe,it,expect} from 'vitest';
import {readDeveloperMode,saveDeveloperMode,simpleStatus,simpleReason,simpleExecutionSummary,simpleError,simpleRepairReason,verificationWarning,developerRowSummary,recoveryNoticeText,verificationProgressText,resultFilename,DEVELOPER_PREFERENCE,executionReadinessText,executionBytesText,REPAIR_REVIEW_TEXT} from '../src/result-presentation';
import type {ExecutionSnapshot,PlanRow,ExecutionItem,ApplyReadiness} from '../src/contracts';
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
 it('distinguishes planned album inclusion from an unexplained missing source',()=>{
  const included={...row,kind:'Vanished',operation_ids:[],included_in_album:0,no_action:true};
  expect(simpleStatus(included,[],false)).toBe('כלול באלבום');
  expect(simpleReason(included)).toBe('הפריט כלול בפעולת האלבום, ללא פעולה נפרדת.');
  expect(simpleStatus({...included,no_action:false},[],false)).toBe('נדרשת בדיקה');
  expect(simpleStatus({...included,included_in_album:undefined,no_action:false},[],false)).toBe('הקובץ לא נמצא');
 });
 it('keeps failure, partial, cancellation and retained sources visible',()=>{
  expect(simpleStatus(row,[item('complete')],true)).toBe('הועתק');
  expect(simpleStatus(row,[item('complete'),item('failed')],true)).toBe('בוצע חלקית');
  expect(simpleStatus(row,[item('complete'),item('cancelled')],true)).toBe('בוצע חלקית');
  expect(simpleStatus(row,[item('cancelled')],true)).toBe('נעצר');
  expect(simpleStatus(row,[item('source_retained')],false)).toBe('הועתק; המקור נשמר');
  expect(simpleStatus(row,[item('source_retained'),item('cancelled')],false)).toBe('בוצע חלקית');
  expect(simpleStatus(row,[item('source_retained'),item('blocked')],false)).toBe('בוצע חלקית');
  expect(simpleStatus(row,[item('needs_recovery')],false)).toBe('נדרשת בדיקה');
  expect(simpleStatus({...row,kind:'ItemError'},[],false)).toBe('לא ניתן לבדוק');
  expect(simpleStatus({...row,kind:'Vanished'},[],false)).toBe('הקובץ לא נמצא');
 });
 it('counts copies rather than inventing distinct completed source files',()=>{
  const execution:Pick<ExecutionSnapshot,'status'|'result'>={status:'partial',result:{source_items:1,committed_destinations:2,moved_sources:0,failed_operations:1,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[item('source_retained')]}};
  expect(simpleExecutionSummary(execution,true)).toBe('נוצרו 2 עותקים · 1 לא הושלמו');
  expect(simpleExecutionSummary(execution,false)).toContain('0 קבצים הועברו · חלק מקובצי המקור נשמרו');
 });
 it('shows retained originals only for an explicitly verified zero-effect completion',()=>{
  const result={no_op:true,source_items:0,committed_destinations:0,moved_sources:0,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[]};
  for(const copy of [true,false])expect(simpleExecutionSummary({status:'complete',result},copy)).toBe('לא בוצעו שינויים. קובצי המקור נשמרו.');
  expect(simpleExecutionSummary({status:'failed',result},false)).not.toContain('קובצי המקור נשמרו');
  expect(simpleExecutionSummary({status:'complete',result:{...result,no_op:undefined}},false)).not.toContain('קובצי המקור נשמרו');
  expect(simpleExecutionSummary({status:'complete',result:{...result,committed_destinations:1}},true)).not.toContain('לא בוצעו שינויים');
  expect(simpleExecutionSummary({status:'complete',result:{...result,failed_operations:1}},false)).not.toContain('לא בוצעו שינויים');
 });
 it('keeps duet copy acknowledgments separate from original retirement',()=>{
  const retained={...item('complete'),duet_retirement_status:'pending' as const};
  expect(simpleStatus(row,[retained],false)).toBe('הועתק; המקור נשמר');
  expect(simpleStatus(row,[{...retained,duet_retirement_status:'needs_recovery'}],false)).toBe('העותקים אומתו; המקור דורש בדיקה');
  expect(simpleStatus(row,[{...retained,duet_retirement_status:'retired'}],false)).toBe('הועבר');
  const result={source_items:2,committed_destinations:2,moved_sources:0,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[retained],duets:{retired_sources:0,no_action_sources:0,unfinished_retirements:1}};
  const summary=simpleExecutionSummary({status:'needs_recovery',result},false);
  expect(summary).toContain('2 יעדים אומתו');expect(summary).toContain('1 מחיקות מקור דואט לא הושלמו');expect(summary).not.toContain('קבצים הועברו');
  expect(simpleExecutionSummary({status:'complete',result:{...result,moved_sources:1,duets:{retired_sources:1,no_action_sources:0,unfinished_retirements:0}}},false)).toContain('1 מקורות דואט נמחקו לאחר אימות כל העותקים הנדרשים');
  expect(simpleExecutionSummary({status:'complete',result:{...result,no_op:true,source_items:0,committed_destinations:0,items:[],duets:{retired_sources:0,no_action_sources:1,unfinished_retirements:0}}},false)).toBe('לא בוצעו שינויים. קובצי המקור נשמרו.');
 });
 it('reports actual Fix outcomes without copy or move claims',()=>{
  const fix={...row,kind:'FixNames'};
  expect(simpleStatus({...fix,blockers:['Observation(PhysicalApplyUnavailable)']},[],false,{repairEligible:true})).toBe('לתיקון');
  expect(simpleStatus({...fix,blockers:['FIX_METADATA_READ_ERROR']},[],false,{repairEligible:true})).toBe('תיקון חלקי');
  expect(simpleStatus(fix,[],false,{terminalResult:true})).toBe('לא בוצע');
  expect(simpleStatus(fix,[],false,{uncertain:true})).toBe('לא אומת');
  const renamed={...item('complete'),repair:{renamed:true,repaired:false,unchanged:true}};
  expect(simpleStatus(fix,[renamed],false)).toBe('תוקן');
  expect(simpleStatus(fix,[{...renamed,repair:{renamed:false,repaired:false,unchanged:true}}],false)).toBe('ללא שינוי');
  const partial={...renamed,status:'unsupported_tags' as const};
  expect(simpleStatus(fix,[partial],false)).toBe('בוצע חלקית');
  expect(simpleRepairReason(partial)).toContain('שם הקובץ תוקן');
  expect(simpleStatus(fix,[{...partial,repair:{renamed:false,repaired:false,unchanged:false}}],false)).toBe('לא הושלם');
  const result={source_items:3,committed_destinations:0,moved_sources:0,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[partial],repair:{total_files:3,processed_files:3,renamed_files:2,repaired_files:1,unchanged_files:1,unsupported_files:1,failed_files:0,retained_backup_bytes:128}};
  const summary=simpleExecutionSummary({status:'partial',result},false);
  expect(summary).toContain('2 שמות תוקנו');expect(summary).toContain('1 קבצים עם תגיות מתוקנות');expect(summary).toContain('1 עם תגיות שלא תוקנו');
  expect(summary).not.toContain('הועברו');expect(summary).not.toContain('עותקים');expect(summary).not.toContain('128');
 });
 it('does not infer retained sources from successful multi-destination Move',()=>{
  const result={source_items:1,committed_destinations:2,moved_sources:1,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[item('complete'),item('complete')]};
  expect(simpleExecutionSummary({status:'complete',result},false)).toBe('1 קבצים הועברו');
  expect(simpleExecutionSummary({status:'partial',result:{...result,moved_sources:0,items:[item('source_retained')]}},false)).toContain('חלק מקובצי המקור נשמרו');
 });
 it('does not claim zero effects from missing final counters or expose raw errors',()=>{
  expect(simpleExecutionSummary({status:'failed',result:null},false)).toContain('לא התקבל אישור');
  expect(simpleError('Stale { expected_hash: "123" }')).not.toContain('123');
  expect(simpleError('Stale { path: C:\\שירים, expected_hash: "123" }')).not.toContain('123');
  expect(simpleError('שגיאה { expected_hash: "123" }')).not.toContain('123');
  expect(simpleError('בחר תיקיית מקור. (INVALID_ROOT)')).toBe('בחר תיקיית מקור.');
  expect(simpleError('RECOGNITION_UNAVAILABLE')).toContain('מודל הזיהוי');
 });
});

describe('repair verification uncertainty',()=>{
 it('keeps the ordinary warning short and reserves raw diagnostics for developer mode',()=>{
  const evidence={recovery_error:'CorruptJournal { C:\\שירים, hash: 123 }'};
  expect(verificationWarning(evidence)).toBe('הבדיקה הסופית לא הושלמה. בדוק את הקבצים והגיבויים לפני ניסיון נוסף.');
  expect(verificationWarning(evidence)).not.toContain('123');
  expect(verificationWarning(evidence,true)).toContain(evidence.recovery_error);
  expect(verificationWarning(null)).toBe('');expect(verificationWarning({})).toBe('');
 });
 it('does not turn observed completed effects into failed mutations',()=>{
  const result={source_items:1,committed_destinations:0,moved_sources:0,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[],repair:{total_files:1,processed_files:1,renamed_files:1,repaired_files:1,unchanged_files:0,unsupported_files:0,failed_files:0,retained_backup_bytes:512}};
  const execution={status:'complete' as const,result,recovery_error:'CorruptJournal'};
  const before=JSON.stringify(execution);
  expect(simpleExecutionSummary(execution,false)).toContain('1 שמות תוקנו');
  expect(verificationWarning(execution)).toContain('לא הושלמה');
  expect(JSON.stringify(execution)).toBe(before);
 });
});

describe('optional inactive rows',()=>{
 it('does not advertise unchanged Fix files as pending repairs',()=>{
  const row={id:'n',source:'unchanged.mp3',kind:'FixNames',destination:null,reason:'',blockers:[],operation_ids:[0],no_action:true};
  expect(simpleStatus(row,[],false,{repairEligible:true})).toBe('ללא שינוי');
  expect(simpleReason(row,true)).toContain('לא נמצא שינוי');
 });
});

describe('actual album result projection',()=>{
 it('keeps partial child-name effects visible without claiming a moved file',()=>{
  expect(simpleStatus({...row,kind:'Album'},[item('partial')],false)).toBe('בוצע חלקית');
  const result={source_items:3,committed_destinations:0,moved_sources:0,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[item('partial')],albums:{completed:0,child_names_changed:1,created_directories:1,removed_directories:0,retained_directories:0}};
  const summary=simpleExecutionSummary({status:'partial',result},false);
  expect(summary).toContain('0 קבצים הועברו');expect(summary).toContain('1 שמות באלבומים שונו');expect(summary).not.toContain('אלבומים הושלמו');
 });
 it('separates file effects, completed albums and retained source folders',()=>{
  const result={source_items:5,committed_destinations:5,moved_sources:5,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[],albums:{completed:1,child_names_changed:0,created_directories:2,removed_directories:0,retained_directories:1}};
  const summary=simpleExecutionSummary({status:'partial',result},false);
  expect(summary).toContain('5 קבצים הועברו');expect(summary).toContain('1 אלבומים הושלמו');expect(summary).toContain('נותרו תיקיות במקור');expect(summary).not.toContain('2');
 });
});


describe('observed repair filenames',()=>{
 const fix:PlanRow={...row,kind:'FixNames',source:'C:\\owned\\old_name.mp3',destination:'planned name.mp3'};
 const repaired:ExecutionItem={...item('complete'),repair:{renamed:true,repaired:true,unchanged:false,observed_filename:'שם מתוקן.mp3'}};
 it('uses only the matching actual leaf without mutating the retained source or plan',()=>{
  const before=JSON.stringify(fix);
  expect(resultFilename(fix,[repaired],'windows')).toBe('שם מתוקן.mp3');
  expect(JSON.stringify(fix)).toBe(before);
  expect(resultFilename(fix,[{...repaired,operation:99}],'windows')).toBe('old_name.mp3');
  expect(resultFilename(fix,[repaired,repaired],'windows')).toBe('old_name.mp3');
  expect(resultFilename({...fix,operation_ids:[0,1]},[repaired],'windows')).toBe('old_name.mp3');
  expect(resultFilename({...fix,kind:'Single'},[repaired],'windows')).toBe('old_name.mp3');
 });
 it('retains a verified filename-only result while leaving partial status intact',()=>{
  for(const status of ['unsupported_tags','metadata_failure'] as const){
   const partial={...repaired,status,repair:{...repaired.repair!,repaired:false}};
   expect(resultFilename(fix,[partial],'windows')).toBe('שם מתוקן.mp3');
   expect(simpleStatus(fix,[partial],false)).toBe('בוצע חלקית');
   expect(resultFilename(fix,[{...partial,repair:{...partial.repair,renamed:false}}],'windows')).toBe('old_name.mp3');
  }
 });
 it('falls back for preview, older hosts, Android display names and missing result evidence',()=>{
  expect(resultFilename(fix,[],'windows')).toBe('old_name.mp3');
  expect(resultFilename(fix,[item('complete')],'windows')).toBe('old_name.mp3');
  expect(resultFilename(fix,[{...repaired,repair:{renamed:true,repaired:true,unchanged:false}}],'windows')).toBe('old_name.mp3');
  expect(resultFilename({...fix,source:'שם מוצג.mp3'},[],'android')).toBe('שם מוצג.mp3');
  expect(resultFilename({...fix,source:'שם מוצג.mp3'},[repaired],'android')).toBe('שם מתוקן.mp3');
 });
 it('never promotes a plan, recovery, cancellation, failure or internal temporary name',()=>{
  for(const status of ['failed','cancelled','needs_recovery','partial','blocked','source_retained'] as const){
   const uncertain={...repaired,status};
   expect(resultFilename(fix,[uncertain],'windows')).toBe('old_name.mp3');
  }
  for(const observed_filename of [null,'','.', '..','.sorter-owned.backup','.sorter-owned.TMP','C:\\backup.mp3','dir/song.mp3','bad\0name.mp3']){
   expect(resultFilename(fix,[{...repaired,repair:{...repaired.repair!,observed_filename}}],'windows')).toBe('old_name.mp3');
  }
 });
 it('preserves valid mixed-direction, long and user-provided punctuation verbatim',()=>{
  const observed_filename='שלום — Song '.repeat(20)+'.flac';
  expect(resultFilename(fix,[{...repaired,repair:{...repaired.repair!,observed_filename}}],'windows')).toBe(observed_filename);
 });
});

describe('observed frozen-v14 summary',()=>{
 it('keeps physical counts separate from ordered original-artist credits',()=>{
  const result={source_items:8,committed_destinations:9,moved_sources:0,failed_operations:1,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[],
   legacy_summary:{songs_sorted:2,artist_folders_created:1,albums_processed:1,top_artists:[['שם מקורי',4],['אמן שני',2]] as [string,number][]},
   albums:{completed:1,child_names_changed:0,created_directories:7,removed_directories:0,retained_directories:0}};
  const snapshot=JSON.stringify(result);
  const summary=simpleExecutionSummary({status:'partial',result},true);
  expect(summary).toContain('9 עותקים');expect(summary).toContain('2 שירים מוינו');
  expect(summary).toContain('1 תיקיות אמנים חדשות');expect(summary.match(/1 אלבומים הושלמו/g)).toHaveLength(1);
  expect(summary).toContain('שם מקורי (4), אמן שני (2)');expect(summary).toContain('1 לא הושלמו');
  expect(summary).not.toContain('created_directories');expect(JSON.stringify(result)).toBe(snapshot);
 });
 it('shows actual zero counts without inventing artist or album successes',()=>{
  const result={source_items:0,committed_destinations:0,moved_sources:0,failed_operations:0,blocked_operations:0,cancelled_operations:0,recovery_operations:0,items:[],legacy_summary:{songs_sorted:0,artist_folders_created:0,albums_processed:0,top_artists:[]}};
  const summary=simpleExecutionSummary({status:'complete',result},true);
  expect(summary).toContain('0 שירים מוינו');expect(summary).not.toContain('אמנים מובילים');expect(summary).not.toContain('אלבומים הושלמו');
 });
});

describe('live final verification',()=>{
 it('shows current read progress without inventing completion',()=>{
  const progress={completed_operations:2,total_operations:5,bytes_read:67108864,elapsed_ms:1500};
  expect(verificationProgressText(progress)).toBe('בודק את התוצאות: 2 מתוך 5');
  expect(verificationProgressText(progress)).not.toContain('בתים');
  expect(verificationProgressText(progress,true)).toContain('67,108,864 בתים נקראו');
  expect(verificationProgressText({...progress,total_operations:0,completed_operations:0})).toBe('בודק את תוצאות הביצוע…');
 });
});

describe('native resolved platform display markers',()=>{
 it('uses an explicitly empty effective set without changing original developer blockers',()=>{
  const resolved={...row,blockers:['MISSING_IDENTITY'],effective_blockers:[]};
  expect(simpleStatus(resolved,[],true)).toBe('להעתקה');expect(simpleReason(resolved)).toBe('');expect(resolved.blockers).toEqual(['MISSING_IDENTITY']);
 });
 it('falls back to original blockers when no native resolution is supplied',()=>{
  expect(simpleStatus({...row,blockers:['MISSING_IDENTITY']},[],true)).toBe('לא ניתן לבצע');
 });
 it('keeps real errors and unresolved blockers visible',()=>{
  const failed={...row,blockers:['MISSING_IDENTITY','ALBUM_READ_ERROR'],effective_blockers:['ALBUM_READ_ERROR']};
  expect(simpleStatus(failed,[],true)).toBe('לא ניתן לבדוק');expect(simpleReason(failed)).toContain('לא ניתן לקרוא');
  expect(simpleStatus({...failed,effective_blockers:['DESTINATION_OCCUPIED']},[],true)).toBe('לא ניתן לבצע');
 });
});

it('unreadable recovery does not claim that prior source files were preserved',()=>{
 const text=recoveryNoticeText({status:'unreadable',journal_count:1,acknowledged_operations:null,unresolved_operations:null});
 expect(text).toContain('הבדיקה הזו לא שינתה קבצים');expect(text).toContain('המקור והיעד');expect(text).not.toContain('הקבצים נשמרו');
});

describe('developer current-state summary',()=>{
 it('uses actual outcomes without changing original blocker evidence',()=>{
  const blocked={...row,blockers:['PHYSICAL_APPLY_UNAVAILABLE']};
  for(const status of ['הועבר','בוצע חלקית','נדרשת בדיקה','לא בוצע'])expect(developerRowSummary(blocked,status,true)).toBe(status);
  expect(blocked.blockers).toEqual(['PHYSICAL_APPLY_UNAVAILABLE']);
 });
 it('uses native effective readiness and preserves unresolved blockers',()=>{
  expect(developerRowSummary({...row,blockers:['PHYSICAL_APPLY_UNAVAILABLE'],effective_blockers:[]},'להעברה',false)).toBe('להעברה');
  const blocked={...row,blockers:['PHYSICAL_APPLY_UNAVAILABLE'],effective_blockers:['AMBIGUOUS_DISPLAY_NAME']};
  expect(developerRowSummary(blocked,'לא ניתן לבצע',false)).not.toBe('להעברה');
  expect(developerRowSummary({...row,blockers:['PHYSICAL_APPLY_UNAVAILABLE']},'להעברה',false)).not.toBe('להעברה');
 });
});


describe('retained repair validation wording',()=>{
 const readiness:ApplyReadiness={supported:true,reason:null,operation_count:1,mode:'FixNames',copy_mode:false,source:'source',target:null};
 it('does not call retained unchanged validation a pending physical repair',()=>{
  expect(executionReadinessText(readiness,true)).toBe('קובץ אחד בתוכנית לאימות ולתיקון לפי הצורך');
  expect(executionReadinessText(readiness,false)).toBe('מוכן לאימות הקבצים ולביצוע תיקונים לפי הצורך.');
  expect(executionReadinessText(readiness,true)).not.toContain('פעולות תיקון ממתינות');
 });
 it('uses the full retained count without a display-page dependency or mutation',()=>{
  const full=Object.freeze({...readiness,operation_count:101});
  expect(executionReadinessText(full,true)).toBe('101 קבצים בתוכנית לאימות ולתיקון לפי הצורך');
  expect(executionReadinessText({...full,operation_count:0},true)).toBe('0 קבצים בתוכנית לאימות ולתיקון לפי הצורך');
  expect(full.operation_count).toBe(101);
 });
 it('preserves older host and Sort labels without new protocol fields',()=>{
  for(const mode of [undefined,'Sort'] as const){
   expect(executionReadinessText({...readiness,mode},false)).toBe('הקבצים מוכנים. אפשר לבדוק ולאשר את הביצוע.');
   expect(executionReadinessText({...readiness,mode},true)).toBe('1 פעולות העברה ממתינות לאישור');
   expect(executionReadinessText({...readiness,mode,copy_mode:true},true)).toBe('1 פעולות העתקה ממתינות לאישור');
  }
 });
 it('makes unchanged validation and planned-only changes explicit before confirmation',()=>{
  expect(REPAIR_REVIEW_TEXT).toContain('בדיקה חוזרת של הקבצים שבתוכנית');
  expect(REPAIR_REVIEW_TEXT).toContain('תיקונים מתוכננים בלבד');
  expect(REPAIR_REVIEW_TEXT).toContain('קבצים שלא נדרש בהם תיקון יישארו ללא שינוי');
  expect(REPAIR_REVIEW_TEXT).toContain('לפני החלפת תגיות נשמר עותק מקורי');
 });
 it('keeps actual unchanged terminal counters separate from readiness',()=>{
  const result={source_items:1,committed_destinations:0,moved_sources:0,failed_operations:0,blocked_operations:0,recovery_operations:0,cancelled_operations:0,items:[],repair:{total_files:1,processed_files:1,renamed_files:0,repaired_files:0,unchanged_files:1,unsupported_files:0,failed_files:0,retained_backup_bytes:0}};
  expect(simpleExecutionSummary({status:'complete',result},false)).toBe('1 ללא שינוי');
  expect(result.repair).toMatchObject({renamed_files:0,repaired_files:0,unchanged_files:1});
 });
});

describe('running byte extent presentation',()=>{
 it('omits an unknown zero denominator while retaining actual observed bytes',()=>{
  expect(executionBytesText(227,0)).toBe('227 בתים עובדו');
  expect(executionBytesText(0,0)).toBe('0 בתים עובדו');
  expect(executionBytesText(1234567,0)).toBe(`${(1234567).toLocaleString('he-IL')} בתים עובדו`);
 });
 it('keeps known totals and the existing localized format',()=>{
  expect(executionBytesText(0,4096)).toBe(`0 מתוך ${(4096).toLocaleString('he-IL')} בתים`);
  expect(executionBytesText(227,4096)).toBe(`227 מתוך ${(4096).toLocaleString('he-IL')} בתים`);
  expect(executionBytesText(4096,4096)).toBe(`${(4096).toLocaleString('he-IL')} מתוך ${(4096).toLocaleString('he-IL')} בתים`);
 });
 it('never invents a total or clamps observed progress',()=>{
  expect(executionBytesText(227,100)).toBe('227 מתוך 100 בתים');
  expect(executionBytesText(227,0)).not.toContain('מתוך');
 });
});
