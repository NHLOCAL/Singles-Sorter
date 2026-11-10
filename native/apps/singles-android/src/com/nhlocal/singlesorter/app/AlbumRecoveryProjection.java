package com.nhlocal.singlesorter.app;

/** Album receipt roles are explicit; a staging container is not the captured child subtree. */
final class AlbumRecoveryProjection {
    private AlbumRecoveryProjection(){}
    static boolean representsSubtree(String action,String role){return !"remove-empty".equals(action)&&!("create".equals(action)&&"move-stage".equals(role));}
    static String associationRole(String role){return "publish-copy".equals(role)||"publish-move".equals(role)?"destination":"stage";}
    static String description(String status,int missing,int current,int expected){
        if("names_currently_match".equals(status))return "שמות הפריטים בתיקייה תואמים כעת לרישום; אין בכך אישור להשלמת הפעולה הקודמת.";
        if("names_currently_differ".equals(status))return "תכולת התיקייה שונה מהרישום: "+missing+" שמות חסרים; נמצאו "+current+" פריטים מתוך "+expected+" צפויים. יש לבדוק את התיקייה לפני המשך העבודה.";
        return "לא ניתן לבדוק כעת את מלוא תכולת התיקייה. חלק מהקבצים עשויים להימצא במיקום אחר; אין אישור להשלמת הפעולה הקודמת.";
    }
}
