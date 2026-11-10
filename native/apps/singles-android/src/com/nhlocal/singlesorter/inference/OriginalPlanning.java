package com.nhlocal.singlesorter.inference;

import java.nio.charset.StandardCharsets;
/** Native planning entry used only by the Android app's owned worker, never its UI thread. */
public final class OriginalPlanning {
    static {System.loadLibrary("singles_android_inference");}
    private OriginalPlanning() {}
    private static native byte[] decodeCsv0(byte[] input,byte[] encoding);
    public static String decodeCsv(byte[] input,String encoding) {
        if(input.length>2*1024*1024||encoding.length()>32)throw new IllegalArgumentException("CSV_LIMIT");
        byte[] result=decodeCsv0(input,encoding.getBytes(StandardCharsets.UTF_8));
        if(result==null)throw new IllegalStateException("JNI_RESULT_MISSING");
        return new String(result,StandardCharsets.UTF_8);
    }
    private static native byte[] stageRepair0(long id,byte[] cache,byte[] request);
    /** Private owned cache only. Caller owns cancellation, partial-output cleanup and provider publication. */
    public static String stageRepair(long id,String cache,String request){
        if(cache==null||request==null||cache.length()>4096||request.length()>1024*1024)throw new IllegalArgumentException("REPAIR_LIMIT");
        byte[] encoded=request.getBytes(StandardCharsets.UTF_8),directory=cache.getBytes(StandardCharsets.UTF_8);
        if(encoded.length>1024*1024||directory.length>4096)throw new IllegalArgumentException("REPAIR_LIMIT");
        byte[] result=stageRepair0(id,directory,encoded);
        if(result==null||result.length>65536)throw new IllegalStateException("REPAIR_RESULT_LIMIT");
        return new String(result,StandardCharsets.UTF_8);
    }
    public interface SnapshotProvider { byte[] snapshot(byte[] token) throws Exception; }
    private static native byte[] plan0(long id,byte[] models,byte[] cache,byte[] input,SnapshotProvider provider);
    public static long begin() {return OriginalInference.begin0();}
    public static void cancel(long id) {OriginalInference.cancel0(id);}
    public static void abandon(long id) {OriginalInference.abandon0(id);}
    public static String run(long id,String models,String cache,String input,SnapshotProvider provider) {
        if(input.length()>2*1024*1024||models.length()>4096||cache.length()>4096)throw new IllegalArgumentException("MANIFEST_LIMIT");
        byte[] bytes=plan0(id,models.getBytes(StandardCharsets.UTF_8),cache.getBytes(StandardCharsets.UTF_8),input.getBytes(StandardCharsets.UTF_8),provider);
        if(bytes==null)throw new IllegalStateException("JNI_RESULT_MISSING");
        if(bytes.length>32*1024*1024)throw new IllegalStateException("JNI_RESULT_LIMIT");
        return new String(bytes,StandardCharsets.UTF_8);
    }
}
