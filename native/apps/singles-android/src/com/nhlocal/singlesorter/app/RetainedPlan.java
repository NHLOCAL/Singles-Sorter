package com.nhlocal.singlesorter.app;

import android.os.CancellationSignal;
import java.nio.charset.StandardCharsets;
import org.json.JSONArray;
import org.json.JSONObject;

/** Native-owned original decisions and source content bindings; never sent to the WebView.
 * A digest binds this preview only. Every provider effect still requires fresh checks.
 */
final class RetainedPlan {
    final String jobId,planId;
    private final String originalPlan,sourceBindings;
    private RetainedPlan(String job,String id,String plan,String bindings){jobId=job;planId=id;originalPlan=plan;sourceBindings=bindings;}
    static RetainedPlan capture(String job,JSONObject result,SafSnapshot snapshots,CancellationSignal cancel)throws Exception{
        String id=result.getString("plan_id"),encoded=result.getString("_retained_plan");
        if(encoded.getBytes(StandardCharsets.UTF_8).length>8*1024*1024)throw new IllegalArgumentException("PLAN_LIMIT");
        JSONObject plan=new JSONObject(encoded);
        if(!id.equals(plan.getString("binding_sha256"))||plan.getJSONArray("operations").length()>10000)throw new IllegalArgumentException("PLAN_BINDING");
        JSONArray tokens=result.getJSONArray("_source_tokens");if(tokens.length()>2000)throw new IllegalArgumentException("SOURCE_LIMIT");
        JSONObject bindings=new JSONObject();
        for(int n=0;n<tokens.length();n++){
            cancel.throwIfCanceled();String token=tokens.getString(n);if(bindings.has(token))throw new IllegalArgumentException("DUPLICATE_SOURCE");
            JSONObject snapshot=new JSONObject(new String(snapshots.snapshot(token.getBytes(StandardCharsets.UTF_8)),StandardCharsets.UTF_8));
            // Deliberately retain no cache path: private snapshots are deleted before completion.
            bindings.put(token,new JSONObject().put("sha256",snapshot.getString("sha256")).put("bytes",snapshot.getLong("snapshot_size")));
        }
        cancel.throwIfCanceled();return new RetainedPlan(job,id,encoded,bindings.toString());
    }
    JSONObject planCopy()throws Exception{return new JSONObject(originalPlan);}
    JSONObject bindingsCopy()throws Exception{return new JSONObject(sourceBindings);}
}
