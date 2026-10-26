package com.nhlocal.singlesorter.app;

import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import com.nhlocal.singlesorter.inference.OriginalPlanning;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.UUID;
import org.json.JSONArray;
import org.json.JSONObject;

/** Explicit document-picker CSV I/O. URI authority never crosses into JavaScript. */
final class AndroidCsv {
    private static final int LIMIT=2*1024*1024;
    private final Context context;
    private byte[] pending;
    private String token,encoding;
    private JSONObject preview;
    AndroidCsv(Context context){this.context=context.getApplicationContext();}
    static Intent importIntent(){return new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE).addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);}
    static Intent exportIntent(){return new Intent(Intent.ACTION_CREATE_DOCUMENT).setType("text/csv").addCategory(Intent.CATEGORY_OPENABLE).putExtra(Intent.EXTRA_TITLE,"personal-singers.csv").addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION);}
    private Uri document(Intent data,int permission)throws IOException {
        Uri uri=data==null?null:data.getData();
        if(uri==null||!"content".equals(uri.getScheme())||!DocumentsContract.isDocumentUri(context,uri)||(data.getFlags()&permission)!=permission)throw new IOException("DOCUMENT_GRANT_REQUIRED");
        return uri;
    }
    private String name(Uri uri,CancellationSignal signal)throws IOException {
        try(Cursor cursor=context.getContentResolver().query(uri,new String[]{OpenableColumns.DISPLAY_NAME},null,null,null,signal)){
            if(cursor==null||!cursor.moveToFirst())throw new IOException("DOCUMENT_NAME_UNAVAILABLE");
            String value=cursor.getString(0);if(value==null||value.length()>4096||cursor.moveToNext())throw new IOException("DOCUMENT_NAME_INVALID");return value;
        }
    }
    private byte[] read(Uri uri,CancellationSignal signal)throws IOException {
        ParcelFileDescriptor descriptor=context.getContentResolver().openFileDescriptor(uri,"r",signal);
        if(descriptor==null)throw new IOException("DOCUMENT_OPEN_FAILED");
        try(ParcelFileDescriptor.AutoCloseInputStream input=new ParcelFileDescriptor.AutoCloseInputStream(descriptor);ByteArrayOutputStream bytes=new ByteArrayOutputStream()){
            byte[] buffer=new byte[16384];int count;
            while((count=input.read(buffer))!=-1){signal.throwIfCanceled();if(bytes.size()+count>LIMIT)throw new IOException("CSV_TOO_LARGE");bytes.write(buffer,0,count);}
            signal.throwIfCanceled();return bytes.toByteArray();
        }
    }
    JSONObject select(Intent data,CancellationSignal signal)throws Exception {
        clear();Uri uri=document(data,Intent.FLAG_GRANT_READ_URI_PERMISSION);String filename=name(uri,signal);byte[] bytes=read(uri,signal);
        token=UUID.randomUUID().toString();pending=bytes;
        boolean validUtf8;
        try{StandardCharsets.UTF_8.newDecoder().onMalformedInput(java.nio.charset.CodingErrorAction.REPORT).onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT).decode(java.nio.ByteBuffer.wrap(bytes));validUtf8=true;}catch(java.nio.charset.CharacterCodingException malformed){validUtf8=false;}
        JSONObject decoded=validUtf8?decode(token,"Utf8"):null;
        return new JSONObject().put("token",token).put("filename",filename).put("encoding_required",!validUtf8).put("preview",decoded==null?JSONObject.NULL:decoded);
    }
    JSONObject decode(String requested,String selected)throws Exception {
        if(pending==null||!requested.equals(token))throw new IllegalStateException("INVALID_IMPORT_TOKEN");
        JSONObject result=new JSONObject(OriginalPlanning.decodeCsv(pending,selected));
        if(!result.getBoolean("ok"))throw new IllegalArgumentException(result.optString("error","CSV_INVALID"));
        JSONObject candidate=result.getJSONObject("data").put("token",token).put("encoding",selected);
        encoding=selected;preview=candidate;return new JSONObject(candidate.toString());
    }
    JSONObject confirm(AndroidProfile profile,String requested,String selected,String revision,JSONArray edits)throws Exception {
        if(preview==null||!requested.equals(token)||!selected.equals(encoding))throw new IllegalStateException("IMPORT_NOT_PREVIEWED");
        JSONArray imported=edited(preview.getJSONArray("rows"),edits);
        JSONObject current=profile.personal();if(!current.getString("revision").equals(revision))throw new IllegalStateException("PERSONAL_LIST_CHANGED");
        JSONArray rows=current.getJSONArray("rows");for(int i=0;i<imported.length();i++)rows.put(imported.getJSONArray(i));
        JSONObject saved=profile.savePersonal(revision,rows);clear();return saved;
    }
    // Original-index edit semantics match desktop: no trimming, repair, reordering
    // or duplicate removal; delete/set conflicts are rejected before any write.
    static JSONArray edited(JSONArray original,JSONArray edits)throws Exception {
        if(edits.length()>original.length()*2)throw new IllegalArgumentException("IMPORT_EDIT_INVALID");
        int[] touched=new int[original.length()];long bytes=0;
        for(int i=0;i<edits.length();i++){
            JSONObject edit=edits.getJSONObject(i);int index=exactIndex(edit,"index"),mask;String kind=edit.getString("kind");
            if(index<0||index>=original.length())throw new IllegalArgumentException("IMPORT_EDIT_INVALID");
            if(kind.equals("set")){int column=exactIndex(edit,"column");if(column<0||column>1||edit.length()!=4||!(edit.get("value") instanceof String))throw new IllegalArgumentException("IMPORT_EDIT_INVALID");mask=1<<column;bytes+=edit.getString("value").getBytes(StandardCharsets.UTF_8).length;if(bytes>LIMIT)throw new IllegalArgumentException("CSV_TOO_LARGE");}
            else if(kind.equals("delete")&&edit.length()==2)mask=4;else throw new IllegalArgumentException("IMPORT_EDIT_INVALID");
            if((touched[index]&mask)!=0||(touched[index]!=0&&(mask==4||touched[index]==4)))throw new IllegalArgumentException("IMPORT_EDIT_INVALID");touched[index]|=mask;
        }
        JSONArray changed=new JSONArray(original.toString());
        for(int i=0;i<edits.length();i++){JSONObject edit=edits.getJSONObject(i);if(edit.getString("kind").equals("set")){JSONArray row=changed.getJSONArray(edit.getInt("index"));int column=edit.getInt("column");while(row.length()<=column)row.put("");row.put(column,edit.getString("value"));}}
        JSONArray result=new JSONArray();for(int i=0;i<changed.length();i++)if(touched[i]!=4)result.put(changed.getJSONArray(i));return result;
    }
    private static int exactIndex(JSONObject value,String key)throws Exception {Object number=value.get(key);if(!(number instanceof Integer))throw new IllegalArgumentException("IMPORT_EDIT_INVALID");return (Integer)number;}
    JSONObject export(Intent data,byte[] bytes,int rows,CancellationSignal signal)throws Exception {
        if(bytes.length>LIMIT)throw new IOException("CSV_TOO_LARGE");
        Uri uri=document(data,Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION);String filename=name(uri,signal);
        // ACTION_CREATE_DOCUMENT creates a new user-selected document, never an
        // overwrite selection. No URI supplied by JavaScript is accepted here.
        ParcelFileDescriptor descriptor=context.getContentResolver().openFileDescriptor(uri,"w",signal);
        if(descriptor==null)throw new IOException("EXPORT_OPEN_FAILED");
        try(ParcelFileDescriptor.AutoCloseOutputStream output=new ParcelFileDescriptor.AutoCloseOutputStream(descriptor)){
            for(int offset=0;offset<bytes.length;offset+=16384){signal.throwIfCanceled();output.write(bytes,offset,Math.min(16384,bytes.length-offset));}output.flush();
        }
        signal.throwIfCanceled();
        if(!Arrays.equals(bytes,read(uri,signal)))throw new IOException("EXPORT_READBACK_MISMATCH");
        return new JSONObject().put("filename",filename).put("rows",rows);
    }
    void clear(){if(pending!=null)Arrays.fill(pending,(byte)0);pending=null;token=null;encoding=null;preview=null;}
}
