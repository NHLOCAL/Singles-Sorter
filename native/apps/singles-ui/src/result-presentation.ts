import {userErrorLabels} from './state';
import type {PlanRow,ExecutionItem,ExecutionSnapshot,Platform} from './contracts';
import {sourceLabel} from './presentation';
export const DEVELOPER_PREFERENCE='singles-sorter.developer-mode.v1';
export function readDeveloperMode(storage:Pick<Storage,'getItem'>):boolean{try{return storage.getItem(DEVELOPER_PREFERENCE)==='true';}catch{return false;}}
export function saveDeveloperMode(storage:Pick<Storage,'setItem'>,enabled:boolean){try{storage.setItem(DEVELOPER_PREFERENCE,String(enabled));}catch{/* The current view still works when local storage is unavailable. */}}
/** A planned destination never proves a rename. Keep older hosts and uncertain
 * per-file outcomes on their retained source label without changing status. */
export function resultFilename(row:PlanRow,items:ExecutionItem[],platform:Platform|undefined):string{
 const original=sourceLabel(row.source,platform);
 if(row.kind!=='FixNames'||row.operation_ids?.length!==1)return original;
 const matching=items.filter(item=>row.operation_ids?.includes(item.operation));
 if(matching.length!==1)return original;
 const item=matching[0],repair=item.repair,name=repair?.observed_filename;
 if(item.status!=='complete'&&!(repair?.renamed&&['unsupported_tags','metadata_failure'].includes(item.status)))return original;
 if(!name||name==='.'||name==='..'||/[\\/\x00-\x1f\x7f-\x9f]/.test(name)||/\.(backup|tmp)$/i.test(name))return original;
 return name;
}
export function simpleStatus(row:PlanRow,items:ExecutionItem[],copy:boolean,context:{repairEligible?:boolean;terminalResult?:boolean;uncertain?:boolean}={}):string{
 if(items.length){
  const completed=items.some(item=>item.status==='complete'||item.status==='source_retained'||item.repair?.renamed||item.repair?.repaired);
  if(items.some(item=>item.status==='needs_recovery'))return 'נדרשת בדיקה';
  if(items.some(item=>item.status==='partial'))return 'בוצע חלקית';
  if(items.some(item=>item.status==='unsupported_tags'||item.status==='metadata_failure'))return completed?'בוצע חלקית':'לא הושלם';
  if(items.some(item=>item.status==='failed'))return completed?'בוצע חלקית':'לא הושלם';
  if(items.some(item=>item.status==='cancelled'))return completed?'בוצע חלקית':'נעצר';
  if(items.some(item=>item.status==='blocked'))return completed?'בוצע חלקית':'לא בוצע';
  if(items.some(item=>item.status==='source_retained'))return 'הועתק; המקור נשמר';
  if(row.kind==='FixNames'){
   if(items.some(item=>item.repair?.renamed||item.repair?.repaired))return 'תוקן';
   if(items.every(item=>item.repair?.unchanged))return 'ללא שינוי';
   return 'תוקן';
  }
  return copy?'הועתק':'הועבר';
 }
 if(context.uncertain&&row.operation_ids?.length)return 'לא אומת';
 if(context.terminalResult&&row.operation_ids?.length)return 'לא בוצע';
 if(row.included_in_album!==undefined)return row.no_action?'כלול באלבום':'נדרשת בדיקה';
 if(row.no_action)return row.kind==='FixNames'?'ללא שינוי':'ללא פעולה';
 if(row.blockers.includes('FIX_METADATA_READ_ERROR'))return context.repairEligible?'תיקון חלקי':'לא ניתן לבדוק';
 if(row.kind==='FixNames'&&context.repairEligible)return 'לתיקון';
 if(row.kind==='ItemError'||(row.effective_blockers??row.blockers).includes('ALBUM_READ_ERROR'))return 'לא ניתן לבדוק';
 if(row.kind==='Vanished')return 'הקובץ לא נמצא';
 if((row.effective_blockers??row.blockers).length)return 'לא ניתן לבצע';
 if(row.kind==='FixNames')return 'לתיקון';
 if(!row.operation_ids?.length)return 'דולג';
 return copy?'להעתקה':'להעברה';
}
export function simpleRepairReason(item:ExecutionItem):string{
 if(item.status==='unsupported_tags')return item.repair?.renamed?'שם הקובץ תוקן. תיקון התגיות בקובץ הזה עדיין אינו נתמך.':'תיקון התגיות בקובץ הזה עדיין אינו נתמך.';
 if(item.status==='metadata_failure')return item.repair?.renamed?'שם הקובץ תוקן, אך לא ניתן היה לקרוא את התגיות.':'לא ניתן היה לקרוא את התגיות לתיקון.';
 return '';
}
export function simpleReason(row:PlanRow,repairEligible=false):string{
 if(row.blockers.includes('FIX_METADATA_READ_ERROR'))return 'לא ניתן לקרוא את התגיות. ייתכן שרק שם הקובץ יתוקן.';
 if(row.kind==='FixNames'&&row.no_action)return 'לא נמצא שינוי נדרש בשם או בתגיות.';
 if(row.kind==='FixNames'&&repairEligible)return '';
 if(row.kind==='ItemError'||(row.effective_blockers??row.blockers).includes('ALBUM_READ_ERROR'))return 'לא ניתן לקרוא את הפריט. בדוק שהוא זמין ושהתוכנה מורשית לקרוא אותו.';
 if(row.included_in_album!==undefined)return 'הפריט כלול בפעולת האלבום, ללא פעולה נפרדת.';
 if(row.kind==='Vanished')return 'הקובץ הועבר או אינו זמין כעת.';
 if((row.effective_blockers??row.blockers).length)return 'הפעולה אינה זמינה לקובץ הזה בתוכנית הנוכחית.';
 if(row.kind==='Single'&&!row.artist)return 'לא זוהה אמן.';
 if(row.kind==='Single'&&!row.operation_ids?.length)return 'לא נמצאה פעולת מיון מתאימה לפי האפשרויות שבחרת.';
 return '';
}
export function simpleExecutionSummary(execution:Pick<ExecutionSnapshot,'status'|'result'>,copy:boolean):string{
 const result=execution.result;
 if(!result)return execution.status==='failed'?'לא התקבל אישור להשלמת הפעולה. בדוק את הקבצים לפני ניסיון נוסף.':execution.status==='needs_recovery'?'יש לבדוק את הקבצים לפני המשך העבודה.':'';
 if(execution.status==='complete'&&result.no_op===true&&result.committed_destinations===0&&result.moved_sources===0&&result.items.length===0&&!result.repair&&!result.albums&&!result.failed_operations&&!result.blocked_operations&&!result.cancelled_operations&&!result.recovery_operations)return 'לא בוצעו שינויים. קובצי המקור נשמרו.';
 const parts:string[]=[];
 if(result.repair){
  const repair=result.repair;
  if(repair.renamed_files)parts.push(`${repair.renamed_files.toLocaleString('he-IL')} שמות תוקנו`);
  if(repair.repaired_files)parts.push(`${repair.repaired_files.toLocaleString('he-IL')} קבצים עם תגיות מתוקנות`);
  if(!repair.renamed_files&&!repair.repaired_files&&repair.unchanged_files)parts.push(`${repair.unchanged_files.toLocaleString('he-IL')} ללא שינוי`);
  if(repair.unsupported_files)parts.push(`${repair.unsupported_files.toLocaleString('he-IL')} עם תגיות שלא תוקנו`);
 }
 else if(copy)parts.push(`נוצרו ${result.committed_destinations.toLocaleString('he-IL')} עותקים`);
 else{
  parts.push(`${result.moved_sources.toLocaleString('he-IL')} קבצים הועברו`);
  if(result.items.some(item=>item.status==='source_retained'))parts.push('חלק מקובצי המקור נשמרו');
 }
 if(result.legacy_summary){
  const summary=result.legacy_summary;
  parts.push(`${summary.songs_sorted.toLocaleString('he-IL')} שירים מוינו`);
  if(summary.albums_processed)parts.push(`${summary.albums_processed.toLocaleString('he-IL')} אלבומים הושלמו`);
  if(summary.artist_folders_created)parts.push(`${summary.artist_folders_created.toLocaleString('he-IL')} תיקיות אמנים חדשות`);
  if(summary.top_artists.length)parts.push(`אמנים מובילים: ${summary.top_artists.map(([artist,count])=>`${artist} (${count.toLocaleString('he-IL')})`).join(', ')}`);
 }
 else if(result.albums?.completed)parts.push(`${result.albums.completed.toLocaleString('he-IL')} אלבומים הושלמו`);
 if(result.albums?.child_names_changed)parts.push(`${result.albums.child_names_changed.toLocaleString('he-IL')} שמות באלבומים שונו`);
 if(result.albums?.retained_directories)parts.push('נותרו תיקיות במקור');
 if(result.failed_operations)parts.push(`${result.failed_operations.toLocaleString('he-IL')} לא הושלמו`);
 if(result.blocked_operations)parts.push(`${result.blocked_operations.toLocaleString('he-IL')} לא בוצעו`);
 if(result.cancelled_operations)parts.push(`${result.cancelled_operations.toLocaleString('he-IL')} נעצרו`);
 if(result.recovery_operations)parts.push('יש פריטים שדורשים בדיקה');
 return parts.join(' · ');
}
const userFacingMessages=new Set([
 ...Object.values(userErrorLabels),
 'בחר תיקיית מקור.',
 'בחר תיקיית יעד למיון.',
 'הגדרות התוכנה אינן תקינות.',
 'תוצאת הביצוע הקודמת טרם אומתה.',
 'שמירת ההגדרות לא אומתה.',
 'שמירת מצב הדואטים לא אומתה.',
 'התקבלה תשובה שאינה שייכת לבדיקה הנוכחית.',
 'הבדיקה אינה זמינה עוד במארח. אפשר להפעיל בדיקה חדשה.',
 'התצוגה המקדימה נכשלה.',
 'התקבלה תשובה שאינה שייכת לפעולה הנוכחית.',
 'מצב הפעולה לא אומת. ממתין לתשובה מהמארח.',
 'הפעולה לא הושלמה. נסה שוב לאחר בדיקת החיבור לתוכנה.',
]);
export function simpleError(message:string):string{
 if(message==='מצב הביצוע אינו זמין. יש לבדוק את יומן הפעולה לפני בדיקה או ביצוע נוספים.')return 'לא התקבל מצב הביצוע. בדוק את הקבצים במקור וביעד לפני המשך העבודה.';
 if(message==='RECOGNITION_UNAVAILABLE')return 'מודל הזיהוי הדרוש אינו זמין. הבדיקה לא הופעלה.';
 const human=message.replace(/\s*\([A-Z_]+\)$/,'');
 if(userFacingMessages.has(human))return human;
 return 'הפעולה לא הושלמה. בדוק שהתיקיות זמינות ושהקבצים לא השתנו.';
}

