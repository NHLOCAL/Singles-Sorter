package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.util.HashMap;
import java.util.HashSet;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit picker parent authority for one selected Album root, never a path-derived grant.
 * A complete listing is an observation under quiescence, not an atomic child-safe delete.
 */
final class AlbumRootParent {
    private AlbumRootParent(){}
    static boolean required(JSONObject plan)throws Exception{
        return required(plan,()->false);
    }
    static boolean required(JSONObject plan,BooleanSupplier cancelled)throws Exception{
        JSONObject request=plan.getJSONObject("request");if(!"Sort".equals(request.getString("mode")))return false;
        JSONArray operations=plan.getJSONArray("operations");if(operations.length()>10000)throw invalid("BOUND");boolean result=false;
        for(int n=0;n<operations.length();n++){JournalExportBounds.checkpoint(cancelled);JSONObject kind=operations.getJSONObject(n).getJSONObject("kind");if(!kind.has("AlbumTransfer"))continue;JSONObject album=kind.getJSONObject("AlbumTransfer");
            if(album.getBoolean("remove_empty_source")&&NativeTransferPlan.sameReference(album.getJSONObject("source").getJSONObject("reference"),request.getJSONObject("source"))){if(result||n!=operations.length()-1)throw invalid("ORDER");result=true;}}
        return result;
    }
    static JSONObject validate(JSONObject evidence,JSONObject plan,BooleanSupplier cancelled)throws Exception{
        JournalExportBounds.checkpoint(cancelled);if(evidence==null||!required(plan,cancelled)||FixProtocol.integer(evidence,"schema",1)!=1||evidence.length()!=8||evidence.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length>256*1024)throw invalid("BINDING");
        JSONObject source=plan.getJSONObject("request").getJSONObject("source");if(!AlbumJson.same(source,evidence.getJSONObject("source_reference"),cancelled))throw invalid("SOURCE");JSONObject sourceDoc=FixBindingIndex.document(source);Uri sourceTree=RecoveryAdmission.root(source),parent=parentTree(evidence);
        String parentId=FixProtocol.string(evidence,"parent_document_id",4096);if(parentId.isEmpty()||parentId.indexOf(0)>=0||!DocumentsContract.getTreeDocumentId(parent).equals(parentId)||parentId.equals(sourceDoc.getString("document_id"))||parent.equals(sourceTree))throw invalid("SCOPE");
        ProviderNames.require(FixProtocol.string(evidence,"parent_name",1024));String sourceName=FixProtocol.string(evidence,"source_name",1024);ProviderNames.require(sourceName);String modified=FixProtocol.string(evidence,"parent_modified",32);if(!modified.matches("[1-9][0-9]{0,18}"))throw invalid("OBSERVATION");
        JSONArray operations=plan.getJSONArray("operations");JSONObject original=operations.getJSONObject(operations.length()-1).getJSONObject("kind").getJSONObject("AlbumTransfer").getJSONObject("source");if(!sourceName.equals(original.getString("name")))throw invalid("SOURCE");
        HashMap<String,JSONObject> children=index(evidence.getJSONArray("children"),cancelled);JSONObject child=children.get(sourceDoc.getString("document_id"));if(child==null||!sourceName.equals(child.getString("name"))||!child.getBoolean("directory"))throw invalid("DIRECT_CHILD");uriBudget(evidence);JournalExportBounds.checkpoint(cancelled);return new JSONObject(evidence.toString());
    }
    static Uri parentTree(JSONObject evidence)throws Exception{
        String raw=FixProtocol.string(evidence,"parent_tree",4096);Uri tree=Uri.parse(raw);if(!"content".equals(tree.getScheme())||!ProviderReferences.AUTHORITY.equals(tree.getAuthority())||tree.getQuery()!=null||tree.getFragment()!=null||tree.getPathSegments().size()!=2||!"tree".equals(tree.getPathSegments().get(0)))throw invalid("SCOPE");DocumentsContract.getTreeDocumentId(tree);return tree;
    }
    static Uri parentDocument(JSONObject evidence)throws Exception{Uri tree=parentTree(evidence);return DocumentsContract.buildDocumentUriUsingTree(tree,DocumentsContract.getTreeDocumentId(tree));}
    static Uri sourceDocument(JSONObject evidence)throws Exception{return DocumentsContract.buildDocumentUriUsingTree(parentTree(evidence),FixBindingIndex.document(evidence.getJSONObject("source_reference")).getString("document_id"));}
    static long uriBudget(JSONObject evidence)throws Exception{long size=1024+Math.max(parentDocument(evidence).toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length,sourceDocument(evidence).toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length);if(size>8192)throw invalid("URI_BOUND");return size;}
    static JSONObject capture(Context context,RetainedPlan retained,String[] picker,CancellationSignal cancel)throws Exception{
        JSONObject plan=retained.planCopy();if(!required(plan,cancel::isCanceled)||picker==null||picker.length!=3||!ProviderReferences.AUTHORITY.equals(picker[1]))throw invalid("SELECTION");Uri tree=Uri.parse(picker[0]);ProviderReferences.requireGrant(context,tree,true);if(!DocumentsContract.getTreeDocumentId(tree).equals(picker[2]))throw invalid("SELECTION");
        JSONObject source=plan.getJSONObject("request").getJSONObject("source"),sourceDoc=FixBindingIndex.document(source);Uri parent=DocumentsContract.buildDocumentUriUsingTree(tree,picker[2]),child=DocumentsContract.buildDocumentUriUsingTree(tree,sourceDoc.getString("document_id"));
        cancel.throwIfCanceled();if(picker[2].equals(sourceDoc.getString("document_id"))||!CheckedLocalAncestry.isChild(context,parent,child)||CheckedLocalAncestry.isChild(context,child,parent))throw invalid("DIRECT_CHILD");
        JSONObject before=directory(context,parent,cancel);JSONArray children=list(context,parent,cancel);JSONObject after=directory(context,parent,cancel);if(!AlbumJson.same(before,after,cancel::isCanceled))throw invalid("CHANGED");
        JSONArray operations=plan.getJSONArray("operations");String name=operations.getJSONObject(operations.length()-1).getJSONObject("kind").getJSONObject("AlbumTransfer").getJSONObject("source").getString("name");
        JSONObject evidence=new JSONObject().put("schema",1).put("source_reference",source).put("source_name",name).put("parent_tree",tree.toString()).put("parent_document_id",picker[2]).put("parent_name",before.getString("name")).put("parent_modified",before.getString("modified")).put("children",children);
        return validate(evidence,plan,cancel::isCanceled);
    }
    static void requireBefore(Context context,JSONObject evidence,CancellationSignal cancel)throws Exception{
        ProviderReferences.requireGrant(context,parentTree(evidence),true);Uri parent=parentDocument(evidence),source=sourceDocument(evidence);JSONObject before=directory(context,parent,cancel);requireParent(evidence,before,true);
        JSONArray current=list(context,parent,cancel);sameChildren(evidence.getJSONArray("children"),current,null,cancel::isCanceled);JSONObject after=directory(context,parent,cancel);if(!AlbumJson.same(before,after,cancel::isCanceled)||!CheckedLocalAncestry.isChild(context,parent,source)||CheckedLocalAncestry.isChild(context,source,parent))throw invalid("CHANGED");
    }
    static void requireAfter(Context context,JSONObject evidence,CancellationSignal cancel)throws Exception{
        ProviderReferences.requireGrant(context,parentTree(evidence),true);Uri parent=parentDocument(evidence);JSONObject before=directory(context,parent,cancel);requireParent(evidence,before,false);JSONArray current=list(context,parent,cancel);sameChildren(evidence.getJSONArray("children"),current,FixBindingIndex.document(evidence.getJSONObject("source_reference")).getString("document_id"),cancel::isCanceled);if(!AlbumJson.same(before,directory(context,parent,cancel),cancel::isCanceled))throw invalid("CHANGED");
    }
    static JSONObject inspectMembership(Context context,JSONObject evidence,Uri selected,CancellationSignal cancel)throws Exception{
        if(selected==null||!parentTree(evidence).equals(selected))throw invalid("FRESH_SELECTION_REQUIRED");ProviderReferences.requireGrant(context,selected,false);Uri parent=parentDocument(evidence);JSONObject before=directory(context,parent,cancel);requireParent(evidence,before,false);JSONArray children=list(context,parent,cancel);if(!AlbumJson.same(before,directory(context,parent,cancel),cancel::isCanceled))throw invalid("CHANGED");
        String source=FixBindingIndex.document(evidence.getJSONObject("source_reference")).getString("document_id"),name=evidence.getString("source_name");String status="source_absent";for(int n=0;n<children.length();n++){cancel.throwIfCanceled();JSONObject child=children.getJSONObject(n);if(source.equals(child.getString("id"))){status=child.getBoolean("directory")&&name.equals(child.getString("name"))?"source_present":"source_identity_changed";break;}if(name.equalsIgnoreCase(child.getString("name")))status="source_name_reused";}
        JSONObject result=new JSONObject().put("status",status).put("parent_tree",selected.toString()).put("parent_document_id",before.getString("id")).put("source_document_id",source).put("source_name",name).put("children",children).put("historical_success_changed",false);validateMembership(evidence,result,cancel::isCanceled);return result;
    }
    static void validateMembership(JSONObject evidence,JSONObject report,BooleanSupplier cancelled)throws Exception{
        if(report.length()!=7||!parentTree(evidence).toString().equals(report.getString("parent_tree"))||!evidence.getString("parent_document_id").equals(report.getString("parent_document_id"))||!FixBindingIndex.document(evidence.getJSONObject("source_reference")).getString("document_id").equals(report.getString("source_document_id"))||!evidence.getString("source_name").equals(report.getString("source_name"))||FixProtocol.bool(report,"historical_success_changed"))throw invalid("REVIEW_BINDING");
        HashMap<String,JSONObject> children=index(report.getJSONArray("children"),cancelled);String id=report.getString("source_document_id"),name=report.getString("source_name"),status="source_absent";JSONObject found=children.get(id);
        if(found!=null)status=found.getBoolean("directory")&&name.equals(found.getString("name"))?"source_present":"source_identity_changed";else for(JSONObject child:children.values()){JournalExportBounds.checkpoint(cancelled);if(name.equalsIgnoreCase(child.getString("name")))status="source_name_reused";}if(!status.equals(report.getString("status")))throw invalid("REVIEW_BINDING");
    }
    private static void requireParent(JSONObject evidence,JSONObject current,boolean modified)throws Exception{if(!evidence.getString("parent_document_id").equals(current.getString("id"))||!evidence.getString("parent_name").equals(current.getString("name"))||modified&&!evidence.getString("parent_modified").equals(current.getString("modified")))throw invalid("CHANGED");}
    private static JSONObject directory(Context context,Uri uri,CancellationSignal cancel)throws Exception{
        cancel.throwIfCanceled();try(Cursor cursor=context.getContentResolver().query(uri,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE,DocumentsContract.Document.COLUMN_LAST_MODIFIED},null,null,null,cancel)){
            if(cursor==null||!cursor.moveToFirst()||!DocumentsContract.getDocumentId(uri).equals(cursor.getString(0))||!DocumentsContract.Document.MIME_TYPE_DIR.equals(cursor.getString(2))||cursor.isNull(3)||cursor.getLong(3)<=0)throw invalid("UNAVAILABLE");String name=cursor.getString(1);ProviderNames.require(name);JSONObject result=new JSONObject().put("id",cursor.getString(0)).put("name",name).put("modified",Long.toString(cursor.getLong(3)));if(cursor.moveToNext())throw invalid("AMBIGUOUS");return result;}
    }
    private static JSONArray list(Context context,Uri parent,CancellationSignal cancel)throws Exception{
        JSONArray children=new JSONArray();cancel.throwIfCanceled();try(Cursor cursor=context.getContentResolver().query(DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
            if(cursor==null)throw invalid("UNAVAILABLE");while(cursor.moveToNext()){cancel.throwIfCanceled();if(children.length()>=2000)throw invalid("BOUND");String id=cursor.getString(0),name=cursor.getString(1),mime=cursor.getString(2);if(id==null||mime==null)throw invalid("AMBIGUOUS");Uri child=DocumentsContract.buildDocumentUriUsingTree(parent,id);if(!CheckedLocalAncestry.isChild(context,parent,child))throw invalid("SCOPE");children.put(new JSONObject().put("id",id).put("name",name).put("directory",DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)));}
            if(cursor.getExtras()!=null&&(cursor.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||cursor.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw invalid("INCOMPLETE");}
        index(children,cancel::isCanceled);return children;
    }
    private static HashMap<String,JSONObject> index(JSONArray children,BooleanSupplier cancelled)throws Exception{
        if(children.length()>2000)throw invalid("BOUND");HashMap<String,JSONObject> result=new HashMap<>();HashSet<String> names=new HashSet<>();for(int n=0;n<children.length();n++){JournalExportBounds.checkpoint(cancelled);JSONObject child=children.getJSONObject(n);String id=FixProtocol.string(child,"id",4096),name=FixProtocol.string(child,"name",1024);ProviderNames.require(name);FixProtocol.bool(child,"directory");if(id.isEmpty()||id.indexOf(0)>=0||child.length()!=3||result.put(id,child)!=null||!names.add(name.toLowerCase(java.util.Locale.ROOT)))throw invalid("AMBIGUOUS");}return result;
    }
    static void sameChildren(JSONArray expected,JSONArray current,String removed,BooleanSupplier cancelled)throws Exception{
        HashMap<String,JSONObject> old=index(expected,cancelled),now=index(current,cancelled);if(removed!=null&&old.remove(removed)==null)throw invalid("BINDING");if(old.size()!=now.size())throw invalid("CHANGED");for(String id:old.keySet()){JournalExportBounds.checkpoint(cancelled);if(!AlbumJson.same(old.get(id),now.get(id),cancelled))throw invalid("CHANGED");}
    }
    private static IOException invalid(String reason){return new IOException("ALBUM_PARENT_"+reason);}
}
