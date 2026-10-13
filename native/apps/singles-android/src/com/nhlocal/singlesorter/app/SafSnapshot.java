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
import java.util.HashMap;
import java.util.Map;
import java.util.Set;
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
    private final Map<String,JSONObject> files=new HashMap<>(),observations=new HashMap<>();
    private final Set<String> readable=new HashSet<>();
    private final DirectoryListings listings=new DirectoryListings();
    /** A missing listing is unknown, never an empty directory. Copies prevent callers
     * from silently changing retained membership after it was observed. */
    static final class DirectoryListings {
        private final Map<String,Set<String>> captured=new HashMap<>();
        boolean complete(String token){return captured.containsKey(token);}
        void observe(String token,List<String> children)throws IOException {
            if(children.size()>2000)throw new IOException("TREE_BUDGET_OR_CYCLE");
            HashSet<String> names=new HashSet<>();
            for(String child:children)if(child==null||child.equals(token)||!names.add(child))throw new IOException("TREE_BUDGET_OR_CYCLE");
            Set<String> before=captured.get(token);
            if(before!=null&&!before.equals(names))throw new IOException("DIRECTORY_MEMBERSHIP_CHANGED");
            captured.put(token,names);
        }
        void require(String token,List<String> children)throws IOException {
            if(!complete(token))throw new IOException("DIRECTORY_NOT_COMPLETELY_CAPTURED");
            observe(token,children);
        }
    }
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
        JSONObject data=observation(entry,parent);
        entries.put(data);observations.put(token,data);
        if(readFiles)readable.add(token);
        if(entry.isDirectory()) {
            if(depth==0||recursive) {
                List<SafReadStorage.Entry> children=storage.children(token,cancel);
                List<String> tokens=new ArrayList<>();
                for(SafReadStorage.Entry child:children){tokens.add(child.token);walk(child.token,token,readFiles,recursive,depth+1);}
                listings.observe(token,tokens);
            }
        } else if(readFiles) { files.put(token,new JSONObject().put("observed_name",entry.displayName)); }
    }
    private JSONObject observation(SafReadStorage.Entry entry,String parent)throws Exception {
        String[] source=storage.provenance(entry.token);
        for(String field:new String[]{entry.token,entry.displayName,source[0],source[1],source[2]})if(field==null||field.length()>4096)throw new IOException("ENTRY_LIMIT");
        return new JSONObject().put("token",entry.token).put("parent",parent==null?JSONObject.NULL:parent)
            .put("tree_uri",source[0]).put("authority",source[1]).put("document_id",source[2])
            .put("name",entry.displayName).put("directory",entry.isDirectory())
            .put("observed_size",entry.size==null?JSONObject.NULL:entry.size)
            .put("observed_modified",entry.lastModified==null?JSONObject.NULL:entry.lastModified.toString());
    }
    private void requireObservation(JSONObject expected,SafReadStorage.Entry current,String parent)throws Exception {
        JSONObject actual=observation(current,parent);
        for(String field:new String[]{"token","parent","tree_uri","authority","document_id","name","directory","observed_size","observed_modified"})
            if(!expected.get(field).equals(actual.get(field)))throw new IOException("DOCUMENT_CHANGED");
    }
    private String tokenForReference(JSONObject reference)throws Exception {
        if(reference.length()!=1||!reference.has("AndroidDocument"))throw new IOException("ALBUM_REFERENCE_SHAPE");
        JSONObject document=reference.getJSONObject("AndroidDocument");if(document.length()!=3)throw new IOException("ALBUM_REFERENCE_SHAPE");
        String found=null;
        for(JSONObject entry:observations.values()) {
            cancel.throwIfCanceled();
            if(entry.getString("tree_uri").equals(document.getString("tree_uri"))&&entry.getString("authority").equals(document.getString("authority"))&&entry.getString("document_id").equals(document.getString("document_id"))) {
                if(found!=null)throw new IOException("DUPLICATE_REFERENCE");found=entry.getString("token");
            }
        }
        if(found==null||!readable.contains(found))throw new IOException("ALBUM_SOURCE_NOT_CAPTURED");
        return found;
    }
    String requireAlbumObservation(JSONObject source)throws Exception {
        String token=tokenForReference(source.getJSONObject("reference"));JSONObject entry=observations.get(token);
        if(!source.getString("name").equals(entry.getString("name"))||!source.getString("kind").equals(entry.getBoolean("directory")?"Directory":"File")
                ||source.isNull("size")!=entry.isNull("observed_size")||!source.isNull("size")&&source.getLong("size")!=entry.getLong("observed_size")
                ||source.isNull("modified")!=entry.isNull("observed_modified")||!source.isNull("modified")&&!source.getString("modified").equals(entry.getString("observed_modified")))
            throw new IOException("ALBUM_OBSERVATION_CHANGED");
        return token;
    }
    private String albumRoot(JSONObject reference)throws Exception {
        String token=tokenForReference(reference);
        if(!observations.get(token).getBoolean("directory"))throw new IOException("ALBUM_SOURCE_NOT_DIRECTORY");return token;
    }
    private int depth(String token)throws Exception {
        HashSet<String> ancestors=new HashSet<>();JSONObject current=observations.get(token);int depth=0;
        while(current!=null&&!current.isNull("parent")) {
            cancel.throwIfCanceled();
            if(++depth>128||!ancestors.add(current.getString("token")))throw new IOException("TREE_BUDGET_OR_CYCLE");
            current=observations.get(current.getString("parent"));
        }
        if(current==null)throw new IOException("SOURCE_PARENT_NOT_CAPTURED");return depth;
    }
    /** Called only AFTER original planning. Completing an unlisted directory must
     * never change which original Album/Single/Fix operations were planned. */
    List<String> completeAlbumSubtree(JSONObject reference)throws Exception {
        String root=albumRoot(reference);List<String> filesToBind=new ArrayList<>();
        verifyAncestors(root);
        completeTree(root,depth(root),new HashSet<>(),filesToBind);
        return filesToBind;
    }
    private void completeTree(String token,int depth,Set<String> seen,List<String> filesToBind)throws Exception {
        cancel.throwIfCanceled();
        if(depth>128||!seen.add(token)||seen.size()>2000)throw new IOException("TREE_BUDGET_OR_CYCLE");
        JSONObject expected=observations.get(token);
        if(expected==null||!readable.contains(token))throw new IOException("ALBUM_SOURCE_NOT_CAPTURED");
        String parent=expected.isNull("parent")?null:expected.getString("parent");
        requireObservation(expected,storage.inspect(token,cancel),parent);
        if(!expected.getBoolean("directory")){filesToBind.add(token);return;}
        List<SafReadStorage.Entry> children=storage.children(token,cancel);List<String> childTokens=new ArrayList<>();
        for(SafReadStorage.Entry child:children){cancel.throwIfCanceled();childTokens.add(child.token);}
        listings.observe(token,childTokens);
        for(SafReadStorage.Entry child:children) {
            cancel.throwIfCanceled();JSONObject prior=observations.get(child.token);
            if(prior==null) {
                if(entries.length()>=2000||!visited.add(child.token))throw new IOException("TREE_BUDGET_OR_CYCLE");
                prior=observation(child,token);entries.put(prior);observations.put(child.token,prior);readable.add(child.token);
                if(!child.isDirectory())files.put(child.token,new JSONObject().put("observed_name",child.displayName));
            } else requireObservation(prior,child,token);
            completeTree(child.token,depth+1,seen,filesToBind);
        }
        verifyDirectory(token,expected,parent);
    }
    private List<SafReadStorage.Entry> verifyDirectory(String token,JSONObject expected,String parent)throws Exception {
        cancel.throwIfCanceled();requireObservation(expected,storage.inspect(token,cancel),parent);
        List<SafReadStorage.Entry> children=storage.children(token,cancel);List<String> childTokens=new ArrayList<>();
        for(SafReadStorage.Entry child:children) {
            cancel.throwIfCanceled();childTokens.add(child.token);JSONObject prior=observations.get(child.token);
            if(prior==null)throw new IOException("DIRECTORY_MEMBERSHIP_CHANGED");requireObservation(prior,child,token);
        }
        listings.require(token,childTokens);return children;
    }
    /** Recheck retained metadata and complete memberships after descendant hashing.
     * This is observation evidence, not provider immutability or mutation authority. */
    void verifyAlbumSubtree(JSONObject reference)throws Exception {
        String root=albumRoot(reference);verifyTree(root,depth(root),new HashSet<>());verifyAncestors(root);
    }
    private void verifyAncestors(String token)throws Exception {
        HashSet<String> seen=new HashSet<>();JSONObject child=observations.get(token);
        while(!child.isNull("parent")) {
            cancel.throwIfCanceled();String parentToken=child.getString("parent");
            if(seen.size()>=128||!seen.add(parentToken))throw new IOException("TREE_BUDGET_OR_CYCLE");
            JSONObject parent=observations.get(parentToken);
            if(parent==null||!parent.getBoolean("directory")||!readable.contains(parentToken))throw new IOException("SOURCE_PARENT_NOT_CAPTURED");
            verifyDirectory(parentToken,parent,parent.isNull("parent")?null:parent.getString("parent"));child=parent;
        }
    }
    private void verifyTree(String token,int depth,Set<String> seen)throws Exception {
        cancel.throwIfCanceled();
        if(depth>128||!seen.add(token)||seen.size()>2000)throw new IOException("TREE_BUDGET_OR_CYCLE");
        JSONObject expected=observations.get(token);if(expected==null)throw new IOException("ALBUM_SOURCE_NOT_CAPTURED");
        String parent=expected.isNull("parent")?null:expected.getString("parent");
        if(expected.getBoolean("directory"))for(SafReadStorage.Entry child:verifyDirectory(token,expected,parent))verifyTree(child.token,depth+1,seen);
        else requireObservation(expected,storage.inspect(token,cancel),parent);
    }
    JSONArray retainedReferences()throws Exception {
        JSONArray retained=new JSONArray();
        for(int i=0;i<entries.length();i++) {
            cancel.throwIfCanceled();JSONObject entry=new JSONObject(entries.getJSONObject(i).toString());
            if(entry.getBoolean("directory"))entry.put("directory_children_complete",listings.complete(entry.getString("token")));
            retained.put(entry);
        }
        return retained;
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
