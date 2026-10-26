import {blockerLabel,reasonLabel} from './presentation';
import {userErrorLabels} from './state';
import type {PlanRow,ExecutionItem,ExecutionSnapshot,Platform,ApplyReadiness} from './contracts';
import {sourceLabel} from './presentation';
/** Readiness counts retained validation work, never completed or guaranteed
 * physical changes. Fix plans also retain unchanged files for fresh validation. */
export function executionReadinessText(readiness:ApplyReadiness,developer:boolean):string{
 if(readiness.mode==='FixNames')return developer?`${readiness.operation_count===1?'קובץ אחד':readiness.operation_count.toLocaleString('he-IL')+' קבצים'} בתוכנית לאימות ולתיקון לפי הצורך`:'מוכן לאימות הקבצים ולביצוע תיקונים לפי הצורך.';
 return developer?`${readiness.operation_count.toLocaleString('he-IL')} פעולות ${readiness.copy_mode?'העתקה':'העברה'} ממתינות לאישור`:'הקבצים מוכנים. אפשר לבדוק ולאשר את הביצוע.';
}
/** Refusal presentation uses exact host plan codes, never guesses from paths or
 * free-form error substrings. This is display only and cannot authorize Apply. */
const executionBlockerMessages:Readonly<Record<string,string>>={
 METADATA_ADAPTER_LIMITATION:'קריאת פרטי הקבצים לא הושלמה, ולכן התוכנית נחסמה. פתח את פרטי החסימה לבירור.',
 SOURCE_TARGET_OVERLAP:'תיקיות המקור והיעד חופפות. חזור להגדרות ובחר תיקיות נפרדות שאינן זו בתוך זו.',
 LINK_OR_REPARSE:'בתיקיות שנבדקו נמצא קישור לקובץ או לתיקייה. ביצוע דרך קישורים אינו נתמך; בדוק את מיקום הקבצים.',
 MISSING_IDENTITY:'לא ניתן לאמת את זהות הקבצים או התיקיות. בדוק שהם זמינים ובחר את התיקיות מחדש.',
 UNKNOWN_CAPABILITY:'לא ניתן לאמת את הגישה הנדרשת לקבצים. בדוק את זמינות התיקיות והרשאות הגישה.',
 AMBIGUOUS_DISPLAY_NAME:'נמצאו שמות שלא ניתן להבחין ביניהם בבטחה. יש לבדוק את הקבצים והתיקיות לפני ניסיון נוסף.',
 CASE_COLLISION_REQUIRES_PROVIDER_RESOLUTION:'נמצאו שמות שמתנגשים זה בזה. יש לבדוק את שמות הקבצים והתיקיות לפני ניסיון נוסף.',
 HARD_LINK_IDENTITY:'נמצאו כמה שמות שמפנים לאותו קובץ. ביצוע בתוכנית זו אינו נתמך; בדוק את הקבצים לפני ניסיון נוסף.',
 RECOGNITION_UNAVAILABLE:'מודל הזיהוי הדרוש אינו זמין. יש לבדוק את הגדרות הזיהוי לפני ניסיון נוסף.',
};
export function executionRefusalSummary(readiness:ApplyReadiness|null|undefined,blockers:readonly string[]):string{
 if(!readiness||readiness.supported)return '';
 if(readiness.no_operations===true)return readiness.reason??'לא נמצאו פעולות לביצוע';
 const messages=[...new Set(blockers)].filter(code=>Object.hasOwn(executionBlockerMessages,code)).map(code=>executionBlockerMessages[code]);
 const diagnostic=readiness.metadata_diagnostics;
 const counts=diagnostic?` נכשלו ${diagnostic.failed_observations.toLocaleString('he-IL')} בדיקות פרטים של ${diagnostic.affected_sources.toLocaleString('he-IL')} קבצים.`:'';
 return `${messages[0]||'התוכנית לא אושרה לביצוע. פתח את פרטי החסימה לבירור הסיבה.'}${messages.length>1?' יש חסימות נוספות בפרטים.':''}${counts} לא בוצעו שינויים בקבצים.`;
}
/** The exact native explanation is opt-in diagnostic text. Do not append source
 * or target paths, infer per-file counts, or replace the native reason. */
