package com.nhlocal.singlesorter.app;

import java.io.IOException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

/** Host-only membership tests: no provider, JNI, Android runtime, or mutation calls. */
public final class AlbumSnapshotEvidenceTest {
    private static int count;
    private AlbumSnapshotEvidenceTest(){}
    interface Checked {void run()throws Exception;}
    private static void check(boolean value){if(!value)throw new AssertionError();count++;}
    private static void rejects(String code,Checked action)throws Exception {
        try {action.run();throw new AssertionError("accepted invalid evidence");}
        catch(IOException expected){check(code.equals(expected.getMessage()));}
    }
    public static void main(String[] args)throws Exception {
        SafSnapshot.DirectoryListings evidence=new SafSnapshot.DirectoryListings();
        check(!evidence.complete("nested"));
        rejects("DIRECTORY_NOT_COMPLETELY_CAPTURED",()->evidence.require("nested",Collections.emptyList()));
        // Main-folder-only capture may list a directory without enumerating it.
        evidence.observe("root",Arrays.asList("album","single"));
        evidence.observe("album",Arrays.asList("track","nested"));
        check(evidence.complete("album")&&!evidence.complete("nested"));
        evidence.observe("nested",Arrays.asList("deep-track"));
        check(evidence.complete("nested"));
        evidence.require("album",Arrays.asList("nested","track"));check(true);
        // Enumeration order is provider observation only; set membership must not change.
        rejects("DIRECTORY_MEMBERSHIP_CHANGED",()->evidence.observe("album",Arrays.asList("track","nested","added")));
        rejects("DIRECTORY_MEMBERSHIP_CHANGED",()->evidence.require("album",Arrays.asList("track")));
        rejects("DIRECTORY_MEMBERSHIP_CHANGED",()->evidence.require("nested",Collections.emptyList()));
        evidence.observe("empty",Collections.emptyList());check(evidence.complete("empty"));
        evidence.require("empty",Collections.emptyList());check(true);
        rejects("DIRECTORY_MEMBERSHIP_CHANGED",()->evidence.require("empty",Arrays.asList("new")));
        rejects("TREE_BUDGET_OR_CYCLE",()->evidence.observe("duplicate",Arrays.asList("a","a")));
        rejects("TREE_BUDGET_OR_CYCLE",()->evidence.observe("self",Arrays.asList("self")));
        rejects("TREE_BUDGET_OR_CYCLE",()->evidence.observe("null",Arrays.asList((String)null)));
        check(!evidence.complete("duplicate")&&!evidence.complete("self")&&!evidence.complete("null"));
        List<String> children=new ArrayList<>(Arrays.asList("x"));evidence.observe("copied",children);children.add("y");
        evidence.require("copied",Arrays.asList("x"));check(true);
        rejects("DIRECTORY_MEMBERSHIP_CHANGED",()->evidence.require("copied",children));
        List<String> large=new ArrayList<>();for(int i=0;i<2000;i++)large.add("entry-"+i);
        evidence.observe("bounded",large);check(evidence.complete("bounded"));large.add("overflow");
        rejects("TREE_BUDGET_OR_CYCLE",()->evidence.observe("overflow",large));check(!evidence.complete("overflow"));
        System.out.println(count+" retained Album completeness, membership and bound checks passed");
    }
}
