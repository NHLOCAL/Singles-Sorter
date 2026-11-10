package com.nhlocal.singlesorter.app;
public final class ExecutionCursorTest {
    private ExecutionCursorTest(){}
    public static void main(String[] args){
        ExecutionCursor cursor=new ExecutionCursor();if(cursor.current()!=-1)throw new AssertionError();
        cursor.begin(0);cursor.verified();if(cursor.current()!=-1)throw new AssertionError("Cancellation after acknowledgement must not target completed item");
        cursor.begin(1);if(cursor.current()!=1)throw new AssertionError("Cancellation before next worker step belongs to next operation");
        try{cursor.begin(2);throw new AssertionError();}catch(IllegalStateException expected){}
        cursor.verified();if(cursor.current()!=-1)throw new AssertionError();
        System.out.println("5 execution cursor checks passed");
    }
}
