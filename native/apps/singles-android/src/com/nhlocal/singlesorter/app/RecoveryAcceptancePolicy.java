package com.nhlocal.singlesorter.app;
/** Acknowledgment admits a new preview; it never changes a historical outcome. */
final class RecoveryAcceptancePolicy {
 static final String POLICY="retain-current-effects-new-preview-v1";
 private RecoveryAcceptancePolicy(){}
 static boolean matches(String policy,boolean retainedEffects,boolean uncertaintyAcknowledged,boolean historicalSuccessChanged,String[] expected,String[] actual){
  if(!POLICY.equals(policy)||!retainedEffects||!uncertaintyAcknowledged||historicalSuccessChanged||expected==null||actual==null||expected.length!=8||actual.length!=8)return false;
  for(int i=0;i<expected.length;i++)if(expected[i]==null||!expected[i].equals(actual[i]))return false;return true;
 }
}
