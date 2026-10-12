package com.nhlocal.singlesorter.app;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.util.concurrent.atomic.AtomicBoolean;
public final class JournalExportBoundsTest {
    private JournalExportBoundsTest(){}
    public static void main(String[] args)throws Exception{
        int checks=0;
        for(String invalid:new String[]{null,"","../state.json","00000000-0000-0000-0000-000000000000/..","AAAAAAAA-0000-0000-0000-000000000000","00000000-0000-0000-0000-00000000000"}){
            if(JournalExportBounds.id(invalid))throw new AssertionError();checks++;
        }
        if(!JournalExportBounds.id("00000000-0000-0000-0000-000000000000"))throw new AssertionError();checks++;
        for(String name:new String[]{"authority.json","state.json","index.json"}){if(JournalExportBounds.limit(name)<=0)throw new AssertionError();checks++;}
        try{JournalExportBounds.limit("../index.json");throw new AssertionError();}catch(IllegalArgumentException expected){checks++;}
        AtomicBoolean flag=new AtomicBoolean();
        if(JournalExportBounds.read(new ByteArrayInputStream(new byte[]{1,2}),2,flag).length!=2)throw new AssertionError();checks++;
        try{JournalExportBounds.read(new ByteArrayInputStream(new byte[]{1,2,3}),2,flag);throw new AssertionError();}catch(IOException expected){checks++;}
        flag.set(true);
        try{JournalExportBounds.read(new ByteArrayInputStream(new byte[0]),0,flag);throw new AssertionError();}catch(IOException expected){checks++;}
        java.nio.file.Path temp=java.nio.file.Files.createTempDirectory("owned-journal-containment-");
        java.nio.file.Path real=java.nio.file.Files.createDirectory(temp.resolve("real"));
        java.nio.file.Path alias=java.nio.file.Files.createSymbolicLink(temp.resolve("alias"),real);
        java.nio.file.Path journal=java.nio.file.Files.createDirectory(real.resolve("provider-executions"));
        if(!JournalExportBounds.root(alias.toFile()).equals(journal.toFile()))throw new AssertionError();checks++;
        String id="00000000-0000-0000-0000-000000000000";
        java.nio.file.Path outside=java.nio.file.Files.createDirectory(temp.resolve("outside"));
        java.nio.file.Path uuid=java.nio.file.Files.createSymbolicLink(journal.resolve(id),outside);
        try{JournalExportBounds.child(journal.toFile(),id);throw new AssertionError();}catch(IOException expected){checks++;}
        java.nio.file.Files.delete(uuid);java.nio.file.Files.delete(journal);
        java.nio.file.Files.createSymbolicLink(journal,outside);
        try{JournalExportBounds.root(alias.toFile());throw new AssertionError();}catch(IOException expected){checks++;}
        java.nio.file.Files.delete(journal);java.nio.file.Files.delete(alias);java.nio.file.Files.delete(real);java.nio.file.Files.delete(outside);java.nio.file.Files.delete(temp);
        System.out.println(checks+" journal export bounds checks passed");
    }
}
