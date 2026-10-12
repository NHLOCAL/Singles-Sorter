package com.nhlocal.singlesorter.app;

import android.content.ContentResolver;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.util.HashMap;
import java.util.HashSet;
import org.json.JSONArray;
import org.json.JSONObject;

/** Detects observable target changes since preview, then tracks only this attempt's effects.
 * Metadata/reference equality is not inode identity or CAS. No-other-writer is required.
 */
final class TargetObservationGuard {
    private final ContentResolver resolver;private final Uri tree;private final CancellationSignal cancel;
    private final HashMap<String,JSONObject> expected=new HashMap<>();
    TargetObservationGuard(Context context,RetainedPlan plan,Uri tree,CancellationSignal cancel)throws Exception{
        resolver=context.getApplicationContext().getContentResolver();this.tree=tree;this.cancel=cancel;
        JSONArray entries=plan.referencesCopy();HashMap<String,JSONObject> tokens=new HashMap<>();
        for(int i=0;i<entries.length();i++)tokens.put(entries.getJSONObject(i).getString("token"),entries.getJSONObject(i));
        for(int i=0;i<entries.length();i++){
            JSONObject e=entries.getJSONObject(i);if(!tree.toString().equals(e.getString("tree_uri")))continue;
            String parent=e.isNull("parent")?null:tokens.get(e.getString("parent")).getString("document_id");
            JSONObject normalized=new JSONObject().put("id",e.getString("document_id")).put("name",e.getString("name")).put("directory",e.getBoolean("directory"))
                .put("modified",e.get("observed_modified")).put("size",e.getBoolean("directory")?JSONObject.NULL:e.get("observed_size")).put("parent",parent==null?JSONObject.NULL:parent);
            if(normalized.getBoolean("directory")&&normalized.isNull("modified"))throw new IOException("TARGET_DIRECTORY_OBSERVATION_UNAVAILABLE");
            expected.put(e.getString("document_id"),normalized);
        }
        if(expected.isEmpty())throw new IOException("TARGET_OBSERVATIONS_MISSING");
    }
    private Uri uri(String id){return DocumentsContract.buildDocumentUriUsingTree(tree,id);}
    private JSONObject observe(Uri uri,Object parent)throws Exception{
        cancel.throwIfCanceled();try(Cursor c=resolver.query(uri,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE,DocumentsContract.Document.COLUMN_LAST_MODIFIED,DocumentsContract.Document.COLUMN_SIZE},null,null,null,cancel)){
            if(c==null||!c.moveToFirst())throw new IOException("TARGET_UNAVAILABLE");boolean directory=DocumentsContract.Document.MIME_TYPE_DIR.equals(c.getString(2));
            Object modified=c.isNull(3)||c.getLong(3)<=0?JSONObject.NULL:Long.toString(c.getLong(3));Object size=directory||c.isNull(4)||c.getLong(4)<0?JSONObject.NULL:c.getLong(4);
            JSONObject result=new JSONObject().put("id",c.getString(0)).put("name",c.getString(1)).put("directory",directory).put("modified",modified).put("size",size).put("parent",parent);
            if(c.moveToNext())throw new IOException("TARGET_AMBIGUOUS");return result;
        }
    }
    void require(Uri uri)throws Exception{
        String id=DocumentsContract.getDocumentId(uri);JSONObject old=expected.get(id);if(old==null)throw new IOException("TARGET_NOT_IN_PREVIEW_OR_OWNED_EFFECT");
        JSONObject now=observe(uri,old.get("parent"));
        if(!fingerprint(old).same(fingerprint(now)))throw new IOException("TARGET_CHANGED_SINCE_OBSERVATION");
    }
    private static TargetFingerprint fingerprint(JSONObject value)throws Exception{
        return new TargetFingerprint(value.getString("id"),value.getString("name"),value.getBoolean("directory"),value.isNull("modified")?null:value.getString("modified"),value.isNull("size")?null:value.getLong("size"),value.isNull("parent")?null:value.getString("parent"));
    }
    void validateTree()throws Exception{HashSet<String> seen=new HashSet<>();walk(uri(DocumentsContract.getTreeDocumentId(tree)),seen,0);if(seen.size()!=expected.size())throw new IOException("TARGET_REMOVED_SINCE_PREVIEW");}
    private void walk(Uri current,HashSet<String> seen,int depth)throws Exception{
        cancel.throwIfCanceled();String id=DocumentsContract.getDocumentId(current);if(depth>128||seen.size()>=10000||!seen.add(id))throw new IOException("TARGET_TREE_LIMIT");require(current);
        if(!expected.get(id).getBoolean("directory"))return;
        try(Cursor c=resolver.query(DocumentsContract.buildChildDocumentsUriUsingTree(tree,id),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID},null,null,null,cancel)){
            if(c==null)throw new IOException("TARGET_LIST_UNAVAILABLE");
            while(c.moveToNext()){
                String child=c.getString(0);JSONObject known=expected.get(child);if(known==null||!id.equals(known.optString("parent")))throw new IOException("TARGET_ADDED_OR_REPARENTED");
                Uri childUri=uri(child);if(!DocumentsContract.isChildDocument(resolver,current,childUri))throw new IOException("TARGET_CHILD_SCOPE");walk(childUri,seen,depth+1);
            }
            if(c.getExtras()!=null&&(c.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||c.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("TARGET_LIST_INCOMPLETE");
        }
    }
    void created(Uri parent,Uri child)throws Exception{
        String id=DocumentsContract.getDocumentId(child);if(expected.containsKey(id))throw new IOException("CREATED_REFERENCE_COLLISION");
        expected.put(id,observe(child,DocumentsContract.getDocumentId(parent)));refresh(parent);
    }
    void changed(Uri old,Uri result,Uri from,Uri to)throws Exception{
        String before=DocumentsContract.getDocumentId(old),after=DocumentsContract.getDocumentId(result);
        if(!before.equals(after)&&expected.containsKey(after))throw new IOException("RETURNED_REFERENCE_COLLISION");expected.remove(before);
        expected.put(after,observe(result,DocumentsContract.getDocumentId(to)));
        if(expected.containsKey(DocumentsContract.getDocumentId(from)))refresh(from);refresh(to);
    }
    void removedEmpty(Uri directory,Uri parent)throws Exception{
        String id=DocumentsContract.getDocumentId(directory);JSONObject known=expected.get(id);
        if(known==null||!known.getBoolean("directory")||!DocumentsContract.getDocumentId(parent).equals(known.getString("parent")))throw new IOException("STAGE_REMOVAL_NOT_TRACKED");
        for(JSONObject entry:expected.values())if(id.equals(entry.optString("parent")))throw new IOException("STAGE_REMOVAL_HAS_TRACKED_CHILDREN");
        expected.remove(id);refresh(parent);
    }
    void refresh(Uri current)throws Exception{
        String id=DocumentsContract.getDocumentId(current);JSONObject old=expected.get(id);if(old==null)throw new IOException("UNOWNED_OBSERVATION_UPDATE");JSONObject now=observe(current,old.get("parent"));
        if(!fingerprint(old).sameStructure(fingerprint(now)))throw new IOException("TARGET_STRUCTURE_CHANGED_DURING_EFFECT");expected.put(id,now);
    }
}
