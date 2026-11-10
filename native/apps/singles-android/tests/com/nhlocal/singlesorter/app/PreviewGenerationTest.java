package com.nhlocal.singlesorter.app;
final class PreviewGenerationTest {
 interface Work{void run()throws Exception;}
 static int checks;static void refuses(Work work)throws Exception{try{work.run();throw new AssertionError("old preview accepted");}catch(java.io.IOException expected){checks++;}}
 public static void main(String[] args)throws Exception{
  PreviewGeneration generation=new PreviewGeneration(0);long before=generation.capture();generation.require(before);checks++;
  generation.invalidate();refuses(()->generation.require(before));long after=generation.capture();generation.require(after);checks++;
  int[] effects={0};refuses(()->{generation.require(before);effects[0]++;});if(effects[0]!=0)throw new AssertionError();checks++;
  // A preview that started before acknowledgment cannot acquire the new epoch at completion.
  generation.invalidate();refuses(()->generation.require(after));generation.require(generation.capture());checks++;
  // Simulate a receipt commit paused after invalidation: no new preview can capture
  // the new epoch until commit/readback (or failure cleanup) has finished.
  long preCommit=generation.capture();generation.beginAcceptance();
  refuses(()->generation.capture());refuses(()->generation.require(preCommit));
  refuses(generation::beginAcceptance);generation.endAcceptance();
  refuses(()->generation.require(preCommit));generation.require(generation.capture());checks++;
  PreviewGeneration overflow=new PreviewGeneration(Long.MAX_VALUE);long last=overflow.capture();refuses(overflow::invalidate);refuses(()->overflow.require(last));refuses(()->overflow.capture());refuses(overflow::invalidate);
  System.out.println(checks+" queued/completing preview generation checks passed");
 }
}
