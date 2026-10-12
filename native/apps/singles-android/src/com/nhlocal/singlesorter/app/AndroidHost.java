package com.nhlocal.singlesorter.app;

import android.app.Activity;
import android.content.Intent;
import android.os.CancellationSignal;
import android.webkit.WebView;
import com.nhlocal.singlesorter.platform.SafReadStorage;
import com.nhlocal.singlesorter.inference.OriginalAssets;
import com.nhlocal.singlesorter.inference.OriginalPlanning;
import java.io.File;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import org.json.JSONArray;
import org.json.JSONObject;

/** Narrow local-app IPC. A request can never supply a URI, file path, model or executable. */
final class AndroidHost implements AutoCloseable {
    private final Activity activity;private final WebView view;
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private final SafReadStorage storage;private final AndroidProfile profile;private final AndroidCsv csv;
    private String documentAction;private long documentMessage;private CancellationSignal documentCancel;private byte[] exportBytes;private int exportRows;
    private String source,target;private boolean closed;
    private final java.util.concurrent.atomic.AtomicBoolean lifetimeCancelled=new java.util.concurrent.atomic.AtomicBoolean();
    private File models;private long pendingPicker,pickerMessage;private String pickerRole;
    private CancellationSignal cancellation;private long nativeJob;private String jobId;
    private RetainedPlan retainedPlan;
    private String previewPersonalRevision,previewStoredSettings,ownedExecution;
    private JSONObject snapshot;private JSONArray rows=new JSONArray();
    private boolean busy;
    AndroidHost(Activity activity,WebView view){this.activity=activity;this.view=view;storage=new SafReadStorage(activity);profile=new AndroidProfile(activity);csv=new AndroidCsv(activity);}
    private synchronized void reply(long id,boolean ok,Object value){
        if(closed)return;
        final String encoded=new JSONArray().put(value).toString();
        final String script="window.__singlesAndroidReply("+id+","+ok+","+encoded.substring(1,encoded.length()-1)+")";
        activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(!closed)view.evaluateJavascript(script,null);}});
    }
    private void error(long id,String code,Throwable cause){try{reply(id,false,new JSONObject().put("code",code).put("message",cause.getMessage()==null?code:cause.getMessage()));}catch(Exception ignored){}}
    synchronized void invoke(String text){
        if(closed)return;
        long id=0;
        try{
            if(text==null||text.length()>2*1024*1024)throw new IllegalArgumentException("IPC_LIMIT");
            JSONObject request=new JSONObject(text);id=request.getLong("id");if(id<=0||id>9007199254740991L)throw new IllegalArgumentException("IPC_ID");
            if(request.length()!=3)throw new IllegalArgumentException("IPC_SHAPE");
            final long requestId=id;String command=request.getString("command");JSONObject args=request.getJSONObject("args");
            switch(command){
                case "execution_snapshot":
                    if(args.length()!=1)throw new IllegalArgumentException("EXECUTION_ARGUMENTS");reply(id,true,ProviderExecutionOwner.snapshot(args.getString("jobId")));return;
                case "cancel_apply":
                    if(args.length()!=1)throw new IllegalArgumentException("EXECUTION_ARGUMENTS");ProviderExecutionOwner.cancel(args.getString("jobId"));reply(id,true,JSONObject.NULL);return;
                case "cancel_preview":
                    if(!args.getString("jobId").equals(jobId))throw new IllegalArgumentException("STALE_JOB");
                    if(cancellation!=null)cancellation.cancel();if(nativeJob!=0)OriginalPlanning.cancel(nativeJob);reply(id,true,JSONObject.NULL);return;
                case "job_snapshot":
                    if(!args.getString("jobId").equals(jobId)||snapshot==null)throw new IllegalArgumentException("STALE_JOB");reply(id,true,new JSONObject(snapshot.toString()));return;
                case "get_plan_page":
                    if(!args.getString("jobId").equals(jobId)||snapshot==null||!"completed".equals(snapshot.optString("status")))throw new IllegalArgumentException("PLAN_UNAVAILABLE");
                    int offset=args.getInt("offset"),limit=args.getInt("limit");
                    if(args.has("developerMode")&&!(args.get("developerMode") instanceof Boolean))throw new IllegalArgumentException("DEVELOPER_MODE");
                    boolean developerMode=args.optBoolean("developerMode",false);
                    if(args.has("showInactive")&&!(args.get("showInactive") instanceof Boolean))throw new IllegalArgumentException("SHOW_INACTIVE");
                    boolean showInactive=args.optBoolean("showInactive",developerMode);
                    JSONArray displayRows=ProviderExecutionOwner.projectRows(jobId,rows);
                    boolean[] developerOnly=new boolean[displayRows.length()],skippedRows=new boolean[displayRows.length()];
                    for(int n=0;n<displayRows.length();n++){JSONObject row=displayRows.getJSONObject(n);developerOnly[n]=row.getBoolean("developer_only");skippedRows[n]=row.getBoolean("no_action");}
                    PlanPageSelection selection=PlanPageSelection.select(developerOnly,skippedRows,developerMode,showInactive,offset,limit);
                    JSONArray page=new JSONArray();for(int index:selection.indices)page.put(displayRows.get(index));
                    reply(id,true,new JSONObject().put("rows",page).put("total",selection.total).put("skipped",selection.skipped));return;
                default:break;
            }
            if(busy||pendingPicker!=0||documentAction!=null||(ProviderExecutionOwner.active()&&!command.equals("get_settings"))){error(id,"BUSY",new IllegalStateException("פעולה קודמת עדיין מסתיימת."));return;}
            if("select_root".equals(command)){
                retainedPlan=null;
                pickerRole=args.getString("role");if(!pickerRole.equals("Source")&&!pickerRole.equals("Target"))throw new IllegalArgumentException("ROOT_ROLE");
                pendingPicker=storage.beginPicker();pickerMessage=id;
                activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(!closed){try{activity.startActivityForResult(LocalTreeGrant.picker(),100);}catch(RuntimeException failure){long failed=pickerMessage;try{storage.finishPicker(pendingPicker,Activity.RESULT_CANCELED,null,new CancellationSignal());}catch(Exception cleanup){failure.addSuppressed(cleanup);}pendingPicker=0;error(failed,"PICKER_UNAVAILABLE",failure);}}}});return;
            }
            if("select_personal_import".equals(command)||"export_personal_list".equals(command)){
                byte[] prepared=null;int count=0;
                if(command.equals("export_personal_list")){JSONArray ownRows=profile.personal().getJSONArray("rows");prepared=AndroidProfile.encodeCsv(ownRows);count=ownRows.length();}
                documentAction=command;documentMessage=id;documentCancel=new CancellationSignal();exportBytes=prepared;exportRows=count;
                activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivityForResult(command.equals("select_personal_import")?AndroidCsv.importIntent():AndroidCsv.exportIntent(),101);}catch(RuntimeException failure){documentAction=null;exportBytes=null;error(requestId,"PICKER_UNAVAILABLE",failure);}}});return;
            }
            if("resolve_typed_root".equals(command))throw new IllegalArgumentException("ב־Android יש לבחור תיקייה בעזרת כפתור בחירת התיקייה.");
            if("start_preview".equals(command)){
                start(id,args.getJSONObject("request"));return;
            }
            busy=true;
            final OwnedCompletion completion=new OwnedCompletion(this,()->busy=false);
            worker.execute(()->{
                try{Object value;
                    switch(command){
                        case "get_settings":value=profile.settings();break;
                        case "latest_execution":
                            if(args.length()!=0)throw new IllegalArgumentException("EXECUTION_ARGUMENTS");value=HistoricalRecovery.latest(activity,lifetimeCancelled);break;
                        case "start_apply":
                            if(args.length()!=2||retainedPlan==null||!retainedPlan.jobId.equals(args.getString("jobId"))||!retainedPlan.planId.equals(args.getString("planId")))throw new IllegalArgumentException("STALE_PLAN");
                            RetainedPlan authorized=retainedPlan;retainedPlan=null;
                            if(!profile.personal().getString("revision").equals(previewPersonalRevision))throw new IllegalArgumentException("PERSONAL_LIST_CHANGED");
                            JSONObject originalSettings=new JSONObject(previewStoredSettings),currentSettings=profile.settings();
                            for(String key:new String[]{"copy_mode","main_folder_only","singles_folder","exist_only","abc_sort","duet_mode"})if(originalSettings.getBoolean(key)!=currentSettings.getBoolean(key))throw new IllegalArgumentException("SETTINGS_CHANGED");
                            ProviderReferences currentReferences=new ProviderReferences(activity,authorized,storage,source,target);
                            String execution=LifecycleAcceptance.accept(AndroidHost.this,()->closed,()->{String accepted=ProviderExecutionOwner.start(activity,authorized,currentReferences);ownedExecution=accepted;return accepted;});
                            value=new JSONObject().put("job_id",execution);break;
                        case "save_main_settings":retainedPlan=null;value=profile.save(args.getJSONObject("settings"));break;
                        case "set_duet":retainedPlan=null;value=profile.duet(args.getBoolean("enabled"));break;
                        case "get_personal_list":value=profile.personal();break;
                        case "save_personal_list":retainedPlan=null;value=profile.savePersonal(args.getString("revision"),args.getJSONArray("rows"));break;
                        case "preview_personal_import":value=csv.decode(args.getString("token"),args.getString("encoding"));break;
                        case "confirm_personal_import":retainedPlan=null;value=csv.confirm(profile,args.getString("token"),args.getString("encoding"),args.getString("revision"),args.has("edits")?args.getJSONArray("edits"):new JSONArray());break;
                        case "get_first_run_status":value=profile.firstRun(false);break;
                        case "mark_first_run_seen":value=profile.firstRun(true);break;
                        case "get_model_status":
                            if(models==null)models=OriginalAssets.prepare(activity.getApplicationContext(),()->{synchronized(AndroidHost.this){return closed;}});
                            value=new JSONObject().put("available",true).put("platform","android").put("execution_available",true).put("message","שני מודלי הזיהוי המקוריים אומתו. הקבצים נבדקים מחדש לפני כל בדיקה.");break;
                        case "open_recovery":
                            if(args.length()!=0)throw new IllegalArgumentException("RECOVERY_ARGUMENTS");
                            activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivity(new Intent(activity,RecoveryActivity.class));completion.finish(()->reply(requestId,true,JSONObject.NULL));}catch(RuntimeException failure){completion.finish(()->error(requestId,"RECOVERY_UNAVAILABLE",failure));}}});return;
                        case "open_execution_diagnostics":
                            if(args.length()!=0)throw new IllegalArgumentException("DIAGNOSTIC_ARGUMENTS");
                            activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivity(new Intent(activity,JournalExportActivity.class));completion.finish(()->reply(requestId,true,JSONObject.NULL));}catch(RuntimeException failure){completion.finish(()->error(requestId,"DIAGNOSTIC_UNAVAILABLE",failure));}}});return;
                        case "open_local_licenses":
                            if(args.length()!=0)throw new IllegalArgumentException("LICENSE_ARGUMENTS");
                            activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivity(new Intent(activity,LicensesActivity.class));completion.finish(()->reply(requestId,true,JSONObject.NULL));}catch(RuntimeException failure){completion.finish(()->error(requestId,"LICENSE_UNAVAILABLE",failure));}}});return;
                        case "open_project_link":
                            if(args.length()!=1||!args.has("target"))throw new IllegalArgumentException("PROJECT_LINK_ARGUMENTS");
                            final String projectUrl=ProjectLinks.resolve(args.getString("target"));
                            activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivity(new Intent(Intent.ACTION_VIEW,android.net.Uri.parse(projectUrl)));completion.finish(()->reply(requestId,true,JSONObject.NULL));}catch(RuntimeException failure){completion.finish(()->error(requestId,"LINK_UNAVAILABLE",failure));}}});return;
                        case "open_footer_link":
                            activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivity(new Intent(Intent.ACTION_VIEW,android.net.Uri.parse("https://nhlocal.github.io/shir-bot?utm_source=singles_sorter_program&utm_medium=android")));completion.finish(()->reply(requestId,true,JSONObject.NULL));}catch(RuntimeException failure){completion.finish(()->error(requestId,"LINK_UNAVAILABLE",failure));}}});return;
                        default:throw new IllegalArgumentException("הפעולה עדיין אינה זמינה במהדורת Android זו: "+command);
                    }
                    final Object result=value;completion.finish(()->reply(requestId,true,result));
                }catch(Throwable e){completion.finish(()->error(requestId,"HOST_ERROR",e));}
            });
        }catch(Throwable e){if(id>0)error(id,"HOST_ERROR",e);}
    }
    private synchronized void start(long message,JSONObject request)throws Exception {
        if(source==null||!source.equals(request.getString("source_token")))throw new IllegalArgumentException("SOURCE_NOT_SELECTED");
        String mode=request.getString("mode");if(!mode.equals("Sort")&&!mode.equals("FixNames"))throw new IllegalArgumentException("MODE");
        if(mode.equals("Sort")&&(target==null||!target.equals(request.getString("target_token"))))throw new IllegalArgumentException("TARGET_NOT_SELECTED");
        if(mode.equals("Sort")){String[] s=storage.provenance(source),t=storage.provenance(target);if(s[1].equals(t[1])&&s[2].equals(t[2]))throw new IllegalArgumentException("SAME_ROOT");}
        JSONObject settings=request.getJSONObject("settings");AndroidProfile.validateSettings(settings,true);
        String ai=request.getString("ai_profile");if(!ai.equals("Disabled")&&!ai.equals("Configured"))throw new IllegalArgumentException("AI_PROFILE");
        final String sourceRoot=source,targetRoot=mode.equals("Sort")?target:null,personalCsv=profile.personalCsv();
        cancellation=new CancellationSignal();final CancellationSignal signal=cancellation;
        previewPersonalRevision=profile.personal().getString("revision");previewStoredSettings=profile.settings().toString();
        final long previewGeneration=PreviewGeneration.PROCESS.capture();
        final long nativeId=OriginalPlanning.begin();if(nativeId==0)throw new IllegalStateException("BUSY");
        nativeJob=nativeId;jobId=java.util.UUID.randomUUID().toString();final String ownJob=jobId;busy=true;rows=new JSONArray();retainedPlan=null;
        snapshot=new JSONObject().put("host_schema",1).put("job_id",jobId).put("status","running").put("plan_id",JSONObject.NULL).put("total_rows",0).put("blockers",new JSONArray()).put("warnings",new JSONArray());
        reply(message,true,new JSONObject().put("job_id",jobId));
        worker.execute(()->{
            JSONObject completed=null,applyReadiness=null;RetainedPlan retained=null;Throwable failure=null;
            try(SafSnapshot captured=new SafSnapshot(activity.getApplicationContext(),storage,signal)){
                signal.throwIfCanceled();
                if(models==null&&ai.equals("Configured"))models=OriginalAssets.prepare(activity.getApplicationContext(),signal::isCanceled);
                signal.throwIfCanceled();
                captured.capture(sourceRoot,true,!settings.getBoolean("main_folder_only"));
                if(targetRoot!=null)captured.capture(targetRoot,false,true);
                signal.throwIfCanceled();
                JSONObject input=new JSONObject().put("schema",1).put("mode",mode).put("source",sourceRoot).put("target",targetRoot==null?JSONObject.NULL:targetRoot).put("settings",settings).put("ai_profile",ai).put("personal_csv",personalCsv).put("entries",captured.entries);
                JSONObject result=new JSONObject(OriginalPlanning.run(nativeId,models==null?"/unconfigured":models.getAbsolutePath(),captured.directory.getAbsolutePath(),input.toString(),captured));
                signal.throwIfCanceled();if(!result.getBoolean("ok"))throw new IllegalStateException(result.optString("error","PLANNING_FAILED"));
                completed=result.getJSONObject("data");
                retained=RetainedPlan.capture(ownJob,completed,captured,signal,previewGeneration);
            }catch(Throwable e){failure=e;}
            finally{OriginalPlanning.abandon(nativeId);}
            // Provider readiness runs after cleanup, outside the lifecycle monitor.
            if(retained!=null&&!signal.isCanceled()){
                try{RetainedPlan readyPlan=retained;applyReadiness=LifecycleIo.run(AndroidHost.this,()->readiness(readyPlan,sourceRoot,targetRoot,signal));}catch(Throwable unavailable){failure=unavailable;}
            }
            // Resource cleanup has finished before terminal state is visible.
            synchronized(AndroidHost.this){
                if(!ownJob.equals(jobId))return;
                nativeJob=0;cancellation=null;
                try{
                    if(signal.isCanceled())snapshot.put("status","cancelled");
                    else if(failure!=null)snapshot.put("status","failed").put("error",failure.toString());
                    else if(!closed&&completed!=null&&retained!=null){retainedPlan=retained;rows=completed.getJSONArray("rows");snapshot.put("status","completed").put("plan_id",completed.getString("plan_id")).put("total_rows",rows.length()).put("blockers",completed.getJSONArray("blockers")).put("warnings",completed.getJSONArray("warnings"));snapshot.put("apply",applyReadiness==null?JSONObject.NULL:applyReadiness);
                        if(applyReadiness!=null&&applyReadiness.getBoolean("supported"))for(int n=0;n<rows.length();n++){
                            JSONObject row=rows.getJSONObject(n);JSONArray original=row.getJSONArray("blockers"),effective=new JSONArray();
                            for(int b=0;b<original.length();b++){String blocker=original.getString(b);if(!blocker.equals("PHYSICAL_APPLY_UNAVAILABLE")&&!blocker.equals("MISSING_IDENTITY")&&!blocker.equals("UNKNOWN_CAPABILITY"))effective.put(blocker);}
                            row.put("effective_blockers",effective);
                        }}
                }catch(Exception invalid){try{snapshot.put("status","failed").put("error","RESULT_INVALID");}catch(Exception ignored){}}
                finally{busy=false;}
            }
        });
    }
    private JSONObject readiness(RetainedPlan retained,String selectedSource,String selectedTarget,CancellationSignal signal)throws Exception{
        JSONObject request=retained.planCopy().getJSONObject("request");
        JSONObject result=new JSONObject().put("supported",false).put("reason",JSONObject.NULL).put("operation_count",0).put("copy_mode",request.getJSONObject("settings").getBoolean("copy_mode")).put("source",storage.inspect(selectedSource,signal).displayName).put("target",selectedTarget==null?"":storage.inspect(selectedTarget,signal).displayName);
        try{
            signal.throwIfCanceled();ProviderReferences references=new ProviderReferences(activity,retained,storage,selectedSource,selectedTarget);NativeTransferPlan plan;
            try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
                lease.check();PreviewGeneration.PROCESS.require(retained.recoveryGeneration);RecoveryAdmission.require(activity,references,signal);plan=NativeTransferPlan.prepare(retained,references);signal.throwIfCanceled();
            }
            result.put("supported",true).put("recovery_scope_admitted",true).put("operation_count",plan.operations.size()).put("reason","יש להשאיר את התיקיות ללא שינויים במהלך הביצוע. במקרה של הפרעה, הביצוע ייעצר והקבצים שכבר נוצרו יישמרו.");
        }catch(Exception unavailable){result.put("reason","לא ניתן לבצע תוכנית זו במכשיר או בהרשאות הנוכחיות. יש לבחור תיקיות מקומיות ולבדוק מחדש.");}
        return result;
    }
    void pickerResult(int resultCode,Intent data){
        final long ticket,message;final String role;final OwnedCompletion completion;
        synchronized(this){if(closed||pendingPicker==0)return;ticket=pendingPicker;message=pickerMessage;role=pickerRole;pendingPicker=0;busy=true;completion=new OwnedCompletion(this,()->busy=false);}
        worker.execute(()->{
            try{SafReadStorage.Entry selected=storage.finishPicker(ticket,resultCode,data,new CancellationSignal());
                Object value=JSONObject.NULL;
                if(selected!=null){
                    LocalTreeGrant.retainIfLocal(activity,data);
                    synchronized(AndroidHost.this){if(closed)return;if(role.equals("Source"))source=selected.token;else target=selected.token;}
                    value=new JSONObject().put("token",selected.token).put("display_name",selected.displayName).put("platform","android").put("capabilities",new JSONObject().put("read",true).put("list",true));
                }
                final Object result=value;completion.finish(()->reply(message,true,result));
            }catch(Throwable e){completion.finish(()->error(message,"SAF_ERROR",e));}
        });
    }
    void documentResult(int resultCode,Intent data){
        final String action;final long message;final CancellationSignal signal;final byte[] bytes;final int count;final OwnedCompletion completion;
        synchronized(this){
            if(closed||documentAction==null)return;action=documentAction;message=documentMessage;signal=documentCancel;bytes=exportBytes;count=exportRows;documentAction=null;exportBytes=null;busy=true;
            completion=new OwnedCompletion(this,()->{if(bytes!=null)java.util.Arrays.fill(bytes,(byte)0);if(documentCancel==signal)documentCancel=null;busy=false;});
        }
        worker.execute(()->{
            try{Object value;
                if(resultCode!=Activity.RESULT_OK||data==null)value=JSONObject.NULL;
                else if(action.equals("select_personal_import"))value=csv.select(data,signal);
                else value=csv.export(data,bytes,count,signal);
                final Object result=value;completion.finish(()->reply(message,true,result));
            }catch(Throwable failure){completion.finish(()->error(message,"CSV_DOCUMENT_ERROR",failure));}
        });
    }
    @Override public synchronized void close(){
        lifetimeCancelled.set(true);
        if(closed)return;closed=true;ProviderExecutionOwner.cancelOwned(ownedExecution);retainedPlan=null;if(documentCancel!=null)documentCancel.cancel();if(cancellation!=null)cancellation.cancel();if(nativeJob!=0)OriginalPlanning.cancel(nativeJob);
        // Scope/asset cleanup is queued after active JNI/provider work, never under it.
        worker.execute(()->{csv.clear();if(exportBytes!=null)java.util.Arrays.fill(exportBytes,(byte)0);try{storage.close();}catch(Exception ignored){}if(models!=null)OriginalAssets.cleanup(models);});worker.shutdown();
    }
}
