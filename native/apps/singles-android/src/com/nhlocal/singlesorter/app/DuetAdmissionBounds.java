package com.nhlocal.singlesorter.app;

import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Whole-job and per-record budget, including the retirement barrier, before any effect. */
final class DuetAdmissionBounds {
    private DuetAdmissionBounds(){}
    static void require(RetainedPlan retained,NativeTransferPlan singles,NativeAlbumPlan albums,NativeDuetPlan duets,ProviderReferences references,BooleanSupplier cancelled)throws Exception{
        if(duets.groups.isEmpty())return;
        FixBindingIndex index=new FixBindingIndex(retained.referencesCopy(),retained.bindingsCopy(),cancelled);
        JSONObject root=index.entry(retained.planCopy().getJSONObject("request").getJSONObject("source"));
        HashMap<Integer,Long> sizes=new HashMap<>();long events=0,bytes=4096;
        String target=references.targetRoot().toString();
        for(NativeTransferPlan.Transfer single:singles.operations){
            JournalExportBounds.checkpoint(cancelled);String source=references.uri(single.source()).toString(),parent=references.parent(single.source()).toString();long uri=bound(source.length()>parent.length()?source:parent,target,single.components());sizes.put(single.id,uri);
            long count=AlbumEffectBudget.single(single)+single.components().length();events+=count;bytes+=count*(512+3*uri);
        }
        for(NativeAlbumPlan.Album album:albums.operations){
            JournalExportBounds.checkpoint(cancelled);long count=AlbumEffectBudget.album(album,albums.tree,cancelled),max=album.source.parent==null&&retained.albumRootParentCopy()!=null?AlbumRootParent.uriBudget(retained.albumRootParentCopy()):0;
            for(AlbumSnapshotTree.Node node:albums.tree.complete(album.source))max=Math.max(max,bound(FixRecoveryEvidence.uri(node.entry()),target,album.components()));
            events+=count;bytes+=count*(512+3*max);
        }
        for(NativeDuetPlan.Group group:duets.groups){
            JournalExportBounds.checkpoint(cancelled);JSONObject source=group.source(),ref=source.getJSONObject("reference"),entry=index.entry(ref);
            if(FixProtocol.bool(entry,"directory")||!entry.getString("name").equals(source.getString("name")))throw new IllegalArgumentException("DUET_SOURCE_BINDING");
            index.requireRoot(ref,root,cancelled);JSONObject binding=index.binding(ref);String uri=references.uri(ref).toString(),parent=references.parent(ref).toString();
            if(uri.length()>8192||parent.length()>8192)throw new IllegalArgumentException("DUET_URI_BUDGET");
            JSONObject evidence=new JSONObject().put("source",uri).put("parent",parent).put("name",source.getString("name")).put("bytes",binding.getLong("bytes")).put("sha256",binding.getString("sha256"));
            long record=2048+evidence.toString().getBytes(StandardCharsets.UTF_8).length;
            for(int id:group.requiredOperations){Long copyUri=sizes.get(id);if(copyUri==null)throw new IllegalArgumentException("DUET_OPERATION_NOT_PROJECTED");record+=512+copyUri;}
            if(record>60000)throw new IllegalArgumentException("DUET_RECORD_BUDGET");
            events+=group.hasRequiredDestinations()?5:1;bytes+=record*5;
        }
        if(events>9000||bytes>7L*1024*1024)throw new IllegalArgumentException("DUET_JOB_BUDGET");
        JournalExportBounds.checkpoint(cancelled);
    }
    private static long bound(String source,String target,JSONArray path)throws Exception{
        long bytes=source.getBytes(StandardCharsets.UTF_8).length+target.getBytes(StandardCharsets.UTF_8).length+1024;
        for(int i=0;i<path.length();i++)bytes+=path.getString(i).getBytes(StandardCharsets.UTF_8).length*3L+3;
        if(bytes>8192)throw new IllegalArgumentException("DUET_URI_BUDGET");return bytes;
    }
}
