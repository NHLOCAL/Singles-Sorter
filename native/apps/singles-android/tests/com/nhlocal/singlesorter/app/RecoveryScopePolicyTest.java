package com.nhlocal.singlesorter.app;
public final class RecoveryScopePolicyTest {
 private RecoveryScopePolicyTest(){}
 public static void main(String[] args){int checks=0;
  for(boolean same:new boolean[]{false,true})for(Boolean forward:new Boolean[]{null,false,true})for(Boolean reverse:new Boolean[]{null,false,true}){
   boolean expected=!same&&Boolean.FALSE.equals(forward)&&Boolean.FALSE.equals(reverse);
   if(RecoveryScopePolicy.disjoint(same,forward,reverse)!=expected)throw new AssertionError();checks++;
  }
  java.util.concurrent.atomic.AtomicInteger calls=new java.util.concurrent.atomic.AtomicInteger();
  try{JournalExportBounds.read(new java.io.ByteArrayInputStream(new byte[10]),20,()->calls.incrementAndGet()>1);throw new AssertionError();}catch(java.io.IOException expected){checks++;}
  System.out.println(checks+" recovery scope/cancellation checks passed");
 }
}
