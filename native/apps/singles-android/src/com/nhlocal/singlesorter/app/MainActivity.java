package com.nhlocal.singlesorter.app;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.net.Uri;
import android.webkit.JavascriptInterface;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import java.io.ByteArrayInputStream;
import java.util.HashMap;

/** Existing shared UI, packaged locally. No arbitrary browsing/file access with a native bridge. */
public final class MainActivity extends Activity {
    private static final String ORIGIN="appassets.androidplatform.net";
    private WebView view;private AndroidHost host;
    public final class Bridge {
        @JavascriptInterface public void invoke(String request){if(host!=null)host.invoke(request);}
    }
    @Override public void onCreate(Bundle state){
        super.onCreate(state);view=new WebView(this);host=new AndroidHost(this,view);
        WebSettings settings=view.getSettings();settings.setJavaScriptEnabled(true);settings.setDomStorageEnabled(false);
        settings.setAllowFileAccess(false);settings.setAllowContentAccess(false);settings.setBlockNetworkLoads(true);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);settings.setSupportMultipleWindows(true);
        view.addJavascriptInterface(new Bridge(),"SinglesAndroid");
        view.setWebViewClient(new WebViewClient(){
            @Override public boolean shouldOverrideUrlLoading(WebView web,WebResourceRequest request){return !local(request.getUrl());}
            @Override public WebResourceResponse shouldInterceptRequest(WebView web,WebResourceRequest request){
                Uri uri=request.getUrl();if(!local(uri)||!"GET".equals(request.getMethod()))return denied();
                String path=uri.getPath();
                if(path==null||(!path.startsWith("/assets/")&&!path.equals("/icon.png"))||path.contains("..")||path.contains("\\")||path.contains("\u0000"))return denied();
                String asset=path.equals("/icon.png")?"ui/icon.png":"ui/"+path.substring(8);
                try{
                    String mime=asset.endsWith(".html")?"text/html":asset.endsWith(".js")?"application/javascript":asset.endsWith(".css")?"text/css":asset.endsWith(".svg")?"image/svg+xml":asset.endsWith(".png")?"image/png":asset.endsWith(".woff2")?"font/woff2":"application/octet-stream";
                    HashMap<String,String> headers=new HashMap<>();headers.put("Content-Security-Policy","default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'");headers.put("X-Content-Type-Options","nosniff");
                    return new WebResourceResponse(mime,"UTF-8",200,"OK",headers,getAssets().open(asset));
                }catch(Exception error){return denied();}
            }
        });
        setContentView(view);view.loadUrl("https://"+ORIGIN+"/assets/index.html");
    }
    private static boolean local(Uri uri){return "https".equals(uri.getScheme())&&ORIGIN.equals(uri.getHost())&&uri.getPort()==-1&&uri.getUserInfo()==null;}
    private static WebResourceResponse denied(){return new WebResourceResponse("text/plain","UTF-8",403,"Forbidden",new HashMap<>(),new ByteArrayInputStream(new byte[0]));}
    @Override protected void onActivityResult(int request,int result,Intent data){super.onActivityResult(request,result,data);if(host!=null){if(request==100)host.pickerResult(result,data);else if(request==101)host.documentResult(result,data);}}
    @SuppressWarnings("deprecation")
    @Override public void onBackPressed(){if(view==null){super.onBackPressed();return;}
        view.evaluateJavascript("(()=>{const all=Array.from(document.querySelectorAll('dialog[open]'));const d=all[all.length-1];if(d){const event=new Event('cancel',{cancelable:true});if(d.dispatchEvent(event))d.close();return true;}if(history.state&&['results','detail'].includes(history.state.singlesView)){history.back();return true;}return false;})()",handled->{if("false".equals(handled)&&!isFinishing())finish();});}
    @Override protected void onDestroy(){if(host!=null)host.close();if(view!=null){view.removeJavascriptInterface("SinglesAndroid");view.destroy();}super.onDestroy();}
}
