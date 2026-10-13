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
import java.io.FileOutputStream;
import java.io.IOException;
import java.security.MessageDigest;
import java.util.HashSet;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** Local-provider effects under explicit quiescence assumptions, never Windows CAS semantics.
 * No rollback deletion. Every changed/uncertain URI remains in the owned journal.
 * Not exposed to JS and not enabled until the original-plan execution owner authorizes it.
 */
final class LocalProviderEffects {
    interface PublishedDestination {void observed(JSONObject evidence)throws Exception;}
    private final ReservedProviderObjects reservations;
    private final PublishedDestination publishedDestination;private boolean destinationObserved;
    private static final String DIRECTORY=DocumentsContract.Document.MIME_TYPE_DIR;
    private static final long FILE_LIMIT=64L*1024*1024;
    private final Context context;private final ContentResolver resolver;private final ProviderReferences references;
    private final ProviderEffectJournal journal;private final CancellationSignal cancel;private final TargetObservationGuard targetGuard;
    LocalProviderEffects(Context context,ProviderReferences references,ProviderEffectJournal journal,CancellationSignal cancel,TargetObservationGuard targetGuard,PublishedDestination publishedDestination)throws Exception{
        this.reservations=ReservedProviderObjects.load(context,cancel);this.context=context.getApplicationContext();resolver=this.context.getContentResolver();this.references=references;this.journal=journal;this.cancel=cancel;this.targetGuard=targetGuard;this.publishedDestination=publishedDestination;
    }
    private void checkpoint(){cancel.throwIfCanceled();}
    private static JSONObject data(String key,Object value)throws Exception{return new JSONObject().put(key,value);}
    private void record(String phase,String step,JSONObject details)throws Exception{journal.record(phase,step,details);}
    private void requireTarget()throws Exception{checkpoint();references.requireWritable(references.targetTree);}
    private JSONObject info(Uri uri)throws Exception{
        checkpoint();try(Cursor c=resolver.query(uri,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
            if(c==null||!c.moveToFirst())throw new IOException("DOCUMENT_UNAVAILABLE");
            JSONObject out=new JSONObject().put("id",c.getString(0)).put("name",c.getString(1)).put("mime",c.getString(2));
            if(c.moveToNext())throw new IOException("AMBIGUOUS_DOCUMENT");return out;
        }
    }
    private Uri targetScoped(Uri returned)throws Exception{
        if(returned==null||!"content".equals(returned.getScheme())||!ProviderReferences.AUTHORITY.equals(returned.getAuthority())||!DocumentsContract.isDocumentUri(context,returned))throw new IOException("RETURNED_REFERENCE_INVALID");
        return DocumentsContract.buildDocumentUriUsingTree(references.targetTree,DocumentsContract.getDocumentId(returned));
    }
    private Uri find(Uri parent,String name)throws Exception{
        checkpoint();String id=DocumentsContract.getDocumentId(parent);Uri children=DocumentsContract.buildChildDocumentsUriUsingTree(parent,id);Uri match=null;HashSet<String> ids=new HashSet<>();int count=0;
        try(Cursor c=resolver.query(children,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
            if(c==null)throw new IOException("LIST_UNAVAILABLE");
            while(c.moveToNext()){
                checkpoint();String childId=c.getString(0),childName=c.getString(1);
                if(++count>10000||childId==null||childName==null||!ids.add(childId))throw new IOException("LIST_BOUND_OR_AMBIGUOUS");
                if(childName.equalsIgnoreCase(name)){
                    if(!childName.equals(name)||match!=null)throw new IOException("CASE_OR_NAME_COLLISION");
                    match=DocumentsContract.buildDocumentUriUsingTree(parent,childId);
                    if(!DocumentsContract.isChildDocument(resolver,parent,match))throw new IOException("CHILD_SCOPE");
                }
            }
            if(c.getExtras()!=null&&(c.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||c.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("LIST_INCOMPLETE");
        }
        return match;
    }
    private Uri create(Uri parent,String name,String mime,String step)throws Exception{
        ProviderNames.require(name);requireTarget();reservations.require(parent);targetGuard.require(parent);if(find(parent,name)!=null)throw new IOException("DESTINATION_OCCUPIED");
        JSONObject intent=new JSONObject().put("parent",parent.toString()).put("name",name).put("mime",mime);record("intent",step,intent);checkpoint();
        Uri made=DocumentsContract.createDocument(resolver,parent,mime,name);
        record("returned",step,data("uri",made==null?JSONObject.NULL:made.toString()));Uri scoped=targetScoped(made);
        JSONObject observed=info(scoped);record("observed",step,new JSONObject().put("uri",scoped.toString()).put("metadata",observed));
        if(!DocumentsContract.isChildDocument(resolver,parent,scoped)||!name.equals(observed.getString("name"))||DIRECTORY.equals(mime)!=DIRECTORY.equals(observed.getString("mime")))throw new IOException("CREATE_EFFECT_MISMATCH");
        targetGuard.created(parent,scoped);return scoped;
    }
    private Uri destinationParent(JSONArray components,String step,boolean duetCopy)throws Exception{
        if(components.length()<1||components.length()>128)throw new IOException("DESTINATION_LIMIT");
        Uri parent=references.targetRoot();
        for(int i=0;i<components.length()-1;i++){
            String name=components.getString(i);ProviderNames.require(name);requireTarget();Uri child=find(parent,name);
            if(child==null)child=create(parent,name,DIRECTORY,step+"/directory-"+i);
            targetGuard.require(child);if(!DIRECTORY.equals(info(child).getString("mime")))throw new IOException("DESTINATION_NOT_DIRECTORY");if(duetCopy)record("duet-parent-observed",step,new JSONObject().put("component",i).put("parent",parent.toString()).put("uri",child.toString()).put("name",name));parent=child;
        }
        ProviderNames.require(components.getString(components.length()-1));return parent;
    }
    private String hash(Uri uri,long expectedBytes)throws Exception{
        if(expectedBytes<0||expectedBytes>FILE_LIMIT)throw new IOException("SOURCE_BYTE_BOUND");checkpoint();MessageDigest digest=MessageDigest.getInstance("SHA-256");long count=0;
        try(ParcelFileDescriptor descriptor=resolver.openFileDescriptor(uri,"r",cancel)){
            if(descriptor==null||!OsConstants.S_ISREG(Os.fstat(descriptor.getFileDescriptor()).st_mode))throw new IOException("REGULAR_SOURCE_REQUIRED");
            try(ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(descriptor.getFileDescriptor()))){
                byte[] buffer=new byte[65536];int n;while((n=input.read(buffer))!=-1){checkpoint();count+=n;if(count>expectedBytes)throw new IOException("SOURCE_SIZE_CHANGED");digest.update(buffer,0,n);}
            }
        }
        if(count!=expectedBytes)throw new IOException("SOURCE_SIZE_CHANGED");return hex(digest.digest());
    }
    private void verify(Uri uri,JSONObject binding)throws Exception{
        if(!hash(uri,binding.getLong("bytes")).equals(binding.getString("sha256")))throw new IOException("SOURCE_CONTENT_CHANGED");
    }
    private Uri rename(Uri source,Uri parent,String name,String step)throws Exception{
        requireTarget();reservations.require(parent);reservations.require(source);targetGuard.require(parent);ProviderNames.require(name);if(find(parent,name)!=null)throw new IOException("DESTINATION_OCCUPIED");
        record("intent",step,new JSONObject().put("source",source.toString()).put("parent",parent.toString()).put("name",name));checkpoint();
        Uri returned=DocumentsContract.renameDocument(resolver,source,name);record("returned",step,data("uri",returned==null?JSONObject.NULL:returned.toString()));Uri scoped=targetScoped(returned);
        JSONObject observed=info(scoped);record("observed",step,new JSONObject().put("uri",scoped.toString()).put("metadata",observed));
        if(!DocumentsContract.isChildDocument(resolver,parent,scoped)||!name.equals(observed.getString("name")))throw new IOException("RENAME_EFFECT_MISMATCH");targetGuard.changed(source,scoped,parent,parent);return scoped;
    }
    private Uri move(Uri source,Uri from,Uri to,String expectedName,String step)throws Exception{
        requireTarget();reservations.require(source);reservations.require(to);targetGuard.require(to);if(find(to,expectedName)!=null)throw new IOException("DESTINATION_OCCUPIED");
        record("intent",step,new JSONObject().put("source",source.toString()).put("from",from.toString()).put("to",to.toString()).put("name",expectedName));checkpoint();
        Uri returned=DocumentsContract.moveDocument(resolver,source,from,to);record("returned",step,data("uri",returned==null?JSONObject.NULL:returned.toString()));Uri scoped=targetScoped(returned);
        JSONObject observed=info(scoped);record("observed",step,new JSONObject().put("uri",scoped.toString()).put("metadata",observed));
        if(!DocumentsContract.isChildDocument(resolver,to,scoped)||!expectedName.equals(observed.getString("name")))throw new IOException("MOVE_EFFECT_MISMATCH");targetGuard.changed(source,scoped,from,to);return scoped;
    }
    private void removeOwnedEmptyStage(Uri directory,Uri parent,String ownedName,String step)throws Exception{
        requireTarget();targetGuard.require(directory);targetGuard.require(parent);
        JSONObject observed=info(directory);
        if(!ownedName.equals(observed.getString("name"))||!DIRECTORY.equals(observed.getString("mime"))||!DocumentsContract.isChildDocument(resolver,parent,directory))throw new IOException("STAGE_CLEANUP_SCOPE_CHANGED");
        Uri children=DocumentsContract.buildChildDocumentsUriUsingTree(references.targetTree,DocumentsContract.getDocumentId(directory));
        try(Cursor c=resolver.query(children,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID},null,null,null,cancel)){
            if(c==null||c.moveToFirst()||c.getExtras()!=null&&(c.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||c.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("STAGE_NOT_OBSERVED_EMPTY");
        }
        record("intent",step,new JSONObject().put("owned_empty_directory",directory.toString()).put("parent",parent.toString()).put("name",ownedName));checkpoint();
        boolean removed=DocumentsContract.deleteDocument(resolver,directory);record("returned",step,data("removed",removed));
        if(!removed||find(parent,ownedName)!=null)throw new IOException("STAGE_CLEANUP_UNCERTAIN");
        targetGuard.removedEmpty(directory,parent);targetGuard.validateTree();record("observed",step,data("owned_empty_stage_absent",true));
    }
    private void observeDestination(Uri published,JSONObject binding,String name,boolean copy,String operation)throws Exception{
        verify(published,binding);
        if(!destinationObserved){
            JSONObject evidence=new JSONObject().put("destination",published.toString()).put("name",name).put("bytes",binding.getLong("bytes")).put("sha256",binding.getString("sha256")).put("copy",copy);
            record("destination-observed",operation,evidence);publishedDestination.observed(evidence);destinationObserved=true;
        }
    }
    JSONObject transfer(JSONObject sourceReference,JSONArray destination,boolean copy,String operation)throws Exception{return transfer(sourceReference,destination,copy,operation,false);}
    JSONObject transfer(JSONObject sourceReference,JSONArray destination,boolean copy,String operation,boolean duetCopy)throws Exception{
        destinationObserved=false;checkpoint();references.requireDisjointRoots();targetGuard.validateTree();Uri source=references.uri(sourceReference),sourceParent=references.parent(sourceReference);JSONObject expected=references.resolve(sourceReference),binding=references.binding(sourceReference);
        if(!expected.getString("name").equals(info(source).getString("name"))||!DocumentsContract.isChildDocument(resolver,sourceParent,source))throw new IOException("SOURCE_OBSERVATION_CHANGED");
        reservations.require(source);verify(source,binding);
        if(duetCopy){if(!copy)throw new IOException("DUET_COPY_REQUIRED");record("duet-copy-start",operation,new JSONObject().put("source",source.toString()).put("parent",sourceParent.toString()).put("name",expected.getString("name")).put("binding",binding).put("target",references.targetRoot().toString()).put("components",destination));}
        Uri parent=destinationParent(destination,operation,duetCopy);reservations.require(parent);String finalName=destination.getString(destination.length()-1);
        if(find(parent,finalName)!=null)throw new IOException("DESTINATION_OCCUPIED");Uri published;
        if(copy){
            Uri stage=create(parent,".singles-stage-"+UUID.randomUUID()+".bin","application/octet-stream",operation+"/create-stage");
            record("intent",operation+"/write-stage",new JSONObject().put("source",source.toString()).put("stage",stage.toString()).put("binding",binding));checkpoint();
            MessageDigest digest=MessageDigest.getInstance("SHA-256");long count=0;
            try(ParcelFileDescriptor inputFd=resolver.openFileDescriptor(source,"r",cancel);ParcelFileDescriptor outputFd=resolver.openFileDescriptor(stage,"rw",cancel)){
                if(inputFd==null||outputFd==null||!OsConstants.S_ISREG(Os.fstat(inputFd.getFileDescriptor()).st_mode)||!OsConstants.S_ISREG(Os.fstat(outputFd.getFileDescriptor()).st_mode))throw new IOException("REGULAR_HANDLES_REQUIRED");
                try(ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(inputFd.getFileDescriptor()));FileOutputStream output=new ParcelFileDescriptor.AutoCloseOutputStream(ParcelFileDescriptor.dup(outputFd.getFileDescriptor()))){
                    byte[] buffer=new byte[65536];int n;while((n=input.read(buffer))!=-1){checkpoint();count+=n;if(count>binding.getLong("bytes")||count>FILE_LIMIT)throw new IOException("COPY_SOURCE_CHANGED");output.write(buffer,0,n);digest.update(buffer,0,n);}output.flush();output.getFD().sync();
                }
            }
            record("returned",operation+"/write-stage",new JSONObject().put("bytes",count).put("sha256",hex(digest.digest())));
            verify(stage,binding);targetGuard.refresh(stage);published=rename(stage,parent,finalName,operation+"/publish-copy");
        }else{
            String originalName=expected.getString("name");
            if(originalName.equals(finalName)){
                references.requireWritable(references.sourceTree);verify(source,binding);published=move(source,sourceParent,parent,finalName,operation+"/move");
            }else{
                // Preserve Move semantics without copy/delete substitution when the original plan renames.
                String ownedStageName=".singles-move-"+UUID.randomUUID();
                Uri stageDirectory=create(parent,ownedStageName,DIRECTORY,operation+"/create-move-stage");
                references.requireWritable(references.sourceTree);verify(source,binding);Uri staged=move(source,sourceParent,stageDirectory,originalName,operation+"/move-stage");
                verify(staged,binding);Uri renamed=rename(staged,stageDirectory,finalName,operation+"/rename-stage");verify(renamed,binding);
                published=move(renamed,stageDirectory,parent,finalName,operation+"/publish-move");
                observeDestination(published,binding,finalName,copy,operation);removeOwnedEmptyStage(stageDirectory,parent,ownedStageName,operation+"/remove-empty-stage");
            }
        }
        observeDestination(published,binding,finalName,copy,operation);
        if(copy){verify(source,binding);}else if(find(sourceParent,expected.getString("name"))!=null){throw new IOException("MOVE_SOURCE_STILL_PRESENT");}
        JSONObject result=new JSONObject().put("destination",published.toString()).put("name",finalName).put("bytes",binding.getLong("bytes")).put("sha256",binding.getString("sha256")).put("copy",copy).put("source_retained",copy).put("source_absent",!copy);
        record("verified",operation,result);return result;
    }
    /** Fresh read-only cohort observations; only the live owner can call the retirement port. */
    DuetRetirement.Port duetPort(java.util.Map<Integer,NativeTransferPlan.Transfer> transfers,TargetObservationGuard sourceGuard){
        return new DuetRetirement.Port(){
            public void checkpoint(){LocalProviderEffects.this.checkpoint();}
            public JSONObject source(NativeDuetPlan.Group group)throws Exception{return observeDuetSource(group,sourceGuard);}
            public void destination(int id,JSONObject receipt)throws Exception{
                NativeTransferPlan.Transfer transfer=transfers.get(id);if(transfer==null||!transfer.copy)throw new IOException("DUET_REQUIRED_COPY_UNVERIFIED");
                references.requireReadable(references.targetTree);references.requireWritable(references.targetTree);references.requireDisjointRoots();targetGuard.validateTree();
                JSONArray components=transfer.components();Uri parent=references.targetRoot(),found=null;
                for(int n=0;n<components.length();n++){
                    checkpoint();String name=components.getString(n);ProviderNames.require(name);found=find(parent,name);
                    if(found==null||!CheckedLocalAncestry.isChild(context,parent,found))throw new IOException("DUET_DESTINATION_MISSING");
                    JSONObject actual=info(found);if(!DocumentsContract.getDocumentId(found).equals(actual.getString("id"))||!name.equals(actual.getString("name"))||(n<components.length()-1)!=DIRECTORY.equals(actual.getString("mime")))throw new IOException("DUET_DESTINATION_CHANGED");
                    targetGuard.require(found);if(n<components.length()-1)parent=found;
                }
                JSONObject binding=references.binding(transfer.source());String name=components.getString(components.length()-1);
                if(found==null||!found.toString().equals(receipt.getString("destination"))||!name.equals(receipt.getString("name"))||!FixProtocol.sameBinding(binding,receipt))throw new IOException("DUET_COPY_RECEIPT_MISMATCH");
                verify(found,binding);targetGuard.require(found);
            }
            public boolean remove(NativeDuetPlan.Group group)throws Exception{
                references.requireWritable(references.sourceTree);references.requireWritable(references.targetTree);references.requireDisjointRoots();
                // This final fresh source check occurs after the durable retirement intent.
                targetGuard.validateTree();observeDuetSource(group,sourceGuard);checkpoint();return DocumentsContract.deleteDocument(resolver,references.uri(group.source().getJSONObject("reference")));
            }
            public void absent(NativeDuetPlan.Group group)throws Exception{
                JSONObject observation=group.source(),ref=observation.getJSONObject("reference");Uri source=references.uri(ref),parent=references.parent(ref);checkpoint();
                RetainedNamespaceListing listing=new RetainedNamespaceListing(DocumentsContract.getDocumentId(source),observation.getString("name"));
                try(Cursor c=resolver.query(DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
                    if(c==null)throw new IOException("DUET_SOURCE_ABSENCE_UNCERTAIN");while(c.moveToNext()){checkpoint();listing.child(c.getString(0),c.getString(1));}
                    if(!listing.absent(c.getExtras()!=null&&c.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false),c.getExtras()!=null&&c.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("DUET_SOURCE_ABSENCE_UNCERTAIN");
                }
                sourceGuard.removedSubtree(source,parent);sourceGuard.validateTree();targetGuard.validateTree();checkpoint();
            }
        };
    }
    private JSONObject observeDuetSource(NativeDuetPlan.Group group,TargetObservationGuard sourceGuard)throws Exception{
        checkpoint();references.requireReadable(references.sourceTree);references.requireDisjointRoots();
        if(sourceGuard==null)throw new IOException("DUET_SOURCE_GUARD");sourceGuard.validateTree();
        JSONObject observation=group.source(),ref=observation.getJSONObject("reference"),binding=references.binding(ref);Uri source=references.uri(ref),parent=references.parent(ref);JSONObject actual=info(source);
        if(!DocumentsContract.getDocumentId(source).equals(actual.getString("id"))||!observation.getString("name").equals(actual.getString("name"))||DIRECTORY.equals(actual.getString("mime"))||!CheckedLocalAncestry.isChild(context,parent,source))throw new IOException("DUET_SOURCE_OBSERVATION_CHANGED");
        reservations.require(source);verify(source,binding);sourceGuard.require(source);checkpoint();
        return new JSONObject().put("source",source.toString()).put("parent",parent.toString()).put("name",observation.getString("name"))
            .put("bytes",binding.getLong("bytes")).put("sha256",binding.getString("sha256"));
    }
    private static String hex(byte[] bytes){StringBuilder out=new StringBuilder();for(byte b:bytes)out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();}
}
