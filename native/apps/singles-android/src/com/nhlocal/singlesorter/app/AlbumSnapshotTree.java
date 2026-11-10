package com.nhlocal.singlesorter.app;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Complete capture evidence only; never reconstructs original sorting decisions. */
final class AlbumSnapshotTree {
    static final class Node {
        final String token,name;final boolean directory;final Node parent;final List<Node> children=new ArrayList<>();
        private final String entry,binding;
        Node(JSONObject entry,JSONObject binding,Node parent)throws Exception{this.entry=entry.toString();this.binding=binding==null?null:binding.toString();this.parent=parent;token=entry.getString("token");name=entry.getString("name");directory=FixProtocol.bool(entry,"directory");}
        JSONObject entry()throws Exception{return new JSONObject(entry);}
        JSONObject reference()throws Exception{JSONObject e=entry();return new JSONObject().put("AndroidDocument",new JSONObject().put("tree_uri",e.getString("tree_uri")).put("authority",e.getString("authority")).put("document_id",e.getString("document_id")));}
        JSONObject binding()throws Exception{if(binding==null)throw new IllegalArgumentException("ALBUM_CONTENT_BINDING");return new JSONObject(binding);}
    }
    private final HashMap<String,JSONObject> raw=new HashMap<>();private final HashMap<String,Node> nodes=new HashMap<>();
    private final JSONObject bindings;private final FixBindingIndex index;private final BooleanSupplier cancelled;
    AlbumSnapshotTree(JSONArray references,JSONObject bindings,BooleanSupplier cancelled)throws Exception{
        this.bindings=bindings;this.cancelled=cancelled;index=new FixBindingIndex(references,bindings,cancelled);
        for(int i=0;i<references.length();i++){JournalExportBounds.checkpoint(cancelled);JSONObject e=references.getJSONObject(i);raw.put(e.getString("token"),e);}
        for(JSONObject e:raw.values())node(e.getString("token"),new HashSet<String>());
        for(int i=0;i<references.length();i++){JournalExportBounds.checkpoint(cancelled);Node n=nodes.get(references.getJSONObject(i).getString("token"));if(n.parent!=null)n.parent.children.add(n);}
    }
    private Node node(String token,HashSet<String> visiting)throws Exception{
        JournalExportBounds.checkpoint(cancelled);Node old=nodes.get(token);if(old!=null)return old;if(visiting.size()>=128||!visiting.add(token))throw new IllegalArgumentException("ALBUM_PARENT_CYCLE");JSONObject e=raw.get(token);if(e==null)throw new IllegalArgumentException("ALBUM_PARENT_MISSING");
        Node parent=e.isNull("parent")?null:node(e.getString("parent"),visiting);Node made=new Node(e,bindings.has(token)?FixProtocol.binding(bindings.getJSONObject(token),FixProtocol.ORIGINAL_LIMIT):null,parent);nodes.put(token,made);visiting.remove(token);return made;
    }
    Node get(JSONObject reference)throws Exception{return nodes.get(index.entry(reference).getString("token"));}
    Node token(String token){Node result=nodes.get(token);if(result==null)throw new IllegalArgumentException("ALBUM_UNKNOWN_TOKEN");return result;}
    void requireSource(Node node,Node root)throws Exception{Node current=node;int depth=0;while(current.parent!=null){JournalExportBounds.checkpoint(cancelled);if(++depth>128)throw new IllegalArgumentException("ALBUM_DEPTH");current=current.parent;}if(current!=root)throw new IllegalArgumentException("ALBUM_SOURCE_ROOT");}
    List<Node> complete(Node root)throws Exception{ArrayList<Node> result=new ArrayList<>();complete(root,result,new HashSet<String>(),0);return Collections.unmodifiableList(result);}
    private void complete(Node node,List<Node> result,HashSet<String> seen,int depth)throws Exception{
        JournalExportBounds.checkpoint(cancelled);if(depth>128||result.size()>=2000||!seen.add(node.token))throw new IllegalArgumentException("ALBUM_SUBTREE_BOUND");ProviderNames.require(node.name);result.add(node);
        if(node.directory){if(!Boolean.TRUE.equals(node.entry().opt("directory_children_complete")))throw new IllegalArgumentException("ALBUM_SUBTREE_INCOMPLETE");HashSet<String> names=new HashSet<>();for(Node child:node.children){if(!names.add(child.name.toLowerCase(java.util.Locale.ROOT)))throw new IllegalArgumentException("ALBUM_CHILD_NAME_AMBIGUOUS");complete(child,result,seen,depth+1);}}
        else node.binding();
    }
}
