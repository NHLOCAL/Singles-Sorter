package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.net.Uri;
import android.os.CancellationSignal;
import android.provider.DocumentsContract;
import android.system.Os;
import android.system.OsConstants;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit retention acceptance. Acquires diagnostic exclusion internally; requires informed UI consent.
 * Creates a new immutable receipt only. Original journals and provider objects are never written.
 */
final class RetainedEffectsAcceptance {
 private static final int LIMIT=3*1024*1024;
 private RetainedEffectsAcceptance(){}
 static String digest(JSONObject inspection)throws Exception{return hex(MessageDigest.getInstance("SHA-256").digest(inspection.toString().getBytes(StandardCharsets.UTF_8)));}
 static JSONObject accept(Context context,String id,String displayedDigest,boolean explicitConsent,CancellationSignal cancel)throws Exception{
  return accept(context,id,displayedDigest,explicitConsent,cancel,null);
 }
 static JSONObject accept(Context context,String id,String displayedDigest,boolean explicitConsent,CancellationSignal cancel,Uri selectedParent)throws Exception{
  if(!explicitConsent)throw new SecurityException("REVIEW_CONSENT_REQUIRED");
  try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){lease.check();return acceptOwned(context,id,displayedDigest,cancel,selectedParent);}
 }
 private static JSONObject acceptOwned(Context context,String id,String displayedDigest,CancellationSignal cancel,Uri selectedParent)throws Exception{
  if(!JournalExportBounds.id(id)||displayedDigest==null||!displayedDigest.matches("[0-9a-f]{64}"))throw new IOException("REVIEW_TOKEN_INVALID");
  RetainedJournals.Record[] selected={null};RetainedJournals.scan(context,cancel::isCanceled,record->{if(record.id.equals(id))selected[0]=record;});
  RetainedJournals.Record record=selected[0];if(record==null||!record.attention)throw new IOException("REVIEW_NOT_AVAILABLE");
  JSONObject fresh=RecoveryInspection.inspect(context,record,cancel,selectedParent);if(!digest(fresh).equals(displayedDigest))throw new IOException("REVIEW_CHANGED");boolean parentReview=AlbumParentRecovery.requireBinding(record.authority(),fresh,cancel::isCanceled);
  requireRoots(context,fresh,cancel);
  JSONObject receipt=new JSONObject().put("schema",1).put("policy",parentReview?AlbumParentRecovery.POLICY:"FixNames".equals(fresh.optString("mode"))?RecoveryAcceptancePolicy.FIX_POLICY:RecoveryAcceptancePolicy.POLICY).put("retained_effects",true).put("uncertainty_acknowledged",true).put("historical_success_changed",false)
   .put("execution_id",record.id).put("job_id",record.jobId).put("plan_id",record.planId).put("authority_sha256",record.authorityHash).put("state_sha256",record.stateHash).put("index_sha256",record.indexHash)
   .put("source_tree",fresh.getString("source_tree")).put("target_tree",fresh.getString("target_tree")).put("review_sha256",displayedDigest).put("review",fresh);
  if("FixNames".equals(fresh.optString("mode")))receipt.put("mode","FixNames");
  if(parentReview)receipt.put("source_parent_tree",fresh.getString("source_parent_tree"));
  byte[] bytes=receipt.toString().getBytes(StandardCharsets.UTF_8);if(bytes.length>LIMIT)throw new IOException("REVIEW_RECEIPT_LIMIT");cancel.throwIfCanceled();
  File directory=root(context);if(!directory.exists()&&!directory.mkdir())throw new IOException("REVIEW_RECEIPT_DIRECTORY");
  int count=0;long total=0;try(java.nio.file.DirectoryStream<java.nio.file.Path> entries=java.nio.file.Files.newDirectoryStream(directory.toPath())){
   for(java.nio.file.Path entry:entries){cancel.throwIfCanceled();if(++count>=1000)throw new IOException("REVIEW_RECEIPT_COUNT");File checked=child(directory,entry.getFileName().toString());total+=checked.length();if(total>64L*1024*1024-bytes.length)throw new IOException("REVIEW_RECEIPT_BUDGET");}
  }
  File file=child(directory,prefix(record)+java.util.UUID.randomUUID().toString()+".json");cancel.throwIfCanceled();PreviewGeneration.PROCESS.beginAcceptance();
  try {
  java.io.FileDescriptor fd=Os.open(file.getAbsolutePath(),OsConstants.O_WRONLY|OsConstants.O_CREAT|OsConstants.O_EXCL|OsConstants.O_NOFOLLOW|OsConstants.O_CLOEXEC,0600);Throwable primary=null;
  try(FileOutputStream output=new FileOutputStream(fd)){cancel.throwIfCanceled();output.write(bytes);output.flush();output.getFD().sync();}
  catch(Exception|Error failure){primary=failure;throw failure;}
  finally{try{Os.close(fd);}catch(android.system.ErrnoException failure){if(primary!=null)primary.addSuppressed(failure);else throw failure;}}
  // A partial or uncertain receipt is retained; it never authorizes an automatic retry.
  byte[] actual=read(file,cancel,new long[]{LIMIT});if(!java.util.Arrays.equals(actual,bytes))throw new IOException("REVIEW_RECEIPT_READBACK");return receipt;
  } finally {PreviewGeneration.PROCESS.endAcceptance();}
 }
 static final class ReceiptIndex {
  private final java.util.HashMap<String,java.util.ArrayList<File>> byHistory=new java.util.HashMap<>();
  private final long[] remaining={64L*1024*1024};
 }
 static ReceiptIndex receipts(Context context,CancellationSignal cancel)throws Exception{
  ReceiptIndex index=new ReceiptIndex();File directory=root(context);cancel.throwIfCanceled();if(!directory.exists())return index;int count=0;
  try(java.nio.file.DirectoryStream<java.nio.file.Path> entries=java.nio.file.Files.newDirectoryStream(directory.toPath())){
   for(java.nio.file.Path entry:entries){cancel.throwIfCanceled();if(++count>1000)throw new IOException("REVIEW_RECEIPT_COUNT");File file=child(directory,entry.getFileName().toString());String history=file.getName().substring(0,102);java.util.ArrayList<File> list=index.byHistory.get(history);if(list==null){list=new java.util.ArrayList<>();index.byHistory.put(history,list);}list.add(file);}
  }return index;
 }
 /** Exact selected execution receipts only, including incomplete bytes as diagnostic evidence. */
 static java.util.LinkedHashMap<String,byte[]> exportReceipts(Context context,String id,CancellationSignal cancel)throws Exception{
  if(!JournalExportBounds.id(id))throw new IOException("REVIEW_TOKEN_INVALID");ReceiptIndex index=receipts(context,cancel);long[] remaining={8L*1024*1024};
  java.util.LinkedHashMap<String,byte[]> result=new java.util.LinkedHashMap<>();java.util.ArrayList<File> selected=new java.util.ArrayList<>();
  for(java.util.Map.Entry<String,java.util.ArrayList<File>> entry:index.byHistory.entrySet()){cancel.throwIfCanceled();if(entry.getKey().startsWith(id+"-"))selected.addAll(entry.getValue());}
  java.util.Collections.sort(selected,(a,b)->a.getName().compareTo(b.getName()));
  for(File file:selected){cancel.throwIfCanceled();result.put("acceptance/"+file.getName(),read(file,cancel,remaining));}return result;
 }
 static boolean accepted(Context context,RetainedJournals.Record record,CancellationSignal cancel,ReceiptIndex index)throws Exception{
  cancel.throwIfCanceled();java.util.ArrayList<File> files=index.byHistory.get(prefix(record));if(files==null)return false;
  for(File file:files){cancel.throwIfCanceled();try{if(matches(context,record,cancel,index.remaining,file))return true;}catch(org.json.JSONException incomplete){/* Retain incomplete receipts; a new explicit acceptance uses a fresh immutable file. */}}
  return false;
 }
 private static boolean matches(Context context,RetainedJournals.Record record,CancellationSignal cancel,long[] budget,File file)throws Exception{
  JSONObject receipt=new JSONObject(new String(read(file,cancel,budget),StandardCharsets.UTF_8));
  if(!Integer.valueOf(1).equals(receipt.get("schema")))return false;
  JSONObject request=record.authority().getJSONObject("original_plan").getJSONObject("request");
  boolean fix=RecoveryAdmission.roles(request).length==1;String source=RecoveryAdmission.root(request.getJSONObject("source")).toString(),target=fix?"":RecoveryAdmission.root(request.getJSONObject("target")).toString();
  if(fix&&!"FixNames".equals(receipt.optString("mode"))||!fix&&receipt.has("mode"))return false;
  String[] expected={record.id,record.jobId,record.planId,record.authorityHash,record.stateHash,record.indexHash,source,target};
  String[] actual={receipt.getString("execution_id"),receipt.getString("job_id"),receipt.getString("plan_id"),receipt.getString("authority_sha256"),receipt.getString("state_sha256"),receipt.getString("index_sha256"),receipt.getString("source_tree"),receipt.getString("target_tree")};
  boolean parentReview=AlbumParentRecovery.evidence(record.authority(),cancel::isCanceled)!=null;String policy=receipt.getString("policy");if(parentReview){if(!AlbumParentRecovery.POLICY.equals(policy)||!receipt.has("source_parent_tree"))return false;policy=RecoveryAcceptancePolicy.POLICY;}else if(receipt.has("source_parent_tree"))return false;
  if(!RecoveryAcceptancePolicy.matchesMode(fix,policy,bool(receipt,"retained_effects"),bool(receipt,"uncertainty_acknowledged"),bool(receipt,"historical_success_changed"),expected,actual))return false;
  JSONObject review=receipt.getJSONObject("review");if(fix&&!"FixNames".equals(review.optString("mode"))||!fix&&review.has("mode"))return false;if(!digest(review).equals(receipt.getString("review_sha256")))return false;
  for(String key:new String[]{"execution_id","job_id","plan_id","authority_sha256","state_sha256","index_sha256","source_tree","target_tree"})if(!receipt.getString(key).equals(review.getString(key)))return false;
  if(AlbumParentRecovery.requireBinding(record.authority(),review,cancel::isCanceled)!=parentReview||parentReview&&!receipt.getString("source_parent_tree").equals(review.getString("source_parent_tree")))return false;
  requireRoots(context,review,cancel);return true;
 }
 private static boolean bool(JSONObject value,String key)throws Exception{Object result=value.get(key);if(!(result instanceof Boolean))throw new IOException("REVIEW_RECEIPT_TYPE");return (Boolean)result;}
 private static void requireRoots(Context context,JSONObject review,CancellationSignal cancel)throws Exception{
  JSONArray observations=review.getJSONArray("observations");
  for(String key:reviewRoles(review)){cancel.throwIfCanceled();Uri tree=Uri.parse(review.getString(key));ProviderReferences.requireGrant(context,tree,false);String document=DocumentsContract.buildDocumentUriUsingTree(tree,DocumentsContract.getTreeDocumentId(tree)).toString();boolean observed=false;
   for(int i=0;i<observations.length();i++){JSONObject row=observations.getJSONObject(i);if(document.equals(row.getString("recorded_uri"))&&"directory_observed".equals(row.getString("status")))observed=true;}
   if(!observed)throw new IOException("REVIEW_ROOT_UNAVAILABLE");
   try(android.database.Cursor cursor=context.getContentResolver().query(Uri.parse(document),new String[]{DocumentsContract.Document.COLUMN_DOCUMENT_ID,DocumentsContract.Document.COLUMN_MIME_TYPE},null,null,null,cancel)){
    if(cursor==null||!cursor.moveToFirst()||!DocumentsContract.getTreeDocumentId(tree).equals(cursor.getString(0))||!DocumentsContract.Document.MIME_TYPE_DIR.equals(cursor.getString(1))||cursor.moveToNext())throw new IOException("REVIEW_ROOT_UNAVAILABLE");}
  }cancel.throwIfCanceled();
 }
 private static String[] reviewRoles(JSONObject review)throws Exception{
  if(review.has("source_parent_tree")){if(review.has("mode")||!review.has("album_root_parent")||!AlbumRootParent.parentTree(review.getJSONObject("album_root_parent")).toString().equals(review.getString("source_parent_tree")))throw new IOException("REVIEW_ROOT_MODE");return new String[]{"source_parent_tree","target_tree"};}
  if(!review.has("mode"))return new String[]{"source_tree","target_tree"};if("FixNames".equals(review.getString("mode"))&&"".equals(review.getString("target_tree")))return new String[]{"source_tree"};throw new IOException("REVIEW_ROOT_MODE");
 }
 private static File root(Context context)throws IOException{File parent=context.getFilesDir().getCanonicalFile(),directory=new File(parent,"provider-recovery-acceptance");if(!directory.getAbsoluteFile().equals(directory.getCanonicalFile()))throw new IOException("REVIEW_RECEIPT_SCOPE");return directory;}
 private static File child(File directory,String name)throws IOException{if(!name.matches("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}-[0-9a-f]{64}-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\.json"))throw new IOException("REVIEW_RECEIPT_NAME");File file=new File(directory,name);if(!file.getAbsoluteFile().equals(file.getCanonicalFile()))throw new IOException("REVIEW_RECEIPT_SCOPE");return file;}
 private static String prefix(RetainedJournals.Record record){return record.id+"-"+record.stateHash+"-";}
 private static byte[] read(File file,CancellationSignal cancel,long[] budget)throws Exception{
  cancel.throwIfCanceled();java.io.FileDescriptor fd=Os.open(file.getAbsolutePath(),OsConstants.O_RDONLY|OsConstants.O_NOFOLLOW|OsConstants.O_NONBLOCK|OsConstants.O_CLOEXEC,0);Throwable primary=null;
  try(FileInputStream input=new FileInputStream(fd)){android.system.StructStat stat=Os.fstat(fd);if(!OsConstants.S_ISREG(stat.st_mode)||stat.st_size<0||stat.st_size>LIMIT||stat.st_size>budget[0])throw new IOException("REVIEW_RECEIPT_LIMIT");byte[] bytes=JournalExportBounds.read(input,(int)Math.min(LIMIT,budget[0]),cancel::isCanceled);budget[0]-=bytes.length;return bytes;}
  catch(Exception|Error failure){primary=failure;throw failure;}
  finally{try{Os.close(fd);}catch(android.system.ErrnoException failure){if(primary!=null)primary.addSuppressed(failure);else throw failure;}}
 }
 private static String hex(byte[] bytes){StringBuilder out=new StringBuilder();for(byte b:bytes)out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();}
}
