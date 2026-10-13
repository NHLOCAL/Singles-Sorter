package com.nhlocal.singlesorter.inference;

import android.content.Context;
import android.os.Looper;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.IOException;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;

/** Extracts only the five pinned original assets from the installed APK.
 * Worker thread only. No network, provider access, arbitrary paths or model changes.
 */
public final class OriginalAssets {
    private OriginalAssets() {}
    private static final String[] NAMES = {"classifier-v14.vocab.txt", "classifier-v14.weights.bin", "manifest.json", "tensors.f32", "tokenizer.json"};
    private static final long[] SIZES = {1268235, 3192809, 32600, 3829976, 73348};
    private static final String[] HASHES = {
        "35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3",
        "aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33",
        "3fb1801718d84c335b31cdae18ca94daf6af821fb29c315bcf485de3810b5580",
        "84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745",
        "33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574"
    };
    public static File prepare(Context context) throws IOException {return prepare(context, () -> false);}
    public static File prepare(Context context, java.util.function.BooleanSupplier cancelled) throws IOException {
        if (cancelled.getAsBoolean()) throw new IOException("cancelled");
        if (Looper.myLooper() == Looper.getMainLooper()) throw new IllegalStateException("worker_required");
        File parent = context.getDir("original-inference", Context.MODE_PRIVATE);
        // A unique extraction directory is never rewritten while an inference may read it.
        File directory = File.createTempFile("bundle-", "", parent);
        if (!directory.delete() || !directory.mkdir()) throw new IOException("bundle_directory");
        try {
            for (int i = 0; i < NAMES.length; i++) {
                MessageDigest digest = MessageDigest.getInstance("SHA-256");
                File output = new File(directory, NAMES[i]);
                long count = 0;
                try (InputStream input = context.getAssets().open("original/" + NAMES[i]);
                     FileOutputStream stream = new FileOutputStream(output)) {
                    byte[] buffer = new byte[16384];
                    int size;
                    while ((size = input.read(buffer)) != -1) {
                        if (Thread.currentThread().isInterrupted() || cancelled.getAsBoolean()) throw new IOException("cancelled");
                        count += size;
                        if (count > SIZES[i]) throw new IOException("asset_size");
                        digest.update(buffer, 0, size); stream.write(buffer, 0, size);
                    }
                    stream.getFD().sync();
                }
                StringBuilder hash = new StringBuilder();
                for (byte value : digest.digest()) hash.append(String.format(java.util.Locale.ROOT, "%02x", value & 255));
                if (count != SIZES[i] || !HASHES[i].equals(hash.toString())) throw new IOException("asset_integrity");
            }
            return directory;
        } catch (IOException | NoSuchAlgorithmException | RuntimeException error) {
            cleanup(directory); throw new IOException("original_assets_unavailable", error);
        }
    }
    /** Only after the owner has exited JNI; caller must retain the directory until then. */
    public static void cleanup(File directory) {
        for (String name : NAMES) new File(directory, name).delete();
        directory.delete();
    }
}
