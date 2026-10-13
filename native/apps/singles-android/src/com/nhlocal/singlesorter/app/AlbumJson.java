package com.nhlocal.singlesorter.app;

import java.util.Iterator;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Bounded structural equality. JSON object ordering is not provenance. */
final class AlbumJson {
    private AlbumJson(){}
    static boolean same(Object left,Object right,BooleanSupplier cancelled)throws Exception{return same(left,right,cancelled,0,new int[]{0});}
    private static boolean same(Object left,Object right,BooleanSupplier cancelled,int depth,int[] visits)throws Exception{
        JournalExportBounds.checkpoint(cancelled);if(depth>150||++visits[0]>200000)throw new IllegalArgumentException("ALBUM_JSON_BOUND");
        if(left==JSONObject.NULL||right==JSONObject.NULL)return left==right;
        if(left instanceof JSONObject&&right instanceof JSONObject){JSONObject a=(JSONObject)left,b=(JSONObject)right;if(a.length()!=b.length())return false;Iterator<String> keys=a.keys();while(keys.hasNext()){String key=keys.next();if(!b.has(key)||!same(a.get(key),b.get(key),cancelled,depth+1,visits))return false;}return true;}
        if(left instanceof JSONArray&&right instanceof JSONArray){JSONArray a=(JSONArray)left,b=(JSONArray)right;if(a.length()!=b.length())return false;for(int i=0;i<a.length();i++)if(!same(a.get(i),b.get(i),cancelled,depth+1,visits))return false;return true;}
        return left!=null&&left.equals(right);
    }
}
