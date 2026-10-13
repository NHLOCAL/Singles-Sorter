package com.nhlocal.singlesorter.app;

import android.net.Uri;
import android.provider.DocumentsContract;
import java.util.HashMap;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Historical cohort receipts only. Missing delete returns never become success or retry authority. */
final class DuetRecoveryEvidence {
    final int acknowledged,unresolved,retired,noAction,pending;
    private DuetRecoveryEvidence(int acknowledged,int unresolved,int retired,int noAction,int pending){this.acknowledged=acknowledged;this.unresolved=unresolved;this.retired=retired;this.noAction=noAction;this.pending=pending;}
    static DuetRecoveryEvidence validate(JSONObject authority,JSONObject state,BooleanSupplier cancelled)throws Exception{
        if(FixProtocol.integer(authority,"duet_protocol",1)!=1||!FixProtocol.bool(authority,"provider_quiescence_required")||FixProtocol.bool(authority,"provider_atomic_batch"))throw invalid();
        JSONObject original=authority.getJSONObject("original_plan"),request=original.getJSONObject("request");
        if(!"Sort".equals(request.getString("mode"))||!authority.getString("plan_id").equals(original.getString("binding_sha256")))throw invalid();
        NativeDuetPlan plan=NativeDuetPlan.inspectOriginal(original,cancelled);if(plan.groups.isEmpty())throw invalid();
        NativeTransferPlan.blockers(original.getJSONArray("blockers"),true,true);
        FixBindingIndex bindings=new FixBindingIndex(authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),cancelled);
        JSONObject root=bindings.entry(request.getJSONObject("source"));JSONArray operations=original.getJSONArray("operations"),events=state.getJSONArray("events"),ordinary=new JSONArray();
        JSONObject[] sources=new JSONObject[plan.groups.size()],prepared=new JSONObject[plan.groups.size()];int[] phases=new int[plan.groups.size()];boolean[] removed=new boolean[plan.groups.size()];
        HashMap<Integer,JSONObject> copies=new HashMap<>();HashMap<Integer,DuetCopyEvidence> certificates=new HashMap<>();
        java.util.HashSet<Integer> requiredCopies=new java.util.HashSet<>();for(NativeDuetPlan.Group group:plan.groups)requiredCopies.addAll(group.requiredOperations);int ordinaryAcknowledged=0,retired=0,noAction=0;
        if(events.length()>10000)throw invalid();
        for(int id=0;id<plan.groups.size();id++){
            JournalExportBounds.checkpoint(cancelled);NativeDuetPlan.Group group=plan.groups.get(id);JSONObject source=group.source(),ref=source.getJSONObject("reference"),entry=bindings.entry(ref),binding=bindings.binding(ref);
            bindings.requireRoot(ref,root,cancelled);if(FixProtocol.bool(entry,"directory")||!source.getString("name").equals(entry.getString("name")))throw invalid();
            sources[id]=new JSONObject().put("source",FixRecoveryEvidence.uri(entry)).put("parent",FixRecoveryEvidence.uri(bindings.parent(ref)))
                .put("name",source.getString("name")).put("bytes",binding.getLong("bytes")).put("sha256",binding.getString("sha256"));
        }
        for(int i=0;i<events.length();i++){
            JournalExportBounds.checkpoint(cancelled);JSONObject event=events.getJSONObject(i);if(FixProtocol.integer(event,"sequence",9999)!=i)throw invalid();
            String operation=FixProtocol.string(event,"operation",100),phase=FixProtocol.string(event,"phase",40);JSONObject detail=event.getJSONObject("detail");
            if(operation.startsWith("duet-")){
                if(!operation.matches("duet-[0-9]{1,5}(/retire-source)?"))throw invalid();int slash=operation.indexOf('/'),id=Integer.parseInt(operation.substring(5,slash<0?operation.length():slash));
                if(id>=plan.groups.size())throw invalid();NativeDuetPlan.Group group=plan.groups.get(id);
                if(group.boundary()!=ordinaryAcknowledged-1||nextDue(plan,phases,ordinaryAcknowledged-1)!=id)throw invalid();
                if(phase.equals("duet-noop")){
                    if(slash>=0||phases[id]!=0||group.hasRequiredDestinations())throw invalid();result(detail,sources[id],group,false,cancelled);phases[id]=5;noAction++;continue;
                }
                if(!group.hasRequiredDestinations())throw invalid();
                if(phase.equals("duet-prepared")){
                    if(slash>=0||phases[id]!=0||detail.length()!=2||!sameSourceEvidence(sources[id],detail.getJSONObject("source")))throw invalid();
                    JSONArray observed=detail.getJSONArray("copies");if(observed.length()!=group.requiredOperations.size())throw invalid();
                    for(int n=0;n<observed.length();n++){
                        int required=group.requiredOperations.get(n);JSONObject actual=observed.getJSONObject(n),verified=copies.get(required);
                        if(actual.length()!=5||FixProtocol.integer(actual,"operation",9999)!=required||verified==null)throw invalid();
                        for(String key:new String[]{"destination","name","bytes","sha256"})if(!actual.get(key).equals(verified.get(key)))throw invalid();
                    }
                    prepared[id]=detail;phases[id]=1;continue;
                }
                if(phase.equals("intent")){
                    if(slash<0||phases[id]!=1||!sameSourceEvidence(sources[id],detail))throw invalid();phases[id]=2;continue;
                }
                if(phase.equals("returned")){
                    if(slash<0||phases[id]!=2||detail.length()!=1)throw invalid();removed[id]=FixProtocol.bool(detail,"removed");phases[id]=3;continue;
                }
                if(phase.equals("observed")){
                    if(slash<0||phases[id]!=3||!removed[id]||detail.length()!=1||!FixProtocol.bool(detail,"source_absent"))throw invalid();phases[id]=4;continue;
                }
                if(phase.equals("duet-verified")){
                    if(slash>=0||phases[id]!=4||prepared[id]==null)throw invalid();result(detail,sources[id],group,true,cancelled);phases[id]=5;retired++;continue;
                }
                throw invalid();
            }
            if(nextDue(plan,phases,ordinaryAcknowledged-1)>=0||!operation.matches("operation-[0-9]{1,5}(/[a-z0-9-]+)?"))throw invalid();
            int slash=operation.indexOf('/'),id=Integer.parseInt(operation.substring(10,slash<0?operation.length():slash));if(id!=ordinaryAcknowledged||id>=operations.length())throw invalid();
            if(requiredCopies.contains(id)){
                DuetCopyEvidence certificate=certificates.get(id);if(certificate==null){certificate=new DuetCopyEvidence(operations.getJSONObject(id).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("transfer"),request.getJSONObject("target"),bindings,cancelled);certificates.put(id,certificate);}
                certificate.event(phase,slash<0?null:operation.substring(slash+1),detail);
            }
            if(!phase.equals("duet-copy-start")&&!phase.equals("duet-parent-observed"))ordinary.put(new JSONObject(event.toString()).put("sequence",ordinary.length()));
            else if(!requiredCopies.contains(id))throw invalid();
            if(phase.equals("verified")||phase.equals("album-verified")){
                if(slash>=0)throw invalid();NativeTransferPlan.requireOrder(operations.getJSONObject(id),id);
                if(phase.equals("verified")){
                    JSONObject transfer=operations.getJSONObject(id).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("transfer");
                    if(requiredCopies.contains(id)){checkCopy(detail,transfer,request,bindings);copies.put(id,new JSONObject(detail.toString()));}
                }
                ordinaryAcknowledged++;
            }
        }
        JSONObject ordinaryState=new JSONObject(state.toString()).put("events",ordinary);
        RetainedJournals.EvidenceCounts base=RetainedJournals.validateNonDuetEvents(authority,ordinaryState,cancelled);
        if(base.acknowledged!=ordinaryAcknowledged)throw invalid();int pending=plan.groups.size()-retired-noAction;
        if("completed".equals(state.getString("status"))&&(base.unresolved!=0||pending!=0))throw invalid();
        JournalExportBounds.checkpoint(cancelled);return new DuetRecoveryEvidence(base.acknowledged,base.unresolved,retired,noAction,pending);
    }
    private static int nextDue(NativeDuetPlan plan,int[] phases,int boundary){
        for(int id=0;id<plan.groups.size();id++)if(plan.groups.get(id).boundary()==boundary&&phases[id]!=5)return id;return -1;
    }
    private static void result(JSONObject actual,JSONObject source,NativeDuetPlan.Group group,boolean retired,BooleanSupplier cancelled)throws Exception{
        if(actual.length()!=5||!sameSourceEvidence(source,actual.getJSONObject("source"))||!AlbumJson.same(new JSONArray(group.requiredOperations),actual.getJSONArray("required_operations"),cancelled)
            ||!(retired?"retired":"no_action").equals(actual.getString("status"))||FixProtocol.bool(actual,"source_retained")==retired||FixProtocol.bool(actual,"source_absent")!=retired)throw invalid();
    }
    /** Producer getLong values and reparsed JSON integers describe the same bounded bytes. */
    static boolean sameSourceEvidence(JSONObject expected,JSONObject actual)throws Exception{
        if(expected.length()!=5||actual.length()!=5)return false;
        for(String key:new String[]{"source","parent","name"}){Object left=expected.get(key),right=actual.get(key);if(!(left instanceof String)||!(right instanceof String)||!left.equals(right))return false;}
        return FixProtocol.integer(expected,"bytes",FixProtocol.ORIGINAL_LIMIT)==FixProtocol.integer(actual,"bytes",FixProtocol.ORIGINAL_LIMIT)
            &&FixProtocol.hash(expected,"sha256").equals(FixProtocol.hash(actual,"sha256"));
    }
    private static void checkCopy(JSONObject actual,JSONObject transfer,JSONObject request,FixBindingIndex bindings)throws Exception{
        JSONObject source=transfer.getJSONObject("source").getJSONObject("reference"),destination=transfer.getJSONObject("destination");
        JSONArray path=destination.getJSONArray("components");if(path.length()<1||!NativeTransferPlan.sameReference(destination.getJSONObject("root"),request.getJSONObject("target"))
            ||!FixProtocol.bool(transfer,"copy")||!FixProtocol.bool(transfer,"predicted_success")||!FixProtocol.bool(actual,"copy")||!FixProtocol.bool(actual,"source_retained")||FixProtocol.bool(actual,"source_absent"))throw invalid();
        NativeTransferPlan.blockers(transfer.getJSONArray("blockers"),false);for(int n=0;n<path.length();n++)ProviderNames.require(path.getString(n));
        if(!path.getString(path.length()-1).equals(actual.getString("name"))||!FixProtocol.sameBinding(bindings.binding(source),actual))throw invalid();
        String raw=FixProtocol.string(actual,"destination",8192);Uri uri=Uri.parse(raw),tree=Uri.parse(FixBindingIndex.document(request.getJSONObject("target")).getString("tree_uri"));
        if(!"content".equals(uri.getScheme())||!ProviderReferences.AUTHORITY.equals(uri.getAuthority())||uri.getQuery()!=null||uri.getFragment()!=null||!DocumentsContract.isTreeUri(uri)
            ||!DocumentsContract.getTreeDocumentId(uri).equals(DocumentsContract.getTreeDocumentId(tree)))throw invalid();DocumentsContract.getDocumentId(uri);
    }
    private static IllegalArgumentException invalid(){return new IllegalArgumentException("DUET_RECOVERY_EVIDENCE");}
}
