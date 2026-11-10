package com.nhlocal.singlesorter.app;

import java.util.function.BooleanSupplier;
import org.json.JSONObject;

/** Separate explicit retained-root-parent review grammar; old Single/Fix reviews are unchanged. */
final class AlbumParentRecovery {
    static final String POLICY="retain-current-album-parent-effects-new-preview-v1";
    private AlbumParentRecovery(){}
    static JSONObject evidence(JSONObject authority,BooleanSupplier cancelled)throws Exception{
        if(!authority.has("album_root_parent"))return null;if(!authority.has("album_protocol"))throw new IllegalArgumentException("ALBUM_PARENT_REVIEW_BINDING");
        return AlbumRootParent.validate(authority.getJSONObject("album_root_parent"),authority.getJSONObject("original_plan"),cancelled);
    }
    static boolean requireBinding(JSONObject authority,JSONObject review,BooleanSupplier cancelled)throws Exception{
        JSONObject expected=evidence(authority,cancelled);if(expected==null){if(review.has("album_root_parent")||review.has("source_parent_tree")||review.has("source_parent_observation"))throw new IllegalArgumentException("ALBUM_PARENT_REVIEW_BINDING");return false;}
        if(!AlbumJson.same(expected,review.getJSONObject("album_root_parent"),cancelled)||!AlbumRootParent.parentTree(expected).toString().equals(review.getString("source_parent_tree")))throw new IllegalArgumentException("ALBUM_PARENT_REVIEW_BINDING");AlbumRootParent.validateMembership(expected,review.getJSONObject("source_parent_observation"),cancelled);return true;
    }
    static String description(String status){
        if(status.equals("source_absent"))return "רשימת הילדים המלאה של תיקיית האב נבדקה כעת: תיקיית המקור ושמה אינם נמצאים. זה אינו מוכיח שההעברה הקודמת הושלמה.";
        if(status.equals("source_present"))return "תיקיית המקור נמצאת כעת בשם ובהפניה שנרשמו. זה אינו מוכיח את זהות התוכן או את הצלחת הפעולה הקודמת.";
        if(status.equals("source_name_reused"))return "נמצא כעת פריט אחר בשם תיקיית המקור. אין למחוק אותו או לייחס אותו לפעולה הקודמת.";
        return "ההפניה שנרשמה למקור מצביעה כעת על שם או סוג שונים. אין שיוך ודאי לפעולה הקודמת.";
    }
}
