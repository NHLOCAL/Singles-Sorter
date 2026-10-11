package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import com.nhlocal.singlesorter.platform.SafReadStorage;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import org.json.JSONArray;
import org.json.JSONObject;

/** Immutable private metadata snapshots. Never claim provider bytes are unchanged.
 * This owner only deletes names it created in its private per-job directory.
 */
final class SafSnapshot implements AutoCloseable, com.nhlocal.singlesorter.inference.OriginalPlanning.SnapshotProvider {
    private static final long FILE_LIMIT=64L*1024*1024, JOB_LIMIT=512L*1024*1024;
    final File directory;
    final JSONArray entries=new JSONArray();
    private final List<File> owned=new ArrayList<>();
    private final HashSet<String> visited=new HashSet<>();
    private final java.util.Map<String,JSONObject> files=new java.util.HashMap<>();
    private final SafReadStorage storage;
    private final CancellationSignal cancel;
    private long total;
    SafSnapshot(Context context,SafReadStorage storage,CancellationSignal cancel)throws IOException {
        this.storage=storage;this.cancel=cancel;
        directory=File.createTempFile("preview-","",context.getCacheDir());
        if(!directory.delete()||!directory.mkdir())throw new IOException("SNAPSHOT_DIRECTORY");
    }
    void capture(String root,boolean readFiles,boolean recursive)throws Exception {walk(root,null,readFiles,recursive,0);}
    private void walk(String token,String parent,boolean readFiles,boolean recursive,int depth)throws Exception {
        cancel.throwIfCanceled();
        if(depth>128||entries.length()>=2000||!visited.add(token))throw new IOException("TREE_BUDGET_OR_CYCLE");
        SafReadStorage.Entry entry=storage.inspect(token,cancel);
        String[] source=storage.provenance(token);
        JSONObject data=new JSONObject().put("token",token).put("parent",parent==null?JSONObject.NULL:parent)
            .put("tree_uri",source[0]).put("authority",source[1]).put("document_id",source[2])
            .put("name",entry.displayName).put("directory",entry.isDirectory())
            .put("observed_size",entry.size==null?JSONObject.NULL:entry.size)
            .put("observed_modified",entry.lastModified==null?JSONObject.NULL:entry.lastModified.toString());
        for(String field:new String[]{token,entry.displayName,source[0],source[1],source[2]})if(field==null||field.length()>4096)throw new IOException("ENTRY_LIMIT");
        entries.put(data);
        if(entry.isDirectory()) {
            if(depth==0||recursive)for(SafReadStorage.Entry child:storage.children(token,cancel))walk(child.token,token,readFiles,recursive,depth+1);
        } else if(readFiles) { files.put(token,new JSONObject().put("observed_name",entry.displayName)); }
    }
    @Override public byte[] snapshot(byte[] tokenBytes) throws Exception {
        if(tokenBytes==null||tokenBytes.length>4096)throw new IOException("TOKEN_LIMIT");
        String token=new String(tokenBytes,java.nio.charset.StandardCharsets.UTF_8);
        JSONObject data=files.get(token);if(data==null)throw new SecurityException("SNAPSHOT_NOT_AUTHORIZED");
        cancel.throwIfCanceled();
        if(data.isNull("snapshot_file")) {
            SafReadStorage.Entry entry=storage.inspect(token,cancel);
            if(entry.isDirectory()||!entry.displayName.equals(data.getString("observed_name")))throw new IOException("DOCUMENT_CHANGED");
            // ReadHandle retains its own descriptor; only the duplicate stream is closed here.
            File file=File.createTempFile("snapshot-",".bin",directory);owned.add(file);
            MessageDigest digest=MessageDigest.getInstance("SHA-256");long size=0;
            try(SafReadStorage.ReadHandle handle=storage.openRead(token,Math.min(FILE_LIMIT,JOB_LIMIT-total),cancel);
                ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(handle.descriptor.getFileDescriptor()));
                FileOutputStream output=new FileOutputStream(file)) {
                byte[] buffer=new byte[32768];int n;
                while((n=input.read(buffer))!=-1) {
                    cancel.throwIfCanceled();
                    if(size+n>FILE_LIMIT||total+n>JOB_LIMIT)throw new IOException("SNAPSHOT_BYTE_BUDGET");
                    output.write(buffer,0,n);digest.update(buffer,0,n);size+=n;total+=n;
                }
                output.getFD().sync();
            }
            cancel.throwIfCanceled();
            SafReadStorage.Entry after=storage.inspect(token,cancel);
            if(!entry.displayName.equals(after.displayName)||entry.isDirectory()!=after.isDirectory())throw new IOException("DOCUMENT_CHANGED");
            StringBuilder hash=new StringBuilder();for(byte b:digest.digest())hash.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
            data.put("snapshot_file",file.getName()).put("snapshot_size",size).put("sha256",hash.toString());
        }
        return new JSONObject().put("snapshot_file",data.getString("snapshot_file")).put("snapshot_size",data.getLong("snapshot_size")).put("sha256",data.getString("sha256")).toString().getBytes(java.nio.charset.StandardCharsets.UTF_8);
    }
    @Override public void close()throws IOException {
        IOException error=null;
        for(File file:owned)if(!file.delete()&&file.exists())error=new IOException("OWNED_SNAPSHOT_CLEANUP");
        if(!directory.delete()&&directory.exists())error=new IOException("OWNED_SNAPSHOT_CLEANUP");
        if(error!=null)throw error;
    }
}
