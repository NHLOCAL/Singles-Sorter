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
        final ExecutionOutcomes outcomes;final boolean fixMode;final JSONObject[] repairs,evidence,albums;final long[] backups;
        boolean active=true;String id,status="preparing",error,journal,phase="preparing",diagnosticCode;int completed;long bytes,moved;final ExecutionCursor cursor=new ExecutionCursor();
        Job(RetainedPlan retained,int count,boolean fixMode){this.fixMode=fixMode;albums=new JSONObject[count];repairs=new JSONObject[count];evidence=new JSONObject[count];backups=new long[count];attempt=retained.attempt;previewId=retained.jobId;planId=retained.planId;this.count=count;outcomes=new ExecutionOutcomes(count);}
    }
    static synchronized boolean active(){return current!=null&&current.active;}
    static synchronized void cancel(String id){if(current==null||!current.previewId.equals(id))throw new IllegalArgumentException("EXECUTION_NOT_FOUND");current.attempt.cancel(id,current.cancel::cancel);}
    static synchronized void cancelOwned(String id){if(id!=null&&current!=null&&current.active&&id.equals(current.previewId))current.cancel.cancel();}
    static synchronized JSONObject snapshot(String id)throws Exception{
        if(current==null||!id.equals(current.previewId))throw new IllegalArgumentException("EXECUTION_NOT_FOUND");Job j=current;
        JSONObject result=null;
        if(!j.active){
            JSONArray items=new JSONArray();for(int operation=0;operation<j.count;operation++)items.put(new JSONObject().put("operation",operation).put("status",j.outcomes.status[operation]).put("detail",j.outcomes.detail[operation]).put("attempted",j.outcomes.attempted[operation]).put("destination_evidence",j.outcomes.destination[operation]==null?JSONObject.NULL:new JSONObject(j.outcomes.destination[operation])).put("repair",j.repairs[operation]==null?JSONObject.NULL:j.repairs[operation]).put("fix_evidence",j.evidence[operation]==null?JSONObject.NULL:j.evidence[operation]).put("album_evidence",j.albums[operation]==null?JSONObject.NULL:j.albums[operation]));
            long albumDestinations=0;boolean hasAlbums=false;for(JSONObject album:j.albums)if(album!=null){hasAlbums=true;albumDestinations+=AlbumUiProjection.files(album);}
            result=new JSONObject().put("source_items",j.count).put("committed_destinations",j.outcomes.committed()+albumDestinations).put("moved_sources",j.moved)
                .put("failed_operations",j.outcomes.count("failed")+j.outcomes.count("metadata_failure")+j.outcomes.count("partial")).put("blocked_operations",j.outcomes.count("blocked"))
                .put("recovery_operations",j.outcomes.count("needs_recovery")).put("cancelled_operations",j.outcomes.count("cancelled")).put("items",items);
            if(hasAlbums)result.put("albums",AlbumUiProjection.summary(j.albums,j.outcomes.status));
            if(j.fixMode){int renamed=0,repaired=0,unchanged=0;long backups=0;for(int i=0;i<j.count;i++){JSONObject flags=j.repairs[i];if(flags!=null){if(flags.getBoolean("renamed"))renamed++;if(flags.getBoolean("repaired"))repaired++;if(flags.getBoolean("unchanged"))unchanged++;}backups+=j.backups[i];}
                result.put("repair",new JSONObject().put("total_files",j.count).put("processed_files",j.completed).put("renamed_files",renamed).put("repaired_files",repaired).put("unchanged_files",unchanged).put("unsupported_files",j.outcomes.count("unsupported_tags")).put("failed_files",j.outcomes.count("failed")+j.outcomes.count("metadata_failure")).put("retained_backup_bytes",backups));}
        }
        return new JSONObject().put("host_schema",1).put("job_id",j.previewId).put("plan_id",j.planId).put("status",j.status).put("operation_count",j.count)
            .put("operation",j.cursor.current()<0?JSONObject.NULL:j.cursor.current()).put("bytes_written",j.bytes).put("source_length",0).put("journal_path",j.journal==null?JSONObject.NULL:j.journal)
            .put("diagnostic",new JSONObject().put("phase",j.phase).put("code",j.diagnosticCode==null?JSONObject.NULL:j.diagnosticCode)).put("error",j.error==null?JSONObject.NULL:j.error).put("result",result==null?JSONObject.NULL:result);
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
        Context app=context.getApplicationContext();Job job=new Job(retained,retained.operationCount,"FixNames".equals(retained.planCopy().getJSONObject("request").getString("mode")));
        synchronized(ProviderExecutionOwner.class){
            if(active()||DIAGNOSTICS.held())throw new IllegalStateException("EXECUTION_ACTIVE");PreviewGeneration.PROCESS.require(retained.recoveryGeneration);retained.attempt.claim();current=job;
            try{WORKER.execute(()->run(app,retained,references,job));}catch(RuntimeException error){job.active=false;job.status="failed";job.error=error.toString();throw error;}
        }
        return job.previewId;
    }
    private static void fixProgress(Job job,String phase,JSONObject evidence)throws Exception{
        synchronized(ProviderExecutionOwner.class){int id=job.cursor.current();if(id<0)throw new IllegalStateException("FIX_PROGRESS_OWNER");job.repairs[id]=FixUiProjection.progress(job.repairs[id],phase,evidence);
            if(phase.equals("fix-backup-observed")||phase.equals("fix-backup-retained"))job.backups[id]=evidence.getJSONObject("binding").getLong("bytes");
            if(phase.equals("destination-observed")){job.repairs[id]=FixUiProjection.flags(evidence);job.evidence[id]=evidence;job.outcomes.destination(id,evidence.toString());job.bytes+=evidence.getLong("bytes");}
        }
    }
    private static synchronized void phase(Job job,String phase){job.phase=phase;}
    private static void run(Context context,RetainedPlan retained,ProviderReferences references,Job job){
        String terminal="completed";String failure=null;ProviderEffectJournal journal=null;
        try{
            phase(job,"recovery-admission");job.cancel.throwIfCanceled();LifecycleIo.run(ProviderExecutionOwner.class,()->{RecoveryAdmission.require(context,references,job.cancel);return null;});job.cancel.throwIfCanceled();
            if(job.fixMode){
                phase(job,"original-fix-projection");NativeFixPlan plan=LifecycleIo.run(ProviderExecutionOwner.class,()->NativeFixPlan.prepare(retained,references,job.cancel::isCanceled));FixAdmissionBounds.require(retained,plan,job.cancel::isCanceled);
                phase(job,"retained-namespace-admission");ReservedProviderObjects reservations=LifecycleIo.run(ProviderExecutionOwner.class,()->ReservedProviderObjects.load(context,job.cancel));
                for(NativeFixPlan.Fix fix:plan.operations){job.cancel.throwIfCanceled();reservations.require(references.uri(fix.source()));}
                phase(job,"journal-creation");journal=LifecycleIo.run(ProviderExecutionOwner.class,()->new ProviderEffectJournal(context,retained));
                synchronized(ProviderExecutionOwner.class){job.id=journal.id;job.journal="provider-executions/"+journal.id;}
                phase(job,"source-observation-validation");TargetObservationGuard sourceGuard=new TargetObservationGuard(context,retained,references.sourceTree,job.cancel,retained.planCopy().getJSONObject("request").getJSONObject("settings").getBoolean("main_folder_only"));LifecycleIo.run(ProviderExecutionOwner.class,()->{sourceGuard.validateTree();return null;});
                FixProviderEffects effects=new FixProviderEffects(context,references,journal,job.cancel,sourceGuard,reservations,(phase,evidence)->fixProgress(job,phase,evidence));
                synchronized(ProviderExecutionOwner.class){job.status="running";}
                phase(job,"fix-provider-effects");for(NativeFixPlan.Fix fix:plan.operations){
                    synchronized(ProviderExecutionOwner.class){job.cursor.begin(fix.id);}job.cancel.throwIfCanceled();synchronized(ProviderExecutionOwner.class){job.outcomes.begin(fix.id);}
                    JSONObject actual=LifecycleIo.run(ProviderExecutionOwner.class,()->effects.fix(fix));String status=actual.getString("status");
                    if(!status.equals("complete")&&!FixAcknowledgedOutcome.mayContinue(status,true,journal.hasPendingEffects()))throw new IllegalStateException("FIX_OUTCOME_UNCONFIRMED");
                    synchronized(ProviderExecutionOwner.class){job.completed++;job.repairs[fix.id]=FixUiProjection.flags(actual);job.evidence[fix.id]=actual;job.outcomes.finish(fix.id,status,actual.toString());job.cursor.verified();}
                    if(!status.equals("complete"))terminal="partial";
                }
            }else{
            phase(job,"original-sort-projection");NativeTransferPlan singles=LifecycleIo.run(ProviderExecutionOwner.class,()->NativeTransferPlan.prepare(retained,references));NativeAlbumPlan albums=LifecycleIo.run(ProviderExecutionOwner.class,()->NativeAlbumPlan.prepare(retained,references,job.cancel::isCanceled));AlbumAdmissionBounds.require(retained,singles,albums,job.cancel::isCanceled);job.cancel.throwIfCanceled();
            phase(job,"retained-namespace-admission");ReservedProviderObjects reservations=LifecycleIo.run(ProviderExecutionOwner.class,()->ReservedProviderObjects.load(context,job.cancel));
            for(NativeTransferPlan.Transfer transfer:singles.operations){job.cancel.throwIfCanceled();reservations.require(references.uri(transfer.source()));}
            for(NativeAlbumPlan.Album album:albums.operations){job.cancel.throwIfCanceled();reservations.requireSubtree(references.uri(album.source.reference()));}
            phase(job,"journal-creation");journal=LifecycleIo.run(ProviderExecutionOwner.class,()->new ProviderEffectJournal(context,retained));
            synchronized(ProviderExecutionOwner.class){job.id=journal.id;job.journal="provider-executions/"+journal.id;}
            job.cancel.throwIfCanceled();TargetObservationGuard target=new TargetObservationGuard(context,retained,references.targetTree,job.cancel);LifecycleIo.run(ProviderExecutionOwner.class,()->{target.validateTree();return null;});
            TargetObservationGuard source=albums.operations.isEmpty()?null:new TargetObservationGuard(context,retained,references.sourceTree,job.cancel,retained.planCopy().getJSONObject("request").getJSONObject("settings").getBoolean("main_folder_only"));
            if(source!=null)LifecycleIo.run(ProviderExecutionOwner.class,()->{source.validateTree();return null;});
            LocalProviderEffects effects=new LocalProviderEffects(context,references,journal,job.cancel,target,evidence->{
                synchronized(ProviderExecutionOwner.class){job.outcomes.destination(job.cursor.current(),evidence.toString());if(evidence.getBoolean("copy"))job.bytes+=evidence.getLong("bytes");}
            });
            AlbumProviderEffects albumEffects=new AlbumProviderEffects(context,references,albums,journal,job.cancel,source,target,evidence->{
                synchronized(ProviderExecutionOwner.class){int id=job.cursor.current();JSONObject prior=job.albums[id];job.bytes+=evidence.getLong("bytes")-(prior==null?0:prior.getLong("bytes"));job.moved+=AlbumUiProjection.movedFiles(evidence)-(prior==null?0:AlbumUiProjection.movedFiles(prior));job.albums[id]=new JSONObject(evidence.toString());}
            });
            java.util.HashMap<Integer,NativeTransferPlan.Transfer> byId=new java.util.HashMap<>();for(NativeTransferPlan.Transfer transfer:singles.operations)byId.put(transfer.id,transfer);
            synchronized(ProviderExecutionOwner.class){job.status="running";}
            for(int id=0;id<job.count;id++){
                synchronized(ProviderExecutionOwner.class){job.cursor.begin(id);}job.cancel.throwIfCanceled();synchronized(ProviderExecutionOwner.class){job.outcomes.begin(id);}
                NativeAlbumPlan.Album album=albums.get(id);
                if(album!=null){phase(job,"album-provider-effects");JSONObject actual=LifecycleIo.run(ProviderExecutionOwner.class,()->albumEffects.transfer(album));String status=actual.getString("status");
                    synchronized(ProviderExecutionOwner.class){job.completed++;job.outcomes.finish(id,status,actual.toString());job.cursor.verified();}if(status.equals("partial"))terminal="partial";
                }else{NativeTransferPlan.Transfer transfer=byId.get(id);if(transfer==null)throw new IllegalStateException("SORT_OPERATION_NOT_PROJECTED");phase(job,"single-provider-effects");
                    if(source!=null)LifecycleIo.run(ProviderExecutionOwner.class,()->{source.validateTree();return null;});
                    JSONObject actual=LifecycleIo.run(ProviderExecutionOwner.class,()->effects.transfer(transfer.source(),transfer.components(),transfer.copy,"operation-"+transfer.id));
                    if(source!=null)LifecycleIo.run(ProviderExecutionOwner.class,()->{if(!transfer.copy)source.removedSubtree(references.uri(transfer.source()),references.parent(transfer.source()));source.validateTree();return null;});
                    synchronized(ProviderExecutionOwner.class){job.completed++;if(!transfer.copy)job.moved++;job.outcomes.finish(id,"complete",actual.getString("name"));job.cursor.verified();}
                }
            }
            }
        }catch(Throwable error){
            failure=error.getClass().getSimpleName()+": "+error.getMessage();synchronized(ProviderExecutionOwner.class){job.diagnosticCode=ReadinessDiagnostic.code(error);}
            terminal=journal!=null&&journal.hasPendingEffects()?"uncertain":error instanceof OperationCanceledException?"cancelled" :journal!=null&&journal.hasAnyEffects()?"partial":"failed";
            synchronized(ProviderExecutionOwner.class){try{if(job.cursor.current()>=0&&job.outcomes.attempted[job.cursor.current()])job.outcomes.finish(job.cursor.current(),terminal.equals("uncertain")?"needs_recovery":terminal.equals("cancelled")?"cancelled":"failed",failure);}catch(Exception ignored){terminal="uncertain";}}
        }finally{
            try{if(journal!=null)journal.finish(terminal);}catch(Exception error){terminal="uncertain";failure="JOURNAL_COMMIT_UNCERTAIN: "+error.getClass().getSimpleName();}
            synchronized(ProviderExecutionOwner.class){job.outcomes.stop();job.error=failure;job.status=terminal.equals("completed")?"complete":terminal.equals("uncertain")?"needs_recovery":terminal;job.active=false;}
        }
    }
}
