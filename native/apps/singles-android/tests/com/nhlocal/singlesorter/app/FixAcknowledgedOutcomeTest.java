package com.nhlocal.singlesorter.app;
public final class FixAcknowledgedOutcomeTest {
 private static int count;private FixAcknowledgedOutcomeTest(){}private static void check(boolean ok){if(!ok)throw new AssertionError();count++;}
 public static void main(String[] args){
  for(String limitation:new String[]{"unsupported_tags","metadata_failure"}){check(FixAcknowledgedOutcome.mayContinue(limitation,true,false));check(!FixAcknowledgedOutcome.mayContinue(limitation,false,false));check(!FixAcknowledgedOutcome.mayContinue(limitation,true,true));}
  for(String stop:new String[]{"failed","cancelled","needs_recovery","complete"})check(!FixAcknowledgedOutcome.mayContinue(stop,true,false));
  check(!FixAcknowledgedOutcome.needsAttention(true,"partial",2,2,8));check(FixAcknowledgedOutcome.needsAttention(true,"partial",2,1,8));check(FixAcknowledgedOutcome.needsAttention(false,"partial",2,2,8));check(!FixAcknowledgedOutcome.needsAttention(false,"failed",2,0,0));
  System.out.println(count+" acknowledged Fix continuation and historical attention checks passed");
 }
}
