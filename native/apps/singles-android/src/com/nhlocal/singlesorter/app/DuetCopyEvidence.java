package com.nhlocal.singlesorter.app;

import android.net.Uri;
import android.provider.DocumentsContract;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Strict new-protocol copy certificate. Legacy Single receipt acceptance is unchanged. */
final class DuetCopyEvidence {
    private final JSONObject sourceBinding;private final JSONArray components;private final BooleanSupplier cancelled;
    private final String original,originalParent,originalName,targetTree,targetRoot;
    private String parent,createdParent,stage,published,step;private JSONObject intent,returned,destination;
    private int component,phase;private boolean started,writeReturned,writeMatches,mismatch,verified;
    DuetCopyEvidence(JSONObject transfer,JSONObject target,FixBindingIndex bindings,BooleanSupplier cancelled)throws Exception{
        this.cancelled=cancelled;JSONObject reference=transfer.getJSONObject("source").getJSONObject("reference"),entry=bindings.entry(reference),root=FixBindingIndex.document(target);
        sourceBinding=bindings.binding(reference);original=FixRecoveryEvidence.uri(entry);originalParent=FixRecoveryEvidence.uri(bindings.parent(reference));originalName=entry.getString("name");
        targetTree=root.getString("tree_uri");targetRoot=DocumentsContract.buildDocumentUriUsingTree(Uri.parse(targetTree),root.getString("document_id")).toString();parent=targetRoot;
        JSONObject destination=transfer.getJSONObject("destination");if(!NativeTransferPlan.sameReference(destination.getJSONObject("root"),target)||!FixProtocol.bool(transfer,"copy"))throw invalid();components=destination.getJSONArray("components");
        if(components.length()<1||components.length()>128)throw invalid();for(int n=0;n<components.length();n++)ProviderNames.require(components.getString(n));
    }
    void event(String event,String suffix,JSONObject data)throws Exception{
        JournalExportBounds.checkpoint(cancelled);if(verified||mismatch)throw invalid();
        if(suffix==null){root(event,data);return;}
        if(!started)throw invalid();
        if(event.equals("intent")){
            if(phase!=0)throw invalid();checkIntent(suffix,data);intent=data;step=suffix;phase=1;return;
        }
        if(!suffix.equals(step))throw invalid();
        if(event.equals("returned")){
            if(phase!=1)throw invalid();returned=data;
            if(step.equals("write-stage")){
                if(data.length()!=2)throw invalid();FixProtocol.integer(data,"bytes",FixProtocol.ORIGINAL_LIMIT);FixProtocol.hash(data,"sha256");writeReturned=true;writeMatches=FixProtocol.sameBinding(sourceBinding,data);phase=0;step=null;return;
            }
            if(data.length()!=1)throw invalid();Object raw=data.get("uri");if(raw!=JSONObject.NULL&&(!(raw instanceof String)||((String)raw).length()>8192))throw invalid();phase=2;return;
        }
        if(!event.equals("observed")||phase!=2||data.length()!=2)throw invalid();
        String actual=checked(FixProtocol.string(data,"uri",8192));JSONObject metadata=data.getJSONObject("metadata");
        if(metadata.length()!=3)throw invalid();String id=FixProtocol.string(metadata,"id",4096),name=FixProtocol.string(metadata,"name",1024),mime=FixProtocol.string(metadata,"mime",256);
        boolean directory=step.startsWith("directory-"),same=returnedIdMatches(actual)&&DocumentsContract.getDocumentId(Uri.parse(actual)).equals(id)
            &&intent.getString("name").equals(name)&&directory==DocumentsContract.Document.MIME_TYPE_DIR.equals(mime);
        // The producer preserves a mismatched observation before stopping. It is
        // an inspectable prefix, never a certificate for a later phase.
        if(!same){mismatch=true;return;}
        if(directory)createdParent=actual;else if(step.equals("create-stage"))stage=actual;else if(step.equals("publish-copy"))published=actual;else throw invalid();
        phase=0;step=null;
    }
    private void root(String event,JSONObject data)throws Exception{
        if(phase!=0)throw invalid();
        if(event.equals("duet-copy-start")){
            if(started||data.length()!=6||!original.equals(data.getString("source"))||!originalParent.equals(data.getString("parent"))||!originalName.equals(data.getString("name"))
                ||!targetRoot.equals(data.getString("target"))||!FixProtocol.sameBinding(sourceBinding,FixProtocol.binding(data.getJSONObject("binding"),FixProtocol.ORIGINAL_LIMIT))||!AlbumJson.same(components,data.getJSONArray("components"),cancelled))throw invalid();started=true;return;
        }
        if(!started)throw invalid();
        if(event.equals("duet-parent-observed")){
            if(stage!=null||component>=components.length()-1||data.length()!=4||FixProtocol.integer(data,"component",127)!=component||!parent.equals(data.getString("parent"))||!components.getString(component).equals(data.getString("name")))throw invalid();
            String actual=checked(data.getString("uri"));if(createdParent!=null&&!createdParent.equals(actual))throw invalid();parent=actual;createdParent=null;component++;return;
        }
        if(event.equals("destination-observed")){
            if(published==null||destination!=null||data.length()!=5)throw invalid();destination(data);destination=data;return;
        }
        if(event.equals("verified")){
            if(destination==null||data.length()!=7||!FixProtocol.bool(data,"source_retained")||FixProtocol.bool(data,"source_absent"))throw invalid();destination(data);
            for(String key:new String[]{"destination","name","bytes","sha256","copy"})if(!destination.get(key).equals(data.get(key)))throw invalid();verified=true;return;
        }
        throw invalid();
    }
    private void destination(JSONObject data)throws Exception{
        if(!published.equals(data.getString("destination"))||!components.getString(components.length()-1).equals(data.getString("name"))||!FixProtocol.bool(data,"copy")||!FixProtocol.sameBinding(sourceBinding,data))throw invalid();
    }
    private void checkIntent(String suffix,JSONObject data)throws Exception{
        if(suffix.startsWith("directory-")){
            if(!suffix.equals("directory-"+component)||stage!=null||createdParent!=null||component>=components.length()-1||data.length()!=3||!parent.equals(data.getString("parent"))||!components.getString(component).equals(data.getString("name"))||!DocumentsContract.Document.MIME_TYPE_DIR.equals(data.getString("mime")))throw invalid();return;
        }
        if(component!=components.length()-1||createdParent!=null)throw invalid();
        if(suffix.equals("create-stage")){
            if(stage!=null||data.length()!=3||!parent.equals(data.getString("parent"))||!"application/octet-stream".equals(data.getString("mime"))||!ReservedProviderObjects.temporaryName("create-stage",data.getString("name")))throw invalid();return;
        }
        if(suffix.equals("write-stage")){
            if(stage==null||writeReturned||data.length()!=3||!original.equals(data.getString("source"))||!stage.equals(data.getString("stage"))||!FixProtocol.sameBinding(sourceBinding,FixProtocol.binding(data.getJSONObject("binding"),FixProtocol.ORIGINAL_LIMIT)))throw invalid();return;
        }
        if(suffix.equals("publish-copy")){
            if(stage==null||!writeReturned||!writeMatches||published!=null||data.length()!=3||!stage.equals(data.getString("source"))||!parent.equals(data.getString("parent"))||!components.getString(components.length()-1).equals(data.getString("name")))throw invalid();return;
        }
        throw invalid();
    }
    private boolean returnedIdMatches(String actual)throws Exception{
        if(returned.isNull("uri"))return false;try{Uri raw=Uri.parse(returned.getString("uri"));return "content".equals(raw.getScheme())&&ProviderReferences.AUTHORITY.equals(raw.getAuthority())&&raw.getQuery()==null&&raw.getFragment()==null&&DocumentsContract.getDocumentId(raw).equals(DocumentsContract.getDocumentId(Uri.parse(actual)));}catch(RuntimeException malformed){return false;}
    }
    private String checked(String value)throws Exception{
        Uri uri=Uri.parse(value);if(value.length()>8192||!"content".equals(uri.getScheme())||!ProviderReferences.AUTHORITY.equals(uri.getAuthority())||uri.getQuery()!=null||uri.getFragment()!=null||!DocumentsContract.isTreeUri(uri)||!DocumentsContract.getTreeDocumentId(uri).equals(DocumentsContract.getTreeDocumentId(Uri.parse(targetTree))))throw invalid();
        String id=DocumentsContract.getDocumentId(uri);if(!DocumentsContract.buildDocumentUriUsingTree(Uri.parse(targetTree),id).toString().equals(value))throw invalid();return value;
    }
    private static IllegalArgumentException invalid(){return new IllegalArgumentException("DUET_COPY_CERTIFICATE");}
}
