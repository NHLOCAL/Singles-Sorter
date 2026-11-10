package com.nhlocal.singlesorter.app;
final class RecoveryOriginalComparisonTest {
 public static void main(String[] args){String hash="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";int checks=0;
  if(!RecoveryOriginalComparison.matches(12,hash,12,hash))throw new AssertionError();checks++;
  if(RecoveryOriginalComparison.matches(12,hash,13,hash))throw new AssertionError();checks++;
  if(RecoveryOriginalComparison.matches(12,hash,12,hash.replace('a','b')))throw new AssertionError();checks++;
  if(RecoveryOriginalComparison.matches(-1,hash,-1,hash))throw new AssertionError();checks++;
  if(RecoveryOriginalComparison.matches(0,null,0,null))throw new AssertionError();checks++;
  if(RecoveryOriginalComparison.matches(0,"short",0,"short"))throw new AssertionError();checks++;
  System.out.println(checks+" original/current content comparison checks passed");
 }
}
