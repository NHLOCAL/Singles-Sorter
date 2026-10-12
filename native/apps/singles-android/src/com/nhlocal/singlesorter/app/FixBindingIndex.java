package com.nhlocal.singlesorter.app;

import java.util.HashMap;
import java.util.HashSet;
import java.util.function.BooleanSupplier;
import org.json.JSONArray;
import org.json.JSONObject;

/** Bounded validated lookup before any per-operation journal parser allocation. */
final class FixBindingIndex {
    private final HashMap<String,JSONObject> entries=new HashMap<>(),bindings=new HashMap<>();
    private final HashMap<String,JSONObject> tokens=new HashMap<>();
    FixBindingIndex(JSONArray references,JSONObject sourceBindings,BooleanSupplier cancelled)throws Exception{
        if(references.length()>2000||sourceBindings.length()>2000)throw new IllegalArgumentException("FIX_REFERENCE_LIMIT");
        for(int i=0;i<references.length();i++){JournalExportBounds.checkpoint(cancelled);JSONObject entry=references.getJSONObject(i);String token=FixProtocol.string(entry,"token",4096),key=key(entry);
            if(token.isEmpty()||entries.put(key,entry)!=null||tokens.put(token,entry)!=null)throw new IllegalArgumentException("FIX_DUPLICATE_REFERENCE");
            if(sourceBindings.has(token)){if(FixProtocol.bool(entry,"directory"))throw new IllegalArgumentException("FIX_DIRECTORY_BINDING");bindings.put(key,FixProtocol.binding(sourceBindings.getJSONObject(token),FixProtocol.ORIGINAL_LIMIT));}}
        if(bindings.size()!=sourceBindings.length())throw new IllegalArgumentException("FIX_FOREIGN_BINDING");
        // Parent membership is exact retained token provenance. IDs are never parsed into ancestry.
        for(JSONObject entry:entries.values()){JournalExportBounds.checkpoint(cancelled);if(entry.isNull("parent"))continue;JSONObject parent=tokens.get(FixProtocol.string(entry,"parent",4096));if(parent==null||!FixProtocol.bool(parent,"directory")||!entry.getString("tree_uri").equals(parent.getString("tree_uri"))||!entry.getString("authority").equals(parent.getString("authority")))throw new IllegalArgumentException("FIX_PARENT_BINDING");
            HashSet<String> seen=new HashSet<>();JSONObject node=entry;while(!node.isNull("parent")){JournalExportBounds.checkpoint(cancelled);if(seen.size()>=128||!seen.add(node.getString("token")))throw new IllegalArgumentException("FIX_PARENT_CYCLE");node=tokens.get(node.getString("parent"));if(node==null)throw new IllegalArgumentException("FIX_PARENT_BINDING");}}
        JournalExportBounds.checkpoint(cancelled);
    }
    static String key(JSONObject value)throws Exception{String tree=FixProtocol.string(value,"tree_uri",4096),authority=FixProtocol.string(value,"authority",256),id=FixProtocol.string(value,"document_id",4096);if(tree.isEmpty()||authority.isEmpty()||id.isEmpty()||tree.indexOf(0)>=0||authority.indexOf(0)>=0||id.indexOf(0)>=0)throw new IllegalArgumentException("FIX_REFERENCE_SHAPE");return tree+'\0'+authority+'\0'+id;}
    static JSONObject document(JSONObject reference)throws Exception{if(reference.length()!=1||!reference.has("AndroidDocument"))throw new IllegalArgumentException("FIX_REFERENCE_SHAPE");JSONObject doc=reference.getJSONObject("AndroidDocument");if(doc.length()!=3)throw new IllegalArgumentException("FIX_REFERENCE_SHAPE");key(doc);return doc;}
    JSONObject entry(JSONObject reference)throws Exception{JSONObject value=entries.get(key(document(reference)));if(value==null)throw new IllegalArgumentException("FIX_SOURCE_NOT_CAPTURED");return value;}
    JSONObject binding(JSONObject reference)throws Exception{JSONObject value=bindings.get(key(document(reference)));if(value==null)throw new IllegalArgumentException("FIX_SOURCE_NOT_BOUND");return new JSONObject(value.toString());}
    void requireRoot(JSONObject reference,JSONObject root,BooleanSupplier cancelled)throws Exception{JSONObject node=entry(reference);while(!node.isNull("parent")){JournalExportBounds.checkpoint(cancelled);node=tokens.get(node.getString("parent"));}if(!node.getString("token").equals(root.getString("token")))throw new IllegalArgumentException("FIX_SOURCE_FOREIGN_ROOT");}
    JSONObject parent(JSONObject reference)throws Exception{JSONObject entry=entry(reference);if(entry.isNull("parent"))throw new IllegalArgumentException("FIX_PARENT_MISSING");return tokens.get(entry.getString("parent"));}
}
