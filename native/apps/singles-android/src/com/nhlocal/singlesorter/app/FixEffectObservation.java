package com.nhlocal.singlesorter.app;
/** Producer-used rule: a provider-adjusted return is retained as mismatch, never a checked observation. */
final class FixEffectObservation {
 private FixEffectObservation(){}
 static String phase(String expectedName,boolean expectedDirectory,String actualName,boolean actualDirectory,boolean child){return child&&expectedName.equals(actualName)&&expectedDirectory==actualDirectory?"observed":"effect-mismatch";}
}
