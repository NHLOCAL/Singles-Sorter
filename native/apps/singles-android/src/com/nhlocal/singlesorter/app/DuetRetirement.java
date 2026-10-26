package com.nhlocal.singlesorter.app;

import java.io.IOException;
import java.util.Map;
import org.json.JSONArray;
import org.json.JSONObject;

/** Exact live-attempt cohort barrier. The port cannot choose members or infer copy success. */
final class DuetRetirement {
    interface Port {
        void checkpoint()throws Exception;
        JSONObject source(NativeDuetPlan.Group group)throws Exception;
        void destination(int operation,JSONObject receipt)throws Exception;
        boolean remove(NativeDuetPlan.Group group)throws Exception;
        void absent(NativeDuetPlan.Group group)throws Exception;
    }
    interface Journal {void record(String phase,String operation,JSONObject detail)throws Exception;}
    private DuetRetirement(){}
    static JSONObject finish(int id,NativeDuetPlan.Group group,Map<Integer,JSONObject> verified,Port port,Journal journal)throws Exception{
        port.checkpoint();String operation="duet-"+id;
        if(!group.hasRequiredDestinations()){
            JSONObject source=port.source(group);port.checkpoint();JSONObject result=new JSONObject().put("source",source)
                .put("required_operations",new JSONArray()).put("status","no_action").put("source_retained",true).put("source_absent",false);
            journal.record("duet-noop",operation,result);return result;
        }
        JSONArray copies=new JSONArray();
        for(int required:group.requiredOperations){
            port.checkpoint();JSONObject receipt=verified.get(required);
            if(receipt==null||!FixProtocol.bool(receipt,"copy")||!FixProtocol.bool(receipt,"source_retained")||FixProtocol.bool(receipt,"source_absent"))throw new IOException("DUET_REQUIRED_COPY_UNVERIFIED");
            port.destination(required,receipt);
            copies.put(new JSONObject().put("operation",required).put("destination",receipt.getString("destination"))
                .put("name",receipt.getString("name")).put("bytes",receipt.getLong("bytes")).put("sha256",receipt.getString("sha256")));
        }
        JSONObject source=port.source(group);port.checkpoint();
        JSONObject prepared=new JSONObject().put("source",source).put("copies",copies);
        journal.record("duet-prepared",operation,prepared);port.checkpoint();
        // Persisting this intent may fail. No effect is performed before a checked commit.
        journal.record("intent",operation+"/retire-source",source);port.checkpoint();
        boolean removed=port.remove(group);journal.record("returned",operation+"/retire-source",new JSONObject().put("removed",removed));
        if(!removed)throw new IOException("DUET_RETIREMENT_UNCERTAIN");
        port.absent(group);journal.record("observed",operation+"/retire-source",new JSONObject().put("source_absent",true));
        JSONObject result=new JSONObject().put("source",source).put("required_operations",new JSONArray(group.requiredOperations))
            .put("status","retired").put("source_retained",false).put("source_absent",true);
        journal.record("duet-verified",operation,result);return result;
    }
}
