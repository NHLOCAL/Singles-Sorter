package com.nhlocal.singlesorter.app;
import java.io.IOException;
import java.io.InputStream;
/** A failed commit permanently removes this owner's authority to start another effect. */
final class CheckedJournalCommit {
    interface Commit {void run()throws Exception;}
    private boolean poisoned;
    void run(Commit commit)throws Exception{
        if(poisoned)throw new IOException("JOURNAL_POISONED");
        try{commit.run();}catch(Exception error){poisoned=true;throw error;}
    }
    static void verify(InputStream input,byte[] expected)throws IOException{
        byte[] buffer=new byte[8192];int offset=0,n;
        while((n=input.read(buffer))!=-1){
            if(n>expected.length-offset)throw new IOException("JOURNAL_READBACK_SIZE");
            for(int i=0;i<n;i++)if(buffer[i]!=expected[offset+i])throw new IOException("JOURNAL_READBACK_MISMATCH");
            offset+=n;
        }
        if(offset!=expected.length)throw new IOException("JOURNAL_READBACK_SIZE");
    }
}
