package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.system.Os;
import android.system.OsConstants;
import com.nhlocal.singlesorter.inference.OriginalPlanning;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.security.MessageDigest;
import java.util.UUID;
import org.json.JSONObject;

/** Owned private staging only; JNI reuses reviewed shared writers and never sees a provider output. */
final class PrivateFixStage implements AutoCloseable {
    final File directory,output;final JSONObject evidence,binding;final String unsupported;
    private final File input;
    private PrivateFixStage(File directory,File input,File output,JSONObject evidence,JSONObject binding,String unsupported){this.directory=directory;this.input=input;this.output=output;this.evidence=evidence;this.binding=binding;this.unsupported=unsupported;}
    static PrivateFixStage prepare(Context context,Uri source,NativeFixPlan.Fix fix,CancellationSignal cancel)throws Exception{
        cancel.throwIfCanceled();File supplied=context.getCacheDir(),root=PrivateFixPaths.trustedRoot(supplied);
        if(!supplied.getAbsoluteFile().equals(root))android.util.Log.i("SinglesFixStage","TRUSTED_CACHE_ALIAS_RESOLVED");
        File directory=PrivateFixPaths.create(root,UUID.randomUUID().toString());
        File input=new File(directory,"source.bin"),output=new File(directory,"staged.bin");PrivateFixStage owner=new PrivateFixStage(directory,input,output,null,null,null);boolean retained=false;
        try{
            JSONObject original=fix.binding();
            try(FileOutputStream out=create(input);ParcelFileDescriptor sourceFd=context.getContentResolver().openFileDescriptor(source,"r",cancel)){
                if(sourceFd==null||!OsConstants.S_ISREG(Os.fstat(sourceFd.getFileDescriptor()).st_mode))throw new IOException("FIX_REGULAR_SOURCE");
                try(ParcelFileDescriptor.AutoCloseInputStream in=new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(sourceFd.getFileDescriptor()))){copy(in,out,original,FixProtocol.ORIGINAL_LIMIT,cancel);out.flush();out.getFD().sync();}
            }
            cancel.throwIfCanceled();long id=OriginalPlanning.begin();if(id<=0)throw new IOException("FIX_NATIVE_OWNER");JSONObject response;
            try{
                cancel.setOnCancelListener(()->OriginalPlanning.cancel(id));cancel.throwIfCanceled();JSONObject request=new JSONObject().put("schema",1).put("source_file","source.bin").put("output_file","staged.bin").put("filename",fix.originalName).put("source_bytes",original.getLong("bytes")).put("source_sha256",original.getString("sha256")).put("repairs",fix.repairs());
                response=new JSONObject(OriginalPlanning.stageRepair(id,directory.getAbsolutePath(),request.toString()));
            }finally{cancel.setOnCancelListener(null);OriginalPlanning.abandon(id);}
            cancel.throwIfCanceled();
            if(!FixProtocol.bool(response,"ok")){String error=FixProtocol.string(response,"error",8192);if(!error.startsWith("Unsupported(")||!error.endsWith(")"))throw new IOException("FIX_NATIVE_STAGE: "+error);retained=true;return new PrivateFixStage(directory,input,output,null,null,error);}
            JSONObject evidence=response.getJSONObject("data"),binding=FixProtocol.nativeEvidence(evidence,original,fix.repairs());
            try(FileInputStream staged=open(output)){verify(staged,binding,FixProtocol.STAGED_LIMIT,cancel);}retained=true;return new PrivateFixStage(directory,input,output,new JSONObject(evidence.toString()),binding,null);
        }finally{if(!retained)owner.close();}
    }
    private static FileOutputStream create(File file)throws Exception{
        java.io.FileDescriptor fd=Os.open(file.getAbsolutePath(),OsConstants.O_WRONLY|OsConstants.O_CREAT|OsConstants.O_EXCL|OsConstants.O_NOFOLLOW|OsConstants.O_CLOEXEC,0600);
        try{return new ParcelFileDescriptor.AutoCloseOutputStream(ParcelFileDescriptor.dup(fd));}finally{Os.close(fd);}
    }
    static FileInputStream open(File file)throws Exception{
        java.io.FileDescriptor fd=Os.open(file.getAbsolutePath(),OsConstants.O_RDONLY|OsConstants.O_NOFOLLOW|OsConstants.O_NONBLOCK|OsConstants.O_CLOEXEC,0);
        try{if(!OsConstants.S_ISREG(Os.fstat(fd).st_mode))throw new IOException("FIX_PRIVATE_REGULAR");return new ParcelFileDescriptor.AutoCloseInputStream(ParcelFileDescriptor.dup(fd));}finally{Os.close(fd);}
    }
    static void verify(java.io.InputStream input,JSONObject binding,long limit,CancellationSignal cancel)throws Exception{copy(input,null,binding,limit,cancel);}
    static void copy(java.io.InputStream input,java.io.OutputStream output,JSONObject binding,long limit,CancellationSignal cancel)throws Exception{
        FixProtocol.binding(binding,limit);MessageDigest hash=MessageDigest.getInstance("SHA-256");long count=0;byte[] buffer=new byte[65536];int n;
        while((n=input.read(buffer))!=-1){cancel.throwIfCanceled();if(n==0)throw new IOException("FIX_READ_STALLED");count+=n;if(count>binding.getLong("bytes")||count>limit)throw new IOException("FIX_CONTENT_LENGTH");if(output!=null)output.write(buffer,0,n);hash.update(buffer,0,n);}
        cancel.throwIfCanceled();if(count!=binding.getLong("bytes")||!hex(hash.digest()).equals(binding.getString("sha256")))throw new IOException("FIX_CONTENT_CHANGED");
    }
    private static String hex(byte[] bytes){StringBuilder result=new StringBuilder();for(byte b:bytes)result.append(String.format(java.util.Locale.ROOT,"%02x",b&255));return result.toString();}
    @Override public void close()throws IOException{IOException failure=null;for(File owned:new File[]{input,output})if(owned.exists()&&!owned.delete())failure=new IOException("FIX_PRIVATE_CLEANUP");if(directory.exists()&&!directory.delete())failure=new IOException("FIX_PRIVATE_CLEANUP");if(failure!=null)throw failure;}
}
