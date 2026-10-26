package com.nhlocal.singlesorter.platform;

import android.app.Activity;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.content.UriPermission;
import android.database.Cursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.Build;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.DocumentsContract.Document;
import android.system.ErrnoException;
import android.system.Os;
import android.system.OsConstants;
import java.io.Closeable;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.Objects;

/** Native-owned read-only SAF scope. Never expose URI/path arguments through WebView IPC. */
public final class SafReadStorage implements Closeable {
    private static final String[] COLUMNS = {Document.COLUMN_DOCUMENT_ID, Document.COLUMN_DISPLAY_NAME,
        Document.COLUMN_MIME_TYPE, Document.COLUMN_SIZE, Document.COLUMN_LAST_MODIFIED};
    private final ContentResolver resolver;
    private final File cache;
    private final ReadGuards picker = new ReadGuards();
    private final Map<String, Ref> refs = new HashMap<>();
    private boolean closed;
    private final HashSet<ReadHandle> handles = new HashSet<>();
    private static final class Ref {
        final Uri tree; final String id;
        Ref(Uri tree, String id) { this.tree = tree; this.id = id; }
        Uri uri() { return DocumentsContract.buildDocumentUriUsingTree(tree, id); }
    }
    public static final class Entry {
        public final String token, displayName, mimeType;
        public final Long size, lastModified;
        Entry(String token, String name, String mime, Long size, Long modified) {
            this.token = token; this.displayName = name; this.mimeType = mime;
            this.size = size; this.lastModified = modified;
        }
        public boolean isDirectory() { return Document.MIME_TYPE_DIR.equals(mimeType); }
    }
    public static final class ReadHandle implements Closeable {
        /** Borrowed descriptor; caller must not close/detach it. Valid only until this handle closes. */
        public final ParcelFileDescriptor descriptor;
        public final boolean spooled;
        private final File temporary;
        private volatile boolean closed;
        private ReadHandle(ParcelFileDescriptor descriptor, File temporary) {
            this.descriptor = descriptor; this.temporary = temporary; this.spooled = temporary != null;
        }
        @Override public synchronized void close() throws IOException {
            if (closed) return;
            closed = true;
            try { descriptor.close(); }
            finally { if (temporary != null && !temporary.delete() && temporary.exists())
                throw new IOException("SPOOL_CLEANUP_FAILED"); }
        }
    }
    public SafReadStorage(Context context) {
        Context app = context.getApplicationContext();
        resolver = app.getContentResolver(); cache = app.getCacheDir();
    }
    public synchronized long beginPicker() { ensureOpen(); return picker.beginPicker(); }
    public static Intent treePickerIntent() {
        return new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE).addFlags(
            Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION);
    }
    /** Call only from the native Activity result callback for the matching request. Null means cancel. */
    public synchronized Entry finishPicker(long request, int resultCode, Intent data,
            CancellationSignal cancellation) throws IOException {
        Objects.requireNonNull(cancellation, "cancellation");
        ensureOpen(); picker.finishPicker(request);
        if (resultCode != Activity.RESULT_OK || data == null || data.getData() == null) return null;
        Uri tree = data.getData();
        if (!"content".equals(tree.getScheme()) || !DocumentsContract.isTreeUri(tree))
            throw new IOException("INVALID_TREE");
        if ((data.getFlags() & Intent.FLAG_GRANT_READ_URI_PERMISSION) == 0 ||
            (data.getFlags() & Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION) == 0)
            throw new IOException("PERSISTABLE_READ_REQUIRED");
        resolver.takePersistableUriPermission(tree, Intent.FLAG_GRANT_READ_URI_PERMISSION);
        Ref ref = new Ref(tree, DocumentsContract.getTreeDocumentId(tree));
        requireGrant(ref);
        Entry entry = queryOne(ref, cancellation);
        if (!entry.isDirectory()) { refs.remove(entry.token); throw new IOException("ROOT_NOT_DIRECTORY"); }
        return entry;
    }
    private void ensureOpen() { if (closed) throw new IllegalStateException("SCOPE_CLOSED"); }
    private void requireGrant(Ref ref) {
        for (UriPermission grant : resolver.getPersistedUriPermissions())
            if (grant.isReadPermission() && grant.getUri().equals(ref.tree)) return;
        throw new SecurityException("TREE_GRANT_REVOKED");
    }
    private synchronized Ref resolve(String token) {
        ensureOpen(); Ref ref = refs.get(token);
        if (ref == null) throw new SecurityException("UNKNOWN_DOCUMENT_TOKEN");
        requireGrant(ref); return ref;
    }
    private synchronized String register(Ref ref) throws IOException {
        ensureOpen();
        for (Map.Entry<String, Ref> entry : refs.entrySet())
            if (entry.getValue().tree.equals(ref.tree) && entry.getValue().id.equals(ref.id)) return entry.getKey();
        if (refs.size() >= 10000) throw new IOException("DOCUMENT_SCOPE_LIMIT");
        String token = UUID.randomUUID().toString(); refs.put(token, ref); return token;
    }
    private Entry row(Ref ref, Cursor c) throws IOException {
        String id = c.getString(0), name = c.getString(1), mime = c.getString(2);
        if (id == null || name == null || mime == null || !id.equals(ref.id))
            throw new IOException("INVALID_DOCUMENT_METADATA");
        Long size = c.isNull(3) ? null : c.getLong(3);
        Long modified = c.isNull(4) ? null : c.getLong(4);
        if (size != null && size < 0) size = null;
        if (modified != null && modified <= 0) modified = null;
        return new Entry(register(ref), name, mime, size, modified);
    }
    private Entry queryOne(Ref ref, CancellationSignal cancellation) throws IOException {
        Entry entry;
        try (Cursor c = resolver.query(ref.uri(), COLUMNS, null, null, null, cancellation)) {
            if (c == null || !c.moveToFirst()) throw new IOException("DOCUMENT_UNAVAILABLE");
            entry = row(ref, c);
            if (c.moveToNext()) throw new IOException("AMBIGUOUS_DOCUMENT");
        }
        validateObservation(ref, cancellation);
        return entry;
    }
    /** Native-only provenance. Never expose URI/document IDs through UI IPC. */
    public synchronized String[] provenance(String token) {
        Ref ref = resolve(token);
        return new String[]{ref.tree.toString(), ref.tree.getAuthority(), ref.id};
    }
    public Entry inspect(String token, CancellationSignal cancellation) throws IOException {
        Objects.requireNonNull(cancellation, "cancellation");
        return queryOne(resolve(token), cancellation);
    }
    public List<Entry> children(String token, CancellationSignal cancellation) throws IOException {
        Objects.requireNonNull(cancellation, "cancellation");
        ReadGuards.requireAncestryApi(Build.VERSION.SDK_INT);
        Ref parent = resolve(token);
        if (!queryOne(parent, cancellation).isDirectory()) throw new IOException("NOT_DIRECTORY");
        Uri children = DocumentsContract.buildChildDocumentsUriUsingTree(parent.tree, parent.id);
        List<Entry> result = new ArrayList<>(); HashSet<String> ids = new HashSet<>();
        try (Cursor c = resolver.query(children, COLUMNS, null, null, null, cancellation)) {
            if (c == null) throw new IOException("PROVIDER_QUERY_FAILED");
            requireComplete(c);
            while (c.moveToNext()) {
                cancellation.throwIfCanceled();
                String id = c.getString(0);
                if (result.size() >= 10000 || id == null || !ids.add(id) || id.equals(parent.id))
                    throw new IOException("INVALID_CHILDREN");
                Ref ref = new Ref(parent.tree, id);
                // Verify ancestry through the provider, never by URI strings or display names.
                if (!DocumentsContract.isChildDocument(resolver, parent.uri(), ref.uri()))
                    throw new IOException("CHILD_OUTSIDE_SCOPE");
                result.add(row(ref, c));
            }
            requireComplete(c);
        }
        validateObservation(parent, cancellation);
        return result;
    }
    private synchronized void validateObservation(Ref ref, CancellationSignal cancellation) {
        ReadGuards.validateObservation(this::ensureOpen, cancellation::throwIfCanceled,
            () -> requireGrant(ref));
    }
    private static void requireComplete(Cursor cursor) throws IOException {
        Bundle extras = cursor.getExtras();
        if (extras != null) ReadGuards.requireCompleteListing(
            extras.getBoolean(DocumentsContract.EXTRA_LOADING, false),
            extras.getString(DocumentsContract.EXTRA_ERROR));
    }
    /** Must run off the UI thread. Provider reads may block; cancellation is not a completion guarantee. */
    public ReadHandle openRead(String token, long spoolLimit, CancellationSignal cancellation) throws IOException {
        Objects.requireNonNull(cancellation, "cancellation");
        if (spoolLimit < 0 || spoolLimit > 64L * 1024 * 1024) throw new IllegalArgumentException("SPOOL_LIMIT");
        Ref ref = resolve(token);
        Entry entry = queryOne(ref, cancellation);
        if (entry.isDirectory()) throw new IOException("NOT_FILE");
        ParcelFileDescriptor fd = resolver.openFileDescriptor(ref.uri(), "r", cancellation);
        if (fd == null) throw new IOException("PROVIDER_OPEN_FAILED");
        try {
            Os.lseek(fd.getFileDescriptor(), 0, OsConstants.SEEK_SET);
            return adopt(ref, fd, null, cancellation);
        } catch (ErrnoException error) {
            if (error.errno != OsConstants.ESPIPE) { fd.close(); throw new IOException("SEEK_FAILED", error); }
        }
        File temp = null;
        try {
            try (ParcelFileDescriptor.AutoCloseInputStream input = new ParcelFileDescriptor.AutoCloseInputStream(fd)) {
                temp = File.createTempFile("singles-read-", ".tmp", cache);
                try (FileOutputStream output = new FileOutputStream(temp)) {
                    ReadGuards.copyBounded(input, output, spoolLimit, cancellation::throwIfCanceled);
                }
            }
            cancellation.throwIfCanceled();
            ParcelFileDescriptor copied = ParcelFileDescriptor.open(temp, ParcelFileDescriptor.MODE_READ_ONLY);
            ReadHandle result = adopt(ref, copied, temp, cancellation); temp = null; return result;
        } finally {
            if (temp != null && !temp.delete() && temp.exists()) throw new IOException("SPOOL_CLEANUP_FAILED");
        }
    }
    private synchronized ReadHandle adopt(Ref ref, ParcelFileDescriptor descriptor, File temp,
            CancellationSignal cancellation) throws IOException {
        try {
            ensureOpen(); cancellation.throwIfCanceled(); requireGrant(ref);
            handles.removeIf(handle -> handle.closed);
            if (handles.size() >= 32) throw new IOException("READ_HANDLE_LIMIT");
            ReadHandle handle = new ReadHandle(descriptor, temp);
            handles.add(handle); return handle;
        } catch (RuntimeException | IOException error) {
            descriptor.close(); throw error;
        }
    }
    @Override public void close() throws IOException {
        List<ReadHandle> owned;
        synchronized (this) {
            closed = true; picker.dispose(); refs.clear();
            owned = new ArrayList<>(handles); handles.clear();
        }
        IOException failure = null;
        for (ReadHandle handle : owned) {
            try { handle.close(); } catch (IOException error) {
                if (failure == null) failure = error; else failure.addSuppressed(error);
            }
        }
        if (failure != null) throw failure;
    }
}
