import type {PlanRow} from './contracts';
const kinds:Record<string,string>={Album:'בדיקת אלבום',Single:'בדיקת שיר',ItemError:'שגיאה בבדיקה',Vanished:'מקור לא זמין',FixNames:'תיקון שם ותגיות מתוכנן'};
export function kindLabel(kind:string):string{return kinds[kind]??'רשומת תכנון';}
export function reasonLabel(row:PlanRow):string{
 if(row.kind==='FixNames')return 'נבדקו שם הקובץ והתגיות לתיקון. לא בוצע שינוי; פרטי ההצעה זמינים במידע הטכני.';
 if(row.kind==='ItemError')return 'לא ניתן היה להשלים את בדיקת הפריט. פרטי השגיאה זמינים במידע הטכני.';
 if(row.kind==='Vanished')return 'המקור אינו זמין בשלב זה של התכנון.';
 if(row.kind==='Single'&&row.destination)return `האמנים שנבחרו: ${row.reason}`;
 return row.reason;
}
const blockers:Record<string,string>={
 PHYSICAL_APPLY_UNAVAILABLE:'ביצוע שינויים אינו זמין בגרסה זו',
 PhysicalApplyUnavailable:'ביצוע שינויים אינו זמין בגרסה זו',
 'Observation(PhysicalApplyUnavailable)':'ביצוע שינויים אינו זמין בגרסה זו',
 RECOGNITION_UNAVAILABLE:'מודל הזיהוי הדרוש אינו זמין',
 UNRECOGNIZED_ARTIST:'לא זוהה אמן',
 UnsafeIntendedName:'שם היעד המוצע אינו בטוח',EmptyIntendedName:'שם היעד המוצע ריק',
 RenameMustSucceedAndRebindOriginal:'כתיבת תגיות תלויה בהצלחת שינוי השם ובאימות אותו קובץ',
 CaseCollisionRequiresProviderResolution:'קיימת התנגשות שמות המחייבת בדיקה במערכת הקבצים',
};
export function blockerLabel(code:string):string{return blockers[code]??'הפעולה חסומה; הסיבה הטכנית מופיעה בפירוט';}
