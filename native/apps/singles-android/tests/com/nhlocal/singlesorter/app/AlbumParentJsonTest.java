package com.nhlocal.singlesorter.app;

import java.io.IOException;
import java.util.Iterator;
import org.json.JSONArray;
import org.json.JSONObject;

/** Fixed Android-runtime JSON checks only. No picker, grants, provider or filesystem effects. */
public final class AlbumParentJsonTest {
    private static int checks,failures;private AlbumParentJsonTest(){}
    interface Checked{void run()throws Exception;}
    interface Change{void run(AlbumJournalFixtures.Fixture fixture)throws Exception;}
    private static void check(boolean value){if(!value)throw new AssertionError();}
    private static void test(String label,Checked action)throws Exception{try{action.run();checks++;}catch(Exception|AssertionError error){failures++;System.err.println("FAIL "+label+": "+error);}}
    private static JSONObject parent(AlbumJournalFixtures.Fixture fixture)throws Exception{return fixture.authority.getJSONObject("album_root_parent");}
    private static NativeAlbumPlan plan(AlbumJournalFixtures.Fixture f)throws Exception{return NativeAlbumPlan.parse(AlbumJournalFixtures.HASH,f.original(),f.projection(),f.authority.getJSONArray("references"),f.authority.getJSONObject("source_bindings"),()->false,f.authority.optJSONObject("album_root_parent"));}
    private static RetainedJournals.EvidenceCounts parse(AlbumJournalFixtures.Fixture f)throws Exception{return RetainedJournals.validateEvents(f.authority,f.state,()->false);}
    private static void reject(String label,AlbumJournalFixtures.Fixture original,Change change)throws Exception{test(label,()->{AlbumJournalFixtures.Fixture f=original.copy();change.run(f);try{parse(f);throw new AssertionError("Accepted malformed parent history");}catch(IOException|IllegalArgumentException|org.json.JSONException expected){}});}
    private static Object rebind(Object input,String selected)throws Exception{
        if(input instanceof JSONObject){JSONObject out=new JSONObject();Iterator<String> keys=((JSONObject)input).keys();while(keys.hasNext()){String key=keys.next();out.put(key,rebind(((JSONObject)input).get(key),selected));}return out;}
        if(input instanceof JSONArray){JSONArray out=new JSONArray(),values=(JSONArray)input;for(int n=0;n<values.length();n++)out.put(rebind(values.get(n),selected));return out;}
        if(input instanceof String){String text=(String)input;if(text.equals(AlbumJournalFixtures.SOURCE_TREE))return selected;if(text.startsWith(AlbumJournalFixtures.SOURCE_TREE+"/document/"))return selected+text.substring(AlbumJournalFixtures.SOURCE_TREE.length());}return input;
    }
    static AlbumJournalFixtures.Fixture fixture(boolean renamed)throws Exception{
        AlbumJournalFixtures.Fixture old=AlbumJournalFixtures.move(renamed);String selected=AlbumJournalFixtures.SOURCE_TREE+"%2FCollection";
        AlbumJournalFixtures.Fixture f=new AlbumJournalFixtures.Fixture((JSONObject)rebind(old.authority,selected),(JSONObject)rebind(old.state,selected));JSONArray refs=f.authority.getJSONArray("references"),kept=new JSONArray();
        for(int n=0;n<refs.length();n++){JSONObject entry=refs.getJSONObject(n);if(entry.getString("token").equals("root"))continue;if(entry.getString("token").equals("album"))entry.put("parent",JSONObject.NULL);kept.put(entry);}f.authority.put("references",kept);JSONObject source=f.album().getJSONObject("source").getJSONObject("reference");f.original().getJSONObject("request").put("source",source);
        JSONObject evidence=new JSONObject().put("schema",1).put("source_reference",source).put("source_name","Collection").put("parent_tree",AlbumJournalFixtures.SOURCE_TREE).put("parent_document_id","primary:songs").put("parent_name","songs").put("parent_modified","1")
            .put("children",new JSONArray().put(child("primary:songs/Collection","Collection",true)).put(child("primary:songs/Other","Other",true)));f.authority.put("album_root_parent",evidence);
        int cleanup=AlbumJournalFixtures.find(f,"intent","source-album");AlbumJournalFixtures.detail(f,cleanup).put("role","source-album-root").put("parent",AlbumJournalFixtures.uri(false,"primary:songs")).put("owned_empty_directory",AlbumJournalFixtures.uri(false,"primary:songs/Collection"));AlbumJournalFixtures.detail(f,cleanup+2).put("exact_parent_membership_without_source",true);return f;
    }
    private static JSONObject child(String id,String name,boolean directory)throws Exception{return new JSONObject().put("id",id).put("name",name).put("directory",directory);}
    private static JSONObject report(JSONObject parent,JSONArray children,String status)throws Exception{return new JSONObject().put("status",status).put("parent_tree",parent.getString("parent_tree")).put("parent_document_id",parent.getString("parent_document_id")).put("source_document_id","primary:songs/Collection").put("source_name","Collection").put("children",children).put("historical_success_changed",false);}
    private static void prefixes(String label,AlbumJournalFixtures.Fixture source)throws Exception{for(int count=0;count<=source.events().length();count++){final int length=count;test(label+" prefix "+count,()->{AlbumJournalFixtures.Fixture f=source.copy();JSONArray events=new JSONArray();for(int n=0;n<length;n++)events.put(f.events().getJSONObject(n));f.state.put("events",events).put("status","running");RetainedJournals.EvidenceCounts result=parse(f);check(result.acknowledged==(length==source.events().length()?1:0));});}}
    public static void main(String[] args)throws Exception{
        AlbumJournalFixtures.Fixture direct=fixture(false),renamed=fixture(true);prefixes("root unchanged Move",direct);prefixes("root renamed Move",renamed);
        test("original root policy retained",()->{NativeAlbumPlan.Album a=plan(direct).get(0);check(a.source.parent==null&&a.removeEmpty&&!a.copy&&a.eligibleCount==1);});
        test("uncertain root deletion never reports retained root",()->{JSONObject album=new JSONObject().put("copy",false).put("source_removed",false).put("created_directories",2).put("removed_directories",0).put("destination",JSONObject.NULL).put("children",new JSONArray());check(AlbumUiProjection.summary(new JSONObject[]{album},new String[]{"needs_recovery"}).getInt("retained_directories")==0);check(AlbumUiProjection.summary(new JSONObject[]{album},new String[]{"partial"}).getInt("retained_directories")==1);});
        test("no extra event budget",()->{NativeAlbumPlan p=plan(renamed);check(AlbumEffectBudget.album(p.get(0),p.tree,()->false)==renamed.events().length());});
        test("complete parent order is not authority",()->{JSONArray before=parent(direct).getJSONArray("children"),after=new JSONArray().put(before.get(1)).put(before.get(0));AlbumRootParent.sameChildren(before,after,null,()->false);});
        test("only exact source member removed",()->{JSONArray before=parent(direct).getJSONArray("children");AlbumRootParent.sameChildren(before,new JSONArray().put(before.get(1)),"primary:songs/Collection",()->false);});
        test("captured parent is immutable",()->{JSONObject original=parent(direct),copy=AlbumRootParent.validate(original,direct.original(),()->false);copy.put("parent_name","Changed");check(original.getString("parent_name").equals("songs"));});
        test("membership cancellation",()->{try{AlbumRootParent.sameChildren(parent(direct).getJSONArray("children"),new JSONArray(),null,()->true);throw new AssertionError();}catch(IOException expected){check(expected.getMessage().equals("cancelled"));}});
        test("validation cancellation",()->{try{AlbumRootParent.validate(parent(direct),direct.original(),()->true);throw new AssertionError();}catch(IOException expected){check(expected.getMessage().equals("cancelled"));}});
        test("opaque parent URI owns an independent receipt bound",()->{JSONObject oversized=AlbumJournalFixtures.copy(parent(direct));StringBuilder id=new StringBuilder();for(int n=0;n<3600;n++)id.append('a');oversized.put("parent_tree","content://"+ProviderReferences.AUTHORITY+"/tree/"+id);try{AlbumRootParent.uriBudget(oversized);throw new AssertionError();}catch(IOException expected){check(expected.getMessage().equals("ALBUM_PARENT_URI_BOUND"));}});
        reject("missing explicit parent",direct,f->f.authority.remove("album_root_parent"));reject("null parent",direct,f->f.authority.put("album_root_parent",JSONObject.NULL));
        reject("wrong parent schema",direct,f->parent(f).put("schema",2));reject("parent ID is source",direct,f->parent(f).put("parent_document_id","primary:songs/Collection"));
        reject("wrong original source",direct,f->parent(f).put("source_reference",AlbumJournalFixtures.ref(false,"primary:songs")));
        reject("foreign parent provider",direct,f->parent(f).put("parent_tree",parent(f).getString("parent_tree").replace(ProviderReferences.AUTHORITY,"foreign.provider")));
        reject("parent query",direct,f->parent(f).put("parent_tree",parent(f).getString("parent_tree")+"?grant=1"));reject("parent descendant document URI",direct,f->parent(f).put("parent_tree",AlbumJournalFixtures.uri(false,"primary:songs")));
        reject("missing immediate source",direct,f->parent(f).put("children",new JSONArray()));reject("source name changed",direct,f->parent(f).put("source_name","Other"));
        reject("source became file",direct,f->parent(f).getJSONArray("children").getJSONObject(0).put("directory",false));reject("parent metadata unknown",direct,f->parent(f).put("parent_modified",JSONObject.NULL));
        reject("duplicate ID",direct,f->parent(f).getJSONArray("children").put(AlbumJournalFixtures.copy(parent(f).getJSONArray("children").getJSONObject(0))));
        reject("case ambiguous sibling",direct,f->parent(f).getJSONArray("children").getJSONObject(1).put("name","collection"));reject("NUL ID",direct,f->parent(f).getJSONArray("children").getJSONObject(1).put("id","bad\u0000id"));
        reject("unknown parent field",direct,f->parent(f).put("path_inferred",true));
        reject("old cleanup role is not root authority",direct,f->AlbumJournalFixtures.detail(f,AlbumJournalFixtures.find(f,"intent","source-album-root")).put("role","source-album"));
        reject("wrong cleanup scope",direct,f->AlbumJournalFixtures.detail(f,AlbumJournalFixtures.find(f,"intent","source-album-root")).put("parent",AlbumJournalFixtures.uri(true,"primary:sorted")));
        reject("cleanup missing exact parent after evidence",direct,f->AlbumJournalFixtures.detail(f,AlbumJournalFixtures.find(f,"intent","source-album-root")+2).remove("exact_parent_membership_without_source"));
        reject("cleanup parent difference cannot acknowledge",direct,f->AlbumJournalFixtures.detail(f,AlbumJournalFixtures.find(f,"intent","source-album-root")+2).put("exact_parent_membership_without_source",false));
        JSONObject p=parent(direct);JSONArray siblings=new JSONArray().put(p.getJSONArray("children").get(1));
        for(String status:new String[]{"source_present","source_absent","source_name_reused","source_identity_changed"}){final String expected=status;test("visible "+status,()->{JSONArray current=new JSONArray(siblings.toString());if(expected.equals("source_present"))current.put(child("primary:songs/Collection","Collection",true));if(expected.equals("source_name_reused"))current.put(child("primary:songs/new","Collection",true));if(expected.equals("source_identity_changed"))current.put(child("primary:songs/Collection","Changed",false));AlbumRootParent.validateMembership(p,report(p,current,expected),()->false);check(!AlbumParentRecovery.description(expected).isEmpty());});}
        test("absent cannot claim historical success",()->{JSONObject report=report(p,siblings,"source_absent").put("historical_success_changed",true);try{AlbumRootParent.validateMembership(p,report,()->false);throw new AssertionError();}catch(IOException expected){}});
        test("missing member cannot claim present",()->{try{AlbumRootParent.validateMembership(p,report(p,siblings,"source_present"),()->false);throw new AssertionError();}catch(IOException expected){}});
        test("review bound to exact parent",()->{JSONObject review=new JSONObject().put("album_root_parent",p).put("source_parent_tree",p.getString("parent_tree")).put("source_parent_observation",report(p,siblings,"source_absent"));check(AlbumParentRecovery.requireBinding(direct.authority,review,()->false));review.put("source_parent_tree",AlbumJournalFixtures.TARGET_TREE);try{AlbumParentRecovery.requireBinding(direct.authority,review,()->false);throw new AssertionError();}catch(IllegalArgumentException expected){}});
        test("old Single or Fix review cannot gain parent scope",()->{try{AlbumParentRecovery.requireBinding(new JSONObject(),new JSONObject().put("source_parent_tree",AlbumJournalFixtures.SOURCE_TREE),()->false);throw new AssertionError();}catch(IllegalArgumentException expected){}});
        test("foreign sibling cannot disappear",()->{try{AlbumRootParent.sameChildren(p.getJSONArray("children"),new JSONArray(),"primary:songs/Collection",()->false);throw new AssertionError();}catch(IOException expected){}});
        test("foreign sibling cannot be renamed",()->{try{AlbumRootParent.sameChildren(p.getJSONArray("children"),new JSONArray().put(child("primary:songs/Other","Changed",true)),"primary:songs/Collection",()->false);throw new AssertionError();}catch(IOException expected){}});
        if(failures>0)throw new AssertionError(failures+" Album parent checks failed; "+checks+" passed");System.out.println(checks+" explicit Album parent authority and recovery JSON checks passed");
    }
}
