package com.nhlocal.singlesorter.app;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
final class RecoveryContentHashTest {
 interface Work {void run()throws Exception;}
 static int checks;
 static void refuse(String expected,Work work)throws Exception{try{work.run();throw new AssertionError("accepted");}catch(IOException failure){if(!expected.equals(failure.getMessage()))throw new AssertionError(failure);}checks++;}
 public static void main(String[] args)throws Exception{
  byte[] abc={97,98,99};long[] budget={3};String hash=RecoveryContentHash.read(new ByteArrayInputStream(abc),3,budget,()->{});
  if(!hash.equals("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")||budget[0]!=0)throw new AssertionError();checks++;
  refuse("REVIEW_CONTENT_CHANGED",()->RecoveryContentHash.read(new ByteArrayInputStream(abc),2,new long[]{8},()->{}));
  refuse("REVIEW_CONTENT_CHANGED",()->RecoveryContentHash.read(new ByteArrayInputStream(abc),4,new long[]{8},()->{}));
  refuse("REVIEW_CONTENT_LIMIT",()->RecoveryContentHash.read(new ByteArrayInputStream(abc),3,new long[]{2},()->{}));
  refuse("REVIEW_CONTENT_LIMIT",()->RecoveryContentHash.read(new ByteArrayInputStream(abc),-1,new long[]{8},()->{}));
  refuse("REVIEW_CONTENT_LIMIT",()->RecoveryContentHash.read(new ByteArrayInputStream(abc),67108865,new long[]{99999999},()->{}));
  InputStream untouched=new InputStream(){@Override public int read(){throw new AssertionError("read after cancel");}};
  refuse("cancelled",()->RecoveryContentHash.read(untouched,0,new long[]{0},()->{throw new IOException("cancelled");}));
  int[] calls={0};refuse("cancelled",()->RecoveryContentHash.read(new ByteArrayInputStream(abc),3,new long[]{3},()->{if(++calls[0]==2)throw new IOException("cancelled");}));
  InputStream stalled=new InputStream(){@Override public int read(){return 0;}@Override public int read(byte[] b,int offset,int length){return 0;}};
  refuse("REVIEW_READ_STALLED",()->RecoveryContentHash.read(stalled,0,new long[]{1},()->{}));
  System.out.println(checks+" recovery current-content hash checks passed");
 }
}
