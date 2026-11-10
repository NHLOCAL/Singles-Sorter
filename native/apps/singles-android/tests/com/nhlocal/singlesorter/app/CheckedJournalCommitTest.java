package com.nhlocal.singlesorter.app;
import java.io.ByteArrayInputStream;
import java.io.IOException;
public final class CheckedJournalCommitTest {
    public static void main(String[] args)throws Exception{
        byte[] expected={1,2,3};CheckedJournalCommit.verify(new ByteArrayInputStream(expected),expected);
        for(byte[] wrong:new byte[][]{{1,2},{1,2,4},{1,2,3,4},{}}){
            try{CheckedJournalCommit.verify(new ByteArrayInputStream(wrong),expected);throw new AssertionError("bad readback accepted");}catch(IOException correct){}
        }
        CheckedJournalCommit guard=new CheckedJournalCommit();int[] actions={0};guard.run(()->actions[0]++);
        try{guard.run(()->{actions[0]++;throw new IOException("simulated rename/readback failure");});throw new AssertionError("commit failure ignored");}catch(IOException correct){}
        try{guard.run(()->actions[0]++);throw new AssertionError("poison ignored");}catch(IOException correct){}
        if(actions[0]!=2)throw new AssertionError("successor action ran after uncertainty");
        CheckedJournalCommit silent=new CheckedJournalCommit();int[] silentActions={0};
        try{silent.run(()->{silentActions[0]++;CheckedJournalCommit.verify(new ByteArrayInputStream(new byte[]{1,2,2}),expected);});throw new AssertionError("old base accepted after silent rename failure");}catch(IOException correct){}
        try{silent.run(()->silentActions[0]++);throw new AssertionError("silent failure poison ignored");}catch(IOException correct){}
        if(silentActions[0]!=1)throw new AssertionError("successor ran after old base readback");
        System.out.println("11 checked journal commit checks passed");
    }
}
