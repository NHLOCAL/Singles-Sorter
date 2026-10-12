package com.nhlocal.singlesorter.app;
import org.json.JSONArray;
import org.json.JSONObject;
public final class FixUiContractTest {
 private static final String OTHER_HASH="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
 private static int count;private FixUiContractTest(){}private static void check(boolean value){if(!value)throw new AssertionError();count++;}
 private static JSONObject event(String phase,String step,JSONObject detail,int sequence)throws Exception{return new JSONObject().put("sequence",sequence).put("phase",phase).put("operation","operation-0"+(step==null?"":"/"+step)).put("detail",detail);}
 private static void copyWriteMutations(JSONArray events,String stage,String parent,String owned,JSONObject authority)throws Exception{
  for(int mutation=0;mutation<9;mutation++){
   JSONArray changed=new JSONArray(events.toString());
   switch(mutation){
    case 0:changed.getJSONObject(3).getJSONObject("detail").put("stage",parent);break;
    case 1:changed.remove(4);changed.remove(3);break;
    case 2:changed.remove(4);break;
    case 3:Object intent=changed.get(3);changed.put(3,changed.get(4));changed.put(4,intent);break;
    case 4:changed.getJSONObject(3).getJSONObject("detail").getJSONObject("binding").put("sha256",OTHER_HASH);break;
    case 5:changed.getJSONObject(3).getJSONObject("detail").getJSONObject("binding").put("bytes",99);break;
    case 6:changed.getJSONObject(4).getJSONObject("detail").put("sha256",OTHER_HASH);break;
    case 7:changed.getJSONObject(4).getJSONObject("detail").put("bytes",99);break;
    case 8:JSONArray duplicate=new JSONArray();for(int i=0;i<changed.length();i++){if(i==5){duplicate.put(new JSONObject(changed.getJSONObject(3).toString()));duplicate.put(new JSONObject(changed.getJSONObject(4).toString()));}duplicate.put(changed.get(i));}changed=duplicate;break;
    default:throw new AssertionError();
   }
   for(int i=0;i<changed.length();i++)changed.getJSONObject(i).put("sequence",i);
   check(!ReservedProviderObjects.temporaryRelease(changed,0,stage,parent,owned,"create-stage",authority));
  }
 }
 private static void oldCopyCertificate()throws Exception{
  JSONObject authority=new JSONObject(FixJournalFixtures.AUTHORITY),plan=authority.getJSONObject("original_plan"),request=plan.getJSONObject("request");authority.remove("fix_protocol");authority.remove("fix_backup_leaf");request.put("mode","Sort");
  JSONObject target=new JSONObject(NativeFixPlanJsonTest.ref("primary:target").toString().replace("primary%3Asongs","primary%3Atarget"));request.put("target",target);
  JSONObject source=plan.getJSONArray("operations").getJSONObject(0).getJSONObject("kind").getJSONObject("FixFile").getJSONObject("source");plan.getJSONArray("operations").getJSONObject(0).put("kind",new JSONObject().put("SingleTransfer",new JSONObject().put("source",source)));plan.put("predicted_effects",new JSONArray());
  String parent=FixRecoveryEvidence.uri(target.getJSONObject("AndroidDocument"));String owned=".singles-stage-55555555-5555-5555-5555-555555555555.bin";JSONObject stageRef=new JSONObject(target.toString());stageRef.getJSONObject("AndroidDocument").put("document_id","primary:target/"+owned);String stage=FixRecoveryEvidence.uri(stageRef.getJSONObject("AndroidDocument"));JSONObject finalRef=new JSONObject(target.toString());finalRef.getJSONObject("AndroidDocument").put("document_id","primary:target/final.mp3");String destination=FixRecoveryEvidence.uri(finalRef.getJSONObject("AndroidDocument"));JSONObject binding=authority.getJSONObject("source_bindings").getJSONObject("file");
  JSONArray events=new JSONArray().put(event("intent","create-stage",new JSONObject().put("parent",parent).put("name",owned).put("mime","application/octet-stream"),0)).put(event("returned","create-stage",new JSONObject().put("uri",stage),1)).put(event("observed","create-stage",new JSONObject().put("uri",stage).put("metadata",new JSONObject().put("id","primary:target/"+owned).put("name",owned).put("mime","application/octet-stream")),2));
  events.put(event("intent","write-stage",new JSONObject().put("stage",stage).put("binding",binding),3)).put(event("returned","write-stage",binding,4));
  events.put(event("intent","publish-copy",new JSONObject().put("source",stage).put("parent",parent).put("name","final.mp3"),5)).put(event("returned","publish-copy",new JSONObject().put("uri",destination),6)).put(event("observed","publish-copy",new JSONObject().put("uri",destination).put("metadata",new JSONObject().put("id","primary:target/final.mp3").put("name","final.mp3").put("mime","audio/mpeg")),7));
  events.put(event("verified",null,new JSONObject().put("destination",destination).put("name","final.mp3").put("bytes",100).put("sha256",binding.getString("sha256")).put("copy",true),8));
  check(RetainedJournals.validateEvents(authority,new JSONObject().put("events",events),()->false).acknowledged==1);
  check(ReservedProviderObjects.temporaryRelease(events,0,stage,parent,owned,"create-stage",authority));
  copyWriteMutations(events,stage,parent,owned,authority);
  events.getJSONObject(8).getJSONObject("detail").put("bytes",99);check(!ReservedProviderObjects.temporaryRelease(events,0,stage,parent,owned,"create-stage",authority));events.getJSONObject(8).getJSONObject("detail").put("bytes",100);
  events.getJSONObject(6).getJSONObject("detail").put("uri",stage);check(!ReservedProviderObjects.temporaryRelease(events,0,stage,parent,owned,"create-stage",authority));events.getJSONObject(6).getJSONObject("detail").put("uri",destination);
  events.getJSONObject(7).getJSONObject("detail").getJSONObject("metadata").put("name","adjusted.mp3");check(!ReservedProviderObjects.temporaryRelease(events,0,stage,parent,owned,"create-stage",authority));
  check(!ReservedProviderObjects.temporaryRelease(events,0,stage,parent,owned,"create-fix-backup",authority));
 }
 public static void main(String[] args)throws Exception{
  JSONObject name=new JSONObject().put("name","fixed.mp3");JSONObject flags=FixUiProjection.progress(null,"fix-renamed",name);check(flags.getBoolean("renamed"));check(!flags.getBoolean("repaired"));check(flags.getString("observed_filename").equals("fixed.mp3"));
  for(String phase:new String[]{"fix-backup-started","fix-backup-observed","fix-backup-retained"}){JSONObject backup=FixUiProjection.progress(flags,phase,new JSONObject());check(backup.getBoolean("renamed"));check(!backup.has("observed_filename"));}
  JSONObject actual=new JSONObject().put("name","fixed.mp3").put("renamed",false).put("repaired",true).put("unchanged",false);JSONObject repaired=FixUiProjection.flags(actual);check(!repaired.getBoolean("renamed")&&repaired.getBoolean("repaired"));check(repaired.getString("observed_filename").equals("fixed.mp3"));
  actual.put("repaired",false).put("unchanged",true);check(!FixUiProjection.flags(actual).has("observed_filename"));
  check(RecoveryAdmission.roles(new JSONObject().put("mode","FixNames").put("source",NativeFixPlanJsonTest.ref("primary:songs")).put("target",JSONObject.NULL)).length==1);
  try{RecoveryAdmission.roles(new JSONObject().put("mode","Sort").put("source",NativeFixPlanJsonTest.ref("primary:songs")).put("target",JSONObject.NULL));throw new AssertionError();}catch(java.io.IOException expected){count++;}
  JSONObject authority=new JSONObject(FixJournalFixtures.AUTHORITY),state=new JSONObject(FixJournalFixtures.STATE);JSONArray events=state.getJSONArray("events");JSONObject created=events.getJSONObject(5).getJSONObject("detail");String stage=events.getJSONObject(7).getJSONObject("detail").getString("uri"),parent=created.getString("parent"),owned=created.getString("name");
  check(ReservedProviderObjects.temporaryRelease(events,5,stage,parent,owned,"create-fix-stage",authority));check(!ReservedProviderObjects.temporaryRelease(events,5,stage,parent,owned,"create-fix-backup",authority));
  events.getJSONObject(31).getJSONObject("detail").put("owned_empty_stage_absent",false);check(!ReservedProviderObjects.temporaryRelease(events,5,stage,parent,owned,"create-fix-stage",authority));
  events.getJSONObject(31).getJSONObject("detail").put("owned_empty_stage_absent",true);events.getJSONObject(29).getJSONObject("detail").put("parent",stage);check(!ReservedProviderObjects.temporaryRelease(events,5,stage,parent,owned,"create-fix-stage",authority));
  check(ReadinessDiagnostic.code(new java.io.IOException("RETAINED_PROVIDER_OBJECT")).equals("RETAINED_PROVIDER_OBJECT"));check(ReadinessDiagnostic.code(new java.io.IOException("secret path or provider text")).equals("UNCLASSIFIED_REFUSAL"));
  check(ReadinessDiagnostic.code(new java.io.IOException("ANCESTRY_UNAVAILABLE")).equals("ANCESTRY_UNAVAILABLE"));check(ReadinessDiagnostic.code(new SecurityException("LOCAL_DOCUMENT_REQUIRED")).equals("LOCAL_DOCUMENT_REQUIRED"));
  String uuid="55555555-5555-5555-5555-555555555555";check(ReservedProviderObjects.temporaryName("create-stage",".singles-stage-"+uuid+".bin"));check(ReservedProviderObjects.temporaryName("create-move-stage",".singles-move-"+uuid));check(ReservedProviderObjects.temporaryName("create-fix-stage",".singles-fix-"+uuid));
  check(!ReservedProviderObjects.temporaryName("create-fix-backup",".singles-original-"+uuid));check(!ReservedProviderObjects.temporaryName("create-stage",".singles-stage-not-a-uuid.bin"));check(!ReservedProviderObjects.temporaryName("create-stage",".singles-stage-"+uuid+".mp3"));
  oldCopyCertificate();
  System.out.println(count+" Android Fix UI, root and temporary-release checks passed");
 }
}
