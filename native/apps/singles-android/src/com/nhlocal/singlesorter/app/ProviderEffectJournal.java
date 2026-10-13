package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.util.AtomicFile;
import java.io.File;
import java.io.FileOutputStream;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** One worker owns this journal. Interrupted intents are uncertain, never automatic retry authority. */
final class ProviderEffectJournal {
    final String id;
    private final File stateFile,indexFile;
    private boolean pendingEffects,anyEffects;
    private final JSONObject state;
    private final JSONArray events=new JSONArray();
    private final CheckedJournalCommit commit=new CheckedJournalCommit();
    ProviderEffectJournal(Context context,RetainedPlan retained)throws Exception{
        id=UUID.randomUUID().toString();File root=new File(context.getFilesDir(),"provider-executions");
        if(!root.isDirectory()&&!root.mkdir())throw new IOException("JOURNAL_DIRECTORY");
        File directory=new File(root,id);if(!directory.mkdir())throw new IOException("JOURNAL_OWNER");
        JSONObject header=new JSONObject().put("schema",1).put("execution_id",id).put("job_id",retained.jobId).put("plan_id",retained.planId)
            .put("original_plan",retained.planCopy()).put("source_bindings",retained.bindingsCopy()).put("references",retained.referencesCopy())
            .put("provider_quiescence_required",true).put("provider_atomic_batch",false);
        if("FixNames".equals(retained.planCopy().getJSONObject("request").getString("mode")))header.put("fix_protocol",1).put("fix_backup_leaf",FixProtocol.BACKUP_LEAF);
        if(!"FixNames".equals(retained.planCopy().getJSONObject("request").getString("mode"))&&retained.albumProjectionCopy().getJSONArray("operations").length()>0)header.put("album_protocol",1).put("album_projection",retained.albumProjectionCopy());
        if(retained.albumRootParentCopy()!=null)header.put("album_root_parent",retained.albumRootParentCopy());
        if(!"FixNames".equals(retained.planCopy().getJSONObject("request").getString("mode"))&&!NativeDuetPlan.inspectOriginal(retained.planCopy(),()->false).groups.isEmpty())header.put("duet_protocol",1);
        String authorityHash=write(new File(directory,"authority.json"),header,16*1024*1024);
        stateFile=new File(directory,"state.json");indexFile=new File(directory,"index.json");state=new JSONObject().put("schema",1).put("execution_id",id).put("job_id",retained.jobId).put("plan_id",retained.planId).put("status","running").put("authority_sha256",authorityHash).put("events",events);
        persist();writeIndex();
    }
    void record(String phase,String operation,JSONObject detail)throws Exception{
        if(events.length()>=10000||detail.toString().getBytes(StandardCharsets.UTF_8).length>65536)throw new IOException("JOURNAL_BUDGET");
        if(phase.equals("intent")){pendingEffects=true;anyEffects=true;}
        events.put(new JSONObject().put("sequence",events.length()).put("phase",phase).put("operation",operation).put("detail",detail));persist();
        if(phase.equals("verified")||phase.equals("album-verified")||phase.equals("duet-verified"))pendingEffects=false;
    }
    void finish(String status)throws Exception{
        if(!status.equals("completed")&&!status.equals("cancelled")&&!status.equals("partial")&&!status.equals("uncertain")&&!status.equals("failed"))throw new IllegalArgumentException("status");
        state.put("status",status);persist();writeIndex();
    }
    boolean hasPendingEffects(){return pendingEffects;}
    boolean hasAnyEffects(){return anyEffects;}
    private void writeIndex()throws Exception{
        JSONObject index=new JSONObject().put("schema",1).put("execution_id",id).put("job_id",state.getString("job_id")).put("plan_id",state.getString("plan_id")).put("status",state.getString("status")).put("authority_sha256",state.getString("authority_sha256"));
        commit.run(()->write(indexFile,index,4096));
    }
    private void persist()throws Exception{commit.run(()->write(stateFile,state,8*1024*1024));}
    private static String write(File path,JSONObject value,int limit)throws Exception{
        byte[] bytes=value.toString().getBytes(StandardCharsets.UTF_8);if(bytes.length>limit)throw new IOException("JOURNAL_BYTES");
        AtomicFile file=new AtomicFile(path);FileOutputStream out=file.startWrite();
        try{out.write(bytes);out.getFD().sync();file.finishWrite(out);}catch(Exception error){file.failWrite(out);throw error;}
        // Android AtomicFile can log a failed rename instead of throwing. Verify the
        // committed base file directly, never let a logged failure authorize an effect.
        try(FileInputStream input=new FileInputStream(path)){
            CheckedJournalCommit.verify(input,bytes);
        }
        StringBuilder hash=new StringBuilder();for(byte b:java.security.MessageDigest.getInstance("SHA-256").digest(bytes))hash.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return hash.toString();
    }
}
