package com.nhlocal.singlesorter.app;

import java.io.IOException;
import java.util.ArrayList;

final class RecoveryContentRoutingTest {
    private static int checks;
    private static void equal(int expected,int actual){if(expected!=actual)throw new AssertionError("scope "+actual);checks++;}
    public static void main(String[] args)throws Exception{
        String source="original-source",target="separate-target",parent="selected-parent";
        ArrayList<String> probes=new ArrayList<>();
        RecoveryContentRouting.Ancestry relation=(scope,boundary,candidate)->{
            if(parent.equals(boundary))throw new AssertionError("parent became content authority");
            probes.add(scope+":"+boundary+":"+candidate);
            return source.equals(boundary)&&"source-file".equals(candidate)||target.equals(boundary)&&"target-file".equals(candidate);
        };
        RecoveryContentRouting.Checkpoint active=()->{};
        equal(0,RecoveryContentRouting.select(source,target,source,relation,active));
        equal(1,RecoveryContentRouting.select(source,target,target,relation,active));
        equal(0,RecoveryContentRouting.select(source,target,"source-file",relation,active));
        equal(1,RecoveryContentRouting.select(source,target,"target-file",relation,active));
        for(String outside:new String[]{parent,"sibling-file","sibling-directory","sibling-directory/track.mp3"})equal(-1,RecoveryContentRouting.select(source,target,outside,relation,active));
        // A removed or identity-changed source never inherits the parent boundary.
        for(String candidate:new String[]{source,"source-file","sibling-file",parent})equal(-1,RecoveryContentRouting.select(null,target,candidate,relation,active));
        equal(1,RecoveryContentRouting.select(null,target,"target-file",relation,active));
        equal(0,RecoveryContentRouting.select(source,null,"source-file",relation,active));
        equal(-1,RecoveryContentRouting.select(source,null,"target-file",relation,active));
        equal(1,RecoveryContentRouting.select(source,target,"both",(scope,boundary,candidate)->true,active));
        if(probes.stream().anyMatch(value->value.startsWith("0:"+parent+":")||value.startsWith("1:"+parent+":")))throw new AssertionError("parent probed");checks++;
        try{RecoveryContentRouting.select(source,target,"unknown",(scope,boundary,candidate)->{throw new IOException("unavailable");},active);throw new AssertionError("unknown accepted");}catch(IOException expected){checks++;}
        equal(0,RecoveryContentRouting.select(source,target,"source-file",(scope,boundary,candidate)->{if(scope==1)throw new IOException("target unavailable");return true;},active));
        final int[] calls={0};
        try{RecoveryContentRouting.select(source,target,"source-file",relation,()->{calls[0]++;throw new IOException("cancelled");});throw new AssertionError("cancel ignored");}catch(IOException expected){checks++;}
        if(calls[0]!=1)throw new AssertionError("work after cancel");checks++;
        final boolean[] cancelled={false};
        try{RecoveryContentRouting.select(source,target,"source-file",(scope,boundary,candidate)->{cancelled[0]=true;return true;},()->{if(cancelled[0])throw new IOException("cancelled");});throw new AssertionError("late cancel ignored");}catch(IOException expected){checks++;}
        for(String invalid:new String[]{null,"",new String(new char[4097])})try{RecoveryContentRouting.select(source,target,invalid,relation,active);throw new AssertionError("invalid candidate");}catch(IOException expected){checks++;}
        System.out.println(checks+" narrow recovery content routing checks passed");
    }
}
