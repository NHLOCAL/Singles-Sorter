import {userErrorLabels} from './state';
import type {PlanRow,ExecutionItem,ExecutionSnapshot} from './contracts';
export const DEVELOPER_PREFERENCE='singles-sorter.developer-mode.v1';
export function readDeveloperMode(storage:Pick<Storage,'getItem'>):boolean{try{return storage.getItem(DEVELOPER_PREFERENCE)==='true';}catch{return false;}}
export function saveDeveloperMode(storage:Pick<Storage,'setItem'>,enabled:boolean){try{storage.setItem(DEVELOPER_PREFERENCE,String(enabled));}catch{/* The current view still works when local storage is unavailable. */}}
export function simpleStatus(row:PlanRow,items:ExecutionItem[],copy:boolean):string{
 if(items.length){
  const completed=items.some(item=>item.status==='complete'||item.status==='source_retained'||item.repair?.renamed||item.repair?.repaired);
  if(items.some(item=>item.status==='needs_recovery'))return 'נדרשת בדיקה';
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
 if(row.kind==='ItemError'||row.blockers.includes('ALBUM_READ_ERROR'))return 'לא ניתן לבדוק';
 if(row.kind==='Vanished')return 'הקובץ לא נמצא';
 if(row.blockers.length)return 'לא ניתן לבצע';
 if(row.kind==='FixNames')return 'לתיקון';
 if(!row.operation_ids?.length)return 'דולג';
 return copy?'להעתקה':'להעברה';
}
export function simpleRepairReason(item:ExecutionItem):string{
 if(item.status==='unsupported_tags')return item.repair?.renamed?'שם הקובץ תוקן. תיקון התגיות בקובץ הזה עדיין אינו נתמך.':'תיקון התגיות בקובץ הזה עדיין אינו נתמך.';
 if(item.status==='metadata_failure')return item.repair?.renamed?'שם הקובץ תוקן, אך לא ניתן היה לקרוא את התגיות.':'לא ניתן היה לקרוא את התגיות לתיקון.';
 return '';
}
export function simpleReason(row:PlanRow):string{
 if(row.kind==='ItemError'||row.blockers.includes('ALBUM_READ_ERROR'))return 'לא ניתן לקרוא את הפריט. בדוק שהוא זמין ושהתוכנה מורשית לקרוא אותו.';
 if(row.kind==='Vanished')return 'הקובץ הועבר או אינו זמין כעת.';
 if(row.blockers.length)return 'הפעולה אינה זמינה לקובץ הזה בתוכנית הנוכחית.';
 if(row.kind==='Single'&&!row.artist)return 'לא זוהה אמן.';
 if(row.kind==='Single'&&!row.operation_ids?.length)return 'לא נמצאה פעולת מיון מתאימה לפי האפשרויות שבחרת.';
 return '';
}
export function simpleExecutionSummary(execution:Pick<ExecutionSnapshot,'status'|'result'>,copy:boolean):string{
 const result=execution.result;
 if(!result)return execution.status==='failed'?'לא התקבל אישור להשלמת הפעולה. בדוק את הקבצים לפני ניסיון נוסף.':execution.status==='needs_recovery'?'יש לבדוק את הקבצים לפני המשך העבודה.':'';
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
  if(result.committed_destinations>result.moved_sources)parts.push('חלק מקובצי המקור נשמרו');
 }
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
