package com.nhlocal.singlesorter.app;

import android.net.Uri;
import android.provider.DocumentsContract;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Comparator;
import org.json.JSONArray;
import org.json.JSONObject;

/** Synthetic producer-shaped receipts. Always serialize and reconstruct Android JSON before validation. */
final class AlbumJournalFixtures {
    static final String HASH="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    static final String DIRECTORY="vnd.android.document/directory";
    static final String SOURCE_TREE="content://com.android.externalstorage.documents/tree/primary%3Asongs";
    static final String TARGET_TREE="content://com.android.externalstorage.documents/tree/primary%3Asorted";
    static final String STAGE=".singles-album-33333333-3333-3333-3333-333333333333";
    static final String MOVE_STAGE=".singles-album-move-44444444-4444-4444-4444-444444444444";
    private AlbumJournalFixtures(){}

    static final class Fixture {
        final JSONObject authority,state;
        Fixture(JSONObject authority,JSONObject state)throws Exception {
            this.authority=new JSONObject(authority.toString());
            this.state=new JSONObject(state.toString());
        }
        Fixture copy()throws Exception{return new Fixture(authority,state);}
        JSONObject original()throws Exception{return authority.getJSONObject("original_plan");}
        JSONObject projection()throws Exception{return authority.getJSONObject("album_projection");}
        JSONArray events()throws Exception{return state.getJSONArray("events");}
        JSONObject album()throws Exception{
            JSONArray operations=original().getJSONArray("operations");
            for(int i=0;i<operations.length();i++){
                JSONObject kind=operations.getJSONObject(i).getJSONObject("kind");
                if(kind.has("AlbumTransfer"))return kind.getJSONObject("AlbumTransfer");
            }
            throw new AssertionError("No fixture Album");
        }
    }

    private static final class Node {
        final String token,name;final boolean directory;final ArrayList<Node> children=new ArrayList<>();
        String id,parent;boolean predicted=true;String blocker;String planned;
        Node(String token,String name,boolean directory){this.token=token;this.name=name;this.directory=directory;planned=name;}
        Node add(Node node){children.add(node);return this;}
        JSONObject binding()throws Exception{return new JSONObject().put("bytes",100).put("sha256",HASH);}
    }
    static JSONObject ref(boolean target,String id)throws Exception{
        return new JSONObject().put("AndroidDocument",new JSONObject().put("tree_uri",target?TARGET_TREE:SOURCE_TREE)
            .put("authority",ProviderReferences.AUTHORITY).put("document_id",id));
    }
    static String uri(boolean target,String id){
        return DocumentsContract.buildDocumentUriUsingTree(Uri.parse(target?TARGET_TREE:SOURCE_TREE),id).toString();
    }
    static JSONObject copy(JSONObject value)throws Exception{return new JSONObject(value.toString());}
    static JSONArray copy(JSONArray value)throws Exception{return new JSONArray(value.toString());}
    static JSONObject detail(Fixture fixture,int index)throws Exception{return fixture.events().getJSONObject(index).getJSONObject("detail");}
    static int find(Fixture fixture,String phase,String role)throws Exception{
        JSONArray events=fixture.events();
        for(int i=0;i<events.length();i++){
            JSONObject event=events.getJSONObject(i);
            if(event.getString("phase").equals(phase)&&(role==null||role.equals(event.getJSONObject("detail").optString("role"))))return i;
        }
        throw new AssertionError("Missing "+phase+" / "+role);
    }
    static void remove(Fixture fixture,int first,int count)throws Exception{
        JSONArray old=fixture.events(),next=new JSONArray();
        for(int i=0;i<old.length();i++)if(i<first||i>=first+count)next.put(copy(old.getJSONObject(i)).put("sequence",next.length()));
        fixture.state.put("events",next);
    }

    static Fixture copyFile()throws Exception{return build(true,false,false,new Node("song","song.mp3",false));}
    static Fixture copyNested()throws Exception{
        Node disc=new Node("disc","Disc 1",true)
            .add(new Node("track","track.mp3",false))
            .add(new Node("art","Art",true).add(new Node("cover","cover.jpg",false)))
            .add(new Node("empty","Empty",true));
        return build(true,false,true,disc,new Node("notes","notes.txt",false));
    }
    static Fixture move(boolean rename)throws Exception{
        Node song=new Node("song",rename?"song?.mp3":"song.mp3",false);song.planned="song.mp3";
        return build(false,true,false,song);
    }
    static Fixture partial(boolean copy)throws Exception{
        Node refused=new Node("refused","refused.mp3",false);refused.predicted=false;
        return build(copy,false,false,new Node("song","song.mp3",false),refused);
    }
    static Fixture allSkipped()throws Exception{
        Node a=new Node("a","a.mp3",false),b=new Node("b","b.mp3",false);a.predicted=false;b.predicted=false;
        return build(true,false,false,a,b);
    }
    static Fixture zeroChildren()throws Exception{return build(true,false,false);}
    static Fixture blocked()throws Exception{
        Node node=new Node("song","song.mp3",false);node.blocker="ALBUM_SANITIZATION_COLLISION_NO_SUFFIX";
        Fixture fixture=build(true,false,false,node);
        // A predicted-success refusal stops the producer before album-verified.
        remove(fixture,fixture.events().length()-1,1);fixture.state.put("status","uncertain");return fixture;
    }

