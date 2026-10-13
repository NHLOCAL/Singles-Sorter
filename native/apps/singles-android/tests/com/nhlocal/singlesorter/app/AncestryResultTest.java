package com.nhlocal.singlesorter.app;
public final class AncestryResultTest {
 private AncestryResultTest(){}
 public static void main(String[] args)throws Exception{int checks=0;
  if(AncestryResult.query(()->Boolean.FALSE)||!AncestryResult.query(()->Boolean.TRUE))throw new AssertionError();checks+=2;
  for(Object value:new Object[]{null,"false",0,1,new boolean[]{false}}){try{AncestryResult.query(()->value);throw new AssertionError();}catch(java.io.IOException expected){checks++;}}
  java.io.IOException io=new java.io.IOException("provider failure");try{AncestryResult.query(()->{throw io;});throw new AssertionError();}catch(java.io.IOException expected){if(expected!=io)throw new AssertionError();checks++;}
  SecurityException denied=new SecurityException("provider denied");try{AncestryResult.query(()->{throw denied;});throw new AssertionError();}catch(SecurityException expected){if(expected!=denied)throw new AssertionError();checks++;}
  System.out.println(checks+" strict ancestry response checks passed");
 }
}
