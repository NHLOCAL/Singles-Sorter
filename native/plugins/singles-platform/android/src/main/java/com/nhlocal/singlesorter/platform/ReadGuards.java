package com.nhlocal.singlesorter.platform;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;

/** Platform-independent guards; contains no sorting decisions. */
public final class ReadGuards {
    private long next = 1;
    private Long pending;
    public synchronized long beginPicker() {
        if (pending != null) throw new IllegalStateException("PICKER_BUSY");
        pending = next++;
        return pending;
    }
    public synchronized void finishPicker(long request) {
        if (pending == null || pending != request) throw new IllegalStateException("STALE_PICKER");
        pending = null;
    }
    public synchronized void dispose() { pending = null; }
    public static long copyBounded(InputStream input, OutputStream output, long limit,
            Runnable checkCancelled) throws IOException {
        if (limit < 0 || limit > 64L * 1024 * 1024) throw new IllegalArgumentException("SPOOL_LIMIT");
        byte[] buffer = new byte[32768];
        long total = 0;
        while (true) {
            checkCancelled.run();
            int count = input.read(buffer, 0, (int) Math.min(buffer.length, limit - total + 1));
            if (count == -1) return total;
            if (count == 0) throw new IOException("PROVIDER_ZERO_READ");
            if (count > limit - total) throw new IOException("SPOOL_LIMIT_EXCEEDED");
            output.write(buffer, 0, count);
            total += count;
        }
    }
    public static void requireCompleteListing(boolean loading, String error) throws IOException {
        if (loading) throw new IOException("PROVIDER_LISTING_PENDING");
        if (error != null && !error.isEmpty()) throw new IOException("PROVIDER_LISTING_FAILED");
    }
    public static void requireAncestryApi(int api) throws IOException {
        if (api < 29) throw new IOException("ANCESTRY_VERIFICATION_UNSUPPORTED");
    }
    /** Acceptance boundary after provider work, including resource close, has completed. */
    public static void validateObservation(Runnable checkScope, Runnable checkCancelled,
            Runnable checkGrant) {
        checkScope.run();
        checkCancelled.run();
        checkGrant.run();
        checkCancelled.run();
    }
    public ReadGuards() { }
}
