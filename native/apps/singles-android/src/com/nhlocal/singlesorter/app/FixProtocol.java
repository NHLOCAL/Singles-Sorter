package com.nhlocal.singlesorter.app;

import org.json.JSONArray;
import org.json.JSONObject;

/** Original-plan repair evidence, independent original/output lengths, and clean outcome policy. */
final class FixProtocol {
    static final long ORIGINAL_LIMIT=64L*1024*1024,STAGED_LIMIT=72L*1024*1024;
    static final String BACKUP_LEAF="original.backup";
    private FixProtocol(){}
    static boolean bool(JSONObject value,String key)throws Exception{Object raw=value.get(key);if(!(raw instanceof Boolean))throw new IllegalArgumentException("FIX_BOOLEAN");return (Boolean)raw;}
    static long integer(JSONObject value,String key,long limit)throws Exception{
        Object raw=value.get(key);if(!(raw instanceof Integer)&&!(raw instanceof Long))throw new IllegalArgumentException("FIX_INTEGER");long n=((Number)raw).longValue();if(n<0||n>limit)throw new IllegalArgumentException("FIX_BYTE_BOUND");return n;
    }
    static String string(JSONObject value,String key,int limit)throws Exception{Object raw=value.get(key);if(!(raw instanceof String)||((String)raw).length()>limit)throw new IllegalArgumentException("FIX_STRING");return (String)raw;}
    static String hash(JSONObject value,String key)throws Exception{String result=string(value,key,64);if(!result.matches("[0-9a-f]{64}"))throw new IllegalArgumentException("FIX_HASH");return result;}
    static JSONObject binding(JSONObject value,long limit)throws Exception{if(value.length()!=2)throw new IllegalArgumentException("FIX_BINDING_SHAPE");integer(value,"bytes",limit);hash(value,"sha256");return new JSONObject(value.toString());}
    static boolean sameBinding(JSONObject a,JSONObject b)throws Exception{return integer(a,"bytes",STAGED_LIMIT)==integer(b,"bytes",STAGED_LIMIT)&&hash(a,"sha256").equals(hash(b,"sha256"));}
    static JSONObject nativeEvidence(JSONObject value,JSONObject original,JSONArray repairs)throws Exception{
        if(value.length()!=8||!hash(value,"source_sha256").equals(hash(original,"sha256"))||!"staged.bin".equals(string(value,"output_file",128)))throw new IllegalArgumentException("FIX_NATIVE_BINDING");
        long length=integer(value,"staged_length",STAGED_LIMIT),media=integer(value,"media_length",ORIGINAL_LIMIT);hash(value,"media_sha256");String staged=hash(value,"staged_sha256");
        JSONArray changed=value.getJSONArray("changed_fields");if(changed.length()!=repairs.length()||!bool(value,"changes_bytes")||repairs.length()==0||media>length)throw new IllegalArgumentException("FIX_NATIVE_REPAIRS");
        for(int i=0;i<changed.length();i++)if(!changed.getString(i).equals(repairs.getJSONObject(i).getString("field")))throw new IllegalArgumentException("FIX_NATIVE_FIELDS");
        return new JSONObject().put("bytes",length).put("sha256",staged);
    }
    static void repairs(JSONArray repairs)throws Exception{
        if(repairs.length()>5)throw new IllegalArgumentException("FIX_REPAIR_LIMIT");String[] fields={"Artist","AlbumArtist","Title","Album","Genre"};int previous=-1;
        for(int i=0;i<repairs.length();i++){JSONObject repair=repairs.getJSONObject(i);if(repair.length()!=3)throw new IllegalArgumentException("FIX_REPAIR_SHAPE");String field=string(repair,"field",32);int found=-1;for(int n=0;n<fields.length;n++)if(fields[n].equals(field))found=n;
            if(found<=previous)throw new IllegalArgumentException("FIX_REPAIR_ORDER");previous=found;String original=string(repair,"original",131072),replacement=string(repair,"replacement",131072);if(original.equals(replacement))throw new IllegalArgumentException("FIX_REPAIR_UNCHANGED");}
    }
    static boolean cleanLimitation(String status,boolean verified,boolean pending){return FixAcknowledgedOutcome.mayContinue(status,verified,pending);}
    static boolean terminalStatus(String status){return "complete".equals(status)||"unsupported_tags".equals(status)||"metadata_failure".equals(status);}
}