    private static Fixture build(boolean copy,boolean remove,boolean existingParent,Node... children)throws Exception{
        Node root=new Node("root","songs",true),album=new Node("album","Collection",true);root.add(album);
        for(Node node:children)album.add(node);
        JSONArray references=new JSONArray();JSONObject bindings=new JSONObject();capture(root,null,"primary:songs",references,bindings);
        JSONArray originalChildren=new JSONArray(),projectedChildren=new JSONArray();
        for(Node node:children){
            JSONArray path=new JSONArray().put("Artist").put("Collection").put(node.planned);
            JSONObject child=new JSONObject().put("source",observation(node)).put("destination",destination(path))
                .put("copy",copy).put("predicted_success",node.predicted)
                .put("blockers",node.blocker==null?new JSONArray().put("MISSING_IDENTITY"):new JSONArray().put(node.blocker));
            originalChildren.put(child);projectedChildren.put(new JSONObject().put("intent",copy(child))
                .put("planned_name",node.planned).put("name_changes",!node.planned.equals(node.name)));
        }
        JSONArray components=new JSONArray().put("Artist").put("Collection");
        JSONObject transfer=new JSONObject().put("source",observation(album)).put("destination",destination(components))
            .put("children",originalChildren).put("remove_empty_source",remove);
        JSONObject operation=new JSONObject().put("id",0).put("depends_on",new JSONArray())
            .put("stop_and_replan_on_effect_mismatch",true).put("kind",new JSONObject().put("AlbumTransfer",transfer));
        JSONArray blockers=new JSONArray().put("PHYSICAL_APPLY_UNAVAILABLE").put("MISSING_IDENTITY");
        JSONObject original=new JSONObject().put("schema_version",2).put("binding_sha256",HASH)
            .put("request",new JSONObject().put("mode","Sort").put("source",ref(false,root.id)).put("target",ref(true,"primary:sorted"))
                .put("settings",new JSONObject().put("copy_mode",copy)))
            .put("operations",new JSONArray().put(operation)).put("blockers",blockers)
            .put("metadata_observations",new JSONArray()).put("predicted_effects",new JSONArray());
        JSONObject projected=new JSONObject().put("id",0).put("source",observation(album)).put("destination",destination(components))
            .put("children",projectedChildren).put("depends_on",new JSONArray()).put("stop_and_replan_on_effect_mismatch",true)
            .put("copy_mode",copy).put("remove_empty_source",remove);
        JSONObject authority=new JSONObject().put("schema",1).put("execution_id","11111111-1111-1111-1111-111111111111")
            .put("job_id","22222222-2222-2222-2222-222222222222").put("plan_id",HASH).put("original_plan",original)
            .put("album_protocol",1).put("album_projection",new JSONObject().put("plan_id",HASH).put("blockers",copy(blockers)).put("operations",new JSONArray().put(projected)))
            .put("references",references).put("source_bindings",bindings).put("provider_quiescence_required",true).put("provider_atomic_batch",false);
        History history=new History(copy,remove,album);history.produce(existingParent);
        JSONObject state=new JSONObject().put("schema",1).put("execution_id",authority.getString("execution_id"))
            .put("job_id",authority.getString("job_id")).put("plan_id",HASH).put("status",history.skipped>0?"partial":"completed").put("events",history.events);
        return new Fixture(authority,state);
    }
    private static JSONObject destination(JSONArray components)throws Exception{return new JSONObject().put("root",ref(true,"primary:sorted")).put("components",copy(components));}
    private static void capture(Node node,Node parent,String id,JSONArray references,JSONObject bindings)throws Exception{
        node.id=id;node.parent=parent==null?null:parent.id;
        JSONObject entry=copy(ref(false,id).getJSONObject("AndroidDocument")).put("token",node.token)
            .put("parent",parent==null?JSONObject.NULL:parent.token).put("name",node.name).put("directory",node.directory)
            .put("observed_size",node.directory?JSONObject.NULL:100).put("observed_modified","1");
        if(node.directory)entry.put("directory_children_complete",true);else bindings.put(node.token,node.binding());
        references.put(entry);for(Node child:node.children)capture(child,node,id+"/"+child.name,references,bindings);
    }
    private static JSONObject observation(Node node)throws Exception{
        return new JSONObject().put("reference",ref(false,node.id)).put("identity",JSONObject.NULL).put("parent_identity",JSONObject.NULL)
            .put("name",node.name).put("kind",node.directory?"Directory":"File").put("size",node.directory?JSONObject.NULL:100)
            .put("modified","1").put("capabilities",new JSONObject().put("read",true).put("list",node.directory).put("seek",JSONObject.NULL));
    }

