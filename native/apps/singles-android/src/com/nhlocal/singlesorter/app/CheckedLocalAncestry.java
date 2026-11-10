package com.nhlocal.singlesorter.app;
import android.content.Context;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;

/** Exact Android11 DocumentsProvider wire contract, restricted to the verified system local provider.
 * Public DocumentsContract.isChildDocument can turn provider failure/wrong-type replies into false.
 */
final class CheckedLocalAncestry {
 private CheckedLocalAncestry(){}
 @SuppressWarnings("deprecation") // Bundle.get is required to validate the actual Boolean type on API29/30.
 static boolean isChild(Context context,Uri parent,Uri child)throws Exception{
  for(Uri uri:new Uri[]{parent,child})if(uri==null||!"content".equals(uri.getScheme())||!ProviderReferences.AUTHORITY.equals(uri.getAuthority())||uri.toString().length()>8192||!DocumentsContract.isDocumentUri(context,uri))throw new SecurityException("LOCAL_DOCUMENT_REQUIRED");
  Bundle input=new Bundle();input.putParcelable("uri",parent);input.putParcelable("android.content.extra.TARGET_URI",child);
  return AncestryResult.query(()->{
   Bundle response=context.getContentResolver().call(ProviderReferences.AUTHORITY,"android:isChildDocument",null,input);
   if(response==null||!response.containsKey("result"))return null;
   return response.get("result");
  });
 }
}
