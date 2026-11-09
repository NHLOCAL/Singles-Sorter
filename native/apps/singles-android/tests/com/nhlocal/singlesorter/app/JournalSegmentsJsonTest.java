package com.nhlocal.singlesorter.app;

import java.io.IOException;
import org.json.JSONArray;
import org.json.JSONObject;

/** Executes on Android's JSON runtime, like the other journal evidence tests. */
public final class JournalSegmentsJsonTest {
    private JournalSegmentsJsonTest(){}
    private static JSONArray events(int first,int count)throws Exception{JSONArray result=new JSONArray();for(int i=0;i<count;i++)result.put(new JSONObject().put("sequence",first+i));return result;}
    private static JSONObject segment(int first,int count)throws Exception{return new JSONObject().put("schema",1).put("execution_id","x").put("first",first).put("events",events(first,count));}
    private static JSONObject entry(int first,int count)throws Exception{return new JSONObject().put("first",first).put("count",count).put("sha256","h");}
    private static JSONObject state()throws Exception{return new JSONObject().put("execution_id","x");}
    private static void refused(JSONArray entries,JSONArray tail,RetainedJournals.SegmentSource source)throws Exception{try{RetainedJournals.merge(state(),entries,tail,source,()->false);}catch(IOException expected){return;}throw new AssertionError("refusal expected");}
    public static void main(String[] args)throws Exception{
        RetainedJournals.SegmentSource good=(name,hash)->{int first=Integer.parseInt(name.substring(7,13));return segment(first,128);};
        JSONArray two=new JSONArray().put(entry(0,128)).put(entry(128,128));
        JSONObject merged=RetainedJournals.merge(state(),two,events(256,5),good,()->false);JSONArray all=merged.getJSONArray("events");
        if(all.length()!=261)throw new AssertionError();for(int i=0;i<all.length();i++)if(all.getJSONObject(i).getInt("sequence")!=i)throw new AssertionError();
        if(!ProviderEffectJournal.segmentName(128).equals("events-000128.json"))throw new AssertionError();
        refused(new JSONArray().put(entry(128,128)),events(256,1),good);
        refused(two,events(300,1),good);
        refused(two,new JSONArray(),(name,hash)->segment(0,127));
        refused(two,new JSONArray(),(name,hash)->new JSONObject(segment(0,128).toString()).put("execution_id","other"));
        if(FixAdmissionBounds.effectEvents(false,0)!=3||FixAdmissionBounds.effectEvents(true,0)!=FixAdmissionBounds.WORST_CASE_EVENTS||FixAdmissionBounds.effectEvents(false,1)!=FixAdmissionBounds.WORST_CASE_EVENTS)throw new AssertionError();
        if(2000*FixAdmissionBounds.WORST_CASE_EVENTS>ProviderEffectJournal.MAX_EVENTS)throw new AssertionError("a full traversal must fit");
        System.out.println("Journal segment continuity and Fix admission checks passed");
    }
}
