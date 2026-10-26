package com.nhlocal.singlesorter.app;
final class RecoveryIntentLookupTest {
 public static void main(String[] args)throws Exception{
  int checks=0;
  for(String step:new String[]{"move","move-stage","publish-move"}){
   if(!"target-parent".equals(RecoveryIntentLookup.parent("intent","operation-1/"+step,null,"target-parent","song.wav")))throw new AssertionError("lost-return move candidate missing");checks++;
  }
  if(!"parent".equals(RecoveryIntentLookup.parent("intent","operation-1/create-stage","parent",null,"stage.bin")))throw new AssertionError();checks++;
  if(RecoveryIntentLookup.parent("returned","operation-1/move",null,null,"song.wav")!=null)throw new AssertionError();checks++;
  if(RecoveryIntentLookup.parent("intent","operation-1/write-stage",null,null,null)!=null)throw new AssertionError();checks++;
  try{RecoveryIntentLookup.parent("intent","operation-1/move","wrong",null,"song.wav");throw new AssertionError("used wrong move parent");}catch(java.io.IOException expected){checks++;}
  String[] statuses={"name_not_observed","named_candidate_observed","ambiguous_name","ambiguous_name"};
  for(int n=0;n<statuses.length;n++){if(!statuses[n].equals(RecoveryIntentLookup.status(n)))throw new AssertionError();checks++;}
  System.out.println(checks+" lost-return intent/candidate ambiguity checks passed");
 }
}
