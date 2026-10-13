package com.nhlocal.singlesorter.app;

import java.io.IOException;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** Android JSON source test: exact retained original/projection binding, not provider or JNI proof. */
public final class AlbumPlanJsonTest {
    private static int checks,failures;
    private AlbumPlanJsonTest(){}
    interface Checked {void run()throws Exception;}
    interface Change {void apply(AlbumJournalFixtures.Fixture fixture)throws Exception;}
    private static void test(String name,Checked action)throws Exception{
        try{action.run();checks++;}catch(Exception|AssertionError failure){failures++;System.err.println("FAIL "+name+": "+failure);}
    }
    private static void check(boolean value){if(!value)throw new AssertionError("Unexpected plan result");}
    private static NativeAlbumPlan parse(AlbumJournalFixtures.Fixture fixture)throws Exception{
        return NativeAlbumPlan.parse(AlbumJournalFixtures.HASH,fixture.original(),fixture.projection(),fixture.authority.getJSONArray("references"),fixture.authority.getJSONObject("source_bindings"),()->false);
    }
    private static void reject(String name,AlbumJournalFixtures.Fixture source,Change change)throws Exception{
        test(name,()->{AlbumJournalFixtures.Fixture fixture=source.copy();change.apply(fixture);
            try{parse(fixture);throw new AssertionError("Accepted malformed plan");}catch(IllegalArgumentException|JSONException expected){/* Expected refusal. */}});
    }
    private static JSONObject projected(AlbumJournalFixtures.Fixture fixture)throws Exception{return fixture.projection().getJSONArray("operations").getJSONObject(0);}
    private static JSONObject originalChild(AlbumJournalFixtures.Fixture fixture,int child)throws Exception{return fixture.album().getJSONArray("children").getJSONObject(child);}
    private static JSONObject projectedChild(AlbumJournalFixtures.Fixture fixture,int child)throws Exception{return projected(fixture).getJSONArray("children").getJSONObject(child);}
    private static AlbumJournalFixtures.Fixture selectedRoot(AlbumJournalFixtures.Fixture source)throws Exception{
        String changed=source.authority.toString().replace("primary%3Asongs","primary%3Asongs%2FCollection");AlbumJournalFixtures.Fixture fixture=new AlbumJournalFixtures.Fixture(new JSONObject(changed),source.state);JSONArray refs=fixture.authority.getJSONArray("references"),kept=new JSONArray();
        for(int i=0;i<refs.length();i++){JSONObject ref=refs.getJSONObject(i);if(ref.getString("token").equals("root"))continue;if(ref.getString("token").equals("album"))ref.put("parent",JSONObject.NULL);kept.put(ref);}fixture.authority.put("references",kept);fixture.original().getJSONObject("request").put("source",fixture.album().getJSONObject("source").getJSONObject("reference"));return fixture;
    }
    private static JSONObject ui(AlbumJournalFixtures.Fixture fixture)throws Exception{
        JSONObject result=AlbumJournalFixtures.copy(fixture.events().getJSONObject(fixture.events().length()-1).getJSONObject("detail"));JSONArray children=new JSONArray();for(int i=0;i<fixture.events().length();i++){JSONObject event=fixture.events().getJSONObject(i);if(event.getString("phase").matches("album-child-destination|album-child-skipped"))children.put(event.getJSONObject("detail"));}return result.put("children",children);
    }
    public static void main(String[] args)throws Exception{
        AlbumJournalFixtures.Fixture direct=AlbumJournalFixtures.copyFile(),nested=AlbumJournalFixtures.copyNested(),move=AlbumJournalFixtures.move(true),mixed=AlbumJournalFixtures.mixed(false);
        test("selected Album root Copy retains root",()->{NativeAlbumPlan.Album album=parse(selectedRoot(direct)).get(0);check(album.source.parent==null&&album.copy&&!album.removeEmpty);});
        reject("selected Album root Move cannot erase picker root",selectedRoot(move),f->{});
        test("nested publication uses actual file units",()->{JSONObject value=ui(nested);check(AlbumUiProjection.files(value)==3&&AlbumUiProjection.directories(value)==3&&AlbumUiProjection.movedFiles(value)==0);JSONObject summary=AlbumUiProjection.summary(new JSONObject[]{value},new String[]{"complete"});check(summary.getInt("published_direct_children")==2&&summary.getInt("published_files")==3&&summary.getInt("published_directories")==3);});
        test("move file units and completed album",()->{JSONObject value=ui(move);check(AlbumUiProjection.movedFiles(value)==1&&AlbumUiProjection.summary(new JSONObject[]{value},new String[]{"complete"}).getInt("completed")==1);});
        test("empty no-destination album is not completed",()->check(AlbumUiProjection.summary(new JSONObject[]{ui(AlbumJournalFixtures.zeroChildren())},new String[]{"complete"}).getInt("completed")==0));
        test("direct copy",()->{NativeAlbumPlan plan=parse(direct);check(plan.operations.size()==1&&plan.get(0).copy&&plan.get(0).eligibleCount==1&&!plan.get(0).removeEmpty);});
        test("nested complete capture",()->{NativeAlbumPlan plan=parse(nested);check(plan.get(0).children.size()==2&&plan.tree.complete(plan.get(0).children.get(0).source).size()==5);});
        test("renamed original move",()->{NativeAlbumPlan.Album album=parse(move).get(0);check(!album.copy&&album.removeEmpty&&album.children.get(0).name.equals("song.mp3")&&album.children.get(0).source.name.equals("song?.mp3"));});
        test("mixed original dependencies",()->{NativeAlbumPlan plan=parse(mixed);check(plan.operations.size()==1&&plan.get(0)==null&&plan.get(1)!=null);});
        test("predicted failure preserved",()->{NativeAlbumPlan.Album album=parse(AlbumJournalFixtures.partial(false)).get(0);check(album.eligibleCount==1&&!album.children.get(1).predictedSuccess&&!album.children.get(1).eligible);});
        test("blocked predicted success preserved",()->{NativeAlbumPlan.Child child=parse(AlbumJournalFixtures.blocked()).get(0).children.get(0);check(child.predictedSuccess&&!child.eligible&&child.refusal.contains("ALBUM_SANITIZATION_COLLISION_NO_SUFFIX"));});
        test("all skipped and empty remain zero eligible",()->{check(parse(AlbumJournalFixtures.allSkipped()).get(0).eligibleCount==0);check(parse(AlbumJournalFixtures.zeroChildren()).get(0).eligibleCount==0);});
        test("structural object order",()->{AlbumJournalFixtures.Fixture fixture=direct.copy();JSONObject old=originalChild(fixture,0),reordered=new JSONObject();String[] keys={"blockers","predicted_success","copy","destination","source"};for(String key:keys)reordered.put(key,old.get(key));projectedChild(fixture,0).put("intent",reordered);check(parse(fixture).get(0).eligibleCount==1);});
        test("owned immutable projection",()->{AlbumJournalFixtures.Fixture fixture=direct.copy();NativeAlbumPlan plan=parse(fixture);projectedChild(fixture,0).getJSONObject("intent").put("predicted_success",false);check(plan.get(0).children.get(0).intent().getBoolean("predicted_success"));});
        test("cancelled before admission",()->{try{NativeAlbumPlan.parse(AlbumJournalFixtures.HASH,direct.original(),direct.projection(),direct.authority.getJSONArray("references"),direct.authority.getJSONObject("source_bindings"),()->true);throw new AssertionError("Ignored cancellation");}catch(IOException expected){check("cancelled".equals(expected.getMessage()));}});

        reject("wrong original binding",direct,f->f.original().put("binding_sha256",AlbumJournalFixtures.HASH.replace('a','b')));
        reject("wrong projection binding",direct,f->f.projection().put("plan_id",AlbumJournalFixtures.HASH.replace('a','b')));
        reject("non Sort mode",direct,f->f.original().getJSONObject("request").put("mode","FixNames"));
        reject("wrong schema",direct,f->f.original().put("schema_version",1));
        reject("removed global blocker",direct,f->f.projection().put("blockers",new JSONArray()));
        reject("unresolved global blocker",direct,f->{f.original().getJSONArray("blockers").put("UNKNOWN_SOURCE");f.projection().getJSONArray("blockers").put("UNKNOWN_SOURCE");});
        reject("missing projection",direct,f->f.projection().put("operations",new JSONArray()));
        reject("extra projection",direct,f->f.projection().getJSONArray("operations").put(AlbumJournalFixtures.copy(projected(f))));
        reject("wrong projection operation",direct,f->projected(f).put("id",1));
        reject("projection dependency altered",mixed,f->projected(f).put("depends_on",new JSONArray()));
        reject("original dependency altered",mixed,f->f.original().getJSONArray("operations").getJSONObject(1).put("depends_on",new JSONArray()));
        reject("both dependency copies altered",mixed,f->{projected(f).put("depends_on",new JSONArray());f.original().getJSONArray("operations").getJSONObject(1).put("depends_on",new JSONArray());});
        reject("projection stop marker disabled",direct,f->projected(f).put("stop_and_replan_on_effect_mismatch",false));
        reject("original stop marker disabled",direct,f->f.original().getJSONArray("operations").getJSONObject(0).put("stop_and_replan_on_effect_mismatch",false));
        reject("changed projection copy mode",direct,f->projected(f).put("copy_mode",false));
        reject("changed child copy",direct,f->{originalChild(f,0).put("copy",false);projectedChild(f,0).getJSONObject("intent").put("copy",false);});
        reject("changed projected source",direct,f->projected(f).getJSONObject("source").put("name","Other"));
        reject("changed projected destination",direct,f->projected(f).getJSONObject("destination").getJSONArray("components").put(1,"Other"));
        reject("changed borrowed child intent",direct,f->projectedChild(f,0).getJSONObject("intent").put("predicted_success",false));
        reject("planned suffix invented",direct,f->projectedChild(f,0).put("planned_name","song (1).mp3"));
        reject("name comparison changed",direct,f->projectedChild(f,0).put("name_changes",true));
        reject("missing direct child projection",direct,f->projected(f).put("children",new JSONArray()));
        reject("wrong child order",nested,f->{JSONArray values=projected(f).getJSONArray("children");Object first=values.get(0);values.put(0,values.get(1));values.put(1,first);});
        reject("duplicate direct source",nested,f->{JSONObject second=AlbumJournalFixtures.copy(originalChild(f,0));f.album().getJSONArray("children").put(1,second);projected(f).getJSONArray("children").put(1,AlbumJournalFixtures.copy(projectedChild(f,0)));});
        reject("copy cannot remove original album",direct,f->{f.album().put("remove_empty_source",true);projected(f).put("remove_empty_source",true);});
        reject("nested listing incomplete",nested,f->f.authority.getJSONArray("references").getJSONObject(4).put("directory_children_complete",false));
        reject("album listing missing",direct,f->f.authority.getJSONArray("references").getJSONObject(1).remove("directory_children_complete"));
        reject("listing string is not boolean",direct,f->f.authority.getJSONArray("references").getJSONObject(1).put("directory_children_complete","true"));
        reject("unbound source content",direct,f->f.authority.getJSONObject("source_bindings").remove("song"));
        reject("foreign content binding",direct,f->f.authority.getJSONObject("source_bindings").put("foreign",new JSONObject().put("bytes",100).put("sha256",AlbumJournalFixtures.HASH)));
        reject("duplicate captured reference",direct,f->f.authority.getJSONArray("references").put(AlbumJournalFixtures.copy(f.authority.getJSONArray("references").getJSONObject(2))));
        reject("cyclic captured parent",direct,f->f.authority.getJSONArray("references").getJSONObject(1).put("parent","album"));
        reject("missing captured parent",direct,f->f.authority.getJSONArray("references").getJSONObject(2).put("parent","missing"));
        reject("source removal omits captured direct child",move,f->{JSONObject extra=AlbumJournalFixtures.copy(f.authority.getJSONArray("references").getJSONObject(2));extra.put("token","extra").put("document_id","primary:songs/Collection/extra.mp3").put("name","extra.mp3");f.authority.getJSONArray("references").put(extra);f.authority.getJSONObject("source_bindings").put("extra",new JSONObject().put("bytes",100).put("sha256",AlbumJournalFixtures.HASH));});
        if(failures>0)throw new AssertionError(failures+" Album plan checks failed; "+checks+" passed");
        System.out.println(checks+" Android retained Album plan JSON checks passed");
    }
}
