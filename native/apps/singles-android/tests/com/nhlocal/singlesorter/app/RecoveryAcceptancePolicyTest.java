package com.nhlocal.singlesorter.app;
final class RecoveryAcceptancePolicyTest {
 public static void main(String[] args){String[] expected={"execution","job","plan","authority","state","index","source-root","target-root"};int checks=0;
  if(!RecoveryAcceptancePolicy.matches(RecoveryAcceptancePolicy.POLICY,true,true,false,expected,expected.clone()))throw new AssertionError();checks++;
  for(int i=0;i<expected.length;i++){String[] changed=expected.clone();changed[i]+="changed";if(RecoveryAcceptancePolicy.matches(RecoveryAcceptancePolicy.POLICY,true,true,false,expected,changed))throw new AssertionError();checks++;}
  if(RecoveryAcceptancePolicy.matches("unknown",true,true,false,expected,expected))throw new AssertionError();checks++;
  if(RecoveryAcceptancePolicy.matches(RecoveryAcceptancePolicy.POLICY,false,true,false,expected,expected))throw new AssertionError();checks++;
  if(RecoveryAcceptancePolicy.matches(RecoveryAcceptancePolicy.POLICY,true,false,false,expected,expected))throw new AssertionError();checks++;
  if(RecoveryAcceptancePolicy.matches(RecoveryAcceptancePolicy.POLICY,true,true,true,expected,expected))throw new AssertionError();checks++;
  if(RecoveryAcceptancePolicy.matches(RecoveryAcceptancePolicy.POLICY,true,true,false,expected,new String[0]))throw new AssertionError();checks++;
  System.out.println(checks+" exact retained-effects acceptance policy checks passed");
 }
}
