package com.nhlocal.singlesorter.app;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import org.json.JSONArray;
import org.json.JSONObject;

/** Execution projection of original native operations. No artist/sorting decisions are recreated. */
final class NativeTransferPlan {
    static final class Transfer {
        final int id;final boolean copy;final String displayName;
        private final String source,components;
        private Transfer(int id,boolean copy,String displayName,JSONObject source,JSONArray components){this.id=id;this.copy=copy;this.displayName=displayName;this.source=source.toString();this.components=components.toString();}
        JSONObject source()throws Exception{return new JSONObject(source);}
        JSONArray components()throws Exception{return new JSONArray(components);}
    }
    final List<Transfer> operations;final boolean copyMode;
    private NativeTransferPlan(List<Transfer> operations,boolean copyMode){this.operations=Collections.unmodifiableList(operations);this.copyMode=copyMode;}
    private static void blockers(JSONArray values,boolean global)throws Exception{
        for(int i=0;i<values.length();i++){
            String value=values.getString(i);
            if(!value.equals("MISSING_IDENTITY")&&!(global&&(value.equals("UNKNOWN_CAPABILITY")||value.equals("PHYSICAL_APPLY_UNAVAILABLE"))))throw new IllegalArgumentException("ORIGINAL_PLAN_BLOCKED: "+value);
        }
        // These unchanged core markers are resolved only by the separate Android
        // provider contract (fresh content/scope checks and explicit quiescence), not CAS.
    }
    private static boolean sameReference(JSONObject a,JSONObject b)throws Exception{
        if(a.length()!=1||b.length()!=1||!a.has("AndroidDocument")||!b.has("AndroidDocument"))return false;
        JSONObject x=a.getJSONObject("AndroidDocument"),y=b.getJSONObject("AndroidDocument");
        for(String key:new String[]{"tree_uri","authority","document_id"})if(!x.getString(key).equals(y.getString(key)))return false;
        return x.length()==3&&y.length()==3;
    }
    static NativeTransferPlan prepare(RetainedPlan retained,ProviderReferences references)throws Exception{
        JSONObject plan=retained.planCopy(),request=plan.getJSONObject("request");
        if(plan.getInt("schema_version")!=2||!retained.planId.equals(plan.getString("binding_sha256")))throw new IllegalArgumentException("PLAN_SCHEMA_OR_BINDING");
        if(!"Sort".equals(request.getString("mode")))throw new IllegalArgumentException("ANDROID_FIX_EXECUTION_PENDING");
        blockers(plan.getJSONArray("blockers"),true);
        JSONObject target=request.getJSONObject("target");JSONArray original=plan.getJSONArray("operations");
        if(original.length()==0||original.length()>10000)throw new IllegalArgumentException("OPERATION_COUNT");
        ArrayList<Transfer> operations=new ArrayList<>();long outputBytes=0;
        for(int i=0;i<original.length();i++){
            JSONObject operation=original.getJSONObject(i);if(operation.getInt("id")!=i||!operation.getBoolean("stop_and_replan_on_effect_mismatch"))throw new IllegalArgumentException("OPERATION_ORDER");
            JSONArray dependencies=operation.getJSONArray("depends_on");if(dependencies.length()!=i)throw new IllegalArgumentException("OPERATION_DEPENDENCIES");
            for(int n=0;n<i;n++)if(dependencies.getInt(n)!=n)throw new IllegalArgumentException("OPERATION_DEPENDENCIES");
            JSONObject kind=operation.getJSONObject("kind");if(kind.length()!=1||!kind.has("SingleTransfer"))throw new IllegalArgumentException("ANDROID_ALBUM_EXECUTION_PENDING");
            JSONObject single=kind.getJSONObject("SingleTransfer"),source=single.getJSONObject("source"),transfer=single.getJSONObject("transfer"),destination=transfer.getJSONObject("destination");
            if(!"File".equals(source.getString("kind"))||!sameReference(source.getJSONObject("reference"),transfer.getJSONObject("source").getJSONObject("reference"))||!sameReference(destination.getJSONObject("root"),target)||!transfer.getBoolean("predicted_success"))throw new IllegalArgumentException("TRANSFER_BINDING");
            blockers(transfer.getJSONArray("blockers"),false);JSONArray components=destination.getJSONArray("components");if(components.length()==0||components.length()>128)throw new IllegalArgumentException("DESTINATION_LIMIT");
            for(int n=0;n<components.length();n++)ProviderNames.require(components.getString(n));
            JSONObject binding=references.binding(source.getJSONObject("reference"));long bytes=binding.getLong("bytes");
            if(bytes<0||bytes>64L*1024*1024||!binding.getString("sha256").matches("[0-9a-f]{64}"))throw new IllegalArgumentException("SOURCE_BINDING");
            boolean copy=transfer.getBoolean("copy");if(copy){outputBytes+=bytes;if(outputBytes>2L*1024*1024*1024)throw new IllegalArgumentException("OUTPUT_BUDGET");}
            operations.add(new Transfer(i,copy,source.getString("name"),source.getJSONObject("reference"),components));
        }
        JSONArray effects=plan.getJSONArray("predicted_effects");
        for(int i=0;i<effects.length();i++){JSONObject effect=effects.getJSONObject(i);if(effect.length()!=1||!effect.has("TargetCreated")&&!effect.has("SourceUnavailable"))throw new IllegalArgumentException("ANDROID_EFFECT_EXECUTION_PENDING");}
        references.requireReadable(references.sourceTree);references.requireWritable(references.targetTree);references.requireDisjointRoots();
        for(Transfer operation:operations)if(!operation.copy)references.requireWritable(references.sourceTree);
        return new NativeTransferPlan(operations,request.getJSONObject("settings").getBoolean("copy_mode"));
    }
}
