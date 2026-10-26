package com.nhlocal.singlesorter.app;
/** A metadata-only limitation can preserve the original sequence after its clean acknowledgment. */
final class FixAcknowledgedOutcome {
    private FixAcknowledgedOutcome(){}
    static boolean mayContinue(String status,boolean verified,boolean pending){return verified&&!pending&&("unsupported_tags".equals(status)||"metadata_failure".equals(status));}
    static boolean needsAttention(boolean fix,String status,int operations,int acknowledged,int eventCount){
        if("completed".equals(status)||eventCount==0&&("failed".equals(status)||"cancelled".equals(status)))return false;
        return !(fix&&"partial".equals(status)&&operations==acknowledged);
    }
}
