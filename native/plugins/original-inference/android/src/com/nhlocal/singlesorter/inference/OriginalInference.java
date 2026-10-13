package com.nhlocal.singlesorter.inference;

import android.os.Handler;
import android.os.Looper;
import java.nio.charset.StandardCharsets;

/** One owner, one job. Native work finishes before ownership becomes idle.
 * Call close() from Activity.onDestroy(); callbacks after close/cancel are suppressed.
 * No native references or model objects cross threads. No sorting/storage API.
 */
public final class OriginalInference implements AutoCloseable {
    static { System.loadLibrary("singles_android_inference"); }
    public interface Callback { void completed(String json); void failed(Throwable error); }
    private final Handler main = new Handler(Looper.getMainLooper());
    private long active;
    private long generation;
    private boolean closed;
    private boolean cancelled;
    static native long begin0();
    static native void cancel0(long id);
    static native int phase0(long id);
    static native void abandon0(long id);
    static native byte[] run0(long id, byte[] directory, byte[] request);

    /** Request JSON schema: {"texts":[...]}; max32 strings,4096 UTF-8 bytes each. */
    public synchronized boolean start(String directory, String request, Callback callback) {
        if (closed || active != 0) return false;
        // Character checks bound encoding allocations; native verifies UTF-8 byte limits.
        if (directory.length() > 4096 || request.length() > 65536) throw new IllegalArgumentException("input_limit");
        final byte[] path = directory.getBytes(StandardCharsets.UTF_8);
        final byte[] input = request.getBytes(StandardCharsets.UTF_8);
        if (path.length > 4096 || input.length > 65536) throw new IllegalArgumentException("input_limit");
        final long id = begin0();
        if (id == 0) return false;
        active = id;
        generation = id;
        cancelled = false;
        try {
            new Thread(() -> {
                String result = null;
                Throwable failure = null;
                try {
                    byte[] bytes = run0(id, path, input);
                    if (bytes == null) throw new IllegalStateException("native_result_missing");
                    result = new String(bytes, StandardCharsets.UTF_8);
                } catch (Throwable error) { failure = error; }
                final String completed = result;
                final Throwable error = failure;
                // Keep the slot until the worker has exited JNI, including cancelled work.
                synchronized (OriginalInference.this) {
                    if (active != id) return;
                    active = 0;
                    if (closed || cancelled) return;
                    main.post(() -> {
                        synchronized (OriginalInference.this) {
                            // New work or close invalidates an already queued callback.
                            if (closed || cancelled || generation != id) return;
                            if (error == null) callback.completed(completed);
                            else callback.failed(error);
                        }
                    });
                }
            }, "original-inference").start();
        } catch (Throwable error) {
            abandon0(id); active = 0; throw error;
        }
        return true;
    }
    public synchronized void cancel() { cancelled = true; if (active != 0) cancel0(active); }
    public synchronized boolean isBusy() { return active != 0; }
    @Override public synchronized void close() { closed = true; cancel(); }
}
