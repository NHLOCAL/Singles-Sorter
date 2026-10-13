package com.nhlocal.singlesorter.app;
import org.json.JSONArray;
import org.json.JSONObject;
/** Fixed in-memory Android JSON runner. No provider, files, JNI, or mutation calls. */
public final class NativeFixPlanJsonTest {
 private static final String HASH="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";private static int count;private NativeFixPlanJsonTest(){}
 interface Checked {void run()throws Exception;}
 private static void check(boolean ok){if(!ok)throw new AssertionError();count++;}
 private static void rejects(Checked action)throws Exception{try{action.run();throw new AssertionError("accepted invalid fixture");}catch(IllegalArgumentException expected){count++;}}
 static JSONObject ref(String id)throws Exception{return new JSONObject().put("AndroidDocument",new JSONObject().put("tree_uri","content://com.android.externalstorage.documents/tree/primary%3Asongs").put("authority",ProviderReferences.AUTHORITY).put("document_id",id));}
 static JSONObject entry(String token,String parent,String id,String name,boolean directory)throws Exception{return new JSONObject(ref(id).getJSONObject("AndroidDocument").toString()).put("token",token).put("parent",parent==null?JSONObject.NULL:parent).put("name",name).put("directory",directory).put("observed_size",directory?JSONObject.NULL:100).put("observed_modified","1");}
 static JSONArray references()throws Exception{return new JSONArray().put(entry("root",null,"primary:songs","songs",true)).put(entry("file","root","primary:songs/old.mp3","old.mp3",false));}
 static JSONObject bindings()throws Exception{return new JSONObject().put("file",new JSONObject().put("bytes",100).put("sha256",HASH));}
 static JSONObject fixture()throws Exception{
  JSONObject source=new JSONObject().put("reference",ref("primary:songs/old.mp3")).put("identity",JSONObject.NULL).put("parent_identity",JSONObject.NULL).put("name","old.mp3").put("kind","File").put("size",100).put("modified","1").put("capabilities",new JSONObject().put("read",true).put("list",false).put("seek",JSONObject.NULL));
  JSONObject fix=new JSONObject().put("source",source).put("intended_name","new.mp3").put("rename_intended",true).put("repairs",new JSONArray().put(new JSONObject().put("field","Artist").put("original","à").put("replacement","א"))).put("save_intended",true).put("failure",JSONObject.NULL).put("blockers",new JSONArray().put("RenameMustSucceedAndRebindOriginal").put(new JSONObject().put("Observation","MissingIdentity"))).put("executable",false);
  return new JSONObject().put("schema_version",2).put("binding_sha256",HASH).put("request",new JSONObject().put("mode","FixNames").put("source",ref("primary:songs")).put("target",JSONObject.NULL)).put("operations",new JSONArray().put(new JSONObject().put("id",0).put("depends_on",new JSONArray()).put("stop_and_replan_on_effect_mismatch",true).put("kind",new JSONObject().put("FixFile",fix)))).put("blockers",new JSONArray().put("PHYSICAL_APPLY_UNAVAILABLE").put("MISSING_IDENTITY")).put("metadata_observations",new JSONArray()).put("predicted_effects",new JSONArray().put(new JSONObject().put("SourceRenamed",new JSONObject().put("operation",0).put("source",source).put("intended_name","new.mp3"))));
 }
 static JSONObject fix(JSONObject plan)throws Exception{return plan.getJSONArray("operations").getJSONObject(0).getJSONObject("kind").getJSONObject("FixFile");}
 static NativeFixPlan parse(JSONObject plan)throws Exception{return NativeFixPlan.parse(HASH,plan,references(),bindings(),()->false);}
 public static void main(String[] args)throws Exception{
  JSONObject plan=fixture();NativeFixPlan parsed=parse(plan);check(parsed.operations.size()==1&&parsed.operations.get(0).rename);check(plan.toString().equals(fixture().toString()));
  JSONObject bad=fixture();bad.getJSONObject("request").put("target",ref("primary:target"));rejects(()->parse(bad));
  JSONObject blocked=fixture();fix(blocked).getJSONArray("blockers").put("CaseCollisionRequiresProviderResolution");rejects(()->parse(blocked));
  JSONObject missing=fixture();missing.put("predicted_effects",new JSONArray());rejects(()->parse(missing));
  JSONObject failure=fixture();fix(failure).put("failure",new JSONObject().put("Open","Parser")).put("save_intended",false);check(!parse(failure).operations.get(0).save);
  JSONObject invalidSave=fixture();fix(invalidSave).put("save_intended",false);rejects(()->parse(invalidSave));
  JSONObject duplicate=fixture();fix(duplicate).getJSONArray("repairs").put(fix(duplicate).getJSONArray("repairs").getJSONObject(0));rejects(()->parse(duplicate));
  JSONObject nativeResult=new JSONObject().put("source_sha256",HASH).put("staged_sha256",HASH.replace('a','b')).put("media_sha256",HASH.replace('a','c')).put("media_length",80).put("staged_length",113).put("changed_fields",new JSONArray().put("Artist")).put("changes_bytes",true).put("output_file","staged.bin");
  JSONObject output=FixProtocol.nativeEvidence(nativeResult,bindings().getJSONObject("file"),fix(plan).getJSONArray("repairs"));check(output.getLong("bytes")==113);check(!FixProtocol.sameBinding(output,bindings().getJSONObject("file")));
  JSONObject wrongFields=new JSONObject(nativeResult.toString()).put("changed_fields",new JSONArray().put("Title"));rejects(()->FixProtocol.nativeEvidence(wrongFields,bindings().getJSONObject("file"),fix(plan).getJSONArray("repairs")));
  JSONArray refs=references().put(references().getJSONObject(1));rejects(()->new FixBindingIndex(refs,bindings(),()->false));
  rejects(()->FixProtocol.binding(new JSONObject().put("bytes","100").put("sha256",HASH),FixProtocol.ORIGINAL_LIMIT));
  check(FixAcknowledgedOutcome.mayContinue("unsupported_tags",true,false)&&parse(plan).operations.get(0).rename);
  System.out.println(count+" Android retained Fix plan and native evidence checks passed");
 }
}
