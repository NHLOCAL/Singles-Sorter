package com.nhlocal.singlesorter.app;

/** One mutually exclusive terminal outcome per retained operation. Job errors remain separate. */
final class ExecutionOutcomes {
    final String[] status,detail,destination;
    final boolean[] attempted;
    ExecutionOutcomes(int count){if(count<0||count>10000)throw new IllegalArgumentException("count");status=new String[count];detail=new String[count];destination=new String[count];attempted=new boolean[count];}
    void begin(int id){check(id);if(status[id]!=null||attempted[id])throw new IllegalStateException("operation owner");attempted[id]=true;}
    void finish(int id,String outcome,String description){
        check(id);if(!attempted[id]||status[id]!=null||!outcome.matches("complete|failed|cancelled|needs_recovery"))throw new IllegalStateException("operation outcome");
        status[id]=outcome;detail[id]=description;
    }
    void destination(int id,String evidence){
        check(id);if(!attempted[id]||evidence==null||destination[id]!=null&&!destination[id].equals(evidence))throw new IllegalStateException("destination evidence");
        destination[id]=evidence;
    }
    int committed(){int count=0;for(String evidence:destination)if(evidence!=null)count++;return count;}
    void stop(){
        for(int id=0;id<status.length;id++)if(status[id]==null){status[id]=attempted[id]?"needs_recovery":"blocked";detail[id]=attempted[id]?"OPERATION_OUTCOME_UNCONFIRMED":"NOT_ATTEMPTED";}
    }
    int count(String value){int count=0;for(String item:status)if(value.equals(item))count++;return count;}
    boolean inactive(int id){check(id);return !attempted[id]&&"blocked".equals(status[id]);}
    private void check(int id){if(id<0||id>=status.length)throw new IllegalArgumentException("operation");}
}
