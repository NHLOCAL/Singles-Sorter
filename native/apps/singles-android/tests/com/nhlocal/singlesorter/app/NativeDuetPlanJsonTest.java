package com.nhlocal.singlesorter.app;

import java.util.concurrent.atomic.AtomicBoolean;
import org.json.JSONArray;
import org.json.JSONObject;

/** Fixed no-argument in-memory Android JSON runner. No provider, files or JNI. */
public final class NativeDuetPlanJsonTest {
    private static int checks;
    private interface Trial { void run() throws Exception; }
    private static void require(boolean value) { if(!value) throw new AssertionError();checks++; }
    private static void refuse(Trial trial,String code) throws Exception {
        try { trial.run();throw new AssertionError("accepted "+code); }
        catch(IllegalArgumentException failure) { require(failure.getMessage().equals("DUET_"+code)); }
    }
    private static JSONObject source(String id) throws Exception {
        return new JSONObject().put("reference",new JSONObject().put("AndroidDocument",new JSONObject()
                .put("tree_uri","content://com.android.externalstorage.documents/tree/primary%3Aqa")
                .put("authority","com.android.externalstorage.documents").put("document_id",id)))
                .put("kind","File").put("name","owned.wav");
    }
    private static JSONObject plan(int count) throws Exception {
        JSONObject p=new JSONObject().put("schema_version",2).put("request",new JSONObject().put("mode","Sort")
                .put("settings",new JSONObject().put("duet_mode",true).put("copy_mode",false)));
        JSONArray ops=new JSONArray(),ids=new JSONArray();
        for(int i=0;i<count;i++) {
            ops.put(new JSONObject().put("id",i).put("kind",new JSONObject().put("SingleTransfer",new JSONObject()
                    .put("source",source("primary:qa/owned.wav")).put("transfer",new JSONObject()
                            .put("source",source("primary:qa/owned.wav")).put("copy",true).put("predicted_success",true)))));
            ids.put(i);
        }
        return p.put("operations",ops).put("blockers",new JSONArray().put(NativeDuetPlan.MARKER))
                .put("predicted_effects",new JSONArray().put(new JSONObject().put("DuetDeleteIntention",new JSONObject()
                        .put("source",source("primary:qa/owned.wav")).put("after_operations",ids).put("blocked",true))));
    }
    private static JSONObject intention(JSONObject p) throws Exception { return p.getJSONArray("predicted_effects").getJSONObject(0).getJSONObject("DuetDeleteIntention"); }
    private static JSONObject transfer(JSONObject p,int id) throws Exception { return p.getJSONArray("operations").getJSONObject(id).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("transfer"); }
    private static NativeDuetPlan inspect(JSONObject p) throws Exception { return NativeDuetPlan.inspectOriginal(p,new AtomicBoolean()); }
    public static void main(String[] args) throws Exception {
        if(args.length!=0) throw new IllegalArgumentException("no arguments");
        for(int count:new int[]{0,1,2}) {
            NativeDuetPlan result=inspect(plan(count));require(result.groups.size()==1);
            require(result.groups.get(0).requiredOperations.size()==count);
            require(result.groups.get(0).hasRequiredDestinations()==(count>0));
        }
        JSONObject p=plan(2);intention(p).put("after_operations",new JSONArray().put(0));final JSONObject missing=p;
        refuse(()->inspect(missing),"INCOMPLETE_MEMBERSHIP");
        p=plan(2);intention(p).put("after_operations",new JSONArray().put(1).put(0));final JSONObject reversed=p;
        refuse(()->inspect(reversed),"MEMBERSHIP");
        p=plan(2);intention(p).put("after_operations",new JSONArray().put(0).put(0));final JSONObject duplicate=p;
        refuse(()->inspect(duplicate),"MEMBERSHIP");
        p=plan(1);intention(p).put("after_operations",new JSONArray().put(2));final JSONObject range=p;
        refuse(()->inspect(range),"MEMBERSHIP");
        p=plan(1);intention(p).put("after_operations",new JSONArray().put("0"));final JSONObject stringId=p;
        refuse(()->inspect(stringId),"INTEGER_TYPE");
        p=plan(1);transfer(p,0).put("copy",false);final JSONObject move=p;refuse(()->inspect(move),"TRANSFER");
        p=plan(1);transfer(p,0).put("copy","true");final JSONObject stringBool=p;refuse(()->inspect(stringBool),"BOOLEAN_TYPE");
        p=plan(1);transfer(p,0).put("predicted_success",false);final JSONObject failed=p;refuse(()->inspect(failed),"TRANSFER");
        p=plan(1);transfer(p,0).put("source",source("primary:qa/other.wav"));final JSONObject foreign=p;refuse(()->inspect(foreign),"TRANSFER");
        p=plan(1);p.getJSONArray("predicted_effects").put(p.getJSONArray("predicted_effects").get(0));final JSONObject twice=p;refuse(()->inspect(twice),"DUPLICATE_SOURCE");
        p=plan(1);p.put("predicted_effects",new JSONArray());final JSONObject noEffect=p;refuse(()->inspect(noEffect),"MARKER_WITHOUT_INTENTION");
        p=plan(1);p.put("blockers",new JSONArray());final JSONObject noMarker=p;refuse(()->inspect(noMarker),"MODE");
        p=plan(1);p.getJSONObject("request").getJSONObject("settings").put("copy_mode",true);final JSONObject copyMode=p;refuse(()->inspect(copyMode),"MODE");
        p=plan(1);p.getJSONObject("request").getJSONObject("settings").put("duet_mode",false);final JSONObject duetOff=p;refuse(()->inspect(duetOff),"MODE");
        p=plan(1);p.getJSONObject("request").put("mode","FixNames");final JSONObject fix=p;refuse(()->inspect(fix),"MODE");
        p=plan(1);intention(p).put("blocked",false);final JSONObject unblocked=p;refuse(()->inspect(unblocked),"INTENTION");
        p=plan(1);p.getJSONArray("operations").getJSONObject(0).put("id",1);final JSONObject order=p;refuse(()->inspect(order),"ORDER");
        p=plan(1);intention(p).getJSONObject("source").getJSONObject("reference").getJSONObject("AndroidDocument").put("authority",3);final JSONObject badRef=p;refuse(()->inspect(badRef),"STRING_TYPE");
        for(String field:new String[]{"size","identity","parent_identity","modified","capabilities","unknown"}) {
            final JSONObject changedObservation=plan(1);
            transfer(changedObservation,0).getJSONObject("source").put(field,123);
            refuse(()->inspect(changedObservation),"SOURCE_OBSERVATION");
        }
        p=plan(1);transfer(p,0).getJSONObject("source").put("name","other.wav");final JSONObject changedName=p;refuse(()->inspect(changedName),"SOURCE_OBSERVATION");
        p=plan(1);p.getJSONArray("operations").getJSONObject(0).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("source").put("size",456);final JSONObject changedOuter=p;refuse(()->inspect(changedOuter),"SOURCE_OBSERVATION");
        p=plan(0);p.put("predicted_effects",new JSONArray()).put("blockers",new JSONArray());require(inspect(p).groups.isEmpty());
        p=plan(1);p.put("predicted_effects",new JSONArray()).put("blockers",new JSONArray());final JSONObject ungrouped=p;refuse(()->inspect(ungrouped),"UNGROUPED_COPY");
        NativeDuetPlan result=inspect(plan(1));JSONObject changed=result.groups.get(0).source();changed.put("name","different");require("owned.wav".equals(result.groups.get(0).source().getString("name")));
        try { result.groups.get(0).requiredOperations.add(99);throw new AssertionError(); } catch(UnsupportedOperationException expected) { checks++; }
        try { NativeDuetPlan.inspectOriginal(plan(1),new AtomicBoolean(true));throw new AssertionError(); } catch(java.io.InterruptedIOException expected) { require("CANCELLED".equals(expected.getMessage())); }
        System.out.println(checks+" original duet grouping JSON checks passed");
    }
}
