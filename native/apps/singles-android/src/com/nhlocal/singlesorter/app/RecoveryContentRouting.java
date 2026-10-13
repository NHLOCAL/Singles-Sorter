package com.nhlocal.singlesorter.app;

import java.io.IOException;

/** Content boundaries are original source and separately selected target IDs.
 * A supplemental parent grant is transport authority only and is never a boundary.
 */
final class RecoveryContentRouting {
    private RecoveryContentRouting(){}
    interface Ancestry { boolean contains(int scope,String boundaryId,String candidateId)throws Exception; }
    interface Checkpoint { void check()throws Exception; }
    static int select(String source,String target,String candidate,Ancestry ancestry,Checkpoint checkpoint)throws Exception{
        if(candidate==null||candidate.isEmpty()||candidate.length()>4096)throw new IOException("REVIEW_REFERENCE_INVALID");
        String[] boundaries={source,target};Exception unknown=null;
        // The target can be beneath a supplemental source parent, but keeps its own grant.
        for(int scope=1;scope>=0;scope--){checkpoint.check();String boundary=boundaries[scope];if(boundary==null)continue;
            if(boundary.isEmpty()||boundary.length()>4096)throw new IOException("REVIEW_ROOT_INVALID");
            if(boundary.equals(candidate))return scope;
            try{boolean contains=ancestry.contains(scope,boundary,candidate);checkpoint.check();if(contains)return scope;}
            catch(Exception unavailable){checkpoint.check();unknown=unavailable;}
        }
        checkpoint.check();if(unknown!=null)throw unknown;return -1;
    }
}
