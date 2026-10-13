package com.nhlocal.singlesorter.app;
public final class FixEffectObservationTest {
 private FixEffectObservationTest(){}
 public static void main(String[] args){int checks=0;for(boolean directory:new boolean[]{false,true}){if(!"observed".equals(FixEffectObservation.phase("exact",directory,"exact",directory,true)))throw new AssertionError();checks++;
  if(!"effect-mismatch".equals(FixEffectObservation.phase("exact",directory,"exact (1)",directory,true)))throw new AssertionError();checks++;
  if(!"effect-mismatch".equals(FixEffectObservation.phase("exact",directory,"exact",!directory,true)))throw new AssertionError();checks++;
  if(!"effect-mismatch".equals(FixEffectObservation.phase("exact",directory,"exact",directory,false)))throw new AssertionError();checks++;}
  System.out.println(checks+" checked Fix observation and retained mismatch checks passed");}
}
