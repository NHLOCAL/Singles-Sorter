package com.nhlocal.singlesorter.app;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.concurrent.atomic.AtomicBoolean;

/** Fixed private journal names only; never a renderer-provided filesystem path. */
final class JournalExportBounds {
    private JournalExportBounds(){}
    static java.io.File root(java.io.File appData)throws IOException{
        return child(appData.getCanonicalFile(),"provider-executions");
    }
    static java.io.File child(java.io.File parent,String name)throws IOException{
        if(!"provider-executions".equals(name)&&!id(name))throw new IOException("journal name");
        java.io.File expected=new java.io.File(parent,name);
        if(!expected.getCanonicalFile().equals(expected.getAbsoluteFile()))throw new IOException("journal containment");
        return expected;
    }
    static boolean id(String value){return value!=null&&value.matches("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}");}
    static int limit(String name){
        if("authority.json".equals(name))return 16*1024*1024;
        if("state.json".equals(name))return 8*1024*1024;
        if("index.json".equals(name))return 4096;
        throw new IllegalArgumentException("JOURNAL_FILE");
    }
    static void checkpoint(AtomicBoolean cancelled)throws IOException{if(cancelled.get())throw new IOException("cancelled");}
    static byte[] read(InputStream input,int limit,AtomicBoolean cancelled)throws IOException{return read(input,limit,cancelled::get);}
    static void checkpoint(java.util.function.BooleanSupplier cancelled)throws IOException{if(cancelled.getAsBoolean())throw new IOException("cancelled");}
    static byte[] read(InputStream input,int limit,java.util.function.BooleanSupplier cancelled)throws IOException{
        if(limit<0||limit>16*1024*1024)throw new IOException("bound");
        ByteArrayOutputStream output=new ByteArrayOutputStream();byte[] buffer=new byte[65536];int n;
        checkpoint(cancelled);
        while((n=input.read(buffer))!=-1){checkpoint(cancelled);if(n>limit-output.size())throw new IOException("bound");output.write(buffer,0,n);}
        checkpoint(cancelled);return output.toByteArray();
    }
}
