package com.nhlocal.singlesorter.app;

import java.io.IOException;
import java.util.HashMap;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** API-30-compatible in-memory reconstruction tests. No provider, files, emulator, JNI or mutation calls. */
public final class AlbumJournalJsonTest {
    private static int checks,failures;
    private AlbumJournalJsonTest(){}
    interface Checked {void run()throws Exception;}
    interface Change {void apply(AlbumJournalFixtures.Fixture fixture)throws Exception;}
    private static void test(String name,Checked action)throws Exception{
        try{action.run();checks++;}catch(Exception|AssertionError failure){failures++;System.err.println("FAIL "+name+": "+failure);}
    }
    private static void check(boolean value){if(!value)throw new AssertionError("Unexpected recovery acknowledgment");}
    private static RetainedJournals.EvidenceCounts parse(AlbumJournalFixtures.Fixture fixture)throws Exception{return RetainedJournals.validateEvents(fixture.authority,fixture.state,()->false);}
    private static void reject(String name,AlbumJournalFixtures.Fixture source,Change change)throws Exception{
        test(name,()->{AlbumJournalFixtures.Fixture fixture=source.copy();change.apply(fixture);
            try{parse(fixture);throw new AssertionError("Accepted malformed history");}catch(IllegalArgumentException|JSONException|IOException expected){/* Expected refusal. */}});
    }
    private static int find(AlbumJournalFixtures.Fixture fixture,String phase,String role)throws Exception{return AlbumJournalFixtures.find(fixture,phase,role);}
    private static JSONObject detail(AlbumJournalFixtures.Fixture fixture,int index)throws Exception{return AlbumJournalFixtures.detail(fixture,index);}
    private static JSONObject result(AlbumJournalFixtures.Fixture fixture)throws Exception{return detail(fixture,find(fixture,"album-child-destination",null));}
    private static JSONObject last(AlbumJournalFixtures.Fixture fixture)throws Exception{return detail(fixture,fixture.events().length()-1);}
    private static void prefix(AlbumJournalFixtures.Fixture fixture,int length)throws Exception{
        JSONArray events=new JSONArray();for(int i=0;i<length;i++)events.put(AlbumJournalFixtures.copy(fixture.events().getJSONObject(i)));
        fixture.state.put("events",events).put("status","running");
    }
    /** Keep receipt counters producer-shaped after deleting a whole effect, to test semantic evidence. */
    private static void removeEffect(AlbumJournalFixtures.Fixture fixture,String role)throws Exception{
        AlbumJournalFixtures.remove(fixture,find(fixture,"intent",role),3);HashMap<String,String> renamed=new HashMap<>();int step=0;
        for(int i=0;i<fixture.events().length();i++){
            JSONObject event=fixture.events().getJSONObject(i);String operation=event.getString("operation");int slash=operation.indexOf('/');
            if(slash<0)continue;String replacement=renamed.get(operation);
            if(replacement==null){replacement=operation.substring(0,slash)+"/album-step-"+step++;renamed.put(operation,replacement);}event.put("operation",replacement);
        }
    }
    private static void append(AlbumJournalFixtures.Fixture fixture,String phase,JSONObject value)throws Exception{
        fixture.events().put(new JSONObject().put("sequence",fixture.events().length()).put("phase",phase).put("operation","operation-0").put("detail",value));
    }
    private static void prefixes(String label,AlbumJournalFixtures.Fixture fixture)throws Exception{
        int acknowledged=0,operations=fixture.original().getJSONArray("operations").length();
        for(int length=0;length<=fixture.events().length();length++){
            if(length>0){String phase=fixture.events().getJSONObject(length-1).getString("phase");if(phase.equals("album-verified")||phase.equals("verified"))acknowledged++;}
            final int expected=acknowledged,end=length;
            test(label+" prefix "+length,()->{AlbumJournalFixtures.Fixture cut=fixture.copy();prefix(cut,end);RetainedJournals.EvidenceCounts count=parse(cut);check(!count.fix&&count.acknowledged==expected&&count.unresolved==operations-expected);});
        }
    }
    public static void main(String[] args)throws Exception{
        AlbumJournalFixtures.Fixture direct=AlbumJournalFixtures.copyFile(),nested=AlbumJournalFixtures.copyNested(),move=AlbumJournalFixtures.move(false),renamed=AlbumJournalFixtures.move(true),partial=AlbumJournalFixtures.partial(false),blocked=AlbumJournalFixtures.blocked();
        prefixes("direct copy",direct);prefixes("nested copy and existing parent",nested);prefixes("original move",move);prefixes("move-stage rename",renamed);
        prefixes("partial move",partial);prefixes("partial copy",AlbumJournalFixtures.partial(true));prefixes("zero eligible",AlbumJournalFixtures.allSkipped());
        prefixes("zero children",AlbumJournalFixtures.zeroChildren());prefixes("blocked predicted-success refusal",blocked);
        prefixes("Single then Album",AlbumJournalFixtures.mixed(false));prefixes("Album then Single",AlbumJournalFixtures.mixed(true));
        test("exact producer phase budgets cover all Album fixtures",()->{for(AlbumJournalFixtures.Fixture fixture:new AlbumJournalFixtures.Fixture[]{direct,nested,move,renamed,partial,AlbumJournalFixtures.partial(true),AlbumJournalFixtures.allSkipped(),AlbumJournalFixtures.zeroChildren(),blocked}){NativeAlbumPlan plan=NativeAlbumPlan.parse(AlbumJournalFixtures.HASH,fixture.original(),fixture.projection(),fixture.authority.getJSONArray("references"),fixture.authority.getJSONObject("source_bindings"),()->false);long budget=AlbumEffectBudget.album(plan.get(0),plan.tree,()->false);check(fixture.events().length()<=budget);}});
        test("reconstruction normalizes tree counts",()->{JSONObject value=result(direct).getJSONObject("tree_binding");check(value.get("files") instanceof Integer);check(parse(direct).acknowledged==1);});
        test("producer null-return prefix stays unresolved",()->{AlbumJournalFixtures.Fixture fixture=direct.copy();int create=find(fixture,"intent","target-directory");detail(fixture,create+1).put("uri",JSONObject.NULL);prefix(fixture,create+2);check(parse(fixture).unresolved==1);});
        test("plain document returned URI can be rebound",()->{AlbumJournalFixtures.Fixture fixture=direct.copy();int create=find(fixture,"intent","target-directory");String id=detail(fixture,create+2).getJSONObject("metadata").getString("id");detail(fixture,create+1).put("uri",DocumentsContractCompat.document(id));check(parse(fixture).acknowledged==1);});
        test("mismatched create effect stays unresolved",()->{AlbumJournalFixtures.Fixture fixture=direct.copy();int create=find(fixture,"intent","target-directory");JSONObject event=fixture.events().getJSONObject(create+2);event.put("phase","effect-mismatch");event.getJSONObject("detail").getJSONObject("metadata").put("name","Artist (1)");prefix(fixture,create+3);fixture.state.put("status","uncertain");check(parse(fixture).acknowledged==0&&parse(fixture).unresolved==1);});
        test("cancelled reconstruction stops",()->{try{RetainedJournals.validateEvents(direct.authority,direct.state,()->true);throw new AssertionError("Ignored cancellation");}catch(IOException expected){check("cancelled".equals(expected.getMessage()));}});

        reject("unknown album protocol",direct,f->f.authority.put("album_protocol",2));
        reject("missing album protocol cannot fall back",direct,f->f.authority.remove("album_protocol"));
        reject("missing projection",direct,f->f.authority.remove("album_projection"));
        reject("sequence starts at one",direct,f->f.events().getJSONObject(0).put("sequence",1));
        reject("sequence is not an integer",direct,f->f.events().getJSONObject(0).put("sequence","0"));
        reject("wrong operation number",direct,f->f.events().getJSONObject(0).put("operation","operation-1"));
        reject("unknown step grammar",direct,f->f.events().getJSONObject(1).put("operation","operation-0/create-album"));
        reject("wrong step counter",direct,f->f.events().getJSONObject(1).put("operation","operation-0/album-step-1"));
        reject("album start absent",direct,f->AlbumJournalFixtures.remove(f,0,1));
        reject("album source changed",direct,f->detail(f,0).put("source",AlbumJournalFixtures.uri(false,"primary:songs/other")));
        reject("album child count changed",direct,f->detail(f,0).put("children",2));
        reject("album mode changed",direct,f->detail(f,0).put("copy",false));
        reject("create has no returned receipt",direct,f->AlbumJournalFixtures.remove(f,2,1));
        reject("create return lacks URI",direct,f->detail(f,2).remove("uri"));
        reject("create return is number",direct,f->detail(f,2).put("uri",1));
        reject("create return null cannot acknowledge",direct,f->detail(f,2).put("uri",JSONObject.NULL));
        reject("create return document changed",direct,f->detail(f,2).put("uri",AlbumJournalFixtures.uri(true,"primary:sorted/other")));
        reject("create return authority changed",direct,f->detail(f,2).put("uri",detail(f,2).getString("uri").replace(ProviderReferences.AUTHORITY,"foreign.provider")));
        reject("create return query injected",direct,f->detail(f,2).put("uri",detail(f,2).getString("uri")+"?foreign=1"));
        reject("create observed source-tree URI",direct,f->detail(f,3).put("uri",AlbumJournalFixtures.uri(false,"primary:sorted/Artist")));
        reject("create observed plain document URI",direct,f->detail(f,3).put("uri",DocumentsContractCompat.document("primary:sorted/Artist")));
        reject("create observation has no metadata",direct,f->detail(f,3).remove("metadata"));
        reject("create metadata ID changed",direct,f->detail(f,3).getJSONObject("metadata").put("id","primary:sorted/Other"));
        reject("create name suffix",direct,f->detail(f,3).getJSONObject("metadata").put("name","Artist (1)"));
        reject("create directory became file",direct,f->detail(f,3).getJSONObject("metadata").put("mime","audio/mpeg"));
        reject("target component order changed",direct,f->detail(f,1).put("component",1));
        reject("target parent changed",direct,f->detail(f,1).put("parent",AlbumJournalFixtures.uri(true,"primary:sorted/Other")));
        reject("target node forged",direct,f->detail(f,1).put("node_token","song"));
        reject("existing parent component wrong",nested,f->detail(f,find(f,"album-parent-observed",null)).put("component",1));
        reject("existing parent source tree",nested,f->detail(f,find(f,"album-parent-observed",null)).put("uri",AlbumJournalFixtures.uri(false,"primary:sorted/Artist")));
        reject("child order changed",nested,f->detail(f,find(f,"album-child-start",null)).put("child",1));
        reject("child source changed",direct,f->detail(f,find(f,"album-child-start",null)).put("source",AlbumJournalFixtures.uri(false,"primary:songs/other.mp3")));
        reject("child planned suffix changed",direct,f->detail(f,find(f,"album-child-start",null)).put("name","song (1).mp3"));
        reject("child token changed",direct,f->detail(f,find(f,"album-child-start",null)).put("node_token","album"));
        reject("copy stage ordinary name",direct,f->detail(f,find(f,"intent","copy-stage")).put("name","ordinary.mp3"));
        reject("copy stage non UUID ownership",direct,f->detail(f,find(f,"intent","copy-stage")).put("name",".singles-album-not-an-owned-id"));
        reject("copy stage wrong parent",direct,f->detail(f,find(f,"intent","copy-stage")).put("parent",AlbumJournalFixtures.uri(true,"primary:sorted")));
        reject("write not own stage",direct,f->detail(f,find(f,"intent","copy-content")).put("stage",AlbumJournalFixtures.uri(false,"primary:songs/Collection/song.mp3")));
        reject("write source altered",direct,f->detail(f,find(f,"intent","copy-content")).put("source",AlbumJournalFixtures.uri(false,"primary:songs/foreign.mp3")));
        reject("write declared bytes altered",direct,f->detail(f,find(f,"intent","copy-content")).getJSONObject("binding").put("bytes",99));
        reject("write returned length altered",direct,f->detail(f,find(f,"intent","copy-content")+1).put("bytes",99));
        reject("write observed hash altered",direct,f->detail(f,find(f,"intent","copy-content")+2).getJSONObject("binding").put("sha256",AlbumJournalFixtures.HASH.replace('a','b')));
        reject("write observed URI altered",direct,f->detail(f,find(f,"intent","copy-content")+2).put("uri",AlbumJournalFixtures.uri(true,"primary:sorted/other")));
        reject("copy publication before any write",direct,f->removeEffect(f,"copy-content"));
        reject("nested publication before exact member rename",nested,f->removeEffect(f,"copy-member-publish"));
        reject("nested file stage lacks bin suffix",nested,f->detail(f,find(f,"intent","copy-member-stage")).put("name",".singles-album-part-66666666-6666-6666-6666-666666666666"));
        reject("nested file publish invents name",nested,f->detail(f,find(f,"intent","copy-member-publish")).put("name","invented.bin"));
        reject("nested publication without member",nested,f->removeEffect(f,"copy-member"));
        reject("nested publication without file content",nested,f->removeEffect(f,"copy-content"));
        reject("nested create outside captured subtree",nested,f->detail(f,find(f,"intent","copy-member")).put("node_token","notes"));
        reject("nested create wrong parent",nested,f->detail(f,find(f,"intent","copy-member")).put("parent",AlbumJournalFixtures.uri(true,"primary:sorted")));
        reject("copy publish altered name",direct,f->detail(f,find(f,"intent","publish-copy")).put("name","song (1).mp3"));
        reject("copy publish is not stage",direct,f->detail(f,find(f,"intent","publish-copy")).put("source",AlbumJournalFixtures.uri(false,"primary:songs/Collection/song.mp3")));
        reject("copy publishes source as move",direct,f->detail(f,find(f,"intent","publish-copy")).put("action","move").put("role","publish-move"));
        reject("forged tree hash",direct,f->result(f).getJSONObject("tree_binding").put("tree_sha256",AlbumJournalFixtures.HASH));
        reject("forged tree files",direct,f->result(f).getJSONObject("tree_binding").put("files",2));
        reject("forged tree directories",nested,f->result(f).getJSONObject("tree_binding").put("directories",0));
        reject("forged tree bytes",nested,f->result(f).getJSONObject("tree_binding").put("bytes",0));
        reject("tree count string",direct,f->result(f).getJSONObject("tree_binding").put("files","1"));
        reject("tree count fraction",direct,f->result(f).getJSONObject("tree_binding").put("files",1.5));
        reject("tree count negative",direct,f->result(f).getJSONObject("tree_binding").put("files",-1));
        reject("tree extra field",direct,f->result(f).getJSONObject("tree_binding").put("invented",true));
        reject("copy claims absent source",direct,f->result(f).put("source_absent",true));
        reject("move claims retained source",move,f->result(f).put("source_retained",true));
        reject("child acknowledgment missing destination",direct,f->AlbumJournalFixtures.remove(f,find(f,"album-child-destination",null),1));
        reject("child acknowledgment altered",direct,f->detail(f,find(f,"album-child-verified",null)).put("name","other.mp3"));
        reject("album acknowledgment missing child acknowledgment",direct,f->AlbumJournalFixtures.remove(f,find(f,"album-child-verified",null),1));
        reject("move wrong source parent",move,f->detail(f,find(f,"intent","publish-move")).put("from",AlbumJournalFixtures.uri(false,"primary:songs")));
        reject("move wrong destination",move,f->detail(f,find(f,"intent","publish-move")).put("to",AlbumJournalFixtures.uri(true,"primary:sorted")));
        reject("renamed move without stage create",renamed,f->removeEffect(f,"move-stage"));
        reject("renamed move without original move",renamed,f->removeEffect(f,"stage-move"));
        reject("renamed move without stage rename",renamed,f->removeEffect(f,"stage-rename"));
        reject("renamed move without stage cleanup",renamed,f->removeEffect(f,"move-stage",true));
        reject("source removal before child completion",move,f->{int removal=find(f,"intent","source-album"),child=find(f,"album-child-verified",null);JSONObject a=f.events().getJSONObject(removal),b=f.events().getJSONObject(child);f.events().put(child,a.put("sequence",child));f.events().put(removal,b.put("sequence",removal));});
        reject("move source removal missing",move,f->removeEffect(f,"source-album"));
        reject("move source removal false return",move,f->detail(f,find(f,"intent","source-album")+1).put("removed",false));
        reject("move source removal no absence",move,f->detail(f,find(f,"intent","source-album")+2).put("owned_empty_stage_absent",false));
        reject("remove foreign source album",move,f->detail(f,find(f,"intent","source-album")).put("owned_empty_directory",AlbumJournalFixtures.uri(false,"primary:songs/Foreign")));
        reject("skip child marked complete",partial,f->detail(f,find(f,"album-child-skipped",null)).put("status","complete"));
        reject("skip original prediction changed",partial,f->detail(f,find(f,"album-child-skipped",null)).put("predicted_success",true));
        reject("skip reason changed",partial,f->detail(f,find(f,"album-child-skipped",null)).put("reason","unknown"));
        reject("skip pretends source removed",partial,f->detail(f,find(f,"album-child-skipped",null)).put("source_retained",false));
        reject("partial album claims complete",partial,f->last(f).put("status","complete"));
        reject("skipped source album cannot be removed",partial,f->last(f).put("source_removed",true));
        reject("zero eligible creates destination",AlbumJournalFixtures.allSkipped(),f->last(f).put("destination",AlbumJournalFixtures.uri(true,"primary:sorted/Artist/Collection")));
        reject("album final tree bytes altered",direct,f->last(f).put("bytes",0));
        reject("album final moved count altered",move,f->last(f).put("moved_children",0));
        reject("album final child count changed",direct,f->last(f).put("children_count",0));
        reject("album final child count missing",direct,f->last(f).remove("children_count"));
        reject("album final child count string",direct,f->last(f).put("children_count","1"));
        reject("album final destination changed",direct,f->last(f).put("destination",AlbumJournalFixtures.uri(true,"primary:sorted/Artist/Other")));
        reject("duplicate final acknowledgment",direct,f->append(f,"album-verified",AlbumJournalFixtures.copy(last(f))));
        reject("effect mismatch cannot continue",direct,f->f.events().getJSONObject(3).put("phase","effect-mismatch"));
        reject("predicted-success blocked child cannot complete",blocked,f->append(f,"album-verified",new JSONObject().put("album",true).put("copy",true).put("destination",JSONObject.NULL).put("source_removed",false).put("moved_children",0).put("bytes",0).put("status","partial").put("children_count",1)));
        reject("mixed altered original dependency",AlbumJournalFixtures.mixed(false),f->f.original().getJSONArray("operations").getJSONObject(1).put("depends_on",new JSONArray()));
        reject("mixed Album cannot skip Single acknowledgment",AlbumJournalFixtures.mixed(false),f->AlbumJournalFixtures.remove(f,find(f,"verified",null),1));
        reject("mixed Single invalid publication",AlbumJournalFixtures.mixed(true),f->{int verified=find(f,"verified",null);detail(f,verified).put("destination",AlbumJournalFixtures.uri(true,"primary:sorted/foreign.mp3"));});
        if(failures>0)throw new AssertionError(failures+" Album journal checks failed; "+checks+" passed");
        System.out.println(checks+" Android reconstructed Album journal JSON checks passed");
    }
    private static void removeEffect(AlbumJournalFixtures.Fixture fixture,String role,boolean cleanup)throws Exception{
        if(!cleanup){removeEffect(fixture,role);return;}
        for(int i=0;i<fixture.events().length();i++){
            JSONObject event=fixture.events().getJSONObject(i),value=event.getJSONObject("detail");
            if(event.getString("phase").equals("intent")&&role.equals(value.optString("role"))&&"remove-empty".equals(value.optString("action"))){
                // The later source-removal step is irrelevant to this omitted cleanup: child verification must fail first.
                AlbumJournalFixtures.remove(fixture,i,3);return;
            }
        }
        throw new AssertionError("Missing cleanup");
    }
    private static final class DocumentsContractCompat {
        private DocumentsContractCompat(){}
        static String document(String id){return android.provider.DocumentsContract.buildDocumentUri(ProviderReferences.AUTHORITY,id).toString();}
    }
}