    /** Independent fixture binding encoder, so production binding comparison is not self-validating. */
    private static JSONObject treeBinding(Node root,String name)throws Exception{
        MessageDigest digest=MessageDigest.getInstance("SHA-256");long[] counts={0,0,0};hash(root,name,digest,counts);
        StringBuilder hex=new StringBuilder();for(byte value:digest.digest())hex.append(String.format(java.util.Locale.ROOT,"%02x",value&255));
        return new JSONObject().put("tree_sha256",hex.toString()).put("files",counts[0]).put("directories",counts[1]).put("bytes",counts[2]);
    }
    private static void field(MessageDigest digest,String value){byte[] bytes=value.getBytes(StandardCharsets.UTF_8);digest.update(ByteBuffer.allocate(4).putInt(bytes.length).array());digest.update(bytes);}
    private static void hash(Node node,String name,MessageDigest digest,long[] counts)throws Exception{
        field(digest,name);field(digest,node.directory?"directory":"file");
        if(node.directory){counts[1]++;ArrayList<Node> children=new ArrayList<>(node.children);children.sort(Comparator.comparing(n->n.name));field(digest,Integer.toString(children.size()));for(Node child:children)hash(child,child.name,digest,counts);}
        else{counts[0]++;counts[2]+=100;field(digest,"100");field(digest,HASH);}
    }

