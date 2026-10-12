package com.nhlocal.singlesorter.app;
import java.nio.charset.StandardCharsets;
import java.util.function.BooleanSupplier;
import org.json.JSONObject;
/** Conservative pre-effect admission below the existing fixed journal and byte ceilings. */
final class FixAdmissionBounds {
 private FixAdmissionBounds(){}
 static void require(RetainedPlan retained,NativeFixPlan plan,BooleanSupplier cancelled)throws Exception{
  JSONObject authority=new JSONObject().put("original_plan",retained.planCopy()).put("references",retained.referencesCopy()).put("source_bindings",retained.bindingsCopy());if(authority.toString().getBytes(StandardCharsets.UTF_8).length>15*1024*1024)throw new IllegalArgumentException("FIX_AUTHORITY_BUDGET");
  if(plan.operations.size()>300)throw new IllegalArgumentException("FIX_EVENT_BUDGET");FixBindingIndex index=new FixBindingIndex(retained.referencesCopy(),retained.bindingsCopy(),cancelled);long estimated=4096,bytes=0;
  for(NativeFixPlan.Fix fix:plan.operations){JournalExportBounds.checkpoint(cancelled);String parent=FixRecoveryEvidence.uri(index.parent(fix.source()));long uriBytes=Math.max(parent.getBytes(StandardCharsets.UTF_8).length+1024,FixRecoveryEvidence.uri(index.entry(fix.source())).getBytes(StandardCharsets.UTF_8).length+512);if(uriBytes>8192)throw new IllegalArgumentException("FIX_URI_BUDGET");long failure=fix.failure().toString().getBytes(StandardCharsets.UTF_8).length;if(failure>32768)throw new IllegalArgumentException("FIX_ERROR_BUDGET");estimated+=32768+128*uriBytes+4*failure;bytes+=fix.binding().getLong("bytes");if(estimated>7L*1024*1024||bytes>512L*1024*1024)throw new IllegalArgumentException("FIX_JOB_BUDGET");}
  JournalExportBounds.checkpoint(cancelled);
 }
}
