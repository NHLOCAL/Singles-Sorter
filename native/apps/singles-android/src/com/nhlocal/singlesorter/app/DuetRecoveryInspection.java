package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.OperationCanceledException;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.util.HashMap;
import java.util.HashSet;
import org.json.JSONArray;
import org.json.JSONObject;

/** Fresh exact planned-path lookup for read-only recovery. Never retries retirement. */
final class DuetRecoveryInspection {
    private DuetRecoveryInspection(){}
    static JSONArray destinations(Context context,JSONObject authority,JSONObject state,CancellationSignal cancel)throws Exception{
        JSONObject plan=authority.getJSONObject("original_plan");NativeDuetPlan groups=NativeDuetPlan.inspectOriginal(plan,cancel::isCanceled);
        HashMap<Integer,String> recorded=new HashMap<>();JSONArray events=state.getJSONArray("events"),operations=plan.getJSONArray("operations");
        for(int i=0;i<events.length();i++){cancel.throwIfCanceled();JSONObject event=events.getJSONObject(i);String op=event.getString("operation");if("verified".equals(event.getString("phase"))&&op.matches("operation-[0-9]{1,5}"))recorded.put(Integer.parseInt(op.substring(10)),event.getJSONObject("detail").getString("destination"));}
        JSONObject target=FixBindingIndex.document(plan.getJSONObject("request").getJSONObject("target"));Uri tree=Uri.parse(target.getString("tree_uri"));ProviderReferences.requireGrant(context,tree,false);
        Uri root=DocumentsContract.buildDocumentUriUsingTree(tree,target.getString("document_id"));JSONArray result=new JSONArray();long[] listed={0};
        for(int group=0;group<groups.groups.size();group++)for(int operation:groups.groups.get(group).requiredOperations){
            cancel.throwIfCanceled();JSONArray path=operations.getJSONObject(operation).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("transfer").getJSONObject("destination").getJSONArray("components");
            JSONObject row=new JSONObject().put("group",group).put("operation",operation).put("planned_components",new JSONArray(path.toString())).put("recorded_destination",recorded.containsKey(operation)?recorded.get(operation):JSONObject.NULL);
            try{
                Uri current=root;for(int n=0;n<path.length();n++){current=find(context,current,path.getString(n),n<path.length()-1,cancel,listed);if(current==null)break;}
                if(current==null)row.put("status","planned_name_not_currently_resolved");
                else row.put("status","planned_name_currently_observed").put("current_named_destination",current.toString()).put("matches_recorded_destination",current.toString().equals(recorded.get(operation)));
            }catch(OperationCanceledException failure){throw failure;}catch(Exception unavailable){cancel.throwIfCanceled();if(listed[0]>20000)throw new IOException("REVIEW_DUET_LIST_LIMIT",unavailable);row.put("status","unavailable").put("diagnostic",unavailable.getClass().getSimpleName());}
            result.put(row);
        }
        cancel.throwIfCanceled();return result;
    }
    private static Uri find(Context context,Uri parent,String name,boolean directory,CancellationSignal cancel,long[] listed)throws Exception{
        ProviderNames.require(name);Uri selected=null;HashSet<String> ids=new HashSet<>();
        try(Cursor cursor=context.getContentResolver().query(DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
            if(cursor==null)throw new IOException("REVIEW_DUET_LIST_UNAVAILABLE");
            while(cursor.moveToNext()){
                cancel.throwIfCanceled();if(++listed[0]>20000)throw new IOException("REVIEW_DUET_LIST_LIMIT");String id=cursor.getString(0),actual=cursor.getString(1),mime=cursor.getString(2);
                if(id==null||actual==null||mime==null||id.length()>4096||!ids.add(id))throw new IOException("REVIEW_DUET_LIST_AMBIGUOUS");
                if(name.equalsIgnoreCase(actual)){
                    if(!name.equals(actual)||selected!=null||directory!=DocumentsContract.Document.MIME_TYPE_DIR.equals(mime))throw new IOException("REVIEW_DUET_NAME_MISMATCH");
                    selected=DocumentsContract.buildDocumentUriUsingTree(parent,id);if(!CheckedLocalAncestry.isChild(context,parent,selected))throw new IOException("REVIEW_DUET_SCOPE");
                }
            }
            if(cursor.getExtras()!=null&&(cursor.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||cursor.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("REVIEW_DUET_LIST_INCOMPLETE");
        }
        return selected;
    }
}
