package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.content.ContentResolver;
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
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** Exact retained Album effects on the fixed local provider in the declared no-other-writer interval.
 * Every incomplete effect retains intent and returned references. No rollback or foreign deletion.
 */
final class AlbumProviderEffects {
    interface Progress {void update(JSONObject children)throws Exception;}
    private static final String DIRECTORY=DocumentsContract.Document.MIME_TYPE_DIR;
    private final Context context;private final ContentResolver resolver;private final ProviderReferences references;private final NativeAlbumPlan plan;
    private final ProviderEffectJournal journal;private final CancellationSignal cancel;private final TargetObservationGuard sourceGuard,targetGuard;private final ReservedProviderObjects reserved;private final Progress progress;
    private NativeAlbumPlan.Album album;private NativeAlbumPlan.Child child;private int step,createdDirectories,removedDirectories;private Uri albumDestination;private boolean selectedRootRemoved;private final JSONArray outcomes=new JSONArray();
    AlbumProviderEffects(Context context,ProviderReferences references,NativeAlbumPlan plan,ProviderEffectJournal journal,CancellationSignal cancel,TargetObservationGuard sourceGuard,TargetObservationGuard targetGuard,Progress progress)throws Exception{
        this.context=context.getApplicationContext();resolver=this.context.getContentResolver();this.references=references;this.plan=plan;this.journal=journal;this.cancel=cancel;this.sourceGuard=sourceGuard;this.targetGuard=targetGuard;this.progress=progress;reserved=ReservedProviderObjects.load(context,cancel);
    }
    private void check()throws Exception{cancel.throwIfCanceled();if(selectedRootRemoved){AlbumRootParent.requireAfter(context,references.rootParentEvidence(),cancel);references.requireWritable(references.targetTree);}else{references.requireDisjointRoots();sourceGuard.validateTree();}targetGuard.validateTree();}
    private String operation(){return "operation-"+album.id;}
    private void root(String phase,JSONObject data)throws Exception{journal.record(phase,operation(),data);}
    private String intent(String action,String role,JSONObject details)throws Exception{
        details.put("action",action).put("role",role).put("child",child==null?-1:child.index);String name=operation()+"/album-step-"+step++;journal.record("intent",name,details);cancel.throwIfCanceled();return name;
    }
    private void returned(String step,JSONObject details)throws Exception{journal.record("returned",step,details);}
    private void observed(String step,JSONObject details)throws Exception{journal.record("observed",step,details);}
    private static JSONObject field(String name,Object value)throws Exception{return new JSONObject().put(name,value);}
    private JSONObject info(Uri uri)throws Exception{
        cancel.throwIfCanceled();try(Cursor cursor=resolver.query(uri,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
            if(cursor==null||!cursor.moveToFirst())throw new IOException("ALBUM_DOCUMENT_UNAVAILABLE");String id=cursor.getString(0),name=cursor.getString(1),mime=cursor.getString(2);if(!DocumentsContract.getDocumentId(uri).equals(id)||name==null||mime==null||cursor.moveToNext())throw new IOException("ALBUM_DOCUMENT_AMBIGUOUS");return new JSONObject().put("id",id).put("name",name).put("mime",mime);
        }
    }
    private Uri scoped(Uri returned)throws Exception{
        if(returned==null||!"content".equals(returned.getScheme())||!ProviderReferences.AUTHORITY.equals(returned.getAuthority())||returned.getQuery()!=null||returned.getFragment()!=null||!DocumentsContract.isDocumentUri(context,returned))throw new IOException("ALBUM_RETURN_INVALID");return DocumentsContract.buildDocumentUriUsingTree(references.targetTree,DocumentsContract.getDocumentId(returned));
    }
    private ArrayList<Uri> list(Uri parent)throws Exception{
        cancel.throwIfCanceled();ArrayList<Uri> results=new ArrayList<>();HashSet<String> ids=new HashSet<>(),names=new HashSet<>();try(Cursor cursor=resolver.query(DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
            if(cursor==null)throw new IOException("ALBUM_LIST_UNAVAILABLE");while(cursor.moveToNext()){cancel.throwIfCanceled();String id=cursor.getString(0),name=cursor.getString(1);if(results.size()>=2000||id==null||name==null||!ids.add(id)||!names.add(name.toLowerCase(java.util.Locale.ROOT)))throw new IOException("ALBUM_LIST_AMBIGUOUS");Uri uri=DocumentsContract.buildDocumentUriUsingTree(parent,id);if(!CheckedLocalAncestry.isChild(context,parent,uri))throw new IOException("ALBUM_CHILD_SCOPE");results.add(uri);}if(cursor.getExtras()!=null&&(cursor.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||cursor.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("ALBUM_LIST_INCOMPLETE");}
        return results;
    }
    private Uri find(Uri parent,String name)throws Exception{for(Uri value:list(parent)){String actual=info(value).getString("name");if(actual.equalsIgnoreCase(name)){if(!actual.equals(name))throw new IOException("ALBUM_CASE_COLLISION");return value;}}return null;}
    private String hash(Uri uri,long bytes)throws Exception{
        if(bytes<0||bytes>64L*1024*1024)throw new IOException("ALBUM_FILE_BOUND");MessageDigest digest=MessageDigest.getInstance("SHA-256");long count=0;cancel.throwIfCanceled();
        try(ParcelFileDescriptor fd=resolver.openFileDescriptor(uri,"r",cancel)){
            if(fd==null||!OsConstants.S_ISREG(Os.fstat(fd.getFileDescriptor()).st_mode))throw new IOException("ALBUM_REGULAR_FILE_REQUIRED");try(ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(fd.getFileDescriptor()))){byte[] buffer=new byte[65536];int n;while((n=input.read(buffer))!=-1){cancel.throwIfCanceled();count+=n;if(count>bytes)throw new IOException("ALBUM_CONTENT_CHANGED");digest.update(buffer,0,n);}}}
        if(count!=bytes)throw new IOException("ALBUM_CONTENT_CHANGED");StringBuilder out=new StringBuilder();for(byte b:digest.digest())out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();
    }
    private void verifyFile(Uri uri,AlbumSnapshotTree.Node node)throws Exception{JSONObject binding=node.binding();if(!binding.getString("sha256").equals(hash(uri,binding.getLong("bytes"))))throw new IOException("ALBUM_CONTENT_CHANGED");}
    private HashMap<String,Uri> verifyTree(Uri root,AlbumSnapshotTree.Node node,String name)throws Exception{
        HashMap<String,Uri> result=new HashMap<>();verifyTree(root,node,name,result,new HashSet<String>(),0);return result;
    }
    private void verifyTree(Uri current,AlbumSnapshotTree.Node node,String name,HashMap<String,Uri> result,HashSet<String> seen,int depth)throws Exception{
        cancel.throwIfCanceled();String id=DocumentsContract.getDocumentId(current);if(depth>128||result.size()>=2000||!seen.add(id))throw new IOException("ALBUM_TREE_BOUND");JSONObject now=info(current);if(!name.equals(now.getString("name"))||node.directory!=DIRECTORY.equals(now.getString("mime")))throw new IOException("ALBUM_TREE_CHANGED");result.put(node.token,current);
        if(!node.directory){verifyFile(current,node);return;}ArrayList<Uri> actual=list(current);if(actual.size()!=node.children.size())throw new IOException("ALBUM_MEMBERSHIP_CHANGED");HashMap<String,Uri> names=new HashMap<>();for(Uri uri:actual)names.put(info(uri).getString("name"),uri);for(AlbumSnapshotTree.Node expected:node.children){Uri uri=names.get(expected.name);if(uri==null)throw new IOException("ALBUM_MEMBERSHIP_CHANGED");verifyTree(uri,expected,expected.name,result,seen,depth+1);}
    }
    private void adopt(Uri parent,AlbumSnapshotTree.Node node,HashMap<String,Uri> actual)throws Exception{Uri current=actual.get(node.token);targetGuard.created(parent,current);for(AlbumSnapshotTree.Node nested:node.children)adopt(current,nested,actual);}
    private Uri create(Uri parent,String name,boolean directory,String role,AlbumSnapshotTree.Node node,int component)throws Exception{
        check();reserved.require(parent);ProviderNames.require(name);if(find(parent,name)!=null)throw new IOException("ALBUM_DESTINATION_OCCUPIED");String mime=directory?DIRECTORY:"application/octet-stream";
        JSONObject details=new JSONObject().put("parent",parent.toString()).put("name",name).put("mime",mime).put("node_token",node==null?JSONObject.NULL:node.token).put("component",component);String event=intent("create",role,details);Uri returned=DocumentsContract.createDocument(resolver,parent,mime,name);returned(event,field("uri",returned==null?JSONObject.NULL:returned.toString()));Uri made=scoped(returned);JSONObject metadata=info(made);JSONObject observation=new JSONObject().put("uri",made.toString()).put("metadata",metadata);
        if(!CheckedLocalAncestry.isChild(context,parent,made)||!name.equals(metadata.getString("name"))||directory!=DIRECTORY.equals(metadata.getString("mime"))){journal.record("effect-mismatch",event,observation);throw new IOException("ALBUM_CREATE_EFFECT_MISMATCH");}observed(event,observation);targetGuard.created(parent,made);if(directory)createdDirectories++;return made;
    }
    private Uri destination()throws Exception{
        Uri parent=references.targetRoot();JSONArray components=album.components();for(int n=0;n<components.length();n++){check();String name=components.getString(n);Uri existing=find(parent,name);
            if(n==components.length()-1&&existing!=null)throw new IOException("ALBUM_DESTINATION_OCCUPIED");
            if(existing==null)existing=create(parent,name,true,"target-directory",null,n);else{targetGuard.require(existing);if(!DIRECTORY.equals(info(existing).getString("mime")))throw new IOException("ALBUM_PARENT_NOT_DIRECTORY");root("album-parent-observed",new JSONObject().put("component",n).put("parent",parent.toString()).put("uri",existing.toString()).put("name",name));}parent=existing;
        }return parent;
    }
    private void write(Uri source,Uri output,AlbumSnapshotTree.Node node)throws Exception{
        check();verifyFile(source,node);JSONObject binding=node.binding();String event=intent("write","copy-content",new JSONObject().put("source",source.toString()).put("stage",output.toString()).put("node_token",node.token).put("binding",binding));long count=0;
        try(ParcelFileDescriptor from=resolver.openFileDescriptor(source,"r",cancel);ParcelFileDescriptor to=resolver.openFileDescriptor(output,"rw",cancel)){
            if(from==null||to==null||!OsConstants.S_ISREG(Os.fstat(from.getFileDescriptor()).st_mode)||!OsConstants.S_ISREG(Os.fstat(to.getFileDescriptor()).st_mode))throw new IOException("ALBUM_REGULAR_HANDLES_REQUIRED");
            try(ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(from.getFileDescriptor()));FileOutputStream out=new ParcelFileDescriptor.AutoCloseOutputStream(ParcelFileDescriptor.dup(to.getFileDescriptor()))){byte[] buffer=new byte[65536];int n;while((n=input.read(buffer))!=-1){cancel.throwIfCanceled();count+=n;if(count>binding.getLong("bytes"))throw new IOException("ALBUM_COPY_SOURCE_CHANGED");out.write(buffer,0,n);}out.flush();out.getFD().sync();}}
        returned(event,field("bytes",count));verifyFile(output,node);verifyFile(source,node);targetGuard.refresh(output);observed(event,new JSONObject().put("uri",output.toString()).put("binding",binding));
    }
    private void copyMembers(Uri parent,AlbumSnapshotTree.Node node)throws Exception{
        for(AlbumSnapshotTree.Node member:node.children){cancel.throwIfCanceled();if(member.directory){Uri output=create(parent,member.name,true,"copy-member",member,-1);copyMembers(output,member);}else{Uri output=create(parent,".singles-album-part-"+UUID.randomUUID()+".bin",false,"copy-member-stage",member,-1);write(references.uri(member.reference()),output,member);rename(output,parent,member.name,member,"copy-member-publish");}}
    }
    private Uri rename(Uri source,Uri parent,String name,AlbumSnapshotTree.Node node,String role)throws Exception{
        check();if(node.directory)reserved.requireSubtree(source);else reserved.require(source);reserved.require(parent);JSONObject before=info(source);verifyTree(source,node,before.getString("name"));ProviderNames.require(name);if(find(parent,name)!=null)throw new IOException("ALBUM_DESTINATION_OCCUPIED");String event=intent("rename",role,new JSONObject().put("source",source.toString()).put("parent",parent.toString()).put("name",name).put("node_token",node.token));Uri returned=DocumentsContract.renameDocument(resolver,source,name);returned(event,field("uri",returned==null?JSONObject.NULL:returned.toString()));Uri made=scoped(returned);JSONObject metadata=info(made),observation=new JSONObject().put("uri",made.toString()).put("metadata",metadata);
        if(!CheckedLocalAncestry.isChild(context,parent,made)||!name.equals(metadata.getString("name"))||node.directory!=DIRECTORY.equals(metadata.getString("mime"))){journal.record("effect-mismatch",event,observation);throw new IOException("ALBUM_RENAME_EFFECT_MISMATCH");}observed(event,observation);HashMap<String,Uri> actual=verifyTree(made,node,name);targetGuard.forgetSubtree(source);adopt(parent,node,actual);if(find(parent,before.getString("name"))!=null)throw new IOException("ALBUM_OLD_NAME_REMAINS");return made;
    }
    private Uri move(Uri source,Uri from,Uri to,String name,AlbumSnapshotTree.Node node,String role,boolean original)throws Exception{
        check();references.requireWritable(references.sourceTree);if(node.directory)reserved.requireSubtree(source);else reserved.require(source);reserved.require(to);verifyTree(source,node,name);if(!CheckedLocalAncestry.isChild(context,from,source)||find(to,name)!=null)throw new IOException("ALBUM_MOVE_SCOPE_OR_COLLISION");String event=intent("move",role,new JSONObject().put("source",source.toString()).put("from",from.toString()).put("to",to.toString()).put("name",name).put("node_token",node.token));Uri returned=DocumentsContract.moveDocument(resolver,source,from,to);returned(event,field("uri",returned==null?JSONObject.NULL:returned.toString()));Uri made=scoped(returned);JSONObject metadata=info(made),observation=new JSONObject().put("uri",made.toString()).put("metadata",metadata);
        if(!CheckedLocalAncestry.isChild(context,to,made)||!name.equals(metadata.getString("name"))||node.directory!=DIRECTORY.equals(metadata.getString("mime"))){journal.record("effect-mismatch",event,observation);throw new IOException("ALBUM_MOVE_EFFECT_MISMATCH");}observed(event,observation);HashMap<String,Uri> actual=verifyTree(made,node,name);if(find(from,name)!=null)throw new IOException("ALBUM_MOVE_SOURCE_REMAINS");if(original)sourceGuard.removedSubtree(source,from);else{targetGuard.forgetSubtree(source);targetGuard.refresh(from);}adopt(to,node,actual);return made;
    }
    private void removeEmpty(Uri directory,Uri parent,String name,boolean source)throws Exception{
        check();if(source){references.requireWritable(references.sourceTree);reserved.requireSubtree(directory);sourceGuard.require(directory);sourceGuard.require(parent);}else{reserved.require(directory);targetGuard.require(directory);targetGuard.require(parent);}
        JSONObject metadata=info(directory);if(!name.equals(metadata.getString("name"))||!DIRECTORY.equals(metadata.getString("mime"))||!CheckedLocalAncestry.isChild(context,parent,directory)||!list(directory).isEmpty())throw new IOException("ALBUM_CLEANUP_NOT_OWNED_EMPTY");
        String event=intent("remove-empty",source?"source-album":"move-stage",new JSONObject().put("owned_empty_directory",directory.toString()).put("parent",parent.toString()).put("name",name).put("node_token",source?album.source.token:child.source.token));boolean removed=DocumentsContract.deleteDocument(resolver,directory);returned(event,field("removed",removed));if(!removed||find(parent,name)!=null)throw new IOException("ALBUM_CLEANUP_UNCERTAIN");observed(event,field("owned_empty_stage_absent",true));removedDirectories++;if(source)sourceGuard.removedEmpty(directory,parent);else targetGuard.removedEmpty(directory,parent);check();
    }
    private void removeSelectedRoot()throws Exception{
        check();JSONObject parent=references.rootParentEvidence();if(parent==null||album.source.parent!=null)throw new IOException("ALBUM_PARENT_BINDING");references.requireWritable(references.sourceTree);AlbumRootParent.requireBefore(context,parent,cancel);
        Uri directory=AlbumRootParent.sourceDocument(parent),container=AlbumRootParent.parentDocument(parent);reserved.requireSubtree(directory);sourceGuard.require(references.uri(album.source.reference()));JSONObject metadata=info(directory);
        if(!album.source.name.equals(metadata.getString("name"))||!DIRECTORY.equals(metadata.getString("mime"))||!list(directory).isEmpty())throw new IOException("ALBUM_CLEANUP_NOT_OWNED_EMPTY");
        // This is deliberately not an atomic empty-directory primitive. Only the
        // explicit no-other-writer provider contract authorizes this final window.
        AlbumRootParent.requireBefore(context,parent,cancel);String event=intent("remove-empty","source-album-root",new JSONObject().put("owned_empty_directory",directory.toString()).put("parent",container.toString()).put("name",album.source.name).put("node_token",album.source.token));
        boolean removed=DocumentsContract.deleteDocument(resolver,directory);returned(event,field("removed",removed));if(!removed)throw new IOException("ALBUM_CLEANUP_UNCERTAIN");AlbumRootParent.requireAfter(context,parent,cancel);selectedRootRemoved=true;removedDirectories++;
        observed(event,new JSONObject().put("owned_empty_stage_absent",true).put("exact_parent_membership_without_source",true));check();
    }
    JSONObject transfer(NativeAlbumPlan.Album value)throws Exception{
        album=value;child=null;step=0;createdDirectories=0;removedDirectories=0;albumDestination=null;while(outcomes.length()>0)outcomes.remove(outcomes.length()-1);check();if(album.source.parent==null&&album.removeEmpty)AlbumRootParent.requireBefore(context,references.rootParentEvidence(),cancel);reserved.requireSubtree(references.uri(album.source.reference()));verifyTree(references.uri(album.source.reference()),album.source,album.source.name);
        root("album-start",new JSONObject().put("source",references.uri(album.source.reference()).toString()).put("copy",album.copy).put("remove_empty_source",album.removeEmpty).put("children",album.children.size()));
        // Creating an empty destination is not evidence of a successful Album.
        if(album.eligibleCount>0)albumDestination=destination();boolean skipped=false,mismatch=false;int moved=0;long bytes=0;
        for(NativeAlbumPlan.Child current:album.children){child=current;cancel.throwIfCanceled();if(!child.eligible){JSONObject outcome=new JSONObject().put("child",child.index).put("status","skipped").put("reason",child.refusal).put("predicted_success",child.predictedSuccess).put("source_retained",true);root("album-child-skipped",outcome);outcomes.put(outcome);progress.update(result(false,moved,bytes));skipped=true;if(child.predictedSuccess){mismatch=true;break;}continue;}
            check();Uri original=references.uri(child.source.reference()),sourceParent=references.parent(child.source.reference());verifyTree(original,child.source,child.source.name);root("album-child-start",new JSONObject().put("child",child.index).put("source",original.toString()).put("name",child.name).put("copy",album.copy).put("node_token",child.source.token));Uri published,moveStage=null;String moveStageName=null;
            if(album.copy){String stageName=".singles-album-"+UUID.randomUUID()+(child.source.directory?"":".bin");Uri stage=create(albumDestination,stageName,child.source.directory,"copy-stage",child.source,-1);if(child.source.directory)copyMembers(stage,child.source);else write(original,stage,child.source);verifyTree(stage,child.source,stageName);published=rename(stage,albumDestination,child.name,child.source,"publish-copy");}
            else if(child.source.name.equals(child.name))published=move(original,sourceParent,albumDestination,child.name,child.source,"publish-move",true);
            else{moveStageName=".singles-album-move-"+UUID.randomUUID();moveStage=create(albumDestination,moveStageName,true,"move-stage",child.source,-1);Uri staged=move(original,sourceParent,moveStage,child.source.name,child.source,"stage-move",true);Uri renamed=rename(staged,moveStage,child.name,child.source,"stage-rename");published=move(renamed,moveStage,albumDestination,child.name,child.source,"publish-move",false);}
            verifyTree(published,child.source,child.name);if(album.copy)verifyTree(original,child.source,child.source.name);else if(find(sourceParent,child.source.name)!=null)throw new IOException("ALBUM_SOURCE_REAPPEARED");JSONObject binding=AlbumTreeBinding.of(child.source,child.name,cancel::isCanceled);JSONObject outcome=new JSONObject().put("child",child.index).put("status","complete").put("destination",published.toString()).put("name",child.name).put("copy",album.copy).put("source_retained",album.copy).put("source_absent",!album.copy).put("tree_binding",binding).put("name_changed",!child.name.equals(child.source.name));root("album-child-destination",outcome);outcomes.put(outcome);if(!album.copy)moved++;else bytes+=binding.getLong("bytes");progress.update(result(false,moved,bytes));
            if(moveStage!=null)removeEmpty(moveStage,albumDestination,moveStageName,false);check();root("album-child-verified",outcome);
        }
        child=null;boolean removed=false;if(!mismatch&&!skipped&&album.eligibleCount>0&&album.removeEmpty){if(album.source.parent==null)removeSelectedRoot();else removeEmpty(references.uri(album.source.reference()),references.parent(album.source.reference()),album.source.name,true);removed=true;}
        if(mismatch)throw new IOException("ALBUM_ORIGINAL_EFFECT_MISMATCH");check();JSONObject result=result(removed,moved,bytes).put("status",skipped?"partial":"complete");JSONObject receipt=new JSONObject(result.toString());receipt.remove("children");receipt.put("children_count",outcomes.length());root("album-verified",receipt);progress.update(result);return result;
    }
    private JSONObject result(boolean removed,int moved,long bytes)throws Exception{return new JSONObject().put("album",true).put("destination",albumDestination==null?JSONObject.NULL:albumDestination.toString()).put("copy",album.copy).put("source_removed",removed).put("moved_children",moved).put("bytes",bytes).put("created_directories",createdDirectories).put("removed_directories",removedDirectories).put("children",new JSONArray(outcomes.toString()));}
}
