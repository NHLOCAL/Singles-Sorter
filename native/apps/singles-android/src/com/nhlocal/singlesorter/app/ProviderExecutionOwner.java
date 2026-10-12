package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.os.CancellationSignal;
import android.os.OperationCanceledException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONArray;
import org.json.JSONObject;

/** Process-owned worker. Activity destruction cancels but never releases an active owner. */
final class ProviderExecutionOwner {
    private static final ExecutorService WORKER=Executors.newSingleThreadExecutor();
    private static Job current;
    private static final DiagnosticReadGate DIAGNOSTICS=new DiagnosticReadGate();
    static synchronized DiagnosticReadGate.Lease beginDiagnosticRead(){return DIAGNOSTICS.acquire(active());}
    private ProviderExecutionOwner(){}
    private static final class Job {
        final CancellationSignal cancel=new CancellationSignal();final ExecutionAttempt attempt;
        final String previewId,planId;final int count;
        final ExecutionOutcomes outcomes;
        boolean active=true;String id,status="preparing",error,journal;int completed,moved;long bytes;final ExecutionCursor cursor=new ExecutionCursor();
        Job(RetainedPlan retained,int count){attempt=retained.attempt;previewId=retained.jobId;planId=retained.planId;this.count=count;outcomes=new ExecutionOutcomes(count);}
    }
    static synchronized boolean active(){return current!=null&&current.active;}
    static synchronized void cancel(String id){if(current==null||!current.previewId.equals(id))throw new IllegalArgumentException("EXECUTION_NOT_FOUND");current.attempt.cancel(id,current.cancel::cancel);}
    static synchronized void cancelOwned(String id){if(id!=null&&current!=null&&current.active&&id.equals(current.previewId))current.cancel.cancel();}
    static synchronized JSONObject snapshot(String id)throws Exception{
        if(current==null||!id.equals(current.previewId))throw new IllegalArgumentException("EXECUTION_NOT_FOUND");Job j=current;
        JSONObject result=null;
        if(!j.active){
            JSONArray items=new JSONArray();for(int operation=0;operation<j.count;operation++)items.put(new JSONObject().put("operation",operation).put("status",j.outcomes.status[operation]).put("detail",j.outcomes.detail[operation]).put("attempted",j.outcomes.attempted[operation]).put("destination_evidence",j.outcomes.destination[operation]==null?JSONObject.NULL:new JSONObject(j.outcomes.destination[operation])));
            result=new JSONObject().put("source_items",j.count).put("committed_destinations",j.outcomes.committed()).put("moved_sources",j.moved)
                .put("failed_operations",j.outcomes.count("failed")).put("blocked_operations",j.outcomes.count("blocked"))
                .put("recovery_operations",j.outcomes.count("needs_recovery")).put("cancelled_operations",j.outcomes.count("cancelled")).put("items",items);
        }
        return new JSONObject().put("host_schema",1).put("job_id",j.previewId).put("plan_id",j.planId).put("status",j.status).put("operation_count",j.count)
            .put("operation",j.cursor.current()<0?JSONObject.NULL:j.cursor.current()).put("bytes_written",j.bytes).put("source_length",0).put("journal_path",j.journal==null?JSONObject.NULL:j.journal)
            .put("error",j.error==null?JSONObject.NULL:j.error).put("result",result==null?JSONObject.NULL:result);
    }
    static synchronized JSONArray projectRows(String id,JSONArray original)throws Exception{
        JSONArray rows=new JSONArray(original.toString());
        if(current==null||current.active||!current.previewId.equals(id))return rows;
        for(int index=0;index<rows.length();index++){
            JSONObject row=rows.getJSONObject(index);JSONArray operations=row.getJSONArray("operation_ids");
            String kind=row.getString("kind");boolean inspectionError=kind.equals("ItemError")||kind.equals("Vanished");
            JSONArray blockers=row.getJSONArray("blockers");for(int n=0;n<blockers.length();n++)if(blockers.getString(n).equals("ALBUM_READ_ERROR"))inspectionError=true;
            if(operations.length()==0||inspectionError)continue;boolean inactive=true;
            for(int n=0;n<operations.length();n++){int operation=operations.getInt(n);if(operation<0||operation>=current.count||!current.outcomes.inactive(operation)){inactive=false;break;}}
            if(inactive)row.put("no_action",true);
        }return rows;
    }
    static String start(Context context,RetainedPlan retained,ProviderReferences references)throws Exception{
        Context app=context.getApplicationContext();Job job=new Job(retained,retained.operationCount);
        synchronized(ProviderExecutionOwner.class){
            if(active()||DIAGNOSTICS.held())throw new IllegalStateException("EXECUTION_ACTIVE");PreviewGeneration.PROCESS.require(retained.recoveryGeneration);retained.attempt.claim();current=job;
            try{WORKER.execute(()->run(app,retained,references,job));}catch(RuntimeException error){job.active=false;job.status="failed";job.error=error.toString();throw error;}
        }
        return job.previewId;
    }
    private static void run(Context context,RetainedPlan retained,ProviderReferences references,Job job){
        String terminal="completed";String failure=null;ProviderEffectJournal journal=null;
        try{
            job.cancel.throwIfCanceled();LifecycleIo.run(ProviderExecutionOwner.class,()->{RecoveryAdmission.require(context,references,job.cancel);return null;});job.cancel.throwIfCanceled();
            NativeTransferPlan plan=LifecycleIo.run(ProviderExecutionOwner.class,()->NativeTransferPlan.prepare(retained,references));job.cancel.throwIfCanceled();
            journal=LifecycleIo.run(ProviderExecutionOwner.class,()->new ProviderEffectJournal(context,retained));
            synchronized(ProviderExecutionOwner.class){job.id=journal.id;job.journal="provider-executions/"+journal.id;}
            job.cancel.throwIfCanceled();TargetObservationGuard target=new TargetObservationGuard(context,retained,references.targetTree,job.cancel);LifecycleIo.run(ProviderExecutionOwner.class,()->{target.validateTree();return null;});
            LocalProviderEffects effects=new LocalProviderEffects(context,references,journal,job.cancel,target,evidence->{
                synchronized(ProviderExecutionOwner.class){job.outcomes.destination(job.cursor.current(),evidence.toString());if(evidence.getBoolean("copy"))job.bytes+=evidence.getLong("bytes");}
            });
            synchronized(ProviderExecutionOwner.class){job.status="running";}
            for(NativeTransferPlan.Transfer transfer:plan.operations){
                synchronized(ProviderExecutionOwner.class){job.cursor.begin(transfer.id);}job.cancel.throwIfCanceled();
                synchronized(ProviderExecutionOwner.class){job.outcomes.begin(transfer.id);}
                JSONObject actual=LifecycleIo.run(ProviderExecutionOwner.class,()->effects.transfer(transfer.source(),transfer.components(),transfer.copy,"operation-"+transfer.id));
                synchronized(ProviderExecutionOwner.class){job.completed++;if(!transfer.copy)job.moved++;job.outcomes.finish(transfer.id,"complete",actual.getString("name"));job.cursor.verified();}
            }
        }catch(Throwable error){
            failure=error.getClass().getSimpleName()+": "+error.getMessage();
            terminal=journal!=null&&journal.hasPendingEffects()?"uncertain":error instanceof OperationCanceledException?"cancelled" :journal!=null&&journal.hasAnyEffects()?"partial":"failed";
            synchronized(ProviderExecutionOwner.class){try{if(job.cursor.current()>=0&&job.outcomes.attempted[job.cursor.current()])job.outcomes.finish(job.cursor.current(),terminal.equals("uncertain")?"needs_recovery":terminal.equals("cancelled")?"cancelled":"failed",failure);}catch(Exception ignored){terminal="uncertain";}}
        }finally{
            try{if(journal!=null)journal.finish(terminal);}catch(Exception error){terminal="uncertain";failure="JOURNAL_COMMIT_UNCERTAIN: "+error.getClass().getSimpleName();}
            synchronized(ProviderExecutionOwner.class){job.outcomes.stop();job.error=failure;job.status=terminal.equals("completed")?"complete":terminal.equals("uncertain")?"needs_recovery":terminal;job.active=false;}
        }
    }
}
