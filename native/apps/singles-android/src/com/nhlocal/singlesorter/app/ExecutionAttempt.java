package com.nhlocal.singlesorter.app;
/** One native retained preview grants at most one attempted execution, even on preflight failure. */
final class ExecutionAttempt {
    private final String id;private boolean consumed,cancelled;
    ExecutionAttempt(String id){this.id=id;}
    synchronized void claim(){if(consumed)throw new IllegalStateException("PLAN_ALREADY_ATTEMPTED");consumed=true;}
    synchronized void cancel(String requested,Runnable signal){if(!consumed||!id.equals(requested))throw new IllegalArgumentException("EXECUTION_NOT_FOUND");cancelled=true;signal.run();}
    synchronized boolean cancelled(){return cancelled;}
}
