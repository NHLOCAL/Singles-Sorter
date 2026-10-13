package com.nhlocal.singlesorter.platform;
import java.io.*;
public final class ReadGuardsTest {
    interface Throwing { void run() throws Exception; }
    static int checks;
    static void rejects(String code, Throwing action) throws Exception {
        try { action.run(); throw new AssertionError("accepted " + code); }
        catch (IllegalStateException | IllegalArgumentException | SecurityException | IOException e) {
            if (!e.getMessage().equals(code)) throw e; checks++;
        }
    }
    public static void main(String[] args) throws Exception {
        ReadGuards guard = new ReadGuards(); long first = guard.beginPicker();
        rejects("PICKER_BUSY", guard::beginPicker);
        rejects("STALE_PICKER", () -> guard.finishPicker(first + 1));
        guard.finishPicker(first);
        rejects("STALE_PICKER", () -> guard.finishPicker(first));
        long second = guard.beginPicker(); guard.dispose();
        rejects("STALE_PICKER", () -> guard.finishPicker(second));
        long third = guard.beginPicker();
        rejects("STALE_PICKER", () -> guard.finishPicker(second)); guard.finishPicker(third);
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        long size = ReadGuards.copyBounded(new ByteArrayInputStream(new byte[32769]), output, 32769, () -> {});
        if (size != 32769 || output.size() != 32769) throw new AssertionError(); checks++;
        rejects("SPOOL_LIMIT_EXCEEDED", () -> ReadGuards.copyBounded(new ByteArrayInputStream(new byte[2]), new ByteArrayOutputStream(), 1, () -> {}));
        rejects("SPOOL_LIMIT", () -> ReadGuards.copyBounded(new ByteArrayInputStream(new byte[0]), output, -1, () -> {}));
        rejects("SPOOL_LIMIT", () -> ReadGuards.copyBounded(new ByteArrayInputStream(new byte[0]), output, 67108865, () -> {}));
        if (ReadGuards.copyBounded(new ByteArrayInputStream(new byte[0]), output, 0, () -> {}) != 0) throw new AssertionError(); checks++;
        rejects("cancel", () -> ReadGuards.copyBounded(new ByteArrayInputStream(new byte[1]), output, 1, () -> { throw new IllegalStateException("cancel"); }));
        InputStream zero = new InputStream() { public int read() { return 0; } public int read(byte[] b,int o,int n) { return 0; } };
        rejects("PROVIDER_ZERO_READ", () -> ReadGuards.copyBounded(zero, output, 1, () -> {}));
        rejects("PROVIDER_LISTING_PENDING", () -> ReadGuards.requireCompleteListing(true, null));
        rejects("PROVIDER_LISTING_FAILED", () -> ReadGuards.requireCompleteListing(false, "failure"));
        ReadGuards.requireCompleteListing(false, null); checks++;
        ReadGuards.requireCompleteListing(false, ""); checks++;
        rejects("ANCESTRY_VERIFICATION_UNSUPPORTED", () -> ReadGuards.requireAncestryApi(24));
        rejects("ANCESTRY_VERIFICATION_UNSUPPORTED", () -> ReadGuards.requireAncestryApi(28));
        ReadGuards.requireAncestryApi(29); checks++;
        StringBuilder order = new StringBuilder();
        ReadGuards.validateObservation(() -> order.append("scope;"),
            () -> order.append("cancel;"), () -> order.append("grant;"));
        if (!order.toString().equals("scope;cancel;grant;cancel;")) throw new AssertionError(order); checks++;
        rejects("SCOPE_CLOSED", () -> ReadGuards.validateObservation(
            () -> { throw new IllegalStateException("SCOPE_CLOSED"); },
            () -> { throw new AssertionError("cancel after closed scope"); },
            () -> { throw new AssertionError("grant after closed scope"); }));
        rejects("cancel", () -> ReadGuards.validateObservation(() -> {},
            () -> { throw new IllegalStateException("cancel"); },
            () -> { throw new AssertionError("grant after cancelled read"); }));
        rejects("TREE_GRANT_REVOKED", () -> ReadGuards.validateObservation(() -> {}, () -> {},
            () -> { throw new SecurityException("TREE_GRANT_REVOKED"); }));
        boolean[] cancelled = {false};
        rejects("cancel", () -> ReadGuards.validateObservation(() -> {},
            () -> { if (cancelled[0]) throw new IllegalStateException("cancel"); },
            () -> cancelled[0] = true));
        System.out.println(checks + " portable guard checks passed; no Android runtime test claimed");
    }
}
