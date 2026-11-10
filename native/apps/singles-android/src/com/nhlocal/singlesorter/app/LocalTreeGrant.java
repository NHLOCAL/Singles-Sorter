package com.nhlocal.singlesorter.app;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.ProviderInfo;
import android.net.Uri;
import android.provider.DocumentsContract;
/** Write access comes only from the real system tree picker, never a renderer URI. */
final class LocalTreeGrant {
    private LocalTreeGrant(){}
    static Intent picker(){return com.nhlocal.singlesorter.platform.SafReadStorage.treePickerIntent().addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION);}
    static void retainIfLocal(Context context,Intent result)throws Exception{
        Uri tree=result==null?null:result.getData();
        boolean validTree=tree!=null&&"content".equals(tree.getScheme())&&DocumentsContract.isTreeUri(tree);
        boolean authority=validTree&&ProviderReferences.AUTHORITY.equals(tree.getAuthority());
        ProviderInfo provider=authority?context.getPackageManager().resolveContentProvider(ProviderReferences.AUTHORITY,0):null;
        boolean system=provider!=null&&provider.applicationInfo!=null&&(provider.applicationInfo.flags&ApplicationInfo.FLAG_SYSTEM)!=0&&"com.android.externalstorage".equals(provider.packageName);
        int flags=result==null?0:result.getFlags();
        if(!LocalWriteGrantPolicy.accept(validTree,authority,system,(flags&Intent.FLAG_GRANT_READ_URI_PERMISSION)!=0,(flags&Intent.FLAG_GRANT_WRITE_URI_PERMISSION)!=0,(flags&Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)!=0))return;
        context.getContentResolver().takePersistableUriPermission(tree,Intent.FLAG_GRANT_READ_URI_PERMISSION|Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
    }
}
