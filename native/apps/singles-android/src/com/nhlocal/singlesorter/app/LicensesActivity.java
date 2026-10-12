package com.nhlocal.singlesorter.app;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.security.DigestOutputStream;
import java.security.MessageDigest;
import java.util.HashMap;
import java.util.HashSet;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;
import org.json.JSONArray;
import org.json.JSONObject;

/** Offline notices. No JavaScript, native bridge, external browsing or broad file access. */
public final class LicensesActivity extends Activity {
    private static final String ORIGIN="appassets.androidplatform.net";
    private static final long MAX_FILE=64L*1024*1024,MAX_TOTAL=256L*1024*1024;
    private final ExecutorService worker=Executors.newSingleThreadExecutor();
    private final AtomicBoolean cancelled=new AtomicBoolean();
    private WebView web;private TextView status;private Button export;private boolean pending;
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout layout=new LinearLayout(this);layout.setOrientation(LinearLayout.VERTICAL);
        Button back=new Button(this);back.setText("חזרה");back.setOnClickListener(v->finish());layout.addView(back);
        export=new Button(this);export.setText("שמירת הודעות הרישוי וקוד המקור");export.setOnClickListener(v->chooseExport());layout.addView(export);
        status=new TextView(this);layout.addView(status);
        web=new WebView(this);WebSettings settings=web.getSettings();
        settings.setJavaScriptEnabled(false);settings.setDomStorageEnabled(false);settings.setAllowFileAccess(false);settings.setAllowContentAccess(false);
        settings.setBlockNetworkLoads(true);settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);settings.setSupportMultipleWindows(false);
        web.setWebViewClient(new WebViewClient(){
            @Override public boolean shouldOverrideUrlLoading(WebView view,WebResourceRequest request){return path(request.getUrl())==null;}
            @Override public WebResourceResponse shouldInterceptRequest(WebView view,WebResourceRequest request){
                String relative=path(request.getUrl());if(relative==null||!"GET".equals(request.getMethod()))return denied();
                // Binary sources are delivered only by the explicit picker export.
                if(!(relative.endsWith(".html")||relative.endsWith(".txt")||relative.endsWith(".md")||relative.endsWith("COPYING")||relative.endsWith("NOTICE")||relative.endsWith(".json")||relative.endsWith(".css")||relative.endsWith("LICENSE")||relative.contains("LICENSE-")))return denied();
                try{HashMap<String,String> headers=new HashMap<>();headers.put("Content-Security-Policy","default-src 'none'; style-src 'unsafe-inline' 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'");headers.put("X-Content-Type-Options","nosniff");
                    return new WebResourceResponse(relative.endsWith(".html")?"text/html":relative.endsWith(".css")?"text/css":"text/plain","UTF-8",200,"OK",headers,getAssets().open("licenses/"+relative));
                }catch(Exception failure){return denied();}
            }
        });
        layout.addView(web,new LinearLayout.LayoutParams(-1,0,1));setContentView(layout);web.loadUrl("https://"+ORIGIN+"/licenses/index.html");
    }
    private static String path(Uri uri){
        if(!"https".equals(uri.getScheme())||!ORIGIN.equals(uri.getHost())||uri.getPort()!=-1||uri.getUserInfo()!=null||uri.getQuery()!=null)return null;
        String path=uri.getPath();if(path==null||!path.startsWith("/licenses/"))return null;
        String relative=path.substring(10);return LicensePaths.safe(relative)?relative:null;
    }
    private static WebResourceResponse denied(){return new WebResourceResponse("text/plain","UTF-8",403,"Forbidden",new HashMap<>(),new ByteArrayInputStream(new byte[0]));}
    private void chooseExport(){
        if(pending)return;pending=true;export.setEnabled(false);status.setText("");
        try{startActivityForResult(new Intent(Intent.ACTION_CREATE_DOCUMENT).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION).addCategory(Intent.CATEGORY_OPENABLE).setType("application/zip").putExtra(Intent.EXTRA_TITLE,"singles-android-licenses-and-sources.zip"),1);}
        catch(RuntimeException error){pending=false;export.setEnabled(true);status.setText("לא ניתן לפתוח את בחירת הקובץ.");}
    }
    @Override protected void onActivityResult(int request,int result,Intent data){
        super.onActivityResult(request,result,data);if(request!=1||!pending)return;
        if(result!=RESULT_OK||data==null||data.getData()==null){pending=false;export.setEnabled(true);return;}
        final Uri destination=data.getData();
        if(!LicensePaths.exportGrant("content".equals(destination.getScheme()),android.provider.DocumentsContract.isDocumentUri(this,destination),(data.getFlags()&Intent.FLAG_GRANT_READ_URI_PERMISSION)!=0,(data.getFlags()&Intent.FLAG_GRANT_WRITE_URI_PERMISSION)!=0)){pending=false;export.setEnabled(true);status.setText("יעד השמירה אינו נתמך.");return;}
        worker.execute(()->{
            String message;
            try{exportBundle(destination);message="הודעות הרישוי וקוד המקור נשמרו ואומתו.";}
            catch(Exception error){message="השמירה לא הושלמה. ייתכן שנוצר קובץ חלקי; הוא לא נמחק אוטומטית.";}
            final String completed=message;runOnUiThread(()->{if(!cancelled.get()){pending=false;export.setEnabled(true);status.setText(completed);}});
        });
    }
    private void checkpoint()throws java.io.IOException{if(cancelled.get())throw new java.io.IOException("cancelled");}
    private byte[] inventory()throws Exception{
        try(InputStream input=getAssets().open("licenses/inventory.json");ByteArrayOutputStream output=new ByteArrayOutputStream()){
            byte[] buffer=new byte[8192];int n;while((n=input.read(buffer))!=-1){checkpoint();if(output.size()+n>2*1024*1024)throw new java.io.IOException("inventory bound");output.write(buffer,0,n);}return output.toByteArray();
        }
    }
    private void exportBundle(Uri destination)throws Exception{
        checkpoint();byte[] inventory=inventory();JSONObject manifest=new JSONObject(new String(inventory,java.nio.charset.StandardCharsets.UTF_8));
        if(manifest.getInt("schema_version")!=1)throw new java.io.IOException("schema");
        JSONArray files=manifest.getJSONArray("files");if(files.length()==0||files.length()>4096)throw new java.io.IOException("file count");
        HashSet<String> names=new HashSet<>();long total=inventory.length;
        for(int i=0;i<files.length();i++){JSONObject f=files.getJSONObject(i);String name=f.getString("path");long size=f.getLong("bytes");
            if(!LicensePaths.safe(name)||name.equals("inventory.json")||!names.add(name)||size<0||size>MAX_FILE||!f.getString("sha256").matches("[0-9a-f]{64}"))throw new java.io.IOException("entry");
            total+=size;if(total>MAX_TOTAL)throw new java.io.IOException("total bound");}
        MessageDigest archive=MessageDigest.getInstance("SHA-256");
        try(OutputStream raw=getContentResolver().openOutputStream(destination,"w")){
            if(raw==null)throw new java.io.IOException("output missing");
            try(ZipOutputStream zip=new ZipOutputStream(new DigestOutputStream(raw,archive))){
                zip.putNextEntry(new ZipEntry("inventory.json"));zip.write(inventory);zip.closeEntry();byte[] buffer=new byte[65536];
                for(int i=0;i<files.length();i++){checkpoint();JSONObject f=files.getJSONObject(i);String name=f.getString("path");long count=0;MessageDigest digest=MessageDigest.getInstance("SHA-256");zip.putNextEntry(new ZipEntry(name));
                    try(InputStream input=getAssets().open("licenses/"+name)){int n;while((n=input.read(buffer))!=-1){checkpoint();count+=n;if(count>f.getLong("bytes"))throw new java.io.IOException("asset grew");digest.update(buffer,0,n);zip.write(buffer,0,n);}}
                    if(count!=f.getLong("bytes")||!hex(digest.digest()).equals(f.getString("sha256")))throw new java.io.IOException("asset mismatch");zip.closeEntry();}
            }
        }
        checkpoint();String expected=hex(archive.digest());MessageDigest actual=MessageDigest.getInstance("SHA-256");long count=0;
        try(InputStream input=getContentResolver().openInputStream(destination)){
            if(input==null)throw new java.io.IOException("readback missing");byte[] buffer=new byte[65536];int n;while((n=input.read(buffer))!=-1){checkpoint();count+=n;if(count>MAX_TOTAL+16L*1024*1024)throw new java.io.IOException("readback bound");actual.update(buffer,0,n);}}
        if(!expected.equals(hex(actual.digest())))throw new java.io.IOException("readback mismatch");
    }
    private static String hex(byte[] bytes){StringBuilder out=new StringBuilder();for(byte b:bytes)out.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return out.toString();}
    @Override protected void onDestroy(){cancelled.set(true);worker.shutdown();if(web!=null)web.destroy();super.onDestroy();}
}