export function executionRefusalDetails(readiness:ApplyReadiness|null|undefined,blockers:readonly string[]):string{
 if(!readiness||readiness.supported)return '';
 if(readiness.no_operations===true)return '';
 const lines=[readiness.reason||'לא נמסרה סיבה מפורטת מהמארח.',`פעולות מתוכננות בתוכנית המלאה: ${readiness.operation_count.toLocaleString('he-IL')}`,`חסימות שדווחו: ${blockers.join(' · ')||'לא דווחו חסימות נוספות.'}`];
 const diagnostic=readiness.metadata_diagnostics;
 if(diagnostic){
  lines.push(`כשלים בקריאת פרטי קבצים: ${diagnostic.failed_observations}; קבצים שונים: ${diagnostic.affected_sources}. אותו קובץ עשוי להיבדק יותר מפעם אחת.`);
  for(const issue of diagnostic.issues)lines.push(`תצפית ${issue.observation_index}; לאחר ${issue.after_operations} פעולות מתוכננות: \u2068${issue.relative_source}\u2069${issue.label_truncated?'… [השם קוצר]':''} — ${issue.error_kind}${issue.storage_kind?' / '+issue.storage_kind:''}`);
  if(diagnostic.truncated)lines.push(`מוצגות ${diagnostic.issues.length} התצפיות הראשונות בלבד מתוך ${diagnostic.failed_observations}.`);
 }
 return lines.join('\n');
}
/** A zero total is an unknown byte extent, not an estimated zero-byte job. */
export function executionBytesText(written:number,total:number):string{
 const observed=written.toLocaleString('he-IL');
 return total>0?`${observed} מתוך ${total.toLocaleString('he-IL')} בתים`:`${observed} בתים עובדו`;
}
export const REPAIR_REVIEW_TEXT='בדיקה חוזרת של הקבצים שבתוכנית וביצוע תיקונים מתוכננים בלבד. קבצים שלא נדרש בהם תיקון יישארו ללא שינוי. לפני החלפת תגיות נשמר עותק מקורי לצד הקובץ.';
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
  if(items.some(item=>item.duet_retirement_status==='needs_recovery'))return 'העותקים אומתו; המקור דורש בדיקה';
  if(items.some(item=>item.duet_retirement_status==='pending'))return 'הועתק; המקור נשמר';
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
 else if(result.duets){
  parts.push(`${result.committed_destinations.toLocaleString('he-IL')} יעדים אומתו`);
  if(result.duets.retired_sources)parts.push(`${result.duets.retired_sources.toLocaleString('he-IL')} מקורות דואט נמחקו לאחר אימות כל העותקים הנדרשים`);
  const otherMoved=result.moved_sources-result.duets.retired_sources;
  if(otherMoved>0)parts.push(`${otherMoved.toLocaleString('he-IL')} קבצים נוספים הועברו`);
  if(result.duets.no_action_sources)parts.push(`${result.duets.no_action_sources.toLocaleString('he-IL')} מקורות ללא יעד מתאים נשמרו ללא פעולה`);
  if(result.duets.unfinished_retirements)parts.push(`${result.duets.unfinished_retirements.toLocaleString('he-IL')} מחיקות מקור דואט לא הושלמו; העותקים שכבר אומתו נשמרו`);
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

// The list summarizes current execution/readiness; the detail panel retains original plan evidence.
export function developerRowSummary(row:PlanRow,currentStatus:string,hasExecutionEvidence:boolean):string{
 if(hasExecutionEvidence)return currentStatus;
 const blockers=row.effective_blockers??row.blockers;
 if(blockers.length)return blockers.map(blockerLabel).join(' · ');
 return row.effective_blockers!==undefined?currentStatus:reasonLabel(row);
}
