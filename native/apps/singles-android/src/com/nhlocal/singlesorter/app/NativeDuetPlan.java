package com.nhlocal.singlesorter.app;

import java.io.InterruptedIOException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Read-only grouping of original deferred duet intentions.
 * This projection neither resolves blockers nor authorizes a provider effect.
 * Required destinations are exactly the original eligible operation IDs, not
 * every recognized artist. An empty group must retain its source without writes.
 */
final class NativeDuetPlan {
    static final String MARKER = "HISTORICAL_DUET_DELETE_REQUIRES_VERIFIED_ALL_DESTINATIONS";
    static final class Group {
        private final String source;
        final List<Integer> requiredOperations;
        int boundary() { return requiredOperations.isEmpty()?-1:requiredOperations.get(requiredOperations.size()-1); }
        private Group(JSONObject source,List<Integer> required) {
            this.source=source.toString();
            this.requiredOperations=Collections.unmodifiableList(new ArrayList<>(required));
        }
        JSONObject source() throws Exception { return new JSONObject(source); }
        boolean hasRequiredDestinations() { return !requiredOperations.isEmpty(); }
    }
    final List<Group> groups;
    private NativeDuetPlan(List<Group> groups) { this.groups=Collections.unmodifiableList(groups); }

    static NativeDuetPlan prepare(RetainedPlan retained,AtomicBoolean cancel) throws Exception {
        JSONObject plan=retained.planCopy();
        if(!retained.planId.equals(text(plan,"binding_sha256"))) throw bad("BINDING");
        return inspectOriginal(plan,cancel::get);
    }
    // Package-private solely for deterministic original-JSON regression fixtures.
    static NativeDuetPlan inspectOriginal(JSONObject plan,AtomicBoolean cancel) throws Exception { return inspectOriginal(plan,cancel::get); }
    static NativeDuetPlan inspectOriginal(JSONObject plan,BooleanSupplier cancel) throws Exception {
        checkpoint(cancel);
        if(integer(plan.get("schema_version"))!=2) throw bad("SCHEMA");
        JSONObject request=plan.getJSONObject("request"),settings=request.getJSONObject("settings");
        JSONArray operations=plan.getJSONArray("operations"),effects=plan.getJSONArray("predicted_effects");
        if(operations.length()>10000||effects.length()>100000) throw bad("LIMIT");
        HashMap<String,List<Integer>> bySource=new HashMap<>();
        for(int i=0;i<operations.length();i++) {
            checkpoint(cancel);JSONObject op=operations.getJSONObject(i);
            if(integer(op.get("id"))!=i) throw bad("ORDER");
            JSONObject kind=op.getJSONObject("kind");
            if(kind.length()!=1) throw bad("KIND");
            if(!kind.has("SingleTransfer")) continue;
            JSONObject single=kind.getJSONObject("SingleTransfer");
            String key=reference(single.getJSONObject("source").getJSONObject("reference"));
            List<Integer> ids=bySource.get(key);
            if(ids==null) { ids=new ArrayList<>();bySource.put(key,ids); }
            ids.add(i);
        }
        boolean marker=false;
        JSONArray blockers=plan.getJSONArray("blockers");
        for(int i=0;i<blockers.length();i++) {
            checkpoint(cancel);
            Object value=blockers.get(i);if(!(value instanceof String)) throw bad("BLOCKER_TYPE");
            if(MARKER.equals(value)) marker=true;
        }
        ArrayList<Group> groups=new ArrayList<>();HashSet<String> grouped=new HashSet<>();
        for(int i=0;i<effects.length();i++) {
            checkpoint(cancel);JSONObject effect=effects.getJSONObject(i);
            if(effect.length()!=1) throw bad("EFFECT");
            if(!effect.has("DuetDeleteIntention")) continue;
            if(!"Sort".equals(text(request,"mode"))||!bool(settings,"duet_mode")||bool(settings,"copy_mode")||!marker) throw bad("MODE");
            JSONObject intention=effect.getJSONObject("DuetDeleteIntention");
            if(intention.length()!=3||!bool(intention,"blocked")) throw bad("INTENTION");
            JSONObject source=intention.getJSONObject("source");
            if(!"File".equals(text(source,"kind"))) throw bad("SOURCE_KIND");
            String key=reference(source.getJSONObject("reference"));
            if(!grouped.add(key)||groups.size()>=10000) throw bad("DUPLICATE_SOURCE");
            JSONArray required=intention.getJSONArray("after_operations");
            if(required.length()>operations.length()) throw bad("MEMBERSHIP");
            ArrayList<Integer> ids=new ArrayList<>();int previous=-1;
            for(int n=0;n<required.length();n++) {
                checkpoint(cancel);int id=integer(required.get(n));
                if(id<=previous||id>=operations.length()) throw bad("MEMBERSHIP");
                previous=id;JSONObject single=operations.getJSONObject(id).getJSONObject("kind").getJSONObject("SingleTransfer");
                JSONObject observed=single.getJSONObject("source"),transfer=single.getJSONObject("transfer");
                if(!key.equals(reference(observed.getJSONObject("reference")))
                        ||!key.equals(reference(transfer.getJSONObject("source").getJSONObject("reference")))
                        ||!text(source,"name").equals(text(observed,"name"))
                        ||!"File".equals(text(observed,"kind"))
                        ||!bool(transfer,"copy")||!bool(transfer,"predicted_success")) throw bad("TRANSFER");
                if(!sameValue(source,observed,0,new int[]{1024},cancel)
                        ||!sameValue(source,transfer.getJSONObject("source"),0,new int[]{1024},cancel)) throw bad("SOURCE_OBSERVATION");
                ids.add(id);
            }
            List<Integer> all=bySource.get(key);
            if(all==null) all=Collections.emptyList();
            if(!ids.equals(all)) throw bad("INCOMPLETE_MEMBERSHIP");
            groups.add(new Group(source,ids));
        }
        if(marker&&groups.isEmpty()) throw bad("MARKER_WITHOUT_INTENTION");
        if(!bool(settings,"copy_mode"))for(int i=0;i<operations.length();i++){
            checkpoint(cancel);JSONObject kind=operations.getJSONObject(i).getJSONObject("kind");if(!kind.has("SingleTransfer"))continue;
            JSONObject single=kind.getJSONObject("SingleTransfer");if(bool(single.getJSONObject("transfer"),"copy")&&!grouped.contains(reference(single.getJSONObject("source").getJSONObject("reference"))))throw bad("UNGROUPED_COPY");
        }
        checkpoint(cancel);return new NativeDuetPlan(groups);
    }
    private static boolean sameValue(Object a,Object b,int depth,int[] budget,BooleanSupplier cancel) throws Exception {
        checkpoint(cancel);
        if(depth>16||--budget[0]<0) throw bad("SOURCE_BUDGET");
        if(a instanceof JSONObject) {
            if(!(b instanceof JSONObject)) return false;
            JSONObject x=(JSONObject)a,y=(JSONObject)b;
            if(x.length()!=y.length()) return false;
            java.util.Iterator<String> keys=x.keys();
            while(keys.hasNext()) {
                String key=keys.next();
                if(!y.has(key)||!sameValue(x.get(key),y.get(key),depth+1,budget,cancel)) return false;
            }
            return true;
        }
        if(a instanceof JSONArray) {
            if(!(b instanceof JSONArray)) return false;
            JSONArray x=(JSONArray)a,y=(JSONArray)b;
            if(x.length()!=y.length()) return false;
            for(int i=0;i<x.length();i++) if(!sameValue(x.get(i),y.get(i),depth+1,budget,cancel)) return false;
            return true;
        }
        return java.util.Objects.equals(a,b);
    }
    private static String reference(JSONObject reference) throws Exception {
        if(reference.length()!=1||!reference.has("AndroidDocument")) throw bad("REFERENCE");
        JSONObject value=reference.getJSONObject("AndroidDocument");
        if(value.length()!=3) throw bad("REFERENCE");
        JSONArray key=new JSONArray();
        for(String field:new String[]{"tree_uri","authority","document_id"}) {
            String text=text(value,field);
            if(text.isEmpty()||text.length()>16384||text.indexOf(0)>=0) throw bad("REFERENCE");
            key.put(text);
        }
        return key.toString();
    }
    private static String text(JSONObject object,String key) throws Exception {
        Object value=object.get(key);if(!(value instanceof String)) throw bad("STRING_TYPE");return (String)value;
    }
    private static boolean bool(JSONObject object,String key) throws Exception {
        Object value=object.get(key);if(!(value instanceof Boolean)) throw bad("BOOLEAN_TYPE");return (Boolean)value;
    }
    private static int integer(Object value) {
        if(!(value instanceof Integer)&&!(value instanceof Long)) throw bad("INTEGER_TYPE");
        long n=((Number)value).longValue();if(n<0||n>Integer.MAX_VALUE) throw bad("INTEGER_RANGE");return (int)n;
    }
    private static IllegalArgumentException bad(String code) { return new IllegalArgumentException("DUET_"+code); }
    private static void checkpoint(BooleanSupplier cancel) throws InterruptedIOException {
        if(cancel.getAsBoolean()) throw new InterruptedIOException("CANCELLED");
    }
}
