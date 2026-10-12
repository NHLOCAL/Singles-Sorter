package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.util.HashSet;
import org.json.JSONObject;

/** Separate admission policy: unresolved journals block their roots; unknown authority blocks all. */
final class RecoveryAdmission {
 private RecoveryAdmission(){}
 static void require(Context context,ProviderReferences selected,CancellationSignal cancelled)throws Exception{
  cancelled.throwIfCanceled();selected.requireSelectedRoots();HashSet<String> checked=new HashSet<>(),budgeted=new HashSet<>();
  RetainedEffectsAcceptance.ReceiptIndex receipts=RetainedEffectsAcceptance.receipts(context,cancelled);
  RetainedJournals.scan(context,cancelled::isCanceled,record->{
   if(!record.attention)return;
   JSONObject authority=record.authority();if(!authority.getBoolean("provider_quiescence_required")||authority.getBoolean("provider_atomic_batch"))throw new IOException("RECOVERY_REQUIRED");
   JSONObject request=authority.getJSONObject("original_plan").getJSONObject("request");
   // Validate both original roots before considering any immutable acknowledgment.
   for(String role:roles(request)){if(request.isNull(role))throw new IOException("RECOVERY_REQUIRED");Uri original=root(request.getJSONObject(role));if(budgeted.add(original.toString())&&budgeted.size()>64)throw new IOException("RECOVERY_SCOPE_BUDGET");}
   if(RetainedEffectsAcceptance.accepted(context,record,cancelled,receipts))return;
   for(String role:roles(request)){
    if(request.isNull(role))throw new IOException("RECOVERY_REQUIRED");Uri affected=root(request.getJSONObject(role));
    if(!checked.add(affected.toString()))continue;if(checked.size()>64)throw new IOException("RECOVERY_SCOPE_BUDGET");
    ProviderReferences.requireGrant(context,affected,false);directory(context,document(affected),cancelled);
    for(Uri current:selected.selectedRoots()){
     cancelled.throwIfCanceled();ProviderReferences.requireGrant(context,current,false);Uri currentRoot=document(current);directory(context,currentRoot,cancelled);
     Uri oldRoot=document(affected);String currentId=DocumentsContract.getTreeDocumentId(current),oldId=DocumentsContract.getTreeDocumentId(affected);
     if(currentId.equals(oldId))throw new IOException("RECOVERY_REQUIRED");
     boolean forward=CheckedLocalAncestry.isChild(context,currentRoot,oldRoot);cancelled.throwIfCanceled();
     boolean reverse=CheckedLocalAncestry.isChild(context,oldRoot,currentRoot);cancelled.throwIfCanceled();
     if(!RecoveryScopePolicy.disjoint(false,forward,reverse))throw new IOException("RECOVERY_REQUIRED");
    }
   }
  });
  cancelled.throwIfCanceled();
 }
 static String[] roles(JSONObject request)throws Exception{
  String mode=request.getString("mode");if(mode.equals("Sort")&&!request.isNull("source")&&!request.isNull("target"))return new String[]{"source","target"};
  if(mode.equals("FixNames")&&!request.isNull("source")&&request.isNull("target"))return new String[]{"source"};throw new IOException("RECOVERY_ROOT_MODE");
 }
 static Uri root(JSONObject reference)throws Exception{
  if(reference.length()!=1||!reference.has("AndroidDocument"))throw new IOException("RECOVERY_REQUIRED");JSONObject value=reference.getJSONObject("AndroidDocument");
  if(value.length()!=3||!ProviderReferences.AUTHORITY.equals(value.getString("authority")))throw new IOException("RECOVERY_REQUIRED");
  if(value.getString("tree_uri").length()>4096||value.getString("document_id").length()>4096)throw new IOException("RECOVERY_REQUIRED");
  Uri tree=Uri.parse(value.getString("tree_uri"));
  if(!"content".equals(tree.getScheme())||!ProviderReferences.AUTHORITY.equals(tree.getAuthority())||!DocumentsContract.isTreeUri(tree))throw new IOException("RECOVERY_REQUIRED");
  if(tree.getQuery()!=null||tree.getFragment()!=null||tree.getPathSegments().size()!=2||!"tree".equals(tree.getPathSegments().get(0))||!DocumentsContract.getTreeDocumentId(tree).equals(value.getString("document_id")))throw new IOException("RECOVERY_REQUIRED");
  return tree;
 }
 private static Uri document(Uri tree){return DocumentsContract.buildDocumentUriUsingTree(tree,DocumentsContract.getTreeDocumentId(tree));}
 private static void directory(Context context,Uri document,CancellationSignal cancelled)throws Exception{
  cancelled.throwIfCanceled();try(Cursor cursor=context.getContentResolver().query(document,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancelled)){
   if(cursor==null||!cursor.moveToFirst()||!DocumentsContract.getDocumentId(document).equals(cursor.getString(0))||!DocumentsContract.Document.MIME_TYPE_DIR.equals(cursor.getString(1))||cursor.moveToNext())throw new IOException("RECOVERY_REQUIRED");
  }cancelled.throwIfCanceled();
 }
}
