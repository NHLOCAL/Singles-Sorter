package com.nhlocal.singlesorter.app;

import android.net.Uri;
import android.provider.DocumentsContract;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Producer-shaped Album receipts. Prefixes remain inspectable but never grant retry authority. */
final class AlbumRecoveryEvidence {
    final int acknowledged,unresolved;
    private AlbumRecoveryEvidence(int acknowledged,int unresolved){this.acknowledged=acknowledged;this.unresolved=unresolved;}
    static AlbumRecoveryEvidence validate(JSONObject authority,JSONObject state,BooleanSupplier cancelled)throws Exception{
        if(FixProtocol.integer(authority,"album_protocol",1)!=1)throw invalid();JSONObject original=authority.getJSONObject("original_plan");NativeAlbumPlan plan=NativeAlbumPlan.parse(authority.getString("plan_id"),original,authority.getJSONObject("album_projection"),authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),cancelled);JSONArray operations=original.getJSONArray("operations"),events=state.getJSONArray("events");if(events.length()>10000)throw invalid();int acknowledged=0;Operation album=null;JSONArray singleEvents=new JSONArray();
        for(int i=0;i<events.length();i++){
            JournalExportBounds.checkpoint(cancelled);JSONObject event=events.getJSONObject(i);if(FixProtocol.integer(event,"sequence",9999)!=i)throw invalid();String operation=FixProtocol.string(event,"operation",100);if(!operation.matches("operation-[0-9]{1,5}(/[a-z0-9-]+)?"))throw invalid();int slash=operation.indexOf('/'),id=Integer.parseInt(operation.substring(10,slash<0?operation.length():slash));if(id!=acknowledged||id>=operations.length())throw invalid();NativeAlbumPlan.Album retained=plan.get(id);
            if(retained==null){JSONObject copied=new JSONObject(event.toString()).put("sequence",singleEvents.length()).put("operation","operation-0"+(slash<0?"":operation.substring(slash)));singleEvents.put(copied);if(event.getString("phase").equals("verified")){RetainedJournals.validateSingleEvents(new JSONArray().put(new JSONObject().put("id",0)),singleEvents,cancelled);acknowledged++;singleEvents=new JSONArray();}continue;}
            if(album==null)album=new Operation(retained,plan.tree,original.getJSONObject("request").getJSONObject("target"),cancelled);album.event(event.getString("phase"),slash<0?null:operation.substring(slash+1),event.getJSONObject("detail"));if(album.verified){acknowledged++;album=null;}
        }
        if(singleEvents.length()>0)RetainedJournals.validateSingleEvents(new JSONArray().put(new JSONObject().put("id",0)),singleEvents,cancelled);JournalExportBounds.checkpoint(cancelled);return new AlbumRecoveryEvidence(acknowledged,operations.length()-acknowledged);
    }
    private static IllegalArgumentException invalid(){return new IllegalArgumentException("ALBUM_RECOVERY_EVIDENCE");}
    private static String uri(AlbumSnapshotTree.Node node)throws Exception{return FixRecoveryEvidence.uri(node.entry());}
    private static void equal(Object expected,Object actual){if(expected==null?actual!=JSONObject.NULL:!expected.equals(actual))throw invalid();}
    private static String text(JSONObject value,String field)throws Exception{return FixProtocol.string(value,field,8192);}
    private static String checked(String value,String tree)throws Exception{
        Uri uri=Uri.parse(value),selected=Uri.parse(tree);if(value.length()>8192||!"content".equals(uri.getScheme())||!ProviderReferences.AUTHORITY.equals(uri.getAuthority())||uri.getQuery()!=null||uri.getFragment()!=null||!DocumentsContract.isTreeUri(uri)||!DocumentsContract.getTreeDocumentId(uri).equals(DocumentsContract.getTreeDocumentId(selected)))throw invalid();DocumentsContract.getDocumentId(uri);return value;
    }
    private static final class Operation {
        final NativeAlbumPlan.Album album;final AlbumSnapshotTree tree;final BooleanSupplier cancelled;final String targetTree,targetRoot;
        final JSONArray outcomes=new JSONArray();final HashMap<String,String> copies=new HashMap<>();final HashSet<String> written=new HashSet<>(),memberPublished=new HashSet<>();
        final String[] parents;int targetIndex,nextChild,nextStep,phase,flow;NativeAlbumPlan.Child child;String currentSource,stage,moveStage,moveStageName,activeStep;JSONObject intent,returned,destination;boolean started,verified,mismatch,published,cleaned,removed;long bytes;int moved,skipped,createdDirectories,removedDirectories;
        Operation(NativeAlbumPlan.Album album,AlbumSnapshotTree tree,JSONObject target,BooleanSupplier cancelled)throws Exception{this.album=album;this.tree=tree;this.cancelled=cancelled;JSONObject doc=FixBindingIndex.document(target);targetTree=doc.getString("tree_uri");targetRoot=DocumentsContract.buildDocumentUriUsingTree(Uri.parse(targetTree),doc.getString("document_id")).toString();parents=new String[album.components().length()+1];parents[0]=targetRoot;}
        void event(String event,String step,JSONObject data)throws Exception{
            JournalExportBounds.checkpoint(cancelled);if(verified||mismatch)throw invalid();if(step==null){root(event,data);return;}if(!started||!step.matches("album-step-[0-9]{1,4}"))throw invalid();
            if(event.equals("intent")){if(phase!=0||Integer.parseInt(step.substring(11))!=nextStep++)throw invalid();checkIntent(data);activeStep=step;intent=data;phase=1;return;}
            if(!step.equals(activeStep))throw invalid();if(event.equals("returned")){if(phase!=1)throw invalid();String action=intent.getString("action");if(action.equals("write"))FixProtocol.integer(data,"bytes",64L*1024*1024);else if(action.equals("remove-empty"))FixProtocol.bool(data,"removed");else{Object raw=data.get("uri");if(raw!=JSONObject.NULL&&(!(raw instanceof String)||((String)raw).length()>8192))throw invalid();}returned=data;phase=2;return;}
            if(event.equals("effect-mismatch")){if(phase!=2||data.length()==0)throw invalid();mismatch=true;return;}
            if(!event.equals("observed")||phase!=2)throw invalid();checkObserved(data);phase=0;activeStep=null;intent=null;returned=null;
        }
        private String albumDestination(){return parents[parents.length-1];}
        private AlbumSnapshotTree.Node node(JSONObject data)throws Exception{return tree.token(FixProtocol.string(data,"node_token",4096));}
        private void childNode(AlbumSnapshotTree.Node node)throws Exception{if(child==null)throw invalid();AlbumSnapshotTree.Node current=node;while(current!=null&&current!=child.source){JournalExportBounds.checkpoint(cancelled);current=current.parent;}if(current!=child.source)throw invalid();}
        private void root(String event,JSONObject data)throws Exception{
            if(phase!=0)throw invalid();
            if(event.equals("album-start")){if(started||!uri(album.source).equals(text(data,"source"))||FixProtocol.bool(data,"copy")!=album.copy||FixProtocol.bool(data,"remove_empty_source")!=album.removeEmpty||FixProtocol.integer(data,"children",2000)!=album.children.size())throw invalid();started=true;return;}
            if(!started)throw invalid();
            if(event.equals("album-parent-observed")){if(child!=null||nextChild!=0||album.eligibleCount==0||targetIndex>=parents.length-2||FixProtocol.integer(data,"component",125)!=targetIndex)throw invalid();equal(parents[targetIndex],data.get("parent"));equal(album.components().getString(targetIndex),data.get("name"));parents[++targetIndex]=checked(text(data,"uri"),targetTree);return;}
            if(event.equals("album-child-start")){if(child!=null||nextChild>=album.children.size()||targetIndex!=parents.length-1)throw invalid();NativeAlbumPlan.Child next=album.children.get(nextChild);if(!next.eligible||FixProtocol.integer(data,"child",1999)!=nextChild||FixProtocol.bool(data,"copy")!=album.copy)throw invalid();equal(uri(next.source),data.get("source"));equal(next.name,data.get("name"));equal(next.source.token,data.get("node_token"));child=next;currentSource=uri(next.source);stage=null;moveStage=null;moveStageName=null;flow=0;published=false;cleaned=false;destination=null;copies.clear();written.clear();memberPublished.clear();return;}
            if(event.equals("album-child-skipped")){if(child!=null||nextChild>=album.children.size())throw invalid();NativeAlbumPlan.Child next=album.children.get(nextChild);if(next.eligible||FixProtocol.integer(data,"child",1999)!=nextChild||!"skipped".equals(data.getString("status"))||FixProtocol.bool(data,"predicted_success")!=next.predictedSuccess||!FixProtocol.bool(data,"source_retained"))throw invalid();equal(next.refusal,data.get("reason"));outcomes.put(data);nextChild++;skipped++;if(next.predictedSuccess)mismatch=true;return;}
            if(event.equals("album-child-destination")){if(child==null||!published||destination!=null)throw invalid();checkChildResult(data);destination=data;outcomes.put(data);if(album.copy)bytes+=data.getJSONObject("tree_binding").getLong("bytes");else moved++;return;}
            if(event.equals("album-child-verified")){if(child==null||destination==null||moveStage!=null&&!cleaned||!AlbumJson.same(destination,data,cancelled))throw invalid();child=null;nextChild++;return;}
            if(event.equals("album-verified")){
                if(child!=null||nextChild!=album.children.size()||album.eligibleCount>0&&targetIndex!=parents.length-1||FixProtocol.bool(data,"album")!=true||FixProtocol.bool(data,"copy")!=album.copy||FixProtocol.bool(data,"source_removed")!=removed||FixProtocol.integer(data,"moved_children",2000)!=moved||FixProtocol.integer(data,"bytes",512L*1024*1024)!=bytes||data.has("children")||FixProtocol.integer(data,"children_count",2000)!=outcomes.length()||FixProtocol.integer(data,"created_directories",9000)!=createdDirectories||FixProtocol.integer(data,"removed_directories",9000)!=removedDirectories)throw invalid();equal(albumDestination(),data.get("destination"));equal(skipped>0?"partial":"complete",data.get("status"));if(removed!=(album.removeEmpty&&album.eligibleCount>0&&skipped==0))throw invalid();verified=true;return;
            }
            throw invalid();
        }
        private void checkIntent(JSONObject data)throws Exception{
            Object childValue=data.get("child");if(!(childValue instanceof Integer)||((Integer)childValue)!=(child==null?-1:child.index))throw invalid();String action=data.getString("action"),role=data.getString("role");
            if(action.equals("create")&&role.equals("target-directory")){
                if(child!=null||nextChild!=0||album.eligibleCount==0||targetIndex>=parents.length-1||FixProtocol.integer(data,"component",125)!=targetIndex||!data.isNull("node_token"))throw invalid();equal(parents[targetIndex],data.get("parent"));equal(album.components().getString(targetIndex),data.get("name"));equal(DocumentsContract.Document.MIME_TYPE_DIR,data.get("mime"));return;
            }
            if(action.equals("remove-empty")&&role.equals("source-album")){
                if(child!=null||nextChild!=album.children.size()||!album.removeEmpty||album.eligibleCount==0||skipped!=0||removed)throw invalid();equal(uri(album.source),data.get("owned_empty_directory"));equal(uri(album.source.parent),data.get("parent"));equal(album.source.name,data.get("name"));equal(album.source.token,data.get("node_token"));return;
            }
            if(child==null)throw invalid();AlbumSnapshotTree.Node node=node(data);childNode(node);
            if(action.equals("create")){
                if(!(data.get("component") instanceof Integer)||data.getInt("component")!=-1)throw invalid();String parent=text(data,"parent"),name=text(data,"name"),mime=text(data,"mime");
                if(role.equals("copy-stage")){if(!album.copy||flow!=0||node!=child.source)throw invalid();equal(albumDestination(),parent);temporary(name,".singles-album-",node.directory?"":".bin");equal(node.directory?DocumentsContract.Document.MIME_TYPE_DIR:"application/octet-stream",mime);return;}
                if(role.equals("move-stage")){if(album.copy||flow!=0||node!=child.source||child.source.name.equals(child.name))throw invalid();equal(albumDestination(),parent);temporary(name,".singles-album-move-");equal(DocumentsContract.Document.MIME_TYPE_DIR,mime);return;}
                if(role.equals("copy-member")||role.equals("copy-member-stage")){if(!album.copy||flow!=1||node==child.source||copies.containsKey(node.token)||role.equals("copy-member")!=node.directory)throw invalid();equal(copies.get(node.parent.token),parent);if(node.directory)equal(node.name,name);else temporary(name,".singles-album-part-",".bin");equal(node.directory?DocumentsContract.Document.MIME_TYPE_DIR:"application/octet-stream",mime);return;}throw invalid();
            }
            if(action.equals("write")&&role.equals("copy-content")){
                if(!album.copy||flow!=1||node.directory||written.contains(node.token))throw invalid();equal(uri(node),data.get("source"));equal(copies.get(node.token),data.get("stage"));if(!FixProtocol.sameBinding(node.binding(),FixProtocol.binding(data.getJSONObject("binding"),FixProtocol.ORIGINAL_LIMIT)))throw invalid();return;
            }
            if(action.equals("rename")){
                if(role.equals("copy-member-publish")){if(!album.copy||flow!=1||node==child.source||node.directory||!written.contains(node.token)||memberPublished.contains(node.token))throw invalid();equal(copies.get(node.token),data.get("source"));equal(copies.get(node.parent.token),data.get("parent"));equal(node.name,data.get("name"));return;}
                if(node!=child.source)throw invalid();equal(currentSource,data.get("source"));equal(child.name,data.get("name"));
                if(role.equals("publish-copy")){if(!album.copy||flow!=1)throw invalid();equal(albumDestination(),data.get("parent"));List<AlbumSnapshotTree.Node> all=tree.complete(child.source);for(AlbumSnapshotTree.Node member:all){JournalExportBounds.checkpoint(cancelled);if(!copies.containsKey(member.token)||!member.directory&&(!written.contains(member.token)||member!=child.source&&!memberPublished.contains(member.token)))throw invalid();}return;}
                if(role.equals("stage-rename")){if(album.copy||flow!=2)throw invalid();equal(moveStage,data.get("parent"));return;}throw invalid();
            }
            if(action.equals("move")){
                if(album.copy||node!=child.source)throw invalid();equal(currentSource,data.get("source"));
                if(role.equals("stage-move")){if(flow!=1||moveStage==null)throw invalid();equal(uri(child.source.parent),data.get("from"));equal(moveStage,data.get("to"));equal(child.source.name,data.get("name"));return;}
                if(role.equals("publish-move")){boolean rename=!child.name.equals(child.source.name);if(flow!=(rename?3:0))throw invalid();equal(rename?moveStage:uri(child.source.parent),data.get("from"));equal(albumDestination(),data.get("to"));equal(child.name,data.get("name"));return;}throw invalid();
            }
            if(action.equals("remove-empty")&&role.equals("move-stage")){if(album.copy||node!=child.source||moveStage==null||destination==null||cleaned)throw invalid();equal(moveStage,data.get("owned_empty_directory"));equal(albumDestination(),data.get("parent"));equal(moveStageName,data.get("name"));return;}
            throw invalid();
        }
        private void checkObserved(JSONObject data)throws Exception{
            String action=intent.getString("action"),role=intent.getString("role");
            if(action.equals("write")){AlbumSnapshotTree.Node node=node(intent);equal(copies.get(node.token),data.get("uri"));if(FixProtocol.integer(returned,"bytes",FixProtocol.ORIGINAL_LIMIT)!=node.binding().getLong("bytes")||!FixProtocol.sameBinding(node.binding(),FixProtocol.binding(data.getJSONObject("binding"),FixProtocol.ORIGINAL_LIMIT)))throw invalid();written.add(node.token);return;}
            if(action.equals("remove-empty")){if(!FixProtocol.bool(returned,"removed")||!FixProtocol.bool(data,"owned_empty_stage_absent"))throw invalid();removedDirectories++;if(role.equals("source-album"))removed=true;else cleaned=true;return;}
            String raw=text(returned,"uri"),actual=checked(text(data,"uri"),targetTree);Uri returnedUri=Uri.parse(raw),observedUri=Uri.parse(actual);if(!"content".equals(returnedUri.getScheme())||!ProviderReferences.AUTHORITY.equals(returnedUri.getAuthority())||returnedUri.getQuery()!=null||returnedUri.getFragment()!=null||!DocumentsContract.getDocumentId(returnedUri).equals(DocumentsContract.getDocumentId(observedUri)))throw invalid();JSONObject metadata=data.getJSONObject("metadata");equal(DocumentsContract.getDocumentId(observedUri),metadata.get("id"));equal(intent.getString("name"),metadata.get("name"));
            boolean directory=role.equals("target-directory")||role.equals("move-stage")||node(intent).directory;if(directory!=DocumentsContract.Document.MIME_TYPE_DIR.equals(metadata.getString("mime")))throw invalid();if(action.equals("create")&&directory)createdDirectories++;
            if(role.equals("target-directory")){parents[++targetIndex]=actual;return;}AlbumSnapshotTree.Node node=node(intent);
            if(role.equals("copy-stage")){stage=actual;currentSource=actual;copies.put(node.token,actual);flow=1;return;}
            if(role.equals("copy-member")||role.equals("copy-member-stage")){copies.put(node.token,actual);return;}
            if(role.equals("copy-member-publish")){copies.put(node.token,actual);memberPublished.add(node.token);return;}
            if(role.equals("move-stage")){moveStage=actual;moveStageName=intent.getString("name");flow=1;return;}
            currentSource=actual;if(role.equals("stage-move")){flow=2;return;}if(role.equals("stage-rename")){flow=3;return;}if(role.equals("publish-copy")||role.equals("publish-move")){published=true;flow=4;return;}throw invalid();
        }
        private void checkChildResult(JSONObject data)throws Exception{
            if(FixProtocol.integer(data,"child",1999)!=child.index||!"complete".equals(data.getString("status"))||FixProtocol.bool(data,"copy")!=album.copy||FixProtocol.bool(data,"source_retained")!=album.copy||FixProtocol.bool(data,"source_absent")==album.copy||FixProtocol.bool(data,"name_changed")==child.name.equals(child.source.name))throw invalid();equal(currentSource,data.get("destination"));equal(child.name,data.get("name"));if(!AlbumTreeBinding.same(AlbumTreeBinding.of(child.source,child.name,cancelled),data.getJSONObject("tree_binding")))throw invalid();
        }
        private void temporary(String name,String prefix){temporary(name,prefix,"");}
        private void temporary(String name,String prefix,String suffix){if(!name.startsWith(prefix)||!name.endsWith(suffix)||name.length()!=prefix.length()+36+suffix.length()||!JournalExportBounds.id(name.substring(prefix.length(),name.length()-suffix.length())))throw invalid();}
    }
}
