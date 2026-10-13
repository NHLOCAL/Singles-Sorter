package com.nhlocal.singlesorter.app;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.List;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Private projection of retained original FixFile decisions. No renderer-derived decisions. */
final class NativeFixPlan {
    static final class Fix {
        final int id;final String originalName,intendedName;final boolean rename,save;private final String original,binding;
        Fix(int id,JSONObject fix,JSONObject binding)throws Exception{this.id=id;original=fix.toString();this.binding=binding.toString();originalName=fix.getJSONObject("source").getString("name");intendedName=fix.getString("intended_name");rename=FixProtocol.bool(fix,"rename_intended");save=FixProtocol.bool(fix,"save_intended");}
        JSONObject original()throws Exception{return new JSONObject(original);}
        JSONObject source()throws Exception{return original().getJSONObject("source").getJSONObject("reference");}
        JSONObject binding()throws Exception{return new JSONObject(binding);}
        JSONArray repairs()throws Exception{return original().getJSONArray("repairs");}
        Object failure()throws Exception{return original().get("failure");}
    }
    final List<Fix> operations;
    private NativeFixPlan(List<Fix> operations){this.operations=Collections.unmodifiableList(operations);}
    static NativeFixPlan parse(String planId,JSONObject plan,JSONArray references,JSONObject bindings,BooleanSupplier cancelled)throws Exception{
        JournalExportBounds.checkpoint(cancelled);JSONObject request=plan.getJSONObject("request");JSONArray operations=plan.getJSONArray("operations");
        if(plan.getInt("schema_version")!=2||!planId.equals(plan.getString("binding_sha256"))||!"FixNames".equals(request.getString("mode"))||!request.isNull("target")||operations.length()==0||operations.length()>10000)throw new IllegalArgumentException("FIX_PLAN_SHAPE");
        FixBindingIndex index=new FixBindingIndex(references,bindings,cancelled);JSONObject root=index.entry(request.getJSONObject("source"));if(!root.getBoolean("directory")||!root.isNull("parent"))throw new IllegalArgumentException("FIX_SOURCE_ROOT");
        ArrayList<Fix> result=new ArrayList<>();HashMap<String,JSONObject> failures=new HashMap<>();
        for(int i=0;i<operations.length();i++){JournalExportBounds.checkpoint(cancelled);JSONObject operation=operations.getJSONObject(i);NativeTransferPlan.requireOrder(operation,i);JSONObject kind=operation.getJSONObject("kind");if(kind.length()!=1||!kind.has("FixFile"))throw new IllegalArgumentException("FIX_OPERATION_REQUIRED");JSONObject fix=kind.getJSONObject("FixFile"),source=fix.getJSONObject("source"),reference=source.getJSONObject("reference"),entry=index.entry(reference);
            if(!"File".equals(source.getString("kind"))||entry.getBoolean("directory")||!source.getString("name").equals(entry.getString("name"))||!source.get("size").toString().equals(entry.get("observed_size").toString())||!source.get("modified").toString().equals(entry.get("observed_modified").toString())||!entry.getString("tree_uri").equals(root.getString("tree_uri")))throw new IllegalArgumentException("FIX_SOURCE_BINDING");index.parent(reference);index.requireRoot(reference,root,cancelled);
            if(fix.isNull("intended_name"))throw new IllegalArgumentException("FIX_NAME_MISSING");String name=FixProtocol.string(fix,"intended_name",255);ProviderNames.require(name);boolean rename=FixProtocol.bool(fix,"rename_intended"),save=FixProtocol.bool(fix,"save_intended");if(rename==name.equals(source.getString("name")))throw new IllegalArgumentException("FIX_RENAME_BINDING");
            JSONArray repairs=fix.getJSONArray("repairs");FixProtocol.repairs(repairs);boolean failed=!fix.isNull("failure");if(save==failed)throw new IllegalArgumentException("FIX_SAVE_BINDING");if(failed)failures.put(FixBindingIndex.key(FixBindingIndex.document(reference)),fix.getJSONObject("failure"));
            JSONArray blockers=fix.getJSONArray("blockers");for(int n=0;n<blockers.length();n++){Object raw=blockers.get(n);boolean allowed=raw instanceof String&&raw.equals("RenameMustSucceedAndRebindOriginal")&&rename;if(raw instanceof JSONObject){JSONObject marker=(JSONObject)raw;allowed=marker.length()==1&&marker.has("Observation")&&marker.getString("Observation").matches("PhysicalApplyUnavailable|MissingIdentity|UnknownCapability");}if(!allowed)throw new IllegalArgumentException("ORIGINAL_FIX_BLOCKED: "+raw);}
            result.add(new Fix(i,fix,index.binding(reference)));
        }
        JSONArray blockers=plan.getJSONArray("blockers");for(int i=0;i<blockers.length();i++){String blocker=blockers.getString(i);if(blocker.equals("METADATA_ADAPTER_LIMITATION")){requireMetadataFailures(plan.getJSONArray("metadata_observations"),failures,cancelled);}else NativeTransferPlan.blockers(new JSONArray().put(blocker),true);}
        JSONArray effects=plan.getJSONArray("predicted_effects");if(effects.length()>operations.length())throw new IllegalArgumentException("FIX_EFFECT_LIMIT");boolean[] renamed=new boolean[operations.length()];for(int i=0;i<effects.length();i++){JournalExportBounds.checkpoint(cancelled);JSONObject effect=effects.getJSONObject(i);if(effect.length()!=1||!effect.has("SourceRenamed"))throw new IllegalArgumentException("FIX_EFFECT_UNSUPPORTED");JSONObject e=effect.getJSONObject("SourceRenamed");int id=(int)FixProtocol.integer(e,"operation",operations.length()-1);Fix fix=result.get(id);if(!fix.rename||renamed[id]||!fix.intendedName.equals(e.getString("intended_name"))||!fix.original().getJSONObject("source").toString().equals(e.getJSONObject("source").toString()))throw new IllegalArgumentException("FIX_EFFECT_BINDING");renamed[id]=true;}
        for(Fix fix:result)if(fix.rename!=renamed[fix.id])throw new IllegalArgumentException("FIX_RENAME_EFFECT_MISSING");JournalExportBounds.checkpoint(cancelled);return new NativeFixPlan(result);
    }
    private static void requireMetadataFailures(JSONArray observations,HashMap<String,JSONObject> failures,BooleanSupplier cancelled)throws Exception{
        if(observations.length()>100000)throw new IllegalArgumentException("FIX_METADATA_LIMIT");boolean found=false;
        for(int i=0;i<observations.length();i++){JournalExportBounds.checkpoint(cancelled);JSONObject observation=observations.getJSONObject(i);if(!observation.has("Open"))continue;JSONObject open=observation.getJSONObject("Open"),result=open.getJSONObject("result");if(!result.has("Err"))continue;Object error=result.get("Err");if(error instanceof JSONObject&&((JSONObject)error).has("LegacyOpen"))continue;
            JSONObject failure=failures.get(FixBindingIndex.key(FixBindingIndex.document(open.getJSONObject("source").getJSONObject("reference"))));if(failure==null||failure.length()!=1||!failure.has("Open")||!failure.get("Open").toString().equals(error.toString()))throw new IllegalArgumentException("ORIGINAL_METADATA_BLOCKER_UNRESOLVED");found=true;}
        if(!found)throw new IllegalArgumentException("ORIGINAL_METADATA_BLOCKER_UNRESOLVED");
    }
    static NativeFixPlan prepare(RetainedPlan retained,ProviderReferences references,BooleanSupplier cancelled)throws Exception{NativeFixPlan plan=parse(retained.planId,retained.planCopy(),retained.referencesCopy(),retained.bindingsCopy(),cancelled);if(references.targetTree!=null)throw new IllegalArgumentException("FIX_UNEXPECTED_TARGET");references.requireWritable(references.sourceTree);return plan;}
}
