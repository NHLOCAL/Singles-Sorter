package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.content.SharedPreferences;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import org.json.JSONArray;
import org.json.JSONObject;

/** App-private settings/list state. Native host is the only authority; strings stay data. */
final class AndroidProfile {
    private static final String[] KEYS={"copy_mode","main_folder_only","singles_folder","exist_only","abc_sort","duet_mode"};
    private final SharedPreferences prefs;
    AndroidProfile(Context context){prefs=context.getSharedPreferences("singles-profile-v1",Context.MODE_PRIVATE);}
    synchronized JSONObject settings()throws Exception {
        JSONObject result=new JSONObject();for(String key:KEYS)result.put(key,prefs.getBoolean(key,"singles_folder".equals(key)));return result;
    }
    static void validateSettings(JSONObject value,boolean duet)throws Exception {
        if(value.length()!=(duet?6:5))throw new IllegalArgumentException("SETTINGS_SHAPE");
        for(String key:KEYS)if(duet||!key.equals("duet_mode"))if(!(value.get(key) instanceof Boolean))throw new IllegalArgumentException("SETTINGS_BOOLEAN");
    }
    synchronized JSONObject save(JSONObject settings)throws Exception {
        validateSettings(settings,false);SharedPreferences.Editor editor=prefs.edit();
        for(String key:KEYS)if(!key.equals("duet_mode"))editor.putBoolean(key,settings.getBoolean(key));
        if(!editor.commit())throw new IllegalStateException("SETTINGS_SAVE_FAILED");return settings();
    }
    synchronized JSONObject duet(boolean value)throws Exception {
        if(!prefs.edit().putBoolean("duet_mode",value).commit())throw new IllegalStateException("SETTINGS_SAVE_FAILED");return settings();
    }
    synchronized JSONObject personal()throws Exception {return present(new JSONArray(prefs.getString("personal","[]")));}
    private static JSONObject present(JSONArray rows)throws Exception {
        JSONArray diagnostics=new JSONArray();
        for(int i=0;i<rows.length();i++)if(rows.getJSONArray(i).length()!=2)diagnostics.put(new JSONObject().put("index",i).put("code","MALFORMED_ROW").put("width",rows.getJSONArray(i).length()));
        byte[] digest=MessageDigest.getInstance("SHA-256").digest(rows.toString().getBytes(StandardCharsets.UTF_8));
        StringBuilder revision=new StringBuilder();for(byte b:digest)revision.append(String.format(java.util.Locale.ROOT,"%02x",b&255));
        return new JSONObject().put("revision",revision.toString()).put("rows",rows).put("diagnostics",diagnostics);
    }
    synchronized JSONObject savePersonal(String revision,JSONArray rows)throws Exception {
        if(!revision.equals(personal().getString("revision")))throw new IllegalStateException("PERSONAL_LIST_CHANGED");
        if(rows.length()>10000||rows.toString().getBytes(StandardCharsets.UTF_8).length>2*1024*1024)throw new IllegalArgumentException("PERSONAL_LIST_LIMIT");
        for(int i=0;i<rows.length();i++){JSONArray row=rows.getJSONArray(i);for(int j=0;j<row.length();j++)if(!(row.get(j) instanceof String)||row.getString(j).length()>131072)throw new IllegalArgumentException("PERSONAL_FIELD_LIMIT");}
        if(!prefs.edit().putString("personal",rows.toString()).commit())throw new IllegalStateException("PERSONAL_SAVE_FAILED");return personal();
    }
    synchronized String personalCsv()throws Exception {
        JSONArray rows=personal().getJSONArray("rows");StringBuilder result=new StringBuilder();
        for(int i=0;i<rows.length();i++){JSONArray row=rows.getJSONArray(i);for(int j=0;j<row.length();j++){if(j>0)result.append(',');result.append('"').append(row.getString(j).replace("\"","\"\"")).append('"');}result.append('\n');}return result.toString();
    }
    synchronized JSONObject firstRun(boolean mark)throws Exception {
        if(mark&&!prefs.edit().putBoolean("seen-v1",true).commit())throw new IllegalStateException("PROFILE_SAVE_FAILED");
        return new JSONObject().put("version","1").put("seen",prefs.getBoolean("seen-v1",false));
    }
}
