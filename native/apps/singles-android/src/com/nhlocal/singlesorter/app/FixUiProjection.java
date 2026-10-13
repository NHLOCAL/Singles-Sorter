package com.nhlocal.singlesorter.app;
import org.json.JSONObject;
/** Observations only. A retained backup does not identify a current source-path filename. */
final class FixUiProjection {
 private FixUiProjection(){}
 static JSONObject flags(JSONObject actual)throws Exception{JSONObject flags=new JSONObject().put("renamed",actual.getBoolean("renamed")).put("repaired",actual.getBoolean("repaired")).put("unchanged",actual.getBoolean("unchanged"));if(actual.getBoolean("renamed")||actual.getBoolean("repaired"))flags.put("observed_filename",actual.getString("name"));return flags;}
 static JSONObject progress(JSONObject previous,String phase,JSONObject evidence)throws Exception{
  JSONObject flags=previous==null?new JSONObject().put("renamed",false).put("repaired",false).put("unchanged",false):new JSONObject(previous.toString());
  if(phase.equals("fix-renamed"))flags.put("renamed",true).put("observed_filename",evidence.getString("name"));
  if(phase.startsWith("fix-backup-"))flags.remove("observed_filename");
  if(phase.equals("destination-observed"))return flags(evidence);return flags;
 }
}
