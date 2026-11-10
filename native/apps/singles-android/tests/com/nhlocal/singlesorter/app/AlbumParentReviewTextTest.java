package com.nhlocal.singlesorter.app;

public final class AlbumParentReviewTextTest {
    private AlbumParentReviewTextTest(){}
    public static void main(String[] args){
        String absent=AlbumParentRecovery.description("source_absent"),present=AlbumParentRecovery.description("source_present"),reused=AlbumParentRecovery.description("source_name_reused"),changed=AlbumParentRecovery.description("source_identity_changed");
        if(!absent.contains("אינו מוכיח")||!present.contains("אינו מוכיח")||!reused.contains("אין למחוק")||!changed.contains("אין שיוך ודאי"))throw new AssertionError();
        for(String code:new String[]{"ALBUM_PARENT_DIRECT_CHILD","ALBUM_PARENT_CHANGED","ALBUM_PARENT_FRESH_SELECTION_REQUIRED","ALBUM_PARENT_REVIEW_BINDING"})if(!ReadinessDiagnostic.code(new IllegalArgumentException(code)).equals(code))throw new AssertionError();
        System.out.println("8 explicit parent inspection uncertainty and diagnostic checks passed");
    }
}