/** Reconciliation uncertainty never rewrites observed mutation counters. */
export function verificationWarning(execution:Pick<ExecutionSnapshot,'recovery_error'>|null,developer=false):string{
 if(!execution?.recovery_error)return '';
 return developer?`בדיקת היומן לא הושלמה: ${execution.recovery_error}. תוצאות הביצוע נשמרו; יש לבדוק את הקבצים והגיבויים לפני ניסיון נוסף.`:'הבדיקה הסופית לא הושלמה. בדוק את הקבצים והגיבויים לפני ניסיון נוסף.';
}

/** Live read-only verification progress never stands in for completed effects. */
export function verificationProgressText(progress:{completed_operations:number;total_operations:number;bytes_read:number;elapsed_ms:number},developer=false):string{
 const base=progress.total_operations>0?`בודק את התוצאות: ${progress.completed_operations.toLocaleString('he-IL')} מתוך ${progress.total_operations.toLocaleString('he-IL')}`:'בודק את תוצאות הביצוע…';
 return developer?`${base} · ${progress.bytes_read.toLocaleString('he-IL')} בתים נקראו · ${(progress.elapsed_ms/1000).toLocaleString('he-IL',{maximumFractionDigits:1})} שניות`:base;
}

export function recoveryNoticeText(notice:import('./contracts').RecoveryNotice|null):string{
 if(!notice)return '';
 if(notice.status==='unreadable')return 'לא ניתן לבדוק את רישום הביצוע הקודם. הבדיקה הזו לא שינתה קבצים. יש לבדוק את המקור והיעד ולשמור את היומן; הביצוע חסום עד לבירור.';
 return `הביצוע הקודם דורש בדיקה. ${notice.acknowledged_operations} פעולות נרשמו כהושלמו, ${notice.unresolved_operations} דורשות בירור. אפשר לבדוק תיקיות ולשמור יומן לבדיקה; לא יתבצע ניסיון חוזר אוטומטי.`;
}
