package com.nhlocal.singlesorter.app;

/** Historical acknowledgments only. Never infers current objects or grants retry authority. */
final class RecoveryAcknowledgments {
    private final boolean[] acknowledged,started,publishedReturned,publishedObserved;
    private int sequence;
    private final String[] publicationStep;
    RecoveryAcknowledgments(int operations){if(operations<0||operations>10000)throw new IllegalArgumentException("recovery operations");acknowledged=new boolean[operations];started=new boolean[operations];publishedReturned=new boolean[operations];publishedObserved=new boolean[operations];publicationStep=new String[operations];}
    void event(int number,String phase,String operation){
        if(number!=sequence++||number>=10000)throw new IllegalArgumentException("recovery sequence");
        if(!phase.matches("intent|returned|observed|destination-observed|verified"))throw new IllegalArgumentException("recovery phase");
        if(operation==null||!operation.matches("operation-[0-9]{1,5}(/(directory-[0-9]{1,3}|create-stage|write-stage|publish-copy|create-move-stage|move|move-stage|rename-stage|publish-move|remove-empty-stage))?"))throw new IllegalArgumentException("recovery operation");
        int slash=operation.indexOf('/');int id=Integer.parseInt(operation.substring(10,slash<0?operation.length():slash));
        if(id>=acknowledged.length)throw new IllegalArgumentException("recovery operation");
        if(acknowledged[id])throw new IllegalArgumentException("recovery after acknowledgment");
        if(phase.equals("intent"))started[id]=true;
        if(!started[id])throw new IllegalArgumentException("recovery missing intent");
        boolean publication=operation.endsWith("/publish-copy")||operation.endsWith("/publish-move")||operation.endsWith("/move");
        if(publication&&phase.equals("returned")){publishedReturned[id]=true;publicationStep[id]=operation;}
        if(publication&&phase.equals("observed")){if(!publishedReturned[id]||!operation.equals(publicationStep[id]))throw new IllegalArgumentException("recovery publication");publishedObserved[id]=true;}
        if((phase.equals("verified")||phase.equals("destination-observed"))&&(!publishedReturned[id]||!publishedObserved[id]))throw new IllegalArgumentException("recovery publication");
        // Schema 1 producers predating destination-observed used returned + observed
        // publication followed by verified. Their historical receipts remain displayable.
        if(phase.equals("verified")){
            if(slash>=0||acknowledged[id])throw new IllegalArgumentException("recovery acknowledgment");
            acknowledged[id]=true;
        }
    }
    int acknowledged(){int count=0;for(boolean value:acknowledged)if(value)count++;return count;}
    int unresolved(){return acknowledged.length-acknowledged();}
}
