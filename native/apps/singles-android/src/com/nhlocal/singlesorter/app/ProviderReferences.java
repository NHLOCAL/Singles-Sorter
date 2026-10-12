package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.content.UriPermission;
import android.content.pm.ApplicationInfo;
import android.content.pm.ProviderInfo;
import android.net.Uri;
import android.provider.DocumentsContract;
import com.nhlocal.singlesorter.platform.SafReadStorage;
import java.util.HashMap;
import org.json.JSONArray;
import org.json.JSONObject;

/** Resolves only references retained from the original native plan and picker scope.
 * Reference equality does not establish content identity or permission to mutate.
 */
final class ProviderReferences {
    static final String AUTHORITY="com.android.externalstorage.documents";
    private final Context context;
    private final HashMap<String,JSONObject> byReference=new HashMap<>(),byToken=new HashMap<>();
    private final JSONObject bindings;
    final Uri sourceTree,targetTree;
    ProviderReferences(Context context,RetainedPlan retained,SafReadStorage current,String sourceToken,String targetToken)throws Exception{
        this.context=context.getApplicationContext();bindings=retained.bindingsCopy();
        JSONObject plan=retained.planCopy(),request=plan.getJSONObject("request");
        JSONArray entries=retained.referencesCopy();if(entries.length()>2000)throw new IllegalArgumentException("REFERENCE_LIMIT");
        for(int n=0;n<entries.length();n++){
            JSONObject entry=entries.getJSONObject(n);String token=entry.getString("token");
            String key=key(entry.getString("tree_uri"),entry.getString("authority"),entry.getString("document_id"));
            if(byReference.put(key,entry)!=null||byToken.put(token,entry)!=null)throw new IllegalArgumentException("DUPLICATE_REFERENCE");
        }
        JSONObject source=resolve(request.getJSONObject("source"));requireSelected(source,current.provenance(sourceToken));sourceTree=Uri.parse(source.getString("tree_uri"));
        if(request.isNull("target")){targetTree=null;if(targetToken!=null)throw new IllegalArgumentException("UNEXPECTED_TARGET");}
        else{if(targetToken==null)throw new IllegalArgumentException("TARGET_REQUIRED");JSONObject target=resolve(request.getJSONObject("target"));requireSelected(target,current.provenance(targetToken));targetTree=Uri.parse(target.getString("tree_uri"));}
    }
    private static String key(String tree,String authority,String id){if(tree.indexOf(0)>=0||authority.indexOf(0)>=0||id.indexOf(0)>=0)throw new IllegalArgumentException("REFERENCE_NUL");return tree+'\u0000'+authority+'\u0000'+id;}
    private static void requireSelected(JSONObject entry,String[] current)throws Exception{
        if(!entry.getBoolean("directory")||!entry.getString("tree_uri").equals(current[0])||!entry.getString("authority").equals(current[1])||!entry.getString("document_id").equals(current[2]))throw new SecurityException("SELECTED_SCOPE_CHANGED");
    }
    JSONObject resolve(JSONObject reference)throws Exception{
        if(reference.length()!=1||!reference.has("AndroidDocument"))throw new SecurityException("NON_ANDROID_REFERENCE");JSONObject r=reference.getJSONObject("AndroidDocument");
        if(r.length()!=3)throw new SecurityException("REFERENCE_SHAPE");JSONObject entry=byReference.get(key(r.getString("tree_uri"),r.getString("authority"),r.getString("document_id")));
        if(entry==null)throw new SecurityException("REFERENCE_NOT_RETAINED");return new JSONObject(entry.toString());
    }
    Uri uri(JSONObject reference)throws Exception{JSONObject entry=resolve(reference);Uri tree=Uri.parse(entry.getString("tree_uri"));requireReadable(tree);return DocumentsContract.buildDocumentUriUsingTree(tree,entry.getString("document_id"));}
    Uri parent(JSONObject reference)throws Exception{
        JSONObject child=resolve(reference);if(child.isNull("parent"))throw new SecurityException("SOURCE_PARENT_MISSING");JSONObject parent=byToken.get(child.getString("parent"));
        if(parent==null||!parent.getBoolean("directory")||!parent.getString("tree_uri").equals(child.getString("tree_uri"))||!parent.getString("authority").equals(child.getString("authority")))throw new SecurityException("SOURCE_PARENT_SCOPE");
        Uri tree=Uri.parse(parent.getString("tree_uri"));requireReadable(tree);return DocumentsContract.buildDocumentUriUsingTree(tree,parent.getString("document_id"));
    }
    JSONObject binding(JSONObject reference)throws Exception{JSONObject entry=resolve(reference);if(entry.getBoolean("directory"))throw new IllegalArgumentException("DIRECTORY_CONTENT_NOT_BOUND");return new JSONObject(bindings.getJSONObject(entry.getString("token")).toString());}
    void requireReadable(Uri tree)throws Exception{requireGrant(tree,false);}
    void requireWritable(Uri tree)throws Exception{requireGrant(tree,true);}
    private void requireGrant(Uri tree,boolean write)throws Exception{requireGrant(context,tree,write);}
    static void requireGrant(Context context,Uri tree,boolean write)throws Exception{
        if(tree==null||!"content".equals(tree.getScheme())||!AUTHORITY.equals(tree.getAuthority())||!DocumentsContract.isTreeUri(tree))throw new SecurityException("LOCAL_PROVIDER_REQUIRED");
        ProviderInfo provider=context.getPackageManager().resolveContentProvider(AUTHORITY,0);
        if(provider==null||(provider.applicationInfo.flags&ApplicationInfo.FLAG_SYSTEM)==0||!"com.android.externalstorage".equals(provider.packageName))throw new SecurityException("SYSTEM_LOCAL_PROVIDER_REQUIRED");
        for(UriPermission grant:context.getContentResolver().getPersistedUriPermissions())if(tree.equals(grant.getUri())&&grant.isReadPermission()&&(!write||grant.isWritePermission()))return;
        throw new SecurityException("PICKER_WRITE_GRANT_REQUIRED");
    }
    void requireDisjointRoots()throws Exception{
        requireReadable(sourceTree);requireWritable(targetTree);
        String sourceId=DocumentsContract.getTreeDocumentId(sourceTree),targetId=DocumentsContract.getTreeDocumentId(targetTree);
        Uri sourceRoot=DocumentsContract.buildDocumentUriUsingTree(sourceTree,sourceId),targetRoot=DocumentsContract.buildDocumentUriUsingTree(targetTree,targetId);
        if(sourceId.equals(targetId)||CheckedLocalAncestry.isChild(context,sourceRoot,targetRoot)||CheckedLocalAncestry.isChild(context,targetRoot,sourceRoot))throw new SecurityException("SOURCE_TARGET_OVERLAP");
    }
    Uri targetRoot()throws Exception{requireWritable(targetTree);return DocumentsContract.buildDocumentUriUsingTree(targetTree,DocumentsContract.getTreeDocumentId(targetTree));}
}
