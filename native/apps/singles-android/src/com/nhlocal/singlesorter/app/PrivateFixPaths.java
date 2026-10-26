package com.nhlocal.singlesorter.app;

import java.io.File;
import java.io.IOException;
import java.nio.file.Files;

/** Only the app-supplied cache root may be canonicalized into acceptance. */
final class PrivateFixPaths {
    private PrivateFixPaths() {}

    static File trustedRoot(File supplied) throws IOException {
        File root=supplied.getCanonicalFile();
        if(!root.isDirectory()) throw new IOException("FIX_PRIVATE_ROOT");
        return root;
    }

    static File create(File root,String id) throws IOException {
        if(id==null||!id.matches("[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"))
            throw new IOException("FIX_PRIVATE_ID");
        if(!root.isAbsolute()||!root.equals(root.getCanonicalFile())||!root.isDirectory())
            throw new IOException("FIX_PRIVATE_ROOT");
        File child=new File(root,"fix-"+id);
        // createDirectory is exclusive: an existing directory, file, or symlink
        // is never adopted. Do not resolve a child symlink into acceptance.
        try{Files.createDirectory(child.toPath());}
        catch(IOException failure){throw new IOException("FIX_PRIVATE_DIRECTORY_CREATE",failure);}
        File observed=child.getCanonicalFile();
        if(!observed.equals(child.getAbsoluteFile())
                ||!root.equals(observed.getParentFile())||!child.isDirectory())
            throw new IOException("FIX_PRIVATE_DIRECTORY");
        return child;
    }
}
