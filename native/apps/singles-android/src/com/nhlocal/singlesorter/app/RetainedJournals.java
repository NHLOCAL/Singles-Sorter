package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.system.Os;
import android.system.OsConstants;
import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Shared bounded private-byte validation. Callers separately own execution/diagnostic exclusion. */
final class RetainedJournals {
    private RetainedJournals(){}
    interface Visitor {void accept(Record record)throws Exception;}
    static final class Record {
        final String id,jobId,planId,authorityHash,stateHash,indexHash,status;
        final int acknowledged,unresolved;final boolean attention;
        private final String authority,state;
        Record(String id,JSONObject authority,JSONObject state,byte[] authorityBytes,byte[] stateBytes,byte[] indexBytes,RecoveryAcknowledgments ledger)throws Exception{
            this.id=id;this.authority=authority.toString();this.state=state.toString();jobId=authority.getString("job_id");planId=authority.getString("plan_id");status=state.getString("status");
            authorityHash=hex(MessageDigest.getInstance("SHA-256").digest(authorityBytes));stateHash=hex(MessageDigest.getInstance("SHA-256").digest(stateBytes));indexHash=hex(MessageDigest.getInstance("SHA-256").digest(indexBytes));
            acknowledged=ledger.acknowledged();unresolved=ledger.unresolved();attention=!status.equals("completed")&&!(state.getJSONArray("events").length()==0&&(status.equals("failed")||status.equals("cancelled")));
        }
        JSONObject authority()throws org.json.JSONException{return new JSONObject(authority);}
        JSONObject state()throws org.json.JSONException{return new JSONObject(state);}
    }
    static int scan(Context context,BooleanSupplier cancelled,Visitor visitor)throws Exception{
        int count=0;long[] budget={64L*1024*1024};JournalExportBounds.checkpoint(cancelled);
        File root=JournalExportBounds.root(context.getFilesDir());if(!root.exists())return 0;
        try(DirectoryStream<Path> entries=Files.newDirectoryStream(root.toPath())){
            for(Path entry:entries){
                JournalExportBounds.checkpoint(cancelled);if(++count>1000)throw new IOException("recovery count");
                File directory=JournalExportBounds.child(root,entry.getFileName().toString());if(!directory.isDirectory())throw new IOException("recovery directory");
                    byte[] authorityBytes=read(directory,"authority.json",cancelled,budget);byte[] stateBytes=read(directory,"state.json",cancelled,budget),indexBytes=read(directory,"index.json",cancelled,budget);JSONObject authority=json(authorityBytes),state=json(stateBytes),index=json(indexBytes);
                    String hash=hex(MessageDigest.getInstance("SHA-256").digest(authorityBytes));String id=directory.getName();
                    if(authority.getInt("schema")!=1||state.getInt("schema")!=1||index.getInt("schema")!=1||!hash.equals(state.getString("authority_sha256"))||!hash.equals(index.getString("authority_sha256")))throw new IOException("recovery binding");
                    for(String key:new String[]{"execution_id","job_id","plan_id"})if(!authority.getString(key).equals(state.getString(key))||!authority.getString(key).equals(index.getString(key)))throw new IOException("recovery binding");
                    if(!id.equals(authority.getString("execution_id"))||!JournalExportBounds.id(authority.getString("job_id"))||!authority.getString("plan_id").matches("[0-9a-f]{64}"))throw new IOException("recovery identity");
                    JSONObject plan=authority.getJSONObject("original_plan");if(!authority.getString("plan_id").equals(plan.getString("binding_sha256")))throw new IOException("recovery plan");
                    JSONArray operations=plan.getJSONArray("operations"),events=state.getJSONArray("events");RecoveryAcknowledgments ledger=new RecoveryAcknowledgments(operations.length());
                    for(int i=0;i<operations.length();i++)if(operations.getJSONObject(i).getInt("id")!=i)throw new IOException("recovery operation");
                    String[] publications=new String[operations.length()];JSONObject[] destinations=new JSONObject[operations.length()];
                    for(int i=0;i<events.length();i++){
                        JSONObject event=events.getJSONObject(i);String phase=event.getString("phase"),operation=event.getString("operation");
                        ledger.event(event.getInt("sequence"),phase,operation);
                        int slash=operation.indexOf('/'),number=Integer.parseInt(operation.substring(10,slash<0?operation.length():slash));JSONObject detail=event.getJSONObject("detail");
                        if(phase.equals("observed")&&(operation.endsWith("/publish-copy")||operation.endsWith("/publish-move")||operation.endsWith("/move")))publications[number]=detail.getString("uri");
                        if(phase.equals("destination-observed"))destinations[number]=detail;
                        if(phase.equals("verified")){
                            if(!detail.getString("destination").equals(publications[number])||detail.getLong("bytes")<0||detail.getLong("bytes")>64L*1024*1024||!detail.getString("sha256").matches("[0-9a-f]{64}"))throw new IOException("recovery acknowledgment");
                            if(destinations[number]!=null)for(String key:new String[]{"destination","name","bytes","sha256","copy"})if(!detail.get(key).equals(destinations[number].get(key)))throw new IOException("recovery destination");
                        }
                    }
                String status=state.getString("status");if(!status.matches("completed|failed|cancelled|partial|uncertain|running")||!status.equals(index.getString("status")))throw new IOException("recovery status");
                if(status.equals("completed")&&ledger.unresolved()!=0)throw new IOException("recovery acknowledgment");
                JournalExportBounds.checkpoint(cancelled);visitor.accept(new Record(id,authority,state,authorityBytes,stateBytes,indexBytes,ledger));
            }
        }catch(java.nio.file.DirectoryIteratorException failure){throw new IOException("recovery directory",failure);}
        JournalExportBounds.checkpoint(cancelled);return count;
    }
    private static JSONObject json(byte[] bytes)throws org.json.JSONException{return new JSONObject(new String(bytes,StandardCharsets.UTF_8));}
    private static byte[] read(File directory,String name,BooleanSupplier cancelled,long[] budget)throws Exception{
        int limit=JournalExportBounds.limit(name);java.io.FileDescriptor fd=Os.open(new File(directory,name).getAbsolutePath(),OsConstants.O_RDONLY|OsConstants.O_NOFOLLOW|OsConstants.O_NONBLOCK|OsConstants.O_CLOEXEC,0);Throwable primary=null;
        try(FileInputStream input=new FileInputStream(fd)){
            android.system.StructStat stat=Os.fstat(fd);if(!OsConstants.S_ISREG(stat.st_mode)||stat.st_size<0||stat.st_size>limit)throw new IOException("recovery size");
            if(stat.st_size>budget[0])throw new IOException("recovery budget");
            byte[] bytes=JournalExportBounds.read(input,(int)Math.min(limit,budget[0]),cancelled);budget[0]-=bytes.length;return bytes;
        }catch(Exception|Error failure){primary=failure;throw failure;}
        finally{try{Os.close(fd);}catch(android.system.ErrnoException failure){if(primary!=null)primary.addSuppressed(failure);else throw failure;}}
    }
    private static String hex(byte[] bytes){StringBuilder value=new StringBuilder();for(byte b:bytes)value.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return value.toString();}
}
