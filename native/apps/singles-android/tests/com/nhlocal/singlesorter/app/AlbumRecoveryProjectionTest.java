package com.nhlocal.singlesorter.app;

public final class AlbumRecoveryProjectionTest {
    private static int checks;private AlbumRecoveryProjectionTest(){}
    private static void check(boolean condition){if(!condition)throw new AssertionError();checks++;}
    public static void main(String[] args){
        check(AlbumRecoveryProjection.representsSubtree("move","stage-move"));check(AlbumRecoveryProjection.representsSubtree("move","publish-move"));check(AlbumRecoveryProjection.representsSubtree("create","copy-stage"));check(AlbumRecoveryProjection.representsSubtree("rename","publish-copy"));
        check(!AlbumRecoveryProjection.representsSubtree("create","move-stage"));check(!AlbumRecoveryProjection.representsSubtree("remove-empty","source-album"));
        check(AlbumRecoveryProjection.associationRole("publish-copy").equals("destination"));check(AlbumRecoveryProjection.associationRole("publish-move").equals("destination"));check(AlbumRecoveryProjection.associationRole("copy-member-publish").equals("stage"));
        check(AlbumRecoveryProjection.description("names_currently_differ",2,3,5).contains("2 שמות חסרים"));check(AlbumRecoveryProjection.description("names_currently_match",0,3,3).contains("אין בכך אישור"));check(AlbumRecoveryProjection.description("unavailable",0,0,0).contains("אין אישור"));
        System.out.println(checks+" Album recovery role and visible uncertainty checks passed");
    }
}
