package com.nhlocal.singlesorter.app;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Validates the borrowed Rust Album projection against its exact retained plan and complete evidence. */
final class NativeAlbumPlan {
    static final class Child {
        final int index;final AlbumSnapshotTree.Node source;final String name;final boolean predictedSuccess,eligible;final String refusal;
        private final String intent;
        Child(int index,AlbumSnapshotTree.Node source,JSONObject projected)throws Exception{
            this.index=index;this.source=source;JSONObject original=projected.getJSONObject("intent");intent=original.toString();name=projected.getString("planned_name");ProviderNames.require(name);predictedSuccess=FixProtocol.bool(original,"predicted_success");
            String blocked=null;JSONArray markers=original.getJSONArray("blockers");for(int i=0;i<markers.length();i++){String marker=markers.getString(i);if(!marker.equals("MISSING_IDENTITY"))blocked=marker;}
            refusal=!predictedSuccess?"ORIGINAL_PREDICTED_FAILURE":blocked==null?null:"ORIGINAL_CHILD_BLOCKED: "+blocked;eligible=refusal==null;
        }
        JSONObject intent()throws Exception{return new JSONObject(intent);}
    }
    static final class Album {
        final int id;final AlbumSnapshotTree.Node source;final List<Child> children;final boolean copy,removeEmpty;final int eligibleCount;
        private final String destination;
        Album(int id,AlbumSnapshotTree.Node source,JSONArray destination,List<Child> children,boolean copy,boolean removeEmpty){this.id=id;this.source=source;this.destination=destination.toString();this.children=Collections.unmodifiableList(children);this.copy=copy;this.removeEmpty=removeEmpty;int count=0;for(Child child:children)if(child.eligible)count++;eligibleCount=count;}
        JSONArray components()throws Exception{return new JSONArray(destination);}
    }
    final List<Album> operations;final AlbumSnapshotTree tree;private final HashMap<Integer,Album> byId=new HashMap<>();
    private NativeAlbumPlan(List<Album> operations,AlbumSnapshotTree tree){this.operations=Collections.unmodifiableList(operations);this.tree=tree;for(Album album:operations)byId.put(album.id,album);}
    Album get(int id){return byId.get(id);}
    static NativeAlbumPlan prepare(RetainedPlan retained,ProviderReferences references,BooleanSupplier cancelled)throws Exception{
        NativeAlbumPlan plan=parse(retained.planId,retained.planCopy(),retained.albumProjectionCopy(),retained.referencesCopy(),retained.bindingsCopy(),cancelled,retained.albumRootParentCopy());references.requireDisjointRoots();for(Album album:plan.operations)if(!album.copy)references.requireWritable(references.sourceTree);return plan;
    }
    static NativeAlbumPlan parse(String id,JSONObject plan,JSONObject projection,JSONArray references,JSONObject bindings,BooleanSupplier cancelled)throws Exception{
        return parse(id,plan,projection,references,bindings,cancelled,null);
    }
    static NativeAlbumPlan parse(String id,JSONObject plan,JSONObject projection,JSONArray references,JSONObject bindings,BooleanSupplier cancelled,JSONObject parentEvidence)throws Exception{
        JournalExportBounds.checkpoint(cancelled);JSONObject request=plan.getJSONObject("request");JSONArray original=plan.getJSONArray("operations"),projected=projection.getJSONArray("operations");
        if(plan.getInt("schema_version")!=2||!id.equals(plan.getString("binding_sha256"))||!id.equals(projection.getString("plan_id"))||!"Sort".equals(request.getString("mode"))||original.length()>10000||projected.length()>2000||!AlbumJson.same(plan.getJSONArray("blockers"),projection.getJSONArray("blockers"),cancelled))throw invalid("BINDING");NativeTransferPlan.blockers(plan.getJSONArray("blockers"),true,!NativeDuetPlan.inspectOriginal(plan,cancelled).groups.isEmpty());
        AlbumSnapshotTree tree=new AlbumSnapshotTree(references,bindings,cancelled);AlbumSnapshotTree.Node root=tree.get(request.getJSONObject("source"));if(!root.directory||root.parent!=null)throw invalid("SOURCE_ROOT");
        boolean copy=FixProtocol.bool(request.getJSONObject("settings"),"copy_mode");ArrayList<Album> albums=new ArrayList<>();int position=0;long bytes=0,effects=0;HashSet<String> selected=new HashSet<>();
        for(int i=0;i<original.length();i++){
            JournalExportBounds.checkpoint(cancelled);JSONObject operation=original.getJSONObject(i);NativeTransferPlan.requireOrder(operation,i);JSONObject kind=operation.getJSONObject("kind");if(kind.length()!=1)throw invalid("OPERATION_SHAPE");if(!kind.has("AlbumTransfer")){if(!kind.has("SingleTransfer"))throw invalid("OPERATION_SHAPE");continue;}if(position>=projected.length())throw invalid("MISSING_PROJECTION");JSONObject exact=kind.getJSONObject("AlbumTransfer"),album=projected.getJSONObject(position++);
            if(FixProtocol.integer(album,"id",9999)!=i||!AlbumJson.same(operation.getJSONArray("depends_on"),album.getJSONArray("depends_on"),cancelled)||!FixProtocol.bool(album,"stop_and_replan_on_effect_mismatch")||FixProtocol.bool(album,"copy_mode")!=copy||FixProtocol.bool(album,"remove_empty_source")!=FixProtocol.bool(exact,"remove_empty_source")||!AlbumJson.same(exact.getJSONObject("source"),album.getJSONObject("source"),cancelled)||!AlbumJson.same(exact.getJSONObject("destination"),album.getJSONObject("destination"),cancelled))throw invalid("PROJECTION_CHANGED");
            JSONObject source=exact.getJSONObject("source"),destination=exact.getJSONObject("destination");AlbumSnapshotTree.Node node=tree.get(source.getJSONObject("reference"));if(!"Directory".equals(source.getString("kind"))||!node.directory||!node.name.equals(source.getString("name"))||!selected.add(node.token))throw invalid("SOURCE");tree.requireSource(node,root);List<AlbumSnapshotTree.Node> subtree=tree.complete(node);if(!NativeTransferPlan.sameReference(destination.getJSONObject("root"),request.getJSONObject("target")))throw invalid("DESTINATION_ROOT");JSONArray components=destination.getJSONArray("components");if(components.length()<1||components.length()>125)throw invalid("DESTINATION_BOUND");for(int n=0;n<components.length();n++)ProviderNames.require(components.getString(n));
            JSONArray children=exact.getJSONArray("children"),projectedChildren=album.getJSONArray("children");if(children.length()>2000||children.length()!=projectedChildren.length())throw invalid("CHILD_BOUND");ArrayList<Child> actual=new ArrayList<>();HashSet<String> seen=new HashSet<>();
            for(int n=0;n<children.length();n++){
                JournalExportBounds.checkpoint(cancelled);JSONObject child=children.getJSONObject(n),borrowed=projectedChildren.getJSONObject(n),observation=child.getJSONObject("source"),childDestination=child.getJSONObject("destination");if(!AlbumJson.same(child,borrowed.getJSONObject("intent"),cancelled)||FixProtocol.bool(child,"copy")!=copy||!NativeTransferPlan.sameReference(childDestination.getJSONObject("root"),destination.getJSONObject("root")))throw invalid("CHILD_PROJECTION");JSONArray path=childDestination.getJSONArray("components");if(path.length()!=components.length()+1)throw invalid("CHILD_DESTINATION");for(int p=0;p<components.length();p++)if(!path.getString(p).equals(components.getString(p)))throw invalid("CHILD_DESTINATION");String name=path.getString(path.length()-1);if(!name.equals(borrowed.getString("planned_name"))||FixProtocol.bool(borrowed,"name_changes")==name.equals(observation.getString("name")))throw invalid("CHILD_NAME");AlbumSnapshotTree.Node childNode=tree.get(observation.getJSONObject("reference"));if(childNode.parent!=node||!seen.add(childNode.token)||!childNode.name.equals(observation.getString("name"))||childNode.directory!=observation.getString("kind").equals("Directory"))throw invalid("CHILD_SOURCE");
                actual.add(new Child(n,childNode,borrowed));
            }
            boolean remove=FixProtocol.bool(exact,"remove_empty_source");if(copy&&remove)throw invalid("COPY_REMOVAL");if(remove){if(seen.size()!=node.children.size())throw invalid("REMOVAL_CHILD_COVERAGE");for(Child selectedChild:actual){JournalExportBounds.checkpoint(cancelled);if(!selectedChild.predictedSuccess)throw invalid("REMOVAL_CHILD_COVERAGE");}}Album parsed=new Album(i,node,components,actual,copy,remove);albums.add(parsed);
            // Bound authority and worst-case provider receipts before admitting any effects.
            effects+=AlbumEffectBudget.album(parsed,tree,cancelled);for(AlbumSnapshotTree.Node member:subtree)if(!member.directory)bytes+=member.binding().getLong("bytes");if(effects>9000||bytes>512L*1024*1024)throw invalid("JOB_BUDGET");
        }
        if(position!=projected.length())throw invalid("EXTRA_PROJECTION");if(AlbumRootParent.required(plan,cancelled)){if(parentEvidence==null)throw invalid("SELECTED_ROOT_REMOVAL_UNAVAILABLE");AlbumRootParent.validate(parentEvidence,plan,cancelled);}else if(parentEvidence!=null)throw invalid("PARENT_NOT_REQUIRED");JournalExportBounds.checkpoint(cancelled);return new NativeAlbumPlan(albums,tree);
    }
    private static IllegalArgumentException invalid(String reason){return new IllegalArgumentException("ALBUM_"+reason);}
}
