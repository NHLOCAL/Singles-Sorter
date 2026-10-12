package com.nhlocal.singlesorter.app;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.function.BooleanSupplier;
import org.json.JSONObject;

/** Canonical relative-name/content evidence. Never provider identity or authorization to retry. */
final class AlbumTreeBinding {
    private AlbumTreeBinding(){}
    static JSONObject of(AlbumSnapshotTree.Node root,String rootName,BooleanSupplier cancelled)throws Exception{
        MessageDigest digest=MessageDigest.getInstance("SHA-256");long[] counts={0,0,0};walk(root,rootName,digest,counts,cancelled,0);return new JSONObject().put("tree_sha256",hex(digest.digest())).put("files",counts[0]).put("directories",counts[1]).put("bytes",counts[2]);
    }
    private static void field(MessageDigest digest,String value){byte[] bytes=value.getBytes(StandardCharsets.UTF_8);digest.update(java.nio.ByteBuffer.allocate(4).putInt(bytes.length).array());digest.update(bytes);}
    private static void walk(AlbumSnapshotTree.Node node,String name,MessageDigest digest,long[] count,BooleanSupplier cancelled,int depth)throws Exception{
        JournalExportBounds.checkpoint(cancelled);if(depth>128||count[0]+count[1]>=2000)throw new IllegalArgumentException("ALBUM_BINDING_BOUND");field(digest,name);field(digest,node.directory?"directory":"file");
        if(node.directory){count[1]++;ArrayList<AlbumSnapshotTree.Node> children=new ArrayList<>(node.children);children.sort(Comparator.comparing(n->n.name));field(digest,Integer.toString(children.size()));for(AlbumSnapshotTree.Node child:children)walk(child,child.name,digest,count,cancelled,depth+1);}
        else{count[0]++;JSONObject binding=node.binding();count[2]+=binding.getLong("bytes");field(digest,Long.toString(binding.getLong("bytes")));field(digest,binding.getString("sha256"));}
    }
    static boolean same(JSONObject expected,JSONObject actual)throws Exception{if(actual.length()!=4||!FixProtocol.hash(expected,"tree_sha256").equals(FixProtocol.hash(actual,"tree_sha256")))return false;for(String key:new String[]{"files","directories","bytes"})if(FixProtocol.integer(expected,key,512L*1024*1024)!=FixProtocol.integer(actual,key,512L*1024*1024))return false;return true;}
    private static String hex(byte[] raw){StringBuilder out=new StringBuilder();for(byte b:raw)out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();}
}
