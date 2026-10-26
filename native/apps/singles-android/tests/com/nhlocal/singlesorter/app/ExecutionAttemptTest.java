package com.nhlocal.singlesorter.app;
public final class ExecutionAttemptTest {
    private ExecutionAttemptTest(){}
    public static void main(String[] args){
        ExecutionAttempt attempt=new ExecutionAttempt("owned");int[] signals={0};
        try{attempt.cancel("owned",()->signals[0]++);throw new AssertionError();}catch(IllegalArgumentException expected){}
        attempt.claim();
        try{attempt.claim();throw new AssertionError();}catch(IllegalStateException expected){}
        try{attempt.cancel("stale",()->signals[0]++);throw new AssertionError();}catch(IllegalArgumentException expected){}
        if(signals[0]!=0||attempt.cancelled())throw new AssertionError();
        attempt.cancel("owned",()->signals[0]++);
        if(!attempt.cancelled()||signals[0]!=1)throw new AssertionError();
        // Submission failure does not restore a consumed plan; cancellation is visible before worker start.
        try{attempt.claim();throw new AssertionError();}catch(IllegalStateException expected){}
        ExecutionAttempt separate=new ExecutionAttempt("next");separate.claim();if(separate.cancelled())throw new AssertionError();
        System.out.println("7 execution attempt ownership checks passed");
    }
}
