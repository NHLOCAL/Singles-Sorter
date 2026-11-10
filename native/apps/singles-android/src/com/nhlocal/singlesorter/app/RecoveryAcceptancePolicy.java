package com.nhlocal.singlesorter.app;
/** Acknowledgment admits a new preview; it never changes a historical outcome. */
final class RecoveryAcceptancePolicy {
 static final String POLICY="retain-current-effects-new-preview-v1";
 static final String FIX_POLICY="retain-current-fix-effects-new-preview-v1";
 private RecoveryAcceptancePolicy(){}
 static boolean matches(String policy,boolean retainedEffects,boolean uncertaintyAcknowledged,boolean historicalSuccessChanged,String[] expected,String[] actual){
  return matchesMode(false,policy,retainedEffects,uncertaintyAcknowledged,historicalSuccessChanged,expected,actual);
 }
 static boolean matchesMode(boolean fix,String policy,boolean retainedEffects,boolean uncertaintyAcknowledged,boolean historicalSuccessChanged,String[] expected,String[] actual){
  if(!(fix?FIX_POLICY:POLICY).equals(policy)||!retainedEffects||!uncertaintyAcknowledged||historicalSuccessChanged||expected==null||actual==null||expected.length!=8||actual.length!=8)return false;
  for(int i=0;i<expected.length;i++)if(expected[i]==null||!expected[i].equals(actual[i]))return false;return true;
 }
}
