package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.OperationCanceledException;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import java.io.IOException;
import java.util.LinkedHashSet;
import org.json.JSONArray;
import org.json.JSONObject;

/** Read-only current observations, never attribution of an unacknowledged historical effect.
 * Caller owns diagnostic exclusion. Provider calls require the declared quiescent local scope.
 */
final class RecoveryInspection {
 private RecoveryInspection(){}
 static JSONObject inspect(Context context,RetainedJournals.Record record,CancellationSignal cancel)throws Exception{
  cancel.throwIfCanceled();JSONObject authority=record.authority(),plan=authority.getJSONObject("original_plan"),request=plan.getJSONObject("request");
  String[] roles=RecoveryAdmission.roles(request);boolean fix=roles.length==1;Uri[] roots=new Uri[roles.length];for(int i=0;i<roles.length;i++)roots[i]=root(context,request.getJSONObject(roles[i]));
  LinkedHashSet<String> candidates=new LinkedHashSet<>();for(Uri tree:roots)add(candidates,DocumentsContract.buildDocumentUriUsingTree(tree,DocumentsContract.getTreeDocumentId(tree)).toString());JSONArray operations=plan.getJSONArray("operations"),associations=new JSONArray();JSONObject[] originals=new JSONObject[operations.length()],staged=new JSONObject[operations.length()];java.util.HashMap<String,JSONObject> bindingIndex=bindingIndex(authority,cancel);NativeAlbumPlan albums=authority.has("album_protocol")?NativeAlbumPlan.parse(authority.getString("plan_id"),plan,authority.getJSONObject("album_projection"),authority.getJSONArray("references"),authority.getJSONObject("source_bindings"),cancel::isCanceled):null;java.util.HashMap<String,JSONObject> albumIntents=new java.util.HashMap<>();JSONArray albumSubtrees=new JSONArray();java.util.HashSet<String> expanded=new java.util.HashSet<>();
  for(int i=0;i<operations.length();i++){cancel.throwIfCanceled();
   if(albums!=null&&albums.get(i)!=null){for(AlbumSnapshotTree.Node member:albums.tree.complete(albums.get(i).source)){cancel.throwIfCanceled();String uri=FixRecoveryEvidence.uri(member.entry());add(candidates,uri);if(!member.directory)associate(associations,i,"source",uri,member.binding());}continue;}
   JSONObject kind=operations.getJSONObject(i).getJSONObject("kind");String operationKind=fix?"FixFile":"SingleTransfer";if(kind.length()!=1||!kind.has(operationKind))throw new IOException("REVIEW_OPERATION_UNSUPPORTED");
   JSONObject ref=kind.getJSONObject(operationKind).getJSONObject("source").getJSONObject("reference").getJSONObject("AndroidDocument");
   if(!ProviderReferences.AUTHORITY.equals(ref.getString("authority")))throw new IOException("REVIEW_REFERENCE_INVALID");
   String sourceUri=DocumentsContract.buildDocumentUriUsingTree(Uri.parse(ref.getString("tree_uri")),ref.getString("document_id")).toString();add(candidates,sourceUri);
   originals[i]=binding(bindingIndex,ref);associate(associations,i,"source",sourceUri,originals[i]);
  }
  JSONArray events=record.state().getJSONArray("events"),intentCandidates=new JSONArray();long[] listed={0};
  for(int i=0;i<events.length();i++){
   cancel.throwIfCanceled();JSONObject event=events.getJSONObject(i),detail=event.getJSONObject("detail");
   String intendedParent=RecoveryIntentLookup.parent(event.getString("phase"),event.getString("operation"),detail.has("parent")?detail.getString("parent"):null,detail.has("to")?detail.getString("to"):null,detail.has("name")?detail.getString("name"):null);
   String operation=event.getString("operation");int slash=operation.indexOf('/'),number=Integer.parseInt(operation.substring(10,slash<0?operation.length():slash));String step=slash<0?"":operation.substring(slash+1);
   JSONObject originalBinding=originals[number];AlbumSnapshotTree.Node albumNode=null;String albumAction=null,albumRole=null;
   if(albums!=null&&albums.get(number)!=null){
    if(event.getString("phase").equals("intent"))albumIntents.put(operation,detail);
    JSONObject intended=albumIntents.get(operation);if(intended!=null){albumAction=intended.getString("action");albumRole=intended.getString("role");if(!intended.isNull("node_token"))albumNode=albums.tree.token(intended.getString("node_token"));}
    if(detail.has("child")&&detail.getInt("child")>=0&&step.isEmpty())albumNode=albums.get(number).children.get(detail.getInt("child")).source;
    if(albumNode!=null&&!albumNode.directory)originalBinding=albumNode.binding();
    if(albumNode!=null&&albumNode.directory&&AlbumRecoveryProjection.representsSubtree(albumAction,albumRole)){String candidate=detail.optString("destination",null);if(candidate==null&&!detail.isNull("uri"))candidate=detail.getString("uri");
     if(candidate!=null)expandAlbum(context,roots,candidate,albumNode,number,candidates,associations,albumSubtrees,expanded,cancel,listed,0);}
   }
   if(fix&&"fix-prepared".equals(event.getString("phase"))&&!detail.isNull("native_evidence")){JSONObject evidence=detail.getJSONObject("native_evidence");staged[number]=new JSONObject().put("bytes",evidence.getLong("staged_length")).put("sha256",evidence.getString("staged_sha256"));}
   if(intendedParent!=null){JSONObject intended=lookup(context,roots,event,intendedParent,candidates,cancel,listed);intentCandidates.put(intended);JSONArray found=intended.optJSONArray("current_candidates");
    if(found!=null)for(int n=0;n<found.length();n++){String candidate=found.getString(n);associate(associations,number,step.equals("move")||step.startsWith("publish-")?"intended_destination":"intended_stage_or_directory",candidate,originalBinding);if(albumNode!=null&&albumNode.directory&&AlbumRecoveryProjection.representsSubtree(albumAction,albumRole))expandAlbum(context,roots,candidate,albumNode,number,candidates,associations,albumSubtrees,expanded,cancel,listed,0);}}
   if(albumNode!=null&&!albumNode.directory&&!detail.isNull("uri"))associate(associations,number,AlbumRecoveryProjection.associationRole(albumRole),detail.getString("uri"),originalBinding);
   if(detail.has("stage")&&!detail.isNull("stage"))associate(associations,number,"stage",detail.getString("stage"),originalBinding);
   if(detail.has("destination")&&!detail.isNull("destination"))associate(associations,number,"destination",detail.getString("destination"),originalBinding);
   if(detail.has("uri")&&!detail.isNull("uri")&&(step.equals("create-stage")||step.equals("move-stage")||step.equals("rename-stage")||step.equals("move")||step.startsWith("publish-")||step.equals("rename-fix")||step.equals("backup-fix")||step.equals("rename-fix-backup")||step.equals("create-fix-output")))associate(associations,number,step.equals("move")||step.startsWith("publish-")?"destination":"stage",detail.getString("uri"),originalBinding);
   if(fix&&event.getString("phase").startsWith("fix-backup-")&&detail.has("uri"))associate(associations,number,"retained_original",detail.getString("uri"),originalBinding);
   for(String key:new String[]{"uri","source","stage","destination","backup","parent","from","to","owned_empty_directory"})if(detail.has(key)&&!detail.isNull(key)){Object value=detail.get(key);if(value instanceof String&&((String)value).startsWith("content://"))add(candidates,(String)value);}
  }
  java.util.HashMap<String,JSONArray> byCandidate=new java.util.HashMap<>();for(int n=0;n<associations.length();n++){JSONObject association=associations.getJSONObject(n);JSONObject output=staged[association.getInt("operation")];if(output!=null)association.put("staged_bytes",output.getLong("bytes")).put("staged_sha256",output.getString("sha256"));String uri=association.getString("recorded_uri");JSONArray list=byCandidate.get(uri);if(list==null){list=new JSONArray();byCandidate.put(uri,list);}list.put(association);}
  JSONArray observations=new JSONArray();long[] remaining={256L*1024*1024};
  for(String candidate:candidates){cancel.throwIfCanceled();JSONObject observed=observe(context,roots,candidate,cancel,remaining,fix);JSONArray related=new JSONArray();
   for(int n=0;byCandidate.containsKey(candidate)&&n<byCandidate.get(candidate).length();n++){JSONObject association=byCandidate.get(candidate).getJSONObject(n);if(candidate.equals(association.getString("recorded_uri"))){JSONObject relation=new JSONObject(association.toString());
    if(observed.getString("status").equals("content_observed"))relation.put("matches_original_content",RecoveryOriginalComparison.matches(association.getLong("original_bytes"),association.getString("original_sha256"),observed.getLong("bytes"),observed.getString("sha256")));if(observed.getString("status").equals("content_observed")&&association.has("staged_bytes"))relation.put("matches_staged_content",RecoveryOriginalComparison.matches(association.getLong("staged_bytes"),association.getString("staged_sha256"),observed.getLong("bytes"),observed.getString("sha256")));related.put(relation);}}
   observed.put("original_associations",related);observations.put(observed);
  }
  JSONObject out=new JSONObject().put("schema",1).put("execution_id",record.id).put("job_id",record.jobId).put("plan_id",record.planId)
   .put("authority_sha256",record.authorityHash).put("state_sha256",record.stateHash).put("index_sha256",record.indexHash)
   .put("album_subtree_observations",albumSubtrees).put("source_bindings",authority.getJSONObject("source_bindings")).put("references",authority.getJSONArray("references")).put("intent_candidates",intentCandidates).put("historical_operations",operations).put("historical_events",events).put("historical_acknowledged",record.acknowledged).put("historical_unresolved",record.unresolved)
   .put("source_tree",roots[0].toString()).put("target_tree",roots.length==1?"":roots[1].toString()).put("observations",observations)
   .put("historical_success_changed",false).put("unrecorded_effects_excluded",false);
  if(fix)out.put("mode","FixNames");
  if(out.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8).length>2*1024*1024)throw new IOException("REVIEW_RESULT_LIMIT");
  cancel.throwIfCanceled();return out;
 }
 private static String referenceKey(JSONObject ref)throws Exception{
  String tree=ref.getString("tree_uri"),authority=ref.getString("authority"),id=ref.getString("document_id");
  if(tree.length()>4096||authority.length()>256||id.length()>4096||tree.indexOf(0)>=0||authority.indexOf(0)>=0||id.indexOf(0)>=0)throw new IOException("REVIEW_REFERENCE_INVALID");return tree+'\0'+authority+'\0'+id;
 }
 private static java.util.HashMap<String,JSONObject> bindingIndex(JSONObject authority,CancellationSignal cancel)throws Exception{
  JSONArray references=authority.getJSONArray("references");if(references.length()>2000)throw new IOException("REVIEW_REFERENCE_LIMIT");JSONObject bindings=authority.getJSONObject("source_bindings");java.util.HashMap<String,JSONObject> index=new java.util.HashMap<>();java.util.HashSet<String> seen=new java.util.HashSet<>();
  for(int i=0;i<references.length();i++){cancel.throwIfCanceled();JSONObject entry=references.getJSONObject(i);String key=referenceKey(entry);if(!seen.add(key))throw new IOException("REVIEW_BINDING_AMBIGUOUS");String token=entry.getString("token");if(bindings.has(token))index.put(key,bindings.getJSONObject(token));}return index;
 }
 private static JSONObject binding(java.util.HashMap<String,JSONObject> index,JSONObject ref)throws Exception{
  JSONObject result=index.get(referenceKey(ref));if(result==null||result.getLong("bytes")<0||result.getLong("bytes")>64L*1024*1024||!result.getString("sha256").matches("[0-9a-f]{64}"))throw new IOException("REVIEW_BINDING_INVALID");return result;
 }
 private static void associate(JSONArray associations,int operation,String role,String uri,JSONObject original)throws Exception{
  if(original==null)return;
  if(associations.length()>=20000||uri.length()>8192)throw new IOException("REVIEW_ASSOCIATION_LIMIT");
  associations.put(new JSONObject().put("operation",operation).put("role",role).put("recorded_uri",uri).put("original_bytes",original.getLong("bytes")).put("original_sha256",original.getString("sha256")));
 }
 private static void add(LinkedHashSet<String> candidates,String value)throws IOException{
  if(value.length()>8192)throw new IOException("REVIEW_REFERENCE_LIMIT");if(!candidates.contains(value)&&candidates.size()>=4000)throw new IOException("REVIEW_REFERENCE_LIMIT");candidates.add(value);
 }
 private static Uri root(Context context,JSONObject ref)throws Exception{
  if(ref.length()!=1)throw new IOException("REVIEW_ROOT_INVALID");JSONObject value=ref.getJSONObject("AndroidDocument");
  if(value.length()!=3||!ProviderReferences.AUTHORITY.equals(value.getString("authority")))throw new IOException("REVIEW_ROOT_INVALID");
  String raw=value.getString("tree_uri");if(raw.length()>4096)throw new IOException("REVIEW_ROOT_INVALID");Uri tree=Uri.parse(raw);
  ProviderReferences.requireGrant(context,tree,false);
  if(tree.getQuery()!=null||tree.getFragment()!=null||tree.getPathSegments().size()!=2||!DocumentsContract.getTreeDocumentId(tree).equals(value.getString("document_id")))throw new IOException("REVIEW_ROOT_INVALID");return tree;
 }
 private static Uri resolve(Context context,Uri[] roots,Uri raw,CancellationSignal cancel)throws Exception{
  if(raw==null||!"content".equals(raw.getScheme())||!ProviderReferences.AUTHORITY.equals(raw.getAuthority())||!DocumentsContract.isDocumentUri(context,raw)||raw.getQuery()!=null||raw.getFragment()!=null||raw.toString().length()>8192)throw new IOException("REVIEW_REFERENCE_INVALID");
  String id=DocumentsContract.getDocumentId(raw);Exception unknown=null;
  for(Uri tree:roots){cancel.throwIfCanceled();Uri parent=DocumentsContract.buildDocumentUriUsingTree(tree,DocumentsContract.getTreeDocumentId(tree)),child=DocumentsContract.buildDocumentUriUsingTree(tree,id);
   try{if(id.equals(DocumentsContract.getTreeDocumentId(tree))||CheckedLocalAncestry.isChild(context,parent,child))return child;}
   catch(OperationCanceledException failure){throw failure;}catch(Exception unavailable){unknown=unavailable;}
  }
  if(unknown!=null)throw unknown;return null;
 }
 private static JSONObject lookup(Context context,Uri[] roots,JSONObject event,String intendedParent,LinkedHashSet<String> candidates,CancellationSignal cancel,long[] listed)throws Exception{
  JSONObject detail=event.getJSONObject("detail"),out=new JSONObject().put("sequence",event.getInt("sequence")).put("operation",event.getString("operation"));
  String name=detail.getString("name");if(name.length()>1024)throw new IOException("REVIEW_NAME_LIMIT");out.put("intended_name",name).put("recorded_parent",intendedParent);
  try{
   Uri parent=resolve(context,roots,Uri.parse(intendedParent),cancel);if(parent==null)return out.put("status","parent_not_currently_resolved");
   Uri children=DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent));JSONArray found=new JSONArray();
   try(Cursor cursor=context.getContentResolver().query(children,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
    if(cursor==null)throw new IOException("REVIEW_LIST_UNAVAILABLE");while(cursor.moveToNext()){
     cancel.throwIfCanceled();if(++listed[0]>20000)throw new IOException("REVIEW_LIST_LIMIT");
     if(name.equals(cursor.getString(1))){String id=cursor.getString(0);if(id==null||id.length()>4096)throw new IOException("REVIEW_ID_INVALID");String uri=DocumentsContract.buildDocumentUriUsingTree(parent,id).toString();add(candidates,uri);found.put(uri);}
    }
   }
   return out.put("status",RecoveryIntentLookup.status(found.length())).put("current_candidates",found);
  }catch(OperationCanceledException failure){throw failure;}catch(Exception unavailable){cancel.throwIfCanceled();if(listed[0]>20000)throw new IOException("REVIEW_LIST_LIMIT",unavailable);return out.put("status","unavailable").put("diagnostic",unavailable.getClass().getSimpleName());}
 }
 private static void expandAlbum(Context context,Uri[] roots,String candidate,AlbumSnapshotTree.Node node,int operation,LinkedHashSet<String> candidates,JSONArray associations,JSONArray reports,java.util.HashSet<String> expanded,CancellationSignal cancel,long[] listed,int depth)throws Exception{
  cancel.throwIfCanceled();if(depth>128)throw new IOException("REVIEW_ALBUM_DEPTH");if(!expanded.add(candidate))return;add(candidates,candidate);if(!node.directory){associate(associations,operation,"album_named_descendant",candidate,node.binding());return;}
  JSONObject report=new JSONObject().put("recorded_uri",candidate).put("operation",operation);reports.put(report);
  try{Uri parent=resolve(context,roots,Uri.parse(candidate),cancel);if(parent==null){report.put("status","not_currently_resolved");return;}java.util.HashMap<String,Uri> actual=new java.util.HashMap<>();java.util.HashSet<String> ids=new java.util.HashSet<>();
   try(Cursor cursor=context.getContentResolver().query(DocumentsContract.buildChildDocumentsUriUsingTree(parent,DocumentsContract.getDocumentId(parent)),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME},null,null,null,cancel)){
    if(cursor==null)throw new IOException("REVIEW_LIST_UNAVAILABLE");while(cursor.moveToNext()){cancel.throwIfCanceled();if(++listed[0]>20000)throw new IOException("REVIEW_LIST_LIMIT");String id=cursor.getString(0),name=cursor.getString(1);if(id==null||name==null||id.length()>4096||name.length()>1024||!ids.add(id)||actual.containsKey(name))throw new IOException("REVIEW_ALBUM_AMBIGUOUS");Uri uri=DocumentsContract.buildDocumentUriUsingTree(parent,id);if(!CheckedLocalAncestry.isChild(context,parent,uri))throw new IOException("REVIEW_ALBUM_SCOPE");actual.put(name,uri);add(candidates,uri.toString());}
    if(cursor.getExtras()!=null&&(cursor.getExtras().getBoolean(DocumentsContract.EXTRA_LOADING,false)||cursor.getExtras().containsKey(DocumentsContract.EXTRA_ERROR)))throw new IOException("REVIEW_ALBUM_INCOMPLETE");
   }
   int missing=0;for(AlbumSnapshotTree.Node member:node.children){cancel.throwIfCanceled();Uri uri=actual.get(member.name);if(uri==null){missing++;continue;}expandAlbum(context,roots,uri.toString(),member,operation,candidates,associations,reports,expanded,cancel,listed,depth+1);}
   report.put("status",missing==0&&actual.size()==node.children.size()?"names_currently_match":"names_currently_differ").put("missing_names",missing).put("current_children",actual.size()).put("expected_children",node.children.size());
  }catch(OperationCanceledException error){throw error;}catch(Exception unavailable){cancel.throwIfCanceled();if(listed[0]>20000)throw new IOException("REVIEW_LIST_LIMIT",unavailable);report.put("status","unavailable").put("diagnostic",unavailable.getClass().getSimpleName());}
 }
 private static JSONObject observe(Context context,Uri[] roots,String candidate,CancellationSignal cancel,long[] remaining,boolean fix)throws Exception{
  JSONObject out=new JSONObject().put("recorded_uri",candidate);
  try{
   Uri raw=Uri.parse(candidate);if(!"content".equals(raw.getScheme())||!ProviderReferences.AUTHORITY.equals(raw.getAuthority())||!DocumentsContract.isDocumentUri(context,raw)||raw.getQuery()!=null||raw.getFragment()!=null)throw new IOException("REVIEW_REFERENCE_INVALID");
   String id=DocumentsContract.getDocumentId(raw);if(id.length()>4096)throw new IOException("REVIEW_REFERENCE_INVALID");out.put("recorded_document_id",id).put("recorded_name",id.substring(id.lastIndexOf('/')+1));
   if(DocumentsContract.isTreeUri(raw))out.put("recorded_scope_role",DocumentsContract.getTreeDocumentId(raw).equals(DocumentsContract.getTreeDocumentId(roots[0]))?"source":roots.length>1&&DocumentsContract.getTreeDocumentId(raw).equals(DocumentsContract.getTreeDocumentId(roots[1]))?"target":"unknown");
   Uri selected=resolve(context,roots,raw,cancel);
   if(selected==null)return out.put("status","not_currently_resolved");
   String name,mime;long size;
   try(Cursor c=context.getContentResolver().query(selected,new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_DISPLAY_NAME,DocumentsContract.Document.COLUMN_MIME_TYPE,DocumentsContract.Document.COLUMN_SIZE},null,null,null,cancel)){
    if(c==null)throw new IOException("REVIEW_QUERY_UNAVAILABLE");if(!c.moveToFirst())return out.put("status","not_currently_resolved");
    if(!id.equals(c.getString(0)))throw new IOException("REVIEW_ID_CHANGED");name=c.getString(1);mime=c.getString(2);size=c.isNull(3)?-1:c.getLong(3);if(c.moveToNext()||name==null||name.length()>1024||mime==null||mime.length()>256)throw new IOException("REVIEW_METADATA_INVALID");
   }
   out.put("current_uri",selected.toString()).put("scope_role",DocumentsContract.getTreeDocumentId(selected).equals(DocumentsContract.getTreeDocumentId(roots[0]))?"source":"target").put("document_id",id).put("name",name).put("mime",mime);
   if(DocumentsContract.Document.MIME_TYPE_DIR.equals(mime))return out.put("status","directory_observed");
   if(size<0||size>(fix?FixProtocol.STAGED_LIMIT:64L*1024*1024)||size>remaining[0])return out.put("status","content_not_checked");
   String hash;
   ParcelFileDescriptor descriptor=context.getContentResolver().openFileDescriptor(selected,"r",cancel);if(descriptor==null)throw new IOException("REVIEW_OPEN_UNAVAILABLE");
   try(ParcelFileDescriptor owned=descriptor;ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(owned)){
    if(!android.system.OsConstants.S_ISREG(android.system.Os.fstat(owned.getFileDescriptor()).st_mode))throw new IOException("REVIEW_REGULAR_FILE_REQUIRED");
    hash=RecoveryContentHash.readBounded(input,size,remaining,cancel::throwIfCanceled,fix?FixProtocol.STAGED_LIMIT:64L*1024*1024);
   }
   cancel.throwIfCanceled();return out.put("status","content_observed").put("bytes",size).put("sha256",hash);
  }catch(OperationCanceledException failure){throw failure;}
  catch(Exception unavailable){cancel.throwIfCanceled();return out.put("status","unavailable").put("diagnostic",unavailable.getClass().getSimpleName());}
 }
}
