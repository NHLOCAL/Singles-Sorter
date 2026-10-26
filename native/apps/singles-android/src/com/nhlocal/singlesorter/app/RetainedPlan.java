package com.nhlocal.singlesorter.app;

import android.os.CancellationSignal;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import org.json.JSONArray;
import org.json.JSONObject;

/** Native-owned original decisions and source content bindings; never sent to the WebView.
 * A digest binds this preview only. Every provider effect still requires fresh checks.
 */
final class RetainedPlan {
    final String jobId,planId;final int operationCount;final long recoveryGeneration;
    final ExecutionAttempt attempt;
    private final String originalPlan,sourceBindings,references,albumProjection,albumRootParent;
    private RetainedPlan(String job,String id,String plan,String bindings,String entries,String albums,int count,long generation){this(job,id,plan,bindings,entries,albums,count,generation,null);}
    private RetainedPlan(String job,String id,String plan,String bindings,String entries,String albums,int count,long generation,String parent){recoveryGeneration=generation;operationCount=count;attempt=new ExecutionAttempt(job);jobId=job;planId=id;originalPlan=plan;sourceBindings=bindings;references=entries;albumProjection=albums;albumRootParent=parent;}
    static RetainedPlan capture(String job,JSONObject result,SafSnapshot snapshots,CancellationSignal cancel,long generation)throws Exception{
        String id=result.getString("plan_id"),encoded=result.getString("_retained_plan");
        if(encoded.getBytes(StandardCharsets.UTF_8).length>8*1024*1024)throw new IllegalArgumentException("PLAN_LIMIT");
        JSONObject plan=new JSONObject(encoded);
        if(!id.equals(plan.getString("binding_sha256"))||plan.getJSONArray("operations").length()>10000)throw new IllegalArgumentException("PLAN_BINDING");
        JSONArray tokens=result.getJSONArray("_source_tokens");if(tokens.length()>2000)throw new IllegalArgumentException("SOURCE_LIMIT");
        JSONObject projection=result.getJSONObject("_album_projection");
        String projected=projection.toString();
        if(!id.equals(projection.getString("plan_id")))throw new IllegalArgumentException("ALBUM_PLAN_BINDING");
        requireBudget(encoded,projected);
        JSONArray albums=projection.getJSONArray("operations");if(albums.length()>10000)throw new IllegalArgumentException("ALBUM_OPERATION_LIMIT");
        LinkedHashMap<String,JSONObject> roots=new LinkedHashMap<>();int children=0;
        for(int n=0;n<albums.length();n++) {
            cancel.throwIfCanceled();JSONObject album=albums.getJSONObject(n),source=album.getJSONObject("source");
            String token=snapshots.requireAlbumObservation(source);
            roots.put(token,source.getJSONObject("reference"));
            JSONArray direct=album.getJSONArray("children");children+=direct.length();
            if(direct.length()>2000||children>100000)throw new IllegalArgumentException("ALBUM_CHILD_LIMIT");
            for(int child=0;child<direct.length();child++) {
                cancel.throwIfCanceled();snapshots.requireAlbumObservation(direct.getJSONObject(child).getJSONObject("intent").getJSONObject("source"));
            }
        }
        // Expand evidence only now; initial manifest and original planner decisions
        // are immutable. All provider reads still belong to this snapshot owner.
        LinkedHashSet<String> descendantFiles=new LinkedHashSet<>();
        for(JSONObject root:roots.values()){cancel.throwIfCanceled();descendantFiles.addAll(snapshots.completeAlbumSubtree(root));}
        JSONObject bindings=new JSONObject();
        for(int n=0;n<tokens.length();n++){
            cancel.throwIfCanceled();String token=tokens.getString(n);if(bindings.has(token))throw new IllegalArgumentException("DUPLICATE_SOURCE");
            bind(bindings,token,snapshots);
        }
        for(String token:descendantFiles){cancel.throwIfCanceled();if(!bindings.has(token))bind(bindings,token,snapshots);}
        // Empty eligible duet cohorts have no SingleTransfer token, but their
        // read-only no-op still needs the original source's captured name/hash.
        if("Sort".equals(plan.getJSONObject("request").getString("mode")))for(NativeDuetPlan.Group group:NativeDuetPlan.inspectOriginal(plan,cancel::isCanceled).groups){
            cancel.throwIfCanceled();String token=snapshots.requireAlbumObservation(group.source());if(!bindings.has(token))bind(bindings,token,snapshots);
        }
        for(JSONObject root:roots.values()){cancel.throwIfCanceled();snapshots.verifyAlbumSubtree(root);}
        String bound=bindings.toString(),entries=snapshots.retainedReferences().toString();requireBudget(encoded,projected,bound,entries);
        cancel.throwIfCanceled();PreviewGeneration.PROCESS.require(generation);return new RetainedPlan(job,id,encoded,bound,entries,projected,plan.getJSONArray("operations").length(),generation);
    }
    private static void bind(JSONObject bindings,String token,SafSnapshot snapshots)throws Exception {
        JSONObject snapshot=new JSONObject(new String(snapshots.snapshot(token.getBytes(StandardCharsets.UTF_8)),StandardCharsets.UTF_8));
        // Deliberately retain no cache path: private snapshots are deleted before completion.
        bindings.put(token,new JSONObject().put("sha256",snapshot.getString("sha256")).put("bytes",snapshot.getLong("snapshot_size")));
    }
    private static void requireBudget(String... values){
        long size=0;for(String value:values){size+=value.getBytes(StandardCharsets.UTF_8).length;if(size>8L*1024*1024)throw new IllegalArgumentException("PLAN_LIMIT");}
    }
    JSONObject albumProjectionCopy()throws Exception{return new JSONObject(albumProjection);}
    JSONObject albumRootParentCopy()throws Exception{return albumRootParent==null?null:new JSONObject(albumRootParent);}
    RetainedPlan withAlbumRootParent(JSONObject parent,java.util.function.BooleanSupplier cancelled)throws Exception{
        String encoded=AlbumRootParent.validate(parent,planCopy(),cancelled).toString();requireBudget(originalPlan,sourceBindings,references,albumProjection,encoded);JournalExportBounds.checkpoint(cancelled);
        return new RetainedPlan(jobId,planId,originalPlan,sourceBindings,references,albumProjection,operationCount,recoveryGeneration,encoded);
    }
    JSONArray referencesCopy()throws Exception{return new JSONArray(references);}
    JSONObject planCopy()throws Exception{return new JSONObject(originalPlan);}
    JSONObject bindingsCopy()throws Exception{return new JSONObject(sourceBindings);}
}
