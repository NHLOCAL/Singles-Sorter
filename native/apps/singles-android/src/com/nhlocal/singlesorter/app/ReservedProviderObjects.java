package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.util.ArrayList;
import org.json.JSONArray;
import org.json.JSONObject;

/** Retained backups remain immutable even after terminal outcomes or explicit recovery acceptance.
 * Fresh provider ancestry/equality is required; path spelling is never proof of disjointness.
 */
final class ReservedProviderObjects {
    private static final class Scope {final Uri object,parent;final String name;final boolean temporaryReleased;Scope(Uri object,Uri parent,String name,boolean released){this.object=object;this.parent=parent;this.name=name;temporaryReleased=released;}}
    private final Context context;private final CancellationSignal cancel;private final ArrayList<Scope> scopes=new ArrayList<>();private int checks;
    private ReservedProviderObjects(Context context,CancellationSignal cancel){this.context=context.getApplicationContext();this.cancel=cancel;}
    static ReservedProviderObjects load(Context context,CancellationSignal cancel)throws Exception{
        ReservedProviderObjects result=new ReservedProviderObjects(context,cancel);
        RetainedJournals.scan(context,cancel::isCanceled,record->{JSONArray events=record.state().getJSONArray("events");JSONObject authority=record.authority();FixBindingIndex index=new FixBindingIndex(authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),cancel::isCanceled);int[] visits={0};
            for(int i=0;i<events.length();i++){cancel.throwIfCanceled();JSONObject event=events.getJSONObject(i);String operation=event.getString("operation"),step=step(operation);
                if(authority.has("album_protocol")&&event.getString("phase").equals("intent")&&step.matches("album-step-[0-9]{1,4}")){
                    JSONObject detail=event.getJSONObject("detail");String role=detail.getString("role");if(!detail.getString("action").equals("create")||!role.matches("copy-stage|move-stage"))continue;
                    Uri parent=checked(detail.getString("parent"));String name=detail.getString("name");ProviderNames.require(name);Uri object=null;
                    if(i+1<events.length()){JSONObject next=events.getJSONObject(i+1);if(operation.equals(next.getString("operation"))&&"returned".equals(next.getString("phase"))&&!next.getJSONObject("detail").isNull("uri"))object=returned(next.getJSONObject("detail").getString("uri"),parent);}
                    boolean released=object!=null&&albumRelease(events,i,object.toString(),parent.toString(),name,role,cancel::isCanceled,visits);
                    if(result.scopes.size()>=4000)throw new IOException("RESERVED_SCOPE_LIMIT");result.scopes.add(new Scope(object,parent,name,released));continue;
                }
                if(!event.getString("phase").equals("intent")||!step.matches("create-stage|create-move-stage|create-fix-stage|create-fix-backup"))continue;
                JSONObject detail=event.getJSONObject("detail");Uri parent=checked(detail.getString("parent"));String name=detail.getString("name");ProviderNames.require(name);Uri object=null;
                if(i+1<events.length()){JSONObject next=events.getJSONObject(i+1);if(operation.equals(next.getString("operation"))&&"returned".equals(next.getString("phase"))&&!next.getJSONObject("detail").isNull("uri"))object=returned(next.getJSONObject("detail").getString("uri"),parent);}
                boolean released=object!=null&&!step.equals("create-fix-backup")&&temporaryRelease(events,i,object.toString(),parent.toString(),name,step,authority,index,cancel::isCanceled,visits);
                if(result.scopes.size()>=4000)throw new IOException("RESERVED_SCOPE_LIMIT");result.scopes.add(new Scope(object,parent,name,released));
            }});return result;
    }
    /** File mutation and directory insertion both refuse equal/below every retained scope. */
    void require(Uri candidate)throws Exception{require(candidate,false);}
    void requireSubtree(Uri candidate)throws Exception{require(candidate,true);}
    private void require(Uri candidate,boolean subtree)throws Exception{
        cancel.throwIfCanceled();checked(candidate.toString());for(Scope scope:scopes){cancel.throwIfCanceled();if(++checks>20000)throw new IOException("RESERVED_CHECK_BUDGET");Uri reserved=scope.object;
            if(reserved!=null){int observedKind=kind(reserved);boolean absent=observedKind!=1&&observedKind!=2&&scope.temporaryReleased&&parentListsAbsent(scope);int policy=ReservedScopePolicy.select(observedKind,scope.temporaryReleased,absent);if(policy==0)continue;if(policy==1)reserved=scope.parent;}
            else reserved=scope.parent;
            // Missing or unobservable backup/lost-return namespaces retain their containing parent.
            ProviderReferences.requireGrant(context,tree(reserved),false);ReservedScopePolicy.requireDirectory(kind(reserved));
            String candidateId=DocumentsContract.getDocumentId(candidate),reservedId=DocumentsContract.getDocumentId(reserved);
            if(candidateId.equals(reservedId)||CheckedLocalAncestry.isChild(context,reserved,candidate)||(subtree&&CheckedLocalAncestry.isChild(context,candidate,reserved)))throw new IOException("RETAINED_PROVIDER_OBJECT");
        }cancel.throwIfCanceled();
    }
    private boolean parentListsAbsent(Scope scope)throws Exception{
        cancel.throwIfCanceled();ProviderReferences.requireGrant(context,tree(scope.parent),false);if(kind(scope.parent)!=2)return false;RetainedNamespaceListing listing=new RetainedNamespaceListing(DocumentsContract.getDocumentId(scope.object),scope.name);
        try(Cursor cursor=context.getContentResolver().query(DocumentsContract.buildChildDocumentsUriUsingTree(scope.parent,DocumentsContract.getDocumentId(scope.parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
            if(cursor==null)return false;while(cursor.moveToNext()){cancel.throwIfCanceled();listing.child(cursor.getString(0),cursor.getString(1));}cancel.throwIfCanceled();return listing.absent(cursor.getExtras()!=null&&cursor.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false),cursor.getExtras()!=null&&cursor.getExtras().containsKey(DocumentsContract.EXTRA_ERROR));
        }catch(android.os.OperationCanceledException failure){throw failure;}catch(Exception unavailable){cancel.throwIfCanceled();return false;}
    }
    private int kind(Uri uri)throws Exception{
        cancel.throwIfCanceled();ProviderReferences.requireGrant(context,tree(uri),false);
        try(Cursor cursor=context.getContentResolver().query(uri,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
            if(cursor==null)return -1;if(!cursor.moveToFirst())return 0;if(!DocumentsContract.getDocumentId(uri).equals(cursor.getString(0)))return -1;String mime=cursor.getString(1);if(cursor.moveToNext()||mime==null)return -1;return DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)?2:1;
        }catch(android.os.OperationCanceledException failure){throw failure;}catch(Exception unavailable){cancel.throwIfCanceled();return -1;}
    }
    private static Uri returned(String raw,Uri parent)throws Exception{Uri value=Uri.parse(raw);if(raw.length()>8192||!"content".equals(value.getScheme())||!ProviderReferences.AUTHORITY.equals(value.getAuthority())||value.getQuery()!=null||value.getFragment()!=null)throw new IOException("RESERVED_RETURN_SHAPE");return checked(DocumentsContract.buildDocumentUriUsingTree(parent,DocumentsContract.getDocumentId(value)).toString());}
    private static Uri checked(String value)throws Exception{if(value.length()>8192)throw new IOException("RESERVED_URI_BOUND");Uri uri=Uri.parse(value);if(!"content".equals(uri.getScheme())||!ProviderReferences.AUTHORITY.equals(uri.getAuthority())||!DocumentsContract.isTreeUri(uri)||uri.getQuery()!=null||uri.getFragment()!=null)throw new IOException("RESERVED_URI_SHAPE");DocumentsContract.getDocumentId(uri);return uri;}
    private static Uri tree(Uri uri){return DocumentsContract.buildTreeDocumentUri(ProviderReferences.AUTHORITY,DocumentsContract.getTreeDocumentId(uri));}
    private static String step(String operation){int slash=operation.indexOf('/');return slash<0?"":operation.substring(slash+1);}
    static boolean temporaryName(String step,String name){
        String prefix,suffix;if(step.equals("create-stage")){prefix=".singles-stage-";suffix=".bin";}else if(step.equals("create-move-stage")){prefix=".singles-move-";suffix="";}else if(step.equals("create-fix-stage")){prefix=".singles-fix-";suffix="";}else return false;
        return name.startsWith(prefix)&&name.endsWith(suffix)&&name.length()==prefix.length()+36+suffix.length()&&JournalExportBounds.id(name.substring(prefix.length(),prefix.length()+36));
    }
    static boolean temporaryRelease(JSONArray events,int created,String uri,String parent,String name,String createStep,JSONObject authority)throws Exception{
        return temporaryRelease(events,created,uri,parent,name,createStep,authority,new FixBindingIndex(authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),()->false),()->false,new int[]{0});
    }
    private static boolean temporaryRelease(JSONArray events,int created,String uri,String parent,String name,String createStep,JSONObject authority,FixBindingIndex index,java.util.function.BooleanSupplier cancelled,int[] visits)throws Exception{
        if(!temporaryName(createStep,name))return false;if(created+2>=events.length())return false;JSONObject made=events.getJSONObject(created),returned=events.getJSONObject(created+1),observed=events.getJSONObject(created+2);
        if(!sameEvent(made,returned,"returned")||!sameEvent(made,observed,"observed"))return false;JSONObject createdObservation=observed.getJSONObject("detail");if(!uri.equals(createdObservation.getString("uri"))||!uri.equals(returned(returned.getJSONObject("detail").getString("uri"),Uri.parse(parent)).toString())||!name.equals(createdObservation.getJSONObject("metadata").getString("name"))||!DocumentsContract.getDocumentId(Uri.parse(uri)).equals(createdObservation.getJSONObject("metadata").getString("id"))||(!createStep.equals("create-stage"))!=DocumentsContract.Document.MIME_TYPE_DIR.equals(createdObservation.getJSONObject("metadata").getString("mime")))return false;
        String operation=events.getJSONObject(created).getString("operation").split("/")[0];JSONObject copyBinding=null;
        if(createStep.equals("create-stage")){
            if(created+4>=events.length())return false;int number=Integer.parseInt(operation.substring(10));JSONObject source=authority.getJSONObject("original_plan").getJSONArray("operations").getJSONObject(number).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("source").getJSONObject("reference");copyBinding=index.binding(source);
            JSONObject writeIntent=events.getJSONObject(created+3),writeReturned=events.getJSONObject(created+4);
            if(!(operation+"/write-stage").equals(writeIntent.getString("operation"))||!"intent".equals(writeIntent.getString("phase"))||!sameEvent(writeIntent,writeReturned,"returned")||!uri.equals(writeIntent.getJSONObject("detail").optString("stage")))return false;
            JSONObject intendedBytes=FixProtocol.binding(writeIntent.getJSONObject("detail").getJSONObject("binding"),FixProtocol.ORIGINAL_LIMIT),returnedBytes=FixProtocol.binding(writeReturned.getJSONObject("detail"),FixProtocol.ORIGINAL_LIMIT);
            if(!FixProtocol.sameBinding(copyBinding,intendedBytes)||!FixProtocol.sameBinding(copyBinding,returnedBytes))return false;
        }
        for(int i=created+1;i<events.length();i++){JournalExportBounds.checkpoint(cancelled);if(++visits[0]>20000)throw new IOException("RESERVED_CERTIFICATE_BUDGET");JSONObject event=events.getJSONObject(i);if(!event.getString("operation").equals(operation)&&!event.getString("operation").startsWith(operation+"/"))break;if(event.getString("operation").equals(operation))continue;String step=step(event.getString("operation"));JSONObject detail=event.getJSONObject("detail");
            if("intent".equals(event.getString("phase"))&&step.matches("remove-empty-stage|remove-empty-fix-stage")&&uri.equals(detail.optString("owned_empty_directory"))&&parent.equals(detail.optString("parent"))&&name.equals(detail.optString("name"))&&i+2<events.length()){
                JSONObject effectReturn=events.getJSONObject(i+1),effectObserved=events.getJSONObject(i+2);if(sameEvent(event,effectReturn,"returned")&&sameEvent(event,effectObserved,"observed")&&Boolean.TRUE.equals(effectReturn.getJSONObject("detail").opt("removed"))&&Boolean.TRUE.equals(effectObserved.getJSONObject("detail").opt("owned_empty_stage_absent")))return true;}
            if(createStep.equals("create-stage")&&"intent".equals(event.getString("phase"))&&step.equals("publish-copy")&&uri.equals(detail.optString("source"))&&parent.equals(detail.optString("parent"))&&i+2<events.length()){
                if(i!=created+5)return false;
                JSONObject effectReturn=events.getJSONObject(i+1),effectObserved=events.getJSONObject(i+2);if(!sameEvent(event,effectReturn,"returned")||!sameEvent(event,effectObserved,"observed"))continue;JSONObject observation=effectObserved.getJSONObject("detail");String published=observation.getString("uri");if(!published.equals(effectReturn.getJSONObject("detail").getString("uri"))||!detail.getString("name").equals(observation.getJSONObject("metadata").getString("name")))continue;
                JSONObject binding=copyBinding;
                for(int n=i+3;n<events.length();n++){JournalExportBounds.checkpoint(cancelled);if(++visits[0]>20000)throw new IOException("RESERVED_CERTIFICATE_BUDGET");JSONObject verified=events.getJSONObject(n);if(operation.equals(verified.getString("operation"))&&"verified".equals(verified.getString("phase"))){JSONObject result=verified.getJSONObject("detail");return published.equals(result.getString("destination"))&&detail.getString("name").equals(result.getString("name"))&&Boolean.TRUE.equals(result.get("copy"))&&binding.getLong("bytes")==result.getLong("bytes")&&binding.getString("sha256").equals(result.getString("sha256"));}}
            }
        }return false;
    }
    private static boolean albumRelease(JSONArray events,int created,String uri,String parent,String name,String role,java.util.function.BooleanSupplier cancelled,int[] visits)throws Exception{
        JSONObject creation=events.getJSONObject(created);String operation=creation.getString("operation").split("/")[0];int child=creation.getJSONObject("detail").getInt("child");
        if(created+2>=events.length())return false;JSONObject made=events.getJSONObject(created+2);if(!sameEvent(creation,made,"observed")||!uri.equals(made.getJSONObject("detail").getString("uri")))return false;
        boolean released=false;
        for(int n=created+3;n<events.length();n++){JournalExportBounds.checkpoint(cancelled);if(++visits[0]>20000)throw new IOException("RESERVED_CERTIFICATE_BUDGET");JSONObject event=events.getJSONObject(n);String current=event.getString("operation");if(!current.equals(operation)&&!current.startsWith(operation+"/"))break;JSONObject detail=event.getJSONObject("detail");
            if(event.getString("phase").equals("intent")&&detail.optInt("child",-1)==child&&n+2<events.length()){
                boolean effect=role.equals("copy-stage")&&"rename".equals(detail.optString("action"))&&"publish-copy".equals(detail.optString("role"))&&uri.equals(detail.optString("source"))&&parent.equals(detail.optString("parent"));
                effect|=role.equals("move-stage")&&"remove-empty".equals(detail.optString("action"))&&"move-stage".equals(detail.optString("role"))&&uri.equals(detail.optString("owned_empty_directory"))&&parent.equals(detail.optString("parent"))&&name.equals(detail.optString("name"));
                if(effect&&sameEvent(event,events.getJSONObject(n+1),"returned")&&sameEvent(event,events.getJSONObject(n+2),"observed"))released=true;
            }
            if(event.getString("phase").equals("album-child-verified")&&detail.getInt("child")==child)return released;
        }return false;
    }
    private static boolean sameEvent(JSONObject first,JSONObject next,String phase)throws Exception{return first.getString("operation").equals(next.getString("operation"))&&phase.equals(next.getString("phase"));}
}
