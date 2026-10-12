package com.nhlocal.singlesorter.app;

import android.content.ContentResolver;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.system.Os;
import android.system.OsConstants;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.HashSet;
import java.util.UUID;
import org.json.JSONObject;

/** One original FixFile at a time, with separately observed filename/tags/retained-original outcomes. */
final class FixProviderEffects {
    interface Progress {void observed(String phase,JSONObject evidence)throws Exception;}
    private final Context context;private final ContentResolver resolver;private final ProviderReferences references;private final ProviderEffectJournal journal;
    private final CancellationSignal cancel;private final TargetObservationGuard guard;private final ReservedProviderObjects reservations;private final Progress progress;
    private static final String DIRECTORY=DocumentsContract.Document.MIME_TYPE_DIR;
    FixProviderEffects(Context context,ProviderReferences references,ProviderEffectJournal journal,CancellationSignal cancel,TargetObservationGuard guard,ReservedProviderObjects reservations,Progress progress){this.context=context.getApplicationContext();resolver=this.context.getContentResolver();this.references=references;this.journal=journal;this.cancel=cancel;this.guard=guard;this.reservations=reservations;this.progress=progress;}
    private void check()throws Exception{cancel.throwIfCanceled();references.requireWritable(references.sourceTree);}
    private void record(String phase,String operation,JSONObject detail)throws Exception{journal.record(phase,operation,detail);}
    private JSONObject info(Uri uri)throws Exception{
        check();try(Cursor cursor=resolver.query(uri,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
            if(cursor==null||!cursor.moveToFirst())throw new IOException("FIX_DOCUMENT_UNAVAILABLE");JSONObject value=new JSONObject().put("id",cursor.getString(0)).put("name",cursor.getString(1)).put("mime",cursor.getString(2));if(cursor.moveToNext()||!DocumentsContract.getDocumentId(uri).equals(value.getString("id")))throw new IOException("FIX_DOCUMENT_AMBIGUOUS");return value;}
    }
    private Uri scoped(Uri uri)throws Exception{if(uri==null||!"content".equals(uri.getScheme())||!ProviderReferences.AUTHORITY.equals(uri.getAuthority())||!DocumentsContract.isDocumentUri(context,uri)||uri.getQuery()!=null||uri.getFragment()!=null)throw new IOException("FIX_RETURNED_REFERENCE");return DocumentsContract.buildDocumentUriUsingTree(references.sourceTree,DocumentsContract.getDocumentId(uri));}
    private Uri find(Uri parent,String name)throws Exception{
        check();HashSet<String> seen=new HashSet<>();Uri result=null;try(Cursor cursor=resolver.query(DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
            if(cursor==null)throw new IOException("FIX_LIST_UNAVAILABLE");while(cursor.moveToNext()){cancel.throwIfCanceled();String id=cursor.getString(0),actual=cursor.getString(1);if(id==null||actual==null||seen.size()>=10000||!seen.add(id))throw new IOException("FIX_LIST_BOUND");if(actual.equalsIgnoreCase(name)){if(!actual.equals(name)||result!=null)throw new IOException("FIX_NAME_COLLISION");result=DocumentsContract.buildDocumentUriUsingTree(parent,id);if(!CheckedLocalAncestry.isChild(context,parent,result))throw new IOException("FIX_CHILD_SCOPE");}}
            complete(cursor);return result;}
    }
    private static void complete(Cursor cursor)throws IOException{if(cursor.getExtras()!=null&&(cursor.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||cursor.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("FIX_LIST_INCOMPLETE");}
    private void verify(Uri uri,JSONObject binding)throws Exception{check();try(ParcelFileDescriptor descriptor=resolver.openFileDescriptor(uri,"r",cancel)){
        if(descriptor==null||!OsConstants.S_ISREG(Os.fstat(descriptor.getFileDescriptor()).st_mode))throw new IOException("FIX_REGULAR_FILE");try(ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(descriptor.getFileDescriptor()))){PrivateFixStage.verify(input,binding,FixProtocol.STAGED_LIMIT,cancel);}}
    }
    private Uri checkedResult(Uri returned,Uri parent,String name,boolean directory,String operation)throws Exception{
        record("returned",operation,new JSONObject().put("uri",returned==null?JSONObject.NULL:returned.toString()));Uri result=scoped(returned);JSONObject metadata=info(result);
        if(FixEffectObservation.phase(name,directory,metadata.getString("name"),DIRECTORY.equals(metadata.getString("mime")),CheckedLocalAncestry.isChild(context,parent,result)).equals("effect-mismatch")){
            record("effect-mismatch",operation,new JSONObject().put("uri",result.toString()).put("metadata",metadata));throw new IOException("FIX_PROVIDER_EFFECT_MISMATCH");}
        record("observed",operation,new JSONObject().put("uri",result.toString()).put("metadata",metadata));return result;
    }
    private Uri create(Uri parent,String name,boolean directory,String operation)throws Exception{
        check();reservations.require(parent);guard.require(parent);ProviderNames.require(name);if(find(parent,name)!=null)throw new IOException("FIX_DESTINATION_OCCUPIED");String mime=directory?DIRECTORY:"application/octet-stream";
        record("intent",operation,new JSONObject().put("parent",parent.toString()).put("name",name).put("mime",mime));check();Uri result=checkedResult(DocumentsContract.createDocument(resolver,parent,mime,name),parent,name,directory,operation);guard.created(parent,result);return result;
    }
    private Uri rename(Uri source,Uri parent,String name,String operation)throws Exception{
        check();reservations.require(source);guard.require(source);guard.require(parent);ProviderNames.require(name);if(find(parent,name)!=null)throw new IOException("FIX_DESTINATION_OCCUPIED");
        record("intent",operation,new JSONObject().put("source",source.toString()).put("parent",parent.toString()).put("name",name));check();Uri result=checkedResult(DocumentsContract.renameDocument(resolver,source,name),parent,name,false,operation);guard.changed(source,result,parent,parent);return result;
    }
    private Uri move(Uri source,Uri from,Uri to,String name,String operation)throws Exception{
        check();reservations.require(source);reservations.require(to);guard.require(source);guard.require(from);guard.require(to);if(find(to,name)!=null)throw new IOException("FIX_DESTINATION_OCCUPIED");
        if(!CheckedLocalAncestry.isChild(context,from,source))throw new IOException("FIX_SOURCE_SCOPE");record("intent",operation,new JSONObject().put("source",source.toString()).put("from",from.toString()).put("to",to.toString()).put("name",name));if(operation.endsWith("/backup-fix"))progress.observed("fix-backup-started",new JSONObject());check();Uri result=checkedResult(DocumentsContract.moveDocument(resolver,source,from,to),to,name,false,operation);guard.changed(source,result,from,to);return result;
    }
    private void write(Uri output,PrivateFixStage stage,String operation)throws Exception{
        check();guard.require(output);record("intent",operation,new JSONObject().put("stage",output.toString()).put("binding",stage.binding));check();
        try(FileInputStream input=PrivateFixStage.open(stage.output);ParcelFileDescriptor descriptor=resolver.openFileDescriptor(output,"rw",cancel)){
            if(descriptor==null||!OsConstants.S_ISREG(Os.fstat(descriptor.getFileDescriptor()).st_mode)||Os.fstat(descriptor.getFileDescriptor()).st_size!=0)throw new IOException("FIX_EMPTY_STAGE_REQUIRED");
            try(FileOutputStream out=new ParcelFileDescriptor.AutoCloseOutputStream(ParcelFileDescriptor.dup(descriptor.getFileDescriptor()))){PrivateFixStage.copy(input,out,stage.binding,FixProtocol.STAGED_LIMIT,cancel);out.flush();out.getFD().sync();}}
        record("returned",operation,stage.binding);verify(output,stage.binding);guard.refresh(output);record("observed",operation,new JSONObject().put("uri",output.toString()).put("binding",stage.binding));
    }
    private void removeEmpty(Uri directory,Uri parent,String name,String operation)throws Exception{
        check();guard.require(directory);guard.require(parent);JSONObject metadata=info(directory);if(!name.equals(metadata.getString("name"))||!DIRECTORY.equals(metadata.getString("mime"))||!CheckedLocalAncestry.isChild(context,parent,directory))throw new IOException("FIX_EMPTY_SCOPE_CHANGED");
        try(Cursor cursor=resolver.query(DocumentsContract.buildChildDocumentsUriUsingTree(directory,DocumentsContract.getDocumentId(directory)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID},null,null,null,cancel)){if(cursor==null||cursor.moveToFirst())throw new IOException("FIX_STAGE_NOT_EMPTY");complete(cursor);}
        record("intent",operation,new JSONObject().put("owned_empty_directory",directory.toString()).put("parent",parent.toString()).put("name",name));check();boolean removed=DocumentsContract.deleteDocument(resolver,directory);record("returned",operation,new JSONObject().put("removed",removed));if(!removed||find(parent,name)!=null)throw new IOException("FIX_EMPTY_REMOVAL_UNCERTAIN");guard.removedEmpty(directory,parent);guard.validateTree();record("observed",operation,new JSONObject().put("owned_empty_stage_absent",true));
    }
    JSONObject fix(NativeFixPlan.Fix fix)throws Exception{
        String operation="operation-"+fix.id;check();Uri source=references.uri(fix.source()),parent=references.parent(fix.source());JSONObject original=fix.binding();reservations.require(source);guard.validateTree();guard.require(source);if(!fix.originalName.equals(info(source).getString("name"))||!CheckedLocalAncestry.isChild(context,parent,source))throw new IOException("FIX_SOURCE_OBSERVATION_CHANGED");verify(source,original);
        PrivateFixStage stage=null;Throwable failure=null;try{
            String outcome;Object nativeError=JSONObject.NULL,nativeEvidence=JSONObject.NULL;
            if(fix.failure()!=JSONObject.NULL)outcome="metadata_failure";
            else if(fix.repairs().length()==0)outcome="unchanged";
            else{stage=PrivateFixStage.prepare(context,source,fix,cancel);if(stage.unsupported!=null){outcome="unsupported_tags";nativeError=stage.unsupported;}else{outcome="staged";nativeEvidence=stage.evidence;}}
            JSONObject prepared=new JSONObject().put("source",source.toString()).put("parent",parent.toString()).put("original_binding",original).put("outcome",outcome).put("native_evidence",nativeEvidence).put("native_error",nativeError).put("metadata_failure",fix.failure());record("fix-prepared",operation,prepared);
            boolean renamed=false;
            if(fix.rename){verify(source,original);source=rename(source,parent,fix.intendedName,operation+"/rename-fix");verify(source,original);if(find(parent,fix.originalName)!=null)throw new IOException("FIX_OLD_NAME_STILL_PRESENT");JSONObject evidence=new JSONObject().put("uri",source.toString()).put("binding",original).put("name",fix.intendedName);record("fix-renamed",operation,evidence);renamed=true;progress.observed("fix-renamed",evidence);}
            if(!outcome.equals("staged")){verify(source,original);guard.validateTree();JSONObject result=result(source,fix,original,outcome.equals("unchanged")?"complete":outcome,renamed,false,outcome.equals("unchanged"),null,0,nativeError);if(stage!=null){stage.close();stage=null;}record("verified",operation,result);return result;}
            String stageName=".singles-fix-"+UUID.randomUUID(),backupName=".singles-original-"+UUID.randomUUID();Uri directory=create(parent,stageName,true,operation+"/create-fix-stage"),output=create(directory,fix.intendedName,false,operation+"/create-fix-output");write(output,stage,operation+"/write-fix-stage");Uri backupDirectory=create(parent,backupName,true,operation+"/create-fix-backup");
            verify(source,original);Uri moved=move(source,parent,backupDirectory,fix.intendedName,operation+"/backup-fix");verify(moved,original);if(find(parent,fix.intendedName)!=null)throw new IOException("FIX_BACKUP_SOURCE_PRESENT");JSONObject backup=new JSONObject().put("uri",moved.toString()).put("binding",original).put("original_name",fix.originalName).put("intended_name",fix.intendedName);record("fix-backup-observed",operation,backup);progress.observed("fix-backup-observed",backup);
            Uri retained=rename(moved,backupDirectory,FixProtocol.BACKUP_LEAF,operation+"/rename-fix-backup");verify(retained,original);if(find(backupDirectory,fix.intendedName)!=null)throw new IOException("FIX_BACKUP_OLD_NAME_PRESENT");backup=new JSONObject(backup.toString()).put("uri",retained.toString()).put("name",FixProtocol.BACKUP_LEAF);record("fix-backup-retained",operation,backup);progress.observed("fix-backup-retained",backup);
            verify(output,stage.binding);verify(retained,original);Uri published=move(output,directory,parent,fix.intendedName,operation+"/publish-fix");verify(published,stage.binding);JSONObject result=result(published,fix,stage.binding,"complete",renamed,true,false,retained,original.getLong("bytes"),JSONObject.NULL);record("destination-observed",operation,result);progress.observed("destination-observed",result);
            removeEmpty(directory,parent,stageName,operation+"/remove-empty-fix-stage");verify(retained,original);verify(published,stage.binding);guard.validateTree();if(stage!=null){stage.close();stage=null;}record("verified",operation,result);return result;
        }catch(Exception|Error error){failure=error;throw error;}finally{if(stage!=null)try{stage.close();}catch(Exception cleanup){if(failure!=null)failure.addSuppressed(cleanup);else throw cleanup;}}
    }
    private JSONObject result(Uri destination,NativeFixPlan.Fix fix,JSONObject binding,String status,boolean renamed,boolean repaired,boolean unchanged,Uri backup,long backupBytes,Object nativeError)throws Exception{
        return new JSONObject().put("destination",destination.toString()).put("name",fix.intendedName).put("bytes",binding.getLong("bytes")).put("sha256",binding.getString("sha256")).put("copy",false).put("status",status).put("renamed",renamed).put("repaired",repaired).put("unchanged",unchanged).put("backup",backup==null?JSONObject.NULL:backup.toString()).put("backup_bytes",backupBytes).put("metadata_failure",fix.failure()).put("native_error",nativeError);
    }
}
