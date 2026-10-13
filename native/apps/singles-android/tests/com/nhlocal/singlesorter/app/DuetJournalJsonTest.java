package com.nhlocal.singlesorter.app;

import java.io.IOException;
import java.util.HashMap;
import org.json.JSONArray;
import org.json.JSONObject;

/** Fixed Android org.json runner. Calls the production cohort sequencer and retained parser. */
public final class DuetJournalJsonTest {
    private static final String HASH=AlbumJournalFixtures.HASH;
    private static int checks;
    private interface Trial {void run()throws Exception;}
    private static void require(boolean value){if(!value)throw new AssertionError("check "+checks);checks++;}
    private static void refuses(Trial trial)throws Exception{try{trial.run();throw new AssertionError("accepted invalid receipt");}catch(IllegalArgumentException|org.json.JSONException|IOException expected){checks++;}}
    private static JSONObject copy(JSONObject value)throws Exception{return new JSONObject(value.toString());}
    private static JSONObject source()throws Exception{return new JSONObject().put("reference",AlbumJournalFixtures.ref(false,"primary:songs/owned.wav")).put("kind","File").put("name","owned.wav").put("size",100).put("identity",JSONObject.NULL).put("parent_identity",JSONObject.NULL).put("modified","1").put("capabilities",new JSONObject().put("read",true));}
    private static JSONObject entry(boolean target,String id,String token,String name,String parent,boolean directory)throws Exception{
        return AlbumJournalFixtures.ref(target,id).getJSONObject("AndroidDocument").put("token",token).put("name",name).put("parent",parent==null?JSONObject.NULL:parent).put("directory",directory).put("observed_size",directory?JSONObject.NULL:100).put("observed_modified","1").put("directory_children_complete",directory);
    }
    private static final class Fixture implements DuetRetirement.Port,DuetRetirement.Journal {
        final JSONObject authority,state;final JSONArray events=new JSONArray();final HashMap<Integer,JSONObject> verified=new HashMap<>();
        boolean cancelled,removed,lostReturn,removeFalse;String failPhase,cancelPhase;int failDestination=-1,destinationCalls,sourceCalls,removeCalls;
        Fixture(int required)throws Exception{
            JSONArray ops=new JSONArray(),ids=new JSONArray();
            for(int id=0;id<required;id++){
                JSONArray deps=new JSONArray();for(int n=0;n<id;n++)deps.put(n);ids.put(id);
                JSONObject destination=new JSONObject().put("root",AlbumJournalFixtures.ref(true,"primary:sorted")).put("components",new JSONArray().put("Artist"+id).put("owned.wav"));
                ops.put(new JSONObject().put("id",id).put("depends_on",deps).put("stop_and_replan_on_effect_mismatch",true).put("kind",new JSONObject().put("SingleTransfer",new JSONObject().put("source",DuetJournalJsonTest.source())
                    .put("transfer",new JSONObject().put("source",DuetJournalJsonTest.source()).put("destination",destination).put("copy",true).put("predicted_success",true).put("blockers",new JSONArray().put("MISSING_IDENTITY"))))));
            }
            JSONObject plan=new JSONObject().put("schema_version",2).put("binding_sha256",HASH).put("operations",ops)
                .put("request",new JSONObject().put("mode","Sort").put("source",AlbumJournalFixtures.ref(false,"primary:songs")).put("target",AlbumJournalFixtures.ref(true,"primary:sorted")).put("settings",new JSONObject().put("duet_mode",true).put("copy_mode",false)))
                .put("blockers",new JSONArray().put("PHYSICAL_APPLY_UNAVAILABLE").put("MISSING_IDENTITY").put(NativeDuetPlan.MARKER))
                .put("predicted_effects",new JSONArray().put(new JSONObject().put("DuetDeleteIntention",new JSONObject().put("source",DuetJournalJsonTest.source()).put("after_operations",ids).put("blocked",true))));
            JSONArray refs=new JSONArray().put(entry(false,"primary:songs","root","songs",null,true)).put(entry(false,"primary:songs/owned.wav","file","owned.wav","root",false)).put(entry(true,"primary:sorted","target","sorted",null,true));
            authority=new JSONObject().put("schema",1).put("plan_id",HASH).put("original_plan",plan).put("references",refs).put("source_bindings",new JSONObject().put("file",new JSONObject().put("bytes",100).put("sha256",HASH)))
                .put("duet_protocol",1).put("provider_quiescence_required",true).put("provider_atomic_batch",false);
            state=new JSONObject().put("status","uncertain").put("events",events);
        }
        NativeDuetPlan.Group group()throws Exception{return NativeDuetPlan.inspectOriginal(authority.getJSONObject("original_plan"),()->false).groups.get(0);}
        public void checkpoint()throws Exception{if(cancelled)throw new IOException("CANCELLED");}
        public JSONObject source(NativeDuetPlan.Group group)throws Exception{checkpoint();sourceCalls++;return sourceEvidence(group);}
        JSONObject sourceEvidence(NativeDuetPlan.Group group)throws Exception{JSONObject source=group.source();return new JSONObject().put("source",AlbumJournalFixtures.uri(false,source.getJSONObject("reference").getJSONObject("AndroidDocument").getString("document_id"))).put("parent",AlbumJournalFixtures.uri(false,"primary:songs")).put("name",source.getString("name")).put("bytes",100).put("sha256",HASH);}
        public void destination(int id,JSONObject receipt)throws Exception{checkpoint();destinationCalls++;if(id==failDestination)throw new IOException("DUET_DESTINATION_CHANGED");}
        public boolean remove(NativeDuetPlan.Group group)throws Exception{checkpoint();removeCalls++;removed=!removeFalse;if(lostReturn)throw new IOException("RETURN_LOST");return removed;}
        public void absent(NativeDuetPlan.Group group)throws Exception{checkpoint();if(!removed)throw new IOException("STILL_PRESENT");}
        public void record(String phase,String operation,JSONObject detail)throws Exception{
            if(phase.equals(failPhase))throw new IOException("JOURNAL_COMMIT_UNCERTAIN");
            events.put(new JSONObject().put("sequence",events.length()).put("phase",phase).put("operation",operation).put("detail",copy(detail)));if(phase.equals(cancelPhase))cancelled=true;
        }
        JSONObject copied(int id)throws Exception{return copied(id,true);}
        JSONObject copied(int id,boolean cohort)throws Exception{return copied(id,cohort,false);}
        JSONObject copied(int id,boolean cohort,boolean createParent)throws Exception{
            JSONObject original=authority.getJSONObject("original_plan").getJSONArray("operations").getJSONObject(id).getJSONObject("kind").getJSONObject("SingleTransfer"),observation=original.getJSONObject("source");String originalName=observation.getString("name");JSONArray path=original.getJSONObject("transfer").getJSONObject("destination").getJSONArray("components");String filename=path.getString(path.length()-1);
            String op="operation-"+id,parent=AlbumJournalFixtures.uri(true,"primary:sorted/Artist"+id),stage=AlbumJournalFixtures.uri(true,"primary:sorted/Artist"+id+"/.singles-stage-33333333-3333-3333-3333-333333333333.bin"),dest=AlbumJournalFixtures.uri(true,"primary:sorted/Artist"+id+"/"+filename);
            String source=AlbumJournalFixtures.uri(false,observation.getJSONObject("reference").getJSONObject("AndroidDocument").getString("document_id"));JSONObject binding=new JSONObject().put("bytes",100).put("sha256",HASH);
            if(cohort){record("duet-copy-start",op,new JSONObject().put("source",source).put("parent",AlbumJournalFixtures.uri(false,"primary:songs")).put("name",originalName).put("binding",binding).put("target",AlbumJournalFixtures.uri(true,"primary:sorted")).put("components",original.getJSONObject("transfer").getJSONObject("destination").getJSONArray("components")));
                if(createParent){String directory=op+"/directory-0";record("intent",directory,new JSONObject().put("parent",AlbumJournalFixtures.uri(true,"primary:sorted")).put("name","Artist"+id).put("mime",AlbumJournalFixtures.DIRECTORY));record("returned",directory,new JSONObject().put("uri",parent));record("observed",directory,new JSONObject().put("uri",parent).put("metadata",new JSONObject().put("id","primary:sorted/Artist"+id).put("name","Artist"+id).put("mime",AlbumJournalFixtures.DIRECTORY)));}
                record("duet-parent-observed",op,new JSONObject().put("component",0).put("parent",AlbumJournalFixtures.uri(true,"primary:sorted")).put("uri",parent).put("name","Artist"+id));}
            record("intent",op+"/create-stage",new JSONObject().put("parent",parent).put("name",".singles-stage-33333333-3333-3333-3333-333333333333.bin").put("mime","application/octet-stream"));
            record("returned",op+"/create-stage",new JSONObject().put("uri",stage));record("observed",op+"/create-stage",new JSONObject().put("uri",stage).put("metadata",new JSONObject().put("id","primary:sorted/Artist"+id+"/.singles-stage-33333333-3333-3333-3333-333333333333.bin").put("name",".singles-stage-33333333-3333-3333-3333-333333333333.bin").put("mime","application/octet-stream")));
            record("intent",op+"/write-stage",new JSONObject().put("source",source).put("stage",stage).put("binding",binding));record("returned",op+"/write-stage",binding);
            record("intent",op+"/publish-copy",new JSONObject().put("source",stage).put("parent",parent).put("name",filename));record("returned",op+"/publish-copy",new JSONObject().put("uri",dest));
            record("observed",op+"/publish-copy",new JSONObject().put("uri",dest).put("metadata",new JSONObject().put("id","primary:sorted/Artist"+id+"/"+filename).put("name",filename).put("mime","application/octet-stream")));
            JSONObject evidence=new JSONObject().put("destination",dest).put("name",filename).put("bytes",100).put("sha256",HASH).put("copy",true);record("destination-observed",op,evidence);
            JSONObject result=copy(evidence).put("source_retained",true).put("source_absent",false);record("verified",op,result);verified.put(id,copy(result));return result;
        }
        JSONObject finish()throws Exception{return finishGroup(0);}
        JSONObject finishGroup(int id)throws Exception{return DuetRetirement.finish(id,NativeDuetPlan.inspectOriginal(authority.getJSONObject("original_plan"),()->false).groups.get(id),verified,this,this);}
        RetainedJournals.EvidenceCounts validate()throws Exception{return RetainedJournals.validateEvents(copy(authority),copy(state),()->false);}
    }
    private static int find(JSONArray events,String phase)throws Exception{for(int i=0;i<events.length();i++)if(events.getJSONObject(i).getString("phase").equals(phase))return i;throw new AssertionError("missing "+phase);}
    private static int step(JSONArray events,String phase,String suffix)throws Exception{for(int i=0;i<events.length();i++){JSONObject event=events.getJSONObject(i);if(event.getString("phase").equals(phase)&&event.getString("operation").endsWith("/"+suffix))return i;}throw new AssertionError("missing "+suffix);}
    private static void remove(Fixture f,int index)throws Exception{JSONArray events=new JSONArray();for(int i=0;i<f.events.length();i++)if(i!=index)events.put(copy(f.events.getJSONObject(i)).put("sequence",events.length()));f.state.put("events",events);}
    private static void addFile(Fixture f,String name)throws Exception{f.authority.getJSONArray("references").put(entry(false,"primary:songs/"+name,name,name,"root",false));f.authority.getJSONObject("source_bindings").put(name,new JSONObject().put("bytes",100).put("sha256",HASH));}
    private static JSONObject namedSource(String name)throws Exception{return source().put("name",name).put("reference",AlbumJournalFixtures.ref(false,"primary:songs/"+name));}
    private static Fixture mixed()throws Exception{
        Fixture f=new Fixture(1);JSONObject plan=f.authority.getJSONObject("original_plan");addFile(f,"zero.wav");addFile(f,"plain.wav");addFile(f,"second.wav");
        JSONArray operations=plan.getJSONArray("operations"),effects=plan.getJSONArray("predicted_effects");
        effects.put(new JSONObject().put("DuetDeleteIntention",new JSONObject().put("source",namedSource("zero.wav")).put("after_operations",new JSONArray()).put("blocked",true)));
        for(int id=1;id<=2;id++){String name=id==1?"plain.wav":"second.wav";JSONObject op=copy(operations.getJSONObject(0)).put("id",id).put("depends_on",id==1?new JSONArray().put(0):new JSONArray().put(0).put(1));JSONObject single=op.getJSONObject("kind").getJSONObject("SingleTransfer");single.put("source",namedSource(name));single.getJSONObject("transfer").put("source",namedSource(name)).put("copy",id==2).getJSONObject("destination").put("components",new JSONArray().put("Artist"+id).put(name));operations.put(op);}
        effects.put(new JSONObject().put("DuetDeleteIntention",new JSONObject().put("source",namedSource("second.wav")).put("after_operations",new JSONArray().put(2)).put("blocked",true)));
        f.finishGroup(1);f.copied(0);f.finishGroup(0);
        String destination=AlbumJournalFixtures.uri(true,"primary:sorted/Artist1/plain.wav"),op="operation-1";JSONObject result=new JSONObject().put("destination",destination).put("name","plain.wav").put("bytes",100).put("sha256",HASH).put("copy",false);
        f.record("intent",op+"/move",new JSONObject().put("source",AlbumJournalFixtures.uri(false,"primary:songs/plain.wav")).put("from",AlbumJournalFixtures.uri(false,"primary:songs")).put("to",AlbumJournalFixtures.uri(true,"primary:sorted/Artist1")).put("name","plain.wav"));f.record("returned",op+"/move",new JSONObject().put("uri",destination));f.record("observed",op+"/move",new JSONObject().put("uri",destination).put("metadata",new JSONObject().put("id","primary:sorted/Artist1/plain.wav").put("name","plain.wav").put("mime","application/octet-stream")));f.record("destination-observed",op,result);f.record("verified",op,copy(result).put("source_retained",false).put("source_absent",true));
        f.copied(2);f.finishGroup(2);f.state.put("status","completed");return f;
    }
    private static boolean attention(Fixture f)throws Exception{
        JSONObject authority=copy(f.authority).put("job_id","22222222-2222-2222-2222-222222222222"),state=copy(f.state);
        return new RetainedJournals.Record("11111111-1111-1111-1111-111111111111",authority,state,authority.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8),state.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8),new byte[0],f.validate(),()->false).attention;
    }
    private static void prefix(Fixture f,int length,String status)throws Exception{JSONArray events=new JSONArray();for(int i=0;i<length;i++)events.put(copy(f.events.getJSONObject(i)));f.state.put("status",status).put("events",events);}
    private static void counts(Fixture f,int acknowledged,int unresolved,int retired,int noAction,int pending)throws Exception{
        RetainedJournals.EvidenceCounts actual=f.validate();require(actual.acknowledged==acknowledged);require(actual.unresolved==unresolved);require(actual.duetRetired==retired);require(actual.duetNoAction==noAction);require(actual.duetUnresolved==pending);
    }
    private static Fixture complete(int count)throws Exception{Fixture f=new Fixture(count);for(int id=0;id<count;id++)f.copied(id);f.finish();f.state.put("status","completed");return f;}
    public static void main(String[] args)throws Exception{
        if(args.length!=0)throw new IllegalArgumentException("no arguments");
        Fixture numericFixture=new Fixture(0);JSONObject producerSource=numericFixture.sourceEvidence(numericFixture.group()).put("bytes",100L),parsedSource=copy(producerSource);
        require(producerSource.get("bytes") instanceof Long);require(parsedSource.get("bytes") instanceof Integer);
        require(DuetRecoveryEvidence.sameSourceEvidence(producerSource,parsedSource));require(DuetRecoveryEvidence.sameSourceEvidence(parsedSource,producerSource));
        for(Object invalid:new Object[]{"100",100.5,-1L,FixProtocol.ORIGINAL_LIMIT+1}){JSONObject changed=copy(parsedSource).put("bytes",invalid);refuses(()->DuetRecoveryEvidence.sameSourceEvidence(producerSource,changed));}
        for(String field:new String[]{"source","parent","name","sha256","bytes"}){JSONObject changed=copy(parsedSource).put(field,field.equals("bytes")?101:field.equals("sha256")?HASH.replace('a','b'):"changed");require(!DuetRecoveryEvidence.sameSourceEvidence(producerSource,changed));}
        JSONObject missing=copy(parsedSource);missing.remove("parent");require(!DuetRecoveryEvidence.sameSourceEvidence(producerSource,missing));require(!DuetRecoveryEvidence.sameSourceEvidence(producerSource,copy(parsedSource).put("extra",true)));
        Fixture zero=complete(0);counts(zero,0,0,0,1,0);require(zero.sourceCalls==1&&zero.destinationCalls==0&&zero.removeCalls==0&&!zero.removed);require("duet-noop".equals(zero.events.getJSONObject(0).getString("phase")));
        for(int n:new int[]{1,2,3}){Fixture f=complete(n);counts(f,n,0,1,0,0);require(f.destinationCalls==n&&f.sourceCalls==1&&f.removeCalls==1&&f.removed);}
        Fixture renamed=new Fixture(1);JSONObject renamedAuthority=new JSONObject(renamed.authority.toString().replace("owned.wav","owned?.wav"));java.util.Iterator<String> keys=renamedAuthority.keys();while(keys.hasNext()){String key=keys.next();renamed.authority.put(key,renamedAuthority.get(key));}renamed.authority.getJSONObject("original_plan").getJSONArray("operations").getJSONObject(0).getJSONObject("kind").getJSONObject("SingleTransfer").getJSONObject("transfer").getJSONObject("destination").getJSONArray("components").put(1,"owned.wav");renamed.copied(0);renamed.finish();renamed.state.put("status","completed");counts(renamed,1,0,1,0,0);
        Fixture created=new Fixture(1);created.copied(0,true,true);created.finish();created.state.put("status","completed");counts(created,1,0,1,0,0);
        for(int n=0;n<created.events.length();n++){Fixture prefix=new Fixture(1);for(int i=0;i<n;i++)prefix.events.put(copy(created.events.getJSONObject(i)));prefix.validate();checks++;}
        Fixture partial=new Fixture(2);partial.copied(0);refuses(partial::finish);require(!partial.removed&&partial.removeCalls==0);counts(partial,1,1,0,0,1);
        Fixture requiredFailure=new Fixture(2);requiredFailure.copied(0);requiredFailure.copied(1);requiredFailure.failDestination=1;refuses(requiredFailure::finish);require(!requiredFailure.removed&&requiredFailure.removeCalls==0);counts(requiredFailure,2,0,0,0,1);
        Fixture notCopy=new Fixture(1);notCopy.copied(0);notCopy.verified.get(0).put("copy",false);refuses(notCopy::finish);require(notCopy.destinationCalls==0&&notCopy.removeCalls==0);
        for(String phase:new String[]{"duet-prepared","intent"}){Fixture cancel=new Fixture(2);cancel.copied(0);cancel.copied(1);cancel.cancelPhase=phase;refuses(cancel::finish);require(!cancel.removed&&cancel.removeCalls==0);counts(cancel,2,0,0,0,1);}
        Fixture cancelled=new Fixture(0);cancelled.cancelled=true;refuses(cancelled::finish);require(cancelled.events.length()==0&&cancelled.removeCalls==0);
        Fixture commitFailure=new Fixture(1);commitFailure.copied(0);commitFailure.failPhase="intent";refuses(commitFailure::finish);require(!commitFailure.removed);counts(commitFailure,1,0,0,0,1);
        Fixture lost=new Fixture(2);lost.copied(0);lost.copied(1);lost.lostReturn=true;refuses(lost::finish);require(lost.removed);counts(lost,2,0,0,0,1);lost.state.put("status","completed");refuses(lost::validate);
        Fixture falseReturn=new Fixture(1);falseReturn.copied(0);falseReturn.removeFalse=true;refuses(falseReturn::finish);require(!falseReturn.removed);counts(falseReturn,1,0,0,0,1);
        Fixture full=complete(2);for(int n=0;n<full.events.length();n++){Fixture prefix=new Fixture(2);for(int i=0;i<n;i++)prefix.events.put(copy(full.events.getJSONObject(i)));prefix.validate();checks++;}
        for(String key:new String[]{"name","sha256","bytes","destination"}){
            Fixture malformed=complete(1);JSONObject detail=malformed.events.getJSONObject(find(malformed.events,"duet-prepared")).getJSONObject("detail").getJSONArray("copies").getJSONObject(0);detail.put(key,key.equals("bytes")?101:"wrong");refuses(malformed::validate);
        }
        for(String field:new String[]{"source","stage","binding"}){
            Fixture changed=complete(1);JSONObject data=changed.events.getJSONObject(step(changed.events,"intent","write-stage")).getJSONObject("detail");
            if(field.equals("binding"))data.getJSONObject("binding").put("sha256",HASH.replace('a','b'));else data.put(field,AlbumJournalFixtures.uri(false,"primary:songs/other.wav"));refuses(changed::validate);
        }
        for(String field:new String[]{"bytes","sha256"}){Fixture changed=complete(1);changed.events.getJSONObject(step(changed.events,"returned","write-stage")).getJSONObject("detail").put(field,field.equals("bytes")?101:HASH.replace('a','b'));refuses(changed::validate);}
        Fixture wrongLength=complete(1);wrongLength.events.getJSONObject(step(wrongLength.events,"intent","write-stage")).getJSONObject("detail").getJSONObject("binding").put("bytes",101);refuses(wrongLength::validate);
        Fixture lostStage=complete(1);JSONObject stageReturn=lostStage.events.getJSONObject(step(lostStage.events,"returned","create-stage"));stageReturn.getJSONObject("detail").put("uri",JSONObject.NULL);refuses(lostStage::validate);
        Fixture observedMismatch=complete(1);observedMismatch.events.getJSONObject(step(observedMismatch.events,"observed","publish-copy")).getJSONObject("detail").getJSONObject("metadata").put("name","owned (1).wav");refuses(observedMismatch::validate);
        Fixture foreignParent=complete(1);foreignParent.events.getJSONObject(step(foreignParent.events,"intent","publish-copy")).getJSONObject("detail").put("parent",AlbumJournalFixtures.uri(true,"primary:sorted/Other"));refuses(foreignParent::validate);
        Fixture omittedStart=complete(1);remove(omittedStart,0);refuses(omittedStart::validate);
        Fixture omittedParent=complete(1);remove(omittedParent,1);refuses(omittedParent::validate);
        Fixture omittedWrite=complete(1);remove(omittedWrite,step(omittedWrite.events,"returned","write-stage"));refuses(omittedWrite::validate);
        Fixture reorder=complete(1);JSONObject first=copy(reorder.events.getJSONObject(0));reorder.events.put(0,copy(reorder.events.getJSONObject(1)).put("sequence",0));reorder.events.put(1,first.put("sequence",1));refuses(reorder::validate);
        Fixture mixed=mixed();counts(mixed,3,0,2,1,0);require(mixed.removeCalls==2&&mixed.destinationCalls==2);
        for(int n=0;n<mixed.events.length();n++){JSONArray prefix=new JSONArray();for(int i=0;i<n;i++)prefix.put(copy(mixed.events.getJSONObject(i)));RetainedJournals.validateEvents(copy(mixed.authority),new JSONObject().put("status","uncertain").put("events",prefix),()->false);checks++;}
        Fixture missingZero=mixed();remove(missingZero,0);refuses(missingZero::validate);
        Fixture lateRetirement=mixed();JSONArray shifted=new JSONArray();JSONObject delayed=null;for(int i=0;i<lateRetirement.events.length();i++){JSONObject event=copy(lateRetirement.events.getJSONObject(i));if(event.getString("operation").equals("duet-0")&&event.getString("phase").equals("duet-verified")){delayed=event;continue;}shifted.put(event.put("sequence",shifted.length()));if(event.getString("operation").equals("operation-1/move")&&delayed!=null){shifted.put(delayed.put("sequence",shifted.length()));delayed=null;}}lateRetirement.state.put("events",shifted);refuses(lateRetirement::validate);
        Fixture wrongSource=complete(1);wrongSource.events.getJSONObject(find(wrongSource.events,"duet-prepared")).getJSONObject("detail").getJSONObject("source").put("name","other.wav");refuses(wrongSource::validate);
        Fixture wrongOutcome=complete(1);wrongOutcome.events.getJSONObject(find(wrongOutcome.events,"duet-verified")).getJSONObject("detail").put("source_retained",true);refuses(wrongOutcome::validate);
        Fixture wrongGroup=complete(1);wrongGroup.events.getJSONObject(find(wrongGroup.events,"duet-prepared")).put("operation","duet-1");refuses(wrongGroup::validate);
        Fixture wrongCohort=complete(2);wrongCohort.authority.getJSONObject("original_plan").getJSONArray("predicted_effects").getJSONObject(0).getJSONObject("DuetDeleteIntention").put("after_operations",new JSONArray().put(0));refuses(wrongCohort::validate);
        Fixture marker=complete(1);marker.authority.getJSONObject("original_plan").getJSONArray("blockers").put("UNSUPPORTED_RULE");refuses(marker::validate);
        Fixture historical=new Fixture(1);historical.copied(0,false);historical.authority.remove("duet_protocol");counts(historical,1,0,0,0,0);
        Fixture noHeader=complete(1);noHeader.authority.remove("duet_protocol");refuses(noHeader::validate);
        Fixture zeroLie=complete(0);zeroLie.events.getJSONObject(0).getJSONObject("detail").put("source_absent",true);refuses(zeroLie::validate);
        for(String status:new String[]{"cancelled","failed"}){
            for(int length:new int[]{0,1,2}){Fixture readOnly=new Fixture(1);readOnly.copied(0);prefix(readOnly,length,status);require(!attention(readOnly));}
            Fixture noOp=new Fixture(0);noOp.finish();noOp.state.put("status",status);require(!attention(noOp));
            Fixture earlierCopy=new Fixture(2);earlierCopy.copied(0);int verifiedPrefix=earlierCopy.events.length();earlierCopy.copied(1);prefix(earlierCopy,verifiedPrefix+1,status);require(attention(earlierCopy));
            Fixture legacy=new Fixture(1);legacy.copied(0,false);legacy.authority.remove("duet_protocol");legacy.state.put("status",status);require(attention(legacy));
        }
        require(!attention(complete(0)));
        Fixture cancellation=new Fixture(1);try{RetainedJournals.validateEvents(cancellation.authority,cancellation.state,()->true);throw new AssertionError();}catch(java.io.InterruptedIOException expected){checks++;}
        System.out.println(checks+" original duet retirement and historical JSON checks passed");
    }
}
