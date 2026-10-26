package com.nhlocal.singlesorter.app;
/** Only an unacknowledged operation can receive a cancellation/failure outcome. */
final class ExecutionCursor {
    private int current=-1;
    void begin(int id){if(id<0||current!=-1)throw new IllegalStateException("OPERATION_OWNERSHIP");current=id;}
    void verified(){if(current<0)throw new IllegalStateException("OPERATION_OWNERSHIP");current=-1;}
    int current(){return current;}
}
