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
    private File models;private long pendingPicker,pickerMessage;private String pickerRole;
    private CancellationSignal cancellation;private long nativeJob;private String jobId;
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
                case "cancel_preview":
                    if(!args.getString("jobId").equals(jobId))throw new IllegalArgumentException("STALE_JOB");
                    if(cancellation!=null)cancellation.cancel();if(nativeJob!=0)OriginalPlanning.cancel(nativeJob);reply(id,true,JSONObject.NULL);return;
                case "job_snapshot":
                    if(!args.getString("jobId").equals(jobId)||snapshot==null)throw new IllegalArgumentException("STALE_JOB");reply(id,true,new JSONObject(snapshot.toString()));return;
                case "get_plan_page":
                    if(!args.getString("jobId").equals(jobId)||snapshot==null||!"completed".equals(snapshot.optString("status")))throw new IllegalArgumentException("PLAN_UNAVAILABLE");
                    int offset=args.getInt("offset"),limit=args.getInt("limit");if(offset<0||limit<1||limit>200)throw new IllegalArgumentException("PAGE_LIMIT");JSONArray page=new JSONArray();for(int n=offset;n<rows.length()&&n-offset<limit;n++)page.put(rows.get(n));reply(id,true,new JSONObject().put("rows",page).put("total",rows.length()));return;
                default:break;
            }
            if(busy||pendingPicker!=0||documentAction!=null){error(id,"BUSY",new IllegalStateException("פעולה קודמת עדיין מסתיימת."));return;}
            if("select_root".equals(command)){
                pickerRole=args.getString("role");if(!pickerRole.equals("Source")&&!pickerRole.equals("Target"))throw new IllegalArgumentException("ROOT_ROLE");
                pendingPicker=storage.beginPicker();pickerMessage=id;
                activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(!closed){try{activity.startActivityForResult(SafReadStorage.treePickerIntent(),100);}catch(RuntimeException failure){long failed=pickerMessage;try{storage.finishPicker(pendingPicker,Activity.RESULT_CANCELED,null,new CancellationSignal());}catch(Exception cleanup){failure.addSuppressed(cleanup);}pendingPicker=0;error(failed,"PICKER_UNAVAILABLE",failure);}}}});return;
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
            worker.execute(()->{
                try{Object value;
                    switch(command){
                        case "get_settings":value=profile.settings();break;
                        case "save_main_settings":value=profile.save(args.getJSONObject("settings"));break;
                        case "set_duet":value=profile.duet(args.getBoolean("enabled"));break;
                        case "get_personal_list":value=profile.personal();break;
                        case "save_personal_list":value=profile.savePersonal(args.getString("revision"),args.getJSONArray("rows"));break;
                        case "preview_personal_import":value=csv.decode(args.getString("token"),args.getString("encoding"));break;
                        case "confirm_personal_import":value=csv.confirm(profile,args.getString("token"),args.getString("encoding"),args.getString("revision"),args.optJSONArray("edits")==null?new JSONArray():args.getJSONArray("edits"));break;
                        case "get_first_run_status":value=profile.firstRun(false);break;
                        case "mark_first_run_seen":value=profile.firstRun(true);break;
                        case "get_model_status":
                            if(models==null)models=OriginalAssets.prepare(activity.getApplicationContext(),()->{synchronized(AndroidHost.this){return closed;}});
                            value=new JSONObject().put("available",true).put("platform","android").put("message","שני מודלי הזיהוי המקוריים אומתו. הקבצים נבדקים מחדש לפני כל בדיקה.");break;
                        case "open_footer_link":
                            activity.runOnUiThread(()->{synchronized(AndroidHost.this){if(closed)return;try{activity.startActivity(new Intent(Intent.ACTION_VIEW,android.net.Uri.parse("https://nhlocal.github.io/shir-bot?utm_source=singles_sorter_program&utm_medium=android")));reply(requestId,true,JSONObject.NULL);}catch(RuntimeException failure){error(requestId,"LINK_UNAVAILABLE",failure);}}});return;
                        default:throw new IllegalArgumentException("הפעולה עדיין אינה זמינה במהדורת Android זו: "+command);
                    }
                    synchronized(AndroidHost.this){busy=false;}reply(requestId,true,value);
                }catch(Throwable e){synchronized(AndroidHost.this){busy=false;}error(requestId,"HOST_ERROR",e);}finally{synchronized(AndroidHost.this){busy=false;}}
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
        final long nativeId=OriginalPlanning.begin();if(nativeId==0)throw new IllegalStateException("BUSY");
        nativeJob=nativeId;jobId=Long.toString(nativeId);final String ownJob=jobId;busy=true;rows=new JSONArray();
        snapshot=new JSONObject().put("host_schema",1).put("job_id",jobId).put("status","running").put("plan_id",JSONObject.NULL).put("total_rows",0).put("blockers",new JSONArray()).put("warnings",new JSONArray());
        reply(message,true,new JSONObject().put("job_id",jobId));
        worker.execute(()->{
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
                JSONObject data=result.getJSONObject("data");
                synchronized(AndroidHost.this){if(closed||!ownJob.equals(jobId)||signal.isCanceled())return;rows=data.getJSONArray("rows");snapshot.put("status","completed").put("plan_id",data.getString("plan_id")).put("total_rows",rows.length()).put("blockers",data.getJSONArray("blockers")).put("warnings",data.getJSONArray("warnings"));}
            }catch(Throwable e){synchronized(AndroidHost.this){try{if(ownJob.equals(jobId)&&snapshot!=null)snapshot.put("status",signal.isCanceled()?"cancelled":"failed").put("error",e.toString());}catch(Exception ignored){}}}
            finally{OriginalPlanning.abandon(nativeId);synchronized(AndroidHost.this){if(ownJob.equals(jobId)){nativeJob=0;busy=false;cancellation=null;}}}
        });
    }
    void pickerResult(int resultCode,Intent data){
        final long ticket,message;final String role;
        synchronized(this){if(closed||pendingPicker==0)return;ticket=pendingPicker;message=pickerMessage;role=pickerRole;pendingPicker=0;busy=true;}
        worker.execute(()->{
            try{SafReadStorage.Entry selected=storage.finishPicker(ticket,resultCode,data,new CancellationSignal());
                synchronized(AndroidHost.this){if(closed)return;if(selected==null){reply(message,true,JSONObject.NULL);return;}if(role.equals("Source"))source=selected.token;else target=selected.token;}
                reply(message,true,new JSONObject().put("token",selected.token).put("display_name",selected.displayName).put("platform","android").put("capabilities",new JSONObject().put("read",true).put("list",true)));
            }catch(Throwable e){error(message,"SAF_ERROR",e);}finally{synchronized(AndroidHost.this){busy=false;}}
        });
    }
    void documentResult(int resultCode,Intent data){
        final String action;final long message;final CancellationSignal signal;final byte[] bytes;final int count;
        synchronized(this){if(closed||documentAction==null)return;action=documentAction;message=documentMessage;signal=documentCancel;bytes=exportBytes;count=exportRows;documentAction=null;exportBytes=null;busy=true;}
        worker.execute(()->{
            try{Object value;
                if(resultCode!=Activity.RESULT_OK||data==null)value=JSONObject.NULL;
                else if(action.equals("select_personal_import"))value=csv.select(data,signal);
                else value=csv.export(data,bytes,count,signal);
                synchronized(AndroidHost.this){busy=false;}reply(message,true,value);
            }catch(Throwable failure){synchronized(AndroidHost.this){busy=false;}error(message,"CSV_DOCUMENT_ERROR",failure);}
            finally{synchronized(AndroidHost.this){busy=false;documentCancel=null;}if(bytes!=null)java.util.Arrays.fill(bytes,(byte)0);}
        });
    }
    @Override public synchronized void close(){
        if(closed)return;closed=true;if(documentCancel!=null)documentCancel.cancel();if(cancellation!=null)cancellation.cancel();if(nativeJob!=0)OriginalPlanning.cancel(nativeJob);
        // Scope/asset cleanup is queued after active JNI/provider work, never under it.
        worker.execute(()->{csv.clear();if(exportBytes!=null)java.util.Arrays.fill(exportBytes,(byte)0);try{storage.close();}catch(Exception ignored){}if(models!=null)OriginalAssets.cleanup(models);});worker.shutdown();
    }
}