    private static final class History {
        final boolean copy,remove;final Node album;final JSONArray events=new JSONArray(),outcomes=new JSONArray();
        int step,child=-1,skipped,moved;long bytes;String albumId;
        History(boolean copy,boolean remove,Node album){this.copy=copy;this.remove=remove;this.album=album;}
        void root(String phase,JSONObject detail)throws Exception{event(phase,"operation-0",detail);}
        void event(String phase,String operation,JSONObject detail)throws Exception{events.put(new JSONObject().put("sequence",events.length()).put("phase",phase).put("operation",operation).put("detail",AlbumJournalFixtures.copy(detail)));}
        void effect(String action,String role,JSONObject intent,JSONObject returned,JSONObject observed)throws Exception{
            String operation="operation-0/album-step-"+step++;
            event("intent",operation,intent.put("action",action).put("role",role).put("child",child));
            event("returned",operation,returned);event("observed",operation,observed);
        }
        JSONObject metadata(String id,String name,boolean directory)throws Exception{return new JSONObject().put("uri",uri(true,id)).put("metadata",new JSONObject().put("id",id).put("name",name).put("mime",directory?DIRECTORY:"application/octet-stream"));}
        String create(String parent,String name,boolean directory,String role,Node node,int component)throws Exception{
            String id=parent+"/"+name;
            effect("create",role,new JSONObject().put("parent",uri(true,parent)).put("name",name).put("mime",directory?DIRECTORY:"application/octet-stream")
                .put("node_token",node==null?JSONObject.NULL:node.token).put("component",component),new JSONObject().put("uri",uri(true,id)),metadata(id,name,directory));return id;
        }
        void write(Node node,String stage)throws Exception{
            effect("write","copy-content",new JSONObject().put("source",uri(false,node.id)).put("stage",uri(true,stage)).put("node_token",node.token).put("binding",node.binding()),
                new JSONObject().put("bytes",100),new JSONObject().put("uri",uri(true,stage)).put("binding",node.binding()));
        }
        void members(Node node,String stage)throws Exception{
            for(Node member:node.children){if(member.directory){String target=create(stage,member.name,true,"copy-member",member,-1);members(member,target);}else{String target=create(stage,".singles-album-part-66666666-6666-6666-6666-666666666666.bin",false,"copy-member-stage",member,-1);write(member,target);rename(target,stage,member.name,member,"copy-member-publish");}}
        }
        String rename(String source,String parent,String name,Node node,String role)throws Exception{
            String id=parent+"/"+name;effect("rename",role,new JSONObject().put("source",uri(true,source)).put("parent",uri(true,parent)).put("name",name).put("node_token",node.token),new JSONObject().put("uri",uri(true,id)),metadata(id,name,node.directory));return id;
        }
        String move(String source,String from,String to,String name,Node node,String role,boolean original)throws Exception{
            String id=to+"/"+name;effect("move",role,new JSONObject().put("source",uri(!original,source)).put("from",uri(!original,from)).put("to",uri(true,to)).put("name",name).put("node_token",node.token),new JSONObject().put("uri",uri(true,id)),metadata(id,name,node.directory));return id;
        }
        void remove(String directory,String parent,String name,Node node,boolean source)throws Exception{
            effect("remove-empty",source?"source-album":"move-stage",new JSONObject().put("owned_empty_directory",uri(!source,directory)).put("parent",uri(!source,parent)).put("name",name).put("node_token",node.token),new JSONObject().put("removed",true),new JSONObject().put("owned_empty_stage_absent",true));
        }
        void produce(boolean existingParent)throws Exception{
            root("album-start",new JSONObject().put("source",uri(false,album.id)).put("copy",copy).put("remove_empty_source",remove).put("children",album.children.size()));
            boolean eligible=false;for(Node node:album.children)eligible|=node.predicted&&node.blocker==null;
            if(eligible){String artist="primary:sorted/Artist";
                if(existingParent)root("album-parent-observed",new JSONObject().put("component",0).put("parent",uri(true,"primary:sorted")).put("uri",uri(true,artist)).put("name","Artist"));
                else create("primary:sorted","Artist",true,"target-directory",null,0);
                albumId=create(artist,"Collection",true,"target-directory",null,1);
            }
            for(int i=0;i<album.children.size();i++){
                Node node=album.children.get(i);
                if(!node.predicted||node.blocker!=null){JSONObject result=new JSONObject().put("child",i).put("status","skipped")
                    .put("reason",!node.predicted?"ORIGINAL_PREDICTED_FAILURE":"ORIGINAL_CHILD_BLOCKED: "+node.blocker).put("predicted_success",node.predicted).put("source_retained",true);
                    root("album-child-skipped",result);outcomes.put(result);skipped++;continue;}
                child=i;root("album-child-start",new JSONObject().put("child",child).put("source",uri(false,node.id)).put("name",node.planned).put("copy",copy).put("node_token",node.token));
                String published,moveStage=null;
                if(copy){String stage=create(albumId,STAGE+(node.directory?"":".bin"),node.directory,"copy-stage",node,-1);if(node.directory)members(node,stage);else write(node,stage);published=rename(stage,albumId,node.planned,node,"publish-copy");}
                else if(node.name.equals(node.planned))published=move(node.id,node.parent,albumId,node.planned,node,"publish-move",true);
                else{moveStage=create(albumId,MOVE_STAGE,true,"move-stage",node,-1);String staged=move(node.id,node.parent,moveStage,node.name,node,"stage-move",true);String renamed=rename(staged,moveStage,node.planned,node,"stage-rename");published=move(renamed,moveStage,albumId,node.planned,node,"publish-move",false);}
                JSONObject binding=treeBinding(node,node.planned),result=new JSONObject().put("child",child).put("status","complete").put("destination",uri(true,published)).put("name",node.planned)
                    .put("copy",copy).put("source_retained",copy).put("source_absent",!copy).put("tree_binding",binding).put("name_changed",!node.planned.equals(node.name));
                root("album-child-destination",result);outcomes.put(result);if(copy)bytes+=binding.getLong("bytes");else moved++;
                if(moveStage!=null)remove(moveStage,albumId,MOVE_STAGE,node,false);
                root("album-child-verified",result);child=-1;
            }
            boolean removed=remove&&eligible&&skipped==0;if(removed)remove(album.id,album.parent,album.name,album,true);
            int createdDirectories=0,removedDirectories=0;for(int n=0;n<events.length();n++){JSONObject e=events.getJSONObject(n);if(!"intent".equals(e.getString("phase")))continue;JSONObject d=e.getJSONObject("detail");if("create".equals(d.optString("action"))&&DIRECTORY.equals(d.optString("mime")))createdDirectories++;if("remove-empty".equals(d.optString("action")))removedDirectories++;}
            root("album-verified",new JSONObject().put("created_directories",createdDirectories).put("removed_directories",removedDirectories).put("album",true).put("destination",albumId==null?JSONObject.NULL:uri(true,albumId)).put("copy",copy)
                .put("source_removed",removed).put("moved_children",moved).put("bytes",bytes).put("children_count",outcomes.length()).put("status",skipped>0?"partial":"complete"));
        }
    }

