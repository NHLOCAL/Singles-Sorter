package com.nhlocal.singlesorter.app;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.system.Os;
import android.system.OsConstants;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.Spinner;
import android.widget.TextView;
import java.io.File;
import java.io.FileInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.security.DigestOutputStream;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit local diagnostic export. No journal mutation, replay, or caller-selected private path. */
public final class JournalExportActivity extends Activity {
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private final AtomicBoolean cancelled=new AtomicBoolean();
    private final android.os.CancellationSignal receiptCancellation=new android.os.CancellationSignal();
    private Spinner choices;private Button export;private TextView status;
    private String[] ids=new String[0];private boolean pending;private String selected;
    @Override public void onCreate(Bundle state){
        super.onCreate(state);LinearLayout layout=new LinearLayout(this);layout.setOrientation(LinearLayout.VERTICAL);
        Button back=new Button(this);back.setText("חזרה");back.setOnClickListener(v->finish());layout.addView(back);
        TextView description=new TextView(this);description.setText("שמירת יומן פעולה לבדיקה. הקובץ כולל שמות קבצים ותיקיות, הגדרות, תוצאות ואישורי בדיקה שנשמרו. יש לשתף אותו רק עם מי שבחרת. השמירה אינה מבצעת פעולה חוזרת או משנה את הקבצים.");layout.addView(description);
        choices=new Spinner(this);layout.addView(choices);
        export=new Button(this);export.setText("שמור יומן נבחר");export.setEnabled(false);export.setOnClickListener(v->choose());layout.addView(export);
        status=new TextView(this);layout.addView(status);setContentView(layout);
        worker.execute(()->{
            String error=null;String[] found=new String[0];String[] labels=new String[0];
            try{try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
                lease.check();
                checkpoint();File root=root();java.util.ArrayList<String> names=new java.util.ArrayList<>();
                if(root.exists())try(java.nio.file.DirectoryStream<java.nio.file.Path> stream=java.nio.file.Files.newDirectoryStream(root.toPath())){
                    for(java.nio.file.Path entry:stream){checkpoint();if(names.size()>=1000)throw new java.io.IOException("journal count");String id=entry.getFileName().toString();directory(id);names.add(id);}
                }
                found=names.toArray(new String[0]);Arrays.sort(found);labels=new String[found.length];
                for(int i=0;i<found.length;i++){String stateName="unknown";try{String candidate=new JSONObject(new String(read(directory(found[i]),"index.json"),java.nio.charset.StandardCharsets.UTF_8)).optString("status","unknown");if(candidate.matches("completed|failed|cancelled|partial|uncertain|running"))stateName=candidate;}catch(org.json.JSONException malformed){stateName="unreadable";}labels[i]=stateName+"  "+found[i];}
            }}
            catch(Exception failure){android.util.Log.e("SinglesJournalExport","Private journal listing unavailable: "+code(failure));found=new String[0];error="לא ניתן לקרוא יומנים בזמן ביצוע או כשהמידע אינו זמין.";}
            final String message=error;final String[] result=found;final String[] display=labels;
            runOnUiThread(()->{if(cancelled.get())return;ids=result;choices.setAdapter(new ArrayAdapter<>(this,android.R.layout.simple_spinner_dropdown_item,display));export.setEnabled(message==null&&ids.length>0);status.setText(message!=null?message:ids.length==0?"אין יומני פעולה שמורים.":"בחר את מזהה הפעולה לשמירה.");});
        });
    }
    private void checkpoint()throws java.io.IOException{JournalExportBounds.checkpoint(cancelled);}
    private File root()throws Exception{return JournalExportBounds.root(getFilesDir());}
    private File directory(String id)throws Exception{
        if(!JournalExportBounds.id(id))throw new java.io.IOException("journal id");
        File directory=JournalExportBounds.child(root(),id);
        if(!directory.isDirectory())throw new java.io.IOException("journal directory");
        return directory;
    }
    private static String code(Exception failure){
        if(failure instanceof android.system.ErrnoException)return "errno "+((android.system.ErrnoException)failure).errno;
        String message=failure.getMessage();
        return message!=null&&message.matches("[a-z ]{1,48}")?message:failure.getClass().getSimpleName();
    }
    private byte[] read(File directory,String name)throws Exception{
        checkpoint();File file=new File(directory,name);int limit=JournalExportBounds.limit(name);
        java.io.FileDescriptor fd=Os.open(file.getAbsolutePath(),OsConstants.O_RDONLY|OsConstants.O_NOFOLLOW|OsConstants.O_CLOEXEC|OsConstants.O_NONBLOCK,0);
        Throwable primary=null;
        try(FileInputStream input=new FileInputStream(fd)){
            android.system.StructStat stat=Os.fstat(fd);
            if(!OsConstants.S_ISREG(stat.st_mode)||stat.st_size<0||stat.st_size>limit)throw new java.io.IOException("journal size");
            return JournalExportBounds.read(input,limit,cancelled);
        }catch(Exception|Error failure){primary=failure;throw failure;}
        finally{try{Os.close(fd);}catch(android.system.ErrnoException closeFailure){if(primary!=null)primary.addSuppressed(closeFailure);else throw closeFailure;}}
    }
    private void choose(){
        if(pending||cancelled.get()||choices.getSelectedItemPosition()<0)return;
        selected=ids[choices.getSelectedItemPosition()];pending=true;export.setEnabled(false);choices.setEnabled(false);
        try{startActivityForResult(new Intent(Intent.ACTION_CREATE_DOCUMENT).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION).addCategory(Intent.CATEGORY_OPENABLE).setType("application/zip").putExtra(Intent.EXTRA_TITLE,"singles-execution-"+selected+".zip"),1);}
        catch(RuntimeException failure){done("לא ניתן לפתוח את בחירת הקובץ.");}
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);if(request!=1||!pending||cancelled.get())return;
        if(result!=RESULT_OK||data==null||data.getData()==null){done("");return;}
        Uri destination=data.getData();
        if(!LicensePaths.exportGrant("content".equals(destination.getScheme()),DocumentsContract.isDocumentUri(this,destination),(data.getFlags()&Intent.FLAG_GRANT_READ_URI_PERMISSION)!=0,(data.getFlags()&Intent.FLAG_GRANT_WRITE_URI_PERMISSION)!=0)){done("יעד השמירה אינו נתמך.");return;}
        final String id=selected;worker.execute(()->{String message;
            try{export(id,destination);message="היומן נשמר ונבדק מול הקובץ שנכתב.";}
            catch(Exception failure){android.util.Log.e("SinglesJournalExport","Owned journal export unavailable: "+code(failure));message="השמירה לא הושלמה. ייתכן שנוצר קובץ חלקי; הוא נשמר לבדיקה.";}
            final String completed=message;runOnUiThread(()->done(completed));
        });
    }
    private void done(String message){if(cancelled.get())return;pending=false;selected=null;choices.setEnabled(true);export.setEnabled(ids.length>0);status.setText(message);}
    private void export(String id,Uri destination)throws Exception{
        checkpoint();int descriptorsBefore=descriptorCount();LinkedHashMap<String,byte[]> snapshot=new LinkedHashMap<>();
        try(DiagnosticReadGate.Lease lease=ProviderExecutionOwner.beginDiagnosticRead()){
                lease.check();File directory=directory(id);for(String name:new String[]{"authority.json","state.json","index.json"})snapshot.put(name,read(directory,name));snapshot.putAll(RetainedEffectsAcceptance.exportReceipts(this,id,receiptCancellation));}
        // The exact bytes are evidence even if an interrupted journal is inconsistent.
        JSONArray entries=new JSONArray();
        for(Map.Entry<String,byte[]> entry:snapshot.entrySet())entries.put(new JSONObject().put("path",entry.getKey()).put("bytes",entry.getValue().length).put("sha256",hex(MessageDigest.getInstance("SHA-256").digest(entry.getValue()))));
        snapshot.put("export-receipt.json",new JSONObject().put("schema_version",1).put("execution_id",id).put("files",entries).put("diagnostic_only",true).put("journal_consistency_not_asserted",true).put("open_descriptors_before_capture",descriptorsBefore).put("open_descriptors_after_capture",descriptorCount()).toString().getBytes(java.nio.charset.StandardCharsets.UTF_8));
        checkpoint();MessageDigest written=MessageDigest.getInstance("SHA-256");
        try(OutputStream raw=getContentResolver().openOutputStream(destination,"w")){
            if(raw==null)throw new java.io.IOException("output missing");
            try(ZipOutputStream zip=new ZipOutputStream(new DigestOutputStream(raw,written))){
                for(Map.Entry<String,byte[]> entry:snapshot.entrySet()){checkpoint();zip.putNextEntry(new ZipEntry(entry.getKey()));byte[] bytes=entry.getValue();for(int offset=0;offset<bytes.length;offset+=65536){checkpoint();zip.write(bytes,offset,Math.min(65536,bytes.length-offset));}zip.closeEntry();}
            }
        }
        checkpoint();String expected=hex(written.digest());MessageDigest actual=MessageDigest.getInstance("SHA-256");long length=0;
        try(InputStream input=getContentResolver().openInputStream(destination)){
            if(input==null)throw new java.io.IOException("readback missing");byte[] buffer=new byte[65536];int n;
            while((n=input.read(buffer))!=-1){checkpoint();length+=n;if(length>34L*1024*1024)throw new java.io.IOException("readback bound");actual.update(buffer,0,n);}
        }
        if(!expected.equals(hex(actual.digest())))throw new java.io.IOException("readback mismatch");
    }
    /** Counts only our process descriptors, never exports their targets or contents. */
    private int descriptorCount()throws Exception{
        int count=0;try(java.nio.file.DirectoryStream<java.nio.file.Path> stream=java.nio.file.Files.newDirectoryStream(java.nio.file.Paths.get("/proc/self/fd"))){
            for(java.nio.file.Path ignored:stream){checkpoint();if(++count>4096)throw new java.io.IOException("descriptor bound");}
        }return count;
    }
    private static String hex(byte[] bytes){StringBuilder out=new StringBuilder();for(byte b:bytes)out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();}
    @Override protected void onDestroy(){cancelled.set(true);receiptCancellation.cancel();worker.shutdown();super.onDestroy();}
}
