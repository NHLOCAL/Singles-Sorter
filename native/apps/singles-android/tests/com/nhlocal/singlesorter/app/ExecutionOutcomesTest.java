package com.nhlocal.singlesorter.app;
public final class ExecutionOutcomesTest {
    private ExecutionOutcomesTest(){}
    public static void main(String[] args){
        ExecutionOutcomes preflight=new ExecutionOutcomes(1);preflight.stop();
        if(preflight.count("failed")!=0||preflight.count("blocked")!=1||!preflight.inactive(0))throw new AssertionError();
        ExecutionOutcomes mixed=new ExecutionOutcomes(2);mixed.begin(0);mixed.finish(0,"complete","copied");mixed.stop();
        if(mixed.count("complete")!=1||mixed.count("blocked")!=1||mixed.inactive(0)||!mixed.inactive(1))throw new AssertionError();
        ExecutionOutcomes failed=new ExecutionOutcomes(2);failed.begin(0);failed.finish(0,"failed","stale");failed.stop();
        if(failed.count("failed")!=1||failed.count("blocked")!=1||failed.inactive(0))throw new AssertionError();
        ExecutionOutcomes cancel=new ExecutionOutcomes(2);cancel.begin(0);cancel.finish(0,"complete","copied");cancel.begin(1);cancel.finish(1,"cancelled","cancelled");cancel.stop();
        if(cancel.count("complete")!=1||cancel.count("cancelled")!=1||cancel.inactive(1))throw new AssertionError();
        ExecutionOutcomes unknown=new ExecutionOutcomes(1);unknown.begin(0);unknown.stop();
        if(unknown.count("needs_recovery")!=1||unknown.inactive(0))throw new AssertionError();
        try{mixed.finish(0,"failed","duplicate");throw new AssertionError();}catch(IllegalStateException expected){}
        ExecutionOutcomes partial=new ExecutionOutcomes(1);partial.begin(0);partial.destination(0,"hash-bound destination");partial.finish(0,"needs_recovery","later source check failed");partial.stop();
        if(partial.committed()!=1||partial.count("complete")!=0||partial.count("needs_recovery")!=1||partial.inactive(0))throw new AssertionError();
        try{partial.destination(0,"different destination");throw new AssertionError();}catch(IllegalStateException expected){}
        System.out.println("8 exact execution outcome checks passed");
    }
}
