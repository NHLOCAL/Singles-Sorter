package com.nhlocal.singlesorter.app;
import java.io.IOException;
import java.io.InputStream;
import java.security.MessageDigest;
/** Streaming current-byte evidence only; input ownership remains with its caller. */
final class RecoveryContentHash {
 private RecoveryContentHash(){}
 interface Checkpoint {void check()throws IOException;}
 static String read(InputStream input,long expected,long[] remaining,Checkpoint cancelled)throws Exception{
  return readBounded(input,expected,remaining,cancelled,64L*1024*1024);
 }
 static String readBounded(InputStream input,long expected,long[] remaining,Checkpoint cancelled,long limit)throws Exception{
  if(limit!=64L*1024*1024&&limit!=72L*1024*1024)throw new IOException("REVIEW_CONTENT_LIMIT");
  if(expected<0||expected>limit||remaining==null||remaining.length!=1||expected>remaining[0])throw new IOException("REVIEW_CONTENT_LIMIT");
  MessageDigest digest=MessageDigest.getInstance("SHA-256");long count=0;byte[] buffer=new byte[65536];
  while(true){cancelled.check();int amount=(int)Math.min(buffer.length,Math.min(expected-count+1,remaining[0]+1));int n=input.read(buffer,0,amount);if(n<0)break;if(n==0)throw new IOException("REVIEW_READ_STALLED");count+=n;remaining[0]-=n;
   if(count>expected||remaining[0]<0)throw new IOException("REVIEW_CONTENT_CHANGED");digest.update(buffer,0,n);
  }
  if(count!=expected)throw new IOException("REVIEW_CONTENT_CHANGED");cancelled.check();StringBuilder out=new StringBuilder();for(byte b:digest.digest())out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();
 }
}