    static Fixture mixed(boolean albumFirst)throws Exception{
        Fixture fixture=copyFile();JSONObject original=fixture.original();JSONArray operations=original.getJSONArray("operations"),events=fixture.events();
        Node single=new Node("single","single.mp3",false);single.id="primary:songs/single.mp3";single.parent="primary:songs";
        JSONObject singleTransfer=new JSONObject().put("source",observation(single)).put("destination",destination(new JSONArray().put("single.mp3"))).put("copy",true).put("predicted_success",true).put("blockers",new JSONArray().put("MISSING_IDENTITY"));
        JSONObject singleOperation=new JSONObject().put("id",albumFirst?1:0).put("depends_on",albumFirst?new JSONArray().put(0):new JSONArray()).put("stop_and_replan_on_effect_mismatch",true)
            .put("kind",new JSONObject().put("SingleTransfer",new JSONObject().put("source",observation(single)).put("artist","Artist").put("transfer",singleTransfer)));
        JSONArray singleReferences=new JSONArray();capture(single,new Node("root","songs",true),single.id,singleReferences,new JSONObject());
        fixture.authority.getJSONArray("references").put(singleReferences.getJSONObject(0));fixture.authority.getJSONObject("source_bindings").put("single",single.binding());
        JSONArray singleEvents=new JSONArray();String operation="operation-"+(albumFirst?1:0),destination=uri(true,"primary:sorted/single.mp3");
        String stageName=".singles-stage-55555555-5555-5555-5555-555555555555.bin",stageId="primary:sorted/"+stageName,stage=uri(true,stageId),parent=uri(true,"primary:sorted");
        singleEvent(singleEvents,"intent",operation+"/create-stage",new JSONObject().put("parent",parent).put("name",stageName).put("mime","application/octet-stream"));
        singleEvent(singleEvents,"returned",operation+"/create-stage",new JSONObject().put("uri",stage));
        singleEvent(singleEvents,"observed",operation+"/create-stage",new JSONObject().put("uri",stage).put("metadata",new JSONObject().put("id",stageId).put("name",stageName).put("mime","application/octet-stream")));
        singleEvent(singleEvents,"intent",operation+"/write-stage",new JSONObject().put("source",uri(false,single.id)).put("stage",stage).put("binding",single.binding()));
        // The existing Single producer records returned bytes, then verifies and publishes without a write observed event.
        singleEvent(singleEvents,"returned",operation+"/write-stage",single.binding());
        singleEvent(singleEvents,"intent",operation+"/publish-copy",new JSONObject().put("source",stage).put("parent",parent).put("name","single.mp3"));
        singleEvent(singleEvents,"returned",operation+"/publish-copy",new JSONObject().put("uri",destination));
        singleEvent(singleEvents,"observed",operation+"/publish-copy",new JSONObject().put("uri",destination).put("metadata",new JSONObject().put("id","primary:sorted/single.mp3").put("name","single.mp3").put("mime","application/octet-stream")));
        JSONObject result=new JSONObject().put("destination",destination).put("name","single.mp3").put("bytes",100).put("sha256",HASH).put("copy",true);
        singleEvent(singleEvents,"destination-observed",operation,result);
        singleEvent(singleEvents,"verified",operation,copy(result).put("source_retained",true).put("source_absent",false));
        JSONArray combined=new JSONArray();
        if(albumFirst){operations.put(singleOperation);for(int i=0;i<events.length();i++)combined.put(events.get(i));for(int i=0;i<singleEvents.length();i++)combined.put(singleEvents.get(i));}
        else{
            JSONObject albumOperation=operations.getJSONObject(0).put("id",1).put("depends_on",new JSONArray().put(0));
            original.put("operations",new JSONArray().put(singleOperation).put(albumOperation));fixture.projection().getJSONArray("operations").getJSONObject(0).put("id",1).put("depends_on",new JSONArray().put(0));
            for(int i=0;i<singleEvents.length();i++)combined.put(singleEvents.get(i));
            for(int i=0;i<events.length();i++){JSONObject event=events.getJSONObject(i);event.put("operation",event.getString("operation").replace("operation-0","operation-1"));combined.put(event);}
        }
        for(int i=0;i<combined.length();i++)combined.getJSONObject(i).put("sequence",i);fixture.state.put("events",combined);return fixture.copy();
    }
    private static void singleEvent(JSONArray events,String phase,String operation,JSONObject detail)throws Exception{
        events.put(new JSONObject().put("sequence",events.length()).put("phase",phase).put("operation",operation).put("detail",copy(detail)));
    }
}
