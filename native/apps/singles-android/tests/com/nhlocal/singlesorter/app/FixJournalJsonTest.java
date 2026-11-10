package com.nhlocal.singlesorter.app;
import org.json.JSONArray;
import org.json.JSONObject;
/** Exact producer JSON parsing with Android JSONObject semantics, no provider/filesystem/JNI. */
public final class FixJournalJsonTest {
 private static int checks;private FixJournalJsonTest(){}interface Change {void apply(JSONObject authority,JSONObject state)throws Exception;}
 private static JSONObject authority()throws Exception{return new JSONObject(FixJournalFixtures.AUTHORITY);}
 private static JSONObject state()throws Exception{return new JSONObject(FixJournalFixtures.STATE);}
 private static void check(boolean ok){if(!ok)throw new AssertionError();checks++;}
 private static void reject(Change change)throws Exception{JSONObject a=authority(),s=state();change.apply(a,s);try{RetainedJournals.validateEvents(a,s,()->false);throw new AssertionError("accepted invalid history");}catch(IllegalArgumentException|org.json.JSONException expected){checks++;}}
 private static JSONObject detail(JSONObject state,int index)throws Exception{return state.getJSONArray("events").getJSONObject(index).getJSONObject("detail");}
 private static void remove(JSONObject state,int first,int count)throws Exception{JSONArray old=state.getJSONArray("events"),next=new JSONArray();for(int i=0;i<old.length();i++)if(i<first||i>=first+count){JSONObject value=old.getJSONObject(i);value.put("sequence",next.length());next.put(value);}state.put("events",next);}
 private static void mixed(String outcome)throws Exception{
  JSONObject a=authority(),s=state(),plan=a.getJSONObject("original_plan");JSONArray ops=plan.getJSONArray("operations"),effects=plan.getJSONArray("predicted_effects");
  JSONObject second=new JSONObject(ops.getJSONObject(0).toString().replace("old.mp3","old2.mp3").replace("new.mp3","new2.mp3"));second.put("id",1).put("depends_on",new JSONArray().put(0));ops.put(second);
  JSONObject effect=new JSONObject(effects.getJSONObject(0).toString().replace("old.mp3","old2.mp3").replace("new.mp3","new2.mp3"));effect.getJSONObject("SourceRenamed").put("operation",1);effects.put(effect);
  JSONObject entry=new JSONObject(a.getJSONArray("references").getJSONObject(1).toString().replace("old.mp3","old2.mp3"));entry.put("token","file2");a.getJSONArray("references").put(entry);a.getJSONObject("source_bindings").put("file2",a.getJSONObject("source_bindings").getJSONObject("file"));
  Object failure=outcome.equals("metadata_failure")?new JSONObject().put("Open","Parser"):JSONObject.NULL;Object error=outcome.equals("unsupported_tags")?"Unsupported(\"fixture\")":JSONObject.NULL;
  if(failure!=JSONObject.NULL)ops.getJSONObject(0).getJSONObject("kind").getJSONObject("FixFile").put("failure",failure).put("save_intended",false);
  JSONArray original=s.getJSONArray("events"),events=new JSONArray();for(int i=0;i<5;i++)events.put(new JSONObject(original.getJSONObject(i).toString()));
  events.getJSONObject(0).getJSONObject("detail").put("outcome",outcome).put("native_evidence",JSONObject.NULL).put("metadata_failure",failure).put("native_error",error);
  JSONObject result=new JSONObject(original.getJSONObject(32).getJSONObject("detail").toString()).put("status",outcome).put("bytes",100).put("sha256",a.getJSONObject("source_bindings").getJSONObject("file").getString("sha256")).put("backup",JSONObject.NULL).put("backup_bytes",0).put("repaired",false).put("metadata_failure",failure).put("native_error",error);
  events.put(new JSONObject().put("sequence",5).put("phase","verified").put("operation","operation-0").put("detail",result));
  for(int i=0;i<original.length();i++){JSONObject event=new JSONObject(original.getJSONObject(i).toString().replace("old.mp3","old2.mp3").replace("new.mp3","new2.mp3").replace("operation-0","operation-1"));event.put("sequence",events.length());events.put(event);}
  s.put("events",events).put("status","partial");RetainedJournals.EvidenceCounts counts=RetainedJournals.validateEvents(a,s,()->false);check(counts.acknowledged==2&&counts.unresolved==0);
 }
 public static void main(String[] args)throws Exception{
  RetainedJournals.EvidenceCounts completed=RetainedJournals.validateEvents(authority(),state(),()->false);check(completed.fix&&completed.acknowledged==1&&completed.unresolved==0);
  JSONArray events=state().getJSONArray("events");for(int length=0;length<events.length();length++){JSONObject s=state();JSONArray partial=new JSONArray();for(int i=0;i<length;i++)partial.put(events.get(i));s.put("events",partial).put("status","running");RetainedJournals.EvidenceCounts count=RetainedJournals.validateEvents(authority(),s,()->false);check(count.acknowledged==0&&count.unresolved==1);}
  reject((a,s)->detail(s,13).put("uri",detail(s,0).getString("source")));reject((a,s)->detail(s,13).remove("uri"));
  reject((a,s)->detail(s,11).getJSONObject("binding").put("bytes",100));reject((a,s)->detail(s,20).getJSONObject("binding").put("sha256",new String(new char[64]).replace('\0','b')));
  reject((a,s)->detail(s,17).put("from",detail(s,17).getString("to")));reject((a,s)->detail(s,25).put("source",detail(s,21).getString("source")));
  reject((a,s)->detail(s,24).put("uri",detail(s,20).getString("uri")));reject((a,s)->detail(s,0).getJSONObject("native_evidence").put("staged_length",100));
  reject((a,s)->remove(s,29,3));reject((a,s)->{JSONObject verified=s.getJSONArray("events").getJSONObject(32);s.getJSONArray("events").put(29,verified.put("sequence",29));});
  reject((a,s)->remove(s,20,1));reject((a,s)->remove(s,24,1));reject((a,s)->a.put("fix_backup_leaf","old.mp3"));
  for(int index:new int[]{3,7,10,16,19,23,27}){final int n=index;reject((a,s)->detail(s,n).remove("metadata"));}
  reject((a,s)->detail(s,27).getJSONObject("metadata").put("name","new (1).mp3"));reject((a,s)->detail(s,26).put("uri",detail(s,23).getString("uri")));
  reject((a,s)->a.getJSONArray("references").put(a.getJSONArray("references").getJSONObject(1)));reject((a,s)->a.getJSONObject("source_bindings").put("foreign",a.getJSONObject("source_bindings").getJSONObject("file")));
  JSONObject mismatch=state();JSONArray partial=new JSONArray();for(int i=0;i<28;i++)partial.put(mismatch.getJSONArray("events").get(i));partial.getJSONObject(27).put("phase","effect-mismatch").put("detail",new JSONObject().put("actual_name","new (1).mp3"));mismatch.put("events",partial).put("status","uncertain");check(RetainedJournals.validateEvents(authority(),mismatch,()->false).unresolved==1);
  mixed("unsupported_tags");mixed("metadata_failure");
  System.out.println(checks+" Android reconstructed Fix journal parser checks passed");
 }
}
