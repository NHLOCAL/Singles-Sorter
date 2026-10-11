package com.nhlocal.singlesorter.inference;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;
import android.util.Log;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicReference;
import org.json.JSONArray;
import org.json.JSONObject;

/** Owned fixture harness, not the Singles Sorter app. No user document access. */
public final class FixtureActivity extends Activity {
    private final OriginalInference owner = new OriginalInference();
    private volatile Thread tests;
    private final JSONArray checks = new JSONArray();
    private TextView status;
    private boolean destroyed;
    private long fixtureJob;
    private synchronized long newJob() {
        if (destroyed) throw new IllegalStateException("activity_destroyed");
        fixtureJob = OriginalInference.begin0(); return fixtureJob;
    }
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        status = new TextView(this);
        status.setText("Original inference fixture only. Sorting and SAF disabled. Running…");
        setContentView(status);
        tests = new Thread(this::runTests, "owned-fixture-tests");
        tests.start();
    }
    private void check(boolean condition, String label) throws Exception {
        if (!condition) throw new IllegalStateException(label);
        checks.put(label); Log.i("OriginalInferenceProof", "PASS " + label);
    }
    private static String repeat(String text, int count) { StringBuilder out = new StringBuilder(); for (int i=0; i<count; i++) out.append(text); return out.toString(); }
    private static byte[] utf8(String text) { return text.getBytes(StandardCharsets.UTF_8); }
    private static JSONObject run(long id, File directory, byte[] request) throws Exception {
        return new JSONObject(new String(OriginalInference.run0(id, utf8(directory.getAbsolutePath()), request), StandardCharsets.UTF_8));
    }
    private static byte[] request(String text, int count) throws Exception {
        JSONArray texts = new JSONArray(); for (int i = 0; i < count; i++) texts.put(text);
        return utf8(new JSONObject().put("texts", texts).toString());
    }
    private void runTests() {
        JSONObject report = new JSONObject();
        File assets = null;
        try {
            assets = OriginalAssets.prepare(getApplicationContext());
            check(assets.isDirectory(), "assets_extracted_and_sha256_verified");
            long id = newJob();
            check(id > 0 && OriginalInference.begin0() == 0, "one_native_job");
            OriginalInference.cancel0(id);
            JSONObject cancelled = run(id, new File("/missing"), request("שלום", 1));
            check(!cancelled.getBoolean("ok") && cancelled.getString("error").equals("cancelled"), "cancel_before_asset_load");
            check(OriginalInference.phase0(id) == -1, "cancel_releases_after_return");
            check(!run(id, assets, request("שלום", 1)).getBoolean("ok"), "stale_id_rejected");
            id = newJob();
            check(!run(id, assets, new byte[65537]).getBoolean("ok"), "jni_length_bound");
            id = newJob();
            check(!run(id, assets, new byte[]{(byte)255}).getBoolean("ok"), "malformed_utf8_rejected");
            id = newJob();
            boolean nullRejected = false;
            try { byte[] rejected = OriginalInference.run0(id, null, request("x", 1)); nullRejected = rejected != null && !new JSONObject(new String(rejected, StandardCharsets.UTF_8)).getBoolean("ok"); } catch (NullPointerException expected) { nullRejected = true; }
            check(nullRejected && OriginalInference.phase0(id) == -1, "jni_null_rejected_and_slot_release");
            id = newJob();
            check(!run(id, assets, request("x", 33)).getBoolean("ok"), "document_count_bound");
            id = newJob();
            check(!run(id, new File("/missing"), request("x", 1)).getBoolean("ok"), "missing_bundle_rejected");
            id = newJob();
            JSONObject normal = run(id, assets, request("אברהם פריד - שלום עולם 🎵\u0000", 1));
            check(normal.getBoolean("ok"), "original_models_execute_unicode_nul_supplementary");
            report.put("recognition", normal);
            File tokenizer = new File(assets, "tokenizer.json");
            byte[] tokenizerBytes = java.nio.file.Files.readAllBytes(tokenizer.toPath());
            try (FileOutputStream corrupt = new FileOutputStream(tokenizer)) { corrupt.write(new byte[]{1}); }
            id = newJob();
            check(!run(id, assets, request("x", 1)).getBoolean("ok"), "tampered_bundle_rejected");
            try (FileOutputStream restore = new FileOutputStream(tokenizer)) { restore.write(tokenizerBytes); }
            final long running = newJob();
            final File fixture = assets;
            final byte[] heavy = request(repeat("אברהם פריד משה כהן שלום עולם ", 25), 32);
            final AtomicReference<JSONObject> result = new AtomicReference<>();
            final AtomicReference<Throwable> error = new AtomicReference<>();
            Thread worker = new Thread(() -> { try { result.set(run(running, fixture, heavy)); } catch (Throwable e) { error.set(e); } }, "cancel-fixture");
            worker.start();
            long deadline = System.nanoTime() + 120_000_000_000L;
            while (OriginalInference.phase0(running) != 3 && worker.isAlive() && System.nanoTime() < deadline) Thread.sleep(1);
            check(OriginalInference.phase0(running) == 3, "observed_inference_phase");
            check(!run(running, assets, request("x", 1)).getBoolean("ok"), "duplicate_run_rejected");
            check(OriginalInference.begin0() == 0, "busy_during_inference");
            OriginalInference.cancel0(running);
            worker.join(120000);
            check(!worker.isAlive() && error.get() == null, "cancel_worker_exits");
            check(!result.get().getBoolean("ok") && result.get().getString("error").toLowerCase(java.util.Locale.ROOT).contains("cancel"), "cancel_during_inference");
            long after = newJob();
            check(after > running, "new_owner_after_cancel");
            OriginalInference.abandon0(after);
            final java.util.concurrent.CountDownLatch complete = new java.util.concurrent.CountDownLatch(1);
            final java.util.concurrent.atomic.AtomicBoolean mainResult = new java.util.concurrent.atomic.AtomicBoolean();
            check(owner.start(assets.getAbsolutePath(), new String(request("אברהם פריד", 1), StandardCharsets.UTF_8), new OriginalInference.Callback() {
                public void completed(String json) { try { mainResult.set(android.os.Looper.myLooper() == android.os.Looper.getMainLooper() && new JSONObject(json).getBoolean("ok")); } catch (Exception ignored) { } finally { complete.countDown(); } }
                public void failed(Throwable failure) { complete.countDown(); }
            }), "java_owner_happy_start");
            check(complete.await(120, java.util.concurrent.TimeUnit.SECONDS) && mainResult.get(), "java_callback_on_main_thread");
            final java.util.concurrent.atomic.AtomicBoolean called = new java.util.concurrent.atomic.AtomicBoolean();
            check(owner.start(assets.getAbsolutePath(), new String(heavy, StandardCharsets.UTF_8), new OriginalInference.Callback() {
                public void completed(String json) { called.set(true); }
                public void failed(Throwable failure) { called.set(true); }
            }), "java_owner_starts_worker");
            owner.close();
            deadline = System.nanoTime() + 120_000_000_000L;
            while (owner.isBusy() && System.nanoTime() < deadline) Thread.sleep(10);
            check(!owner.isBusy(), "disposed_owner_waits_for_native_exit");
            Thread.sleep(100);
            check(!called.get(), "disposed_owner_suppresses_callback");
            check(!owner.start(assets.getAbsolutePath(), "{}", null), "disposed_owner_rejects_restart");
            report.put("ok", true);
        } catch (Throwable error) {
            Log.e("OriginalInferenceProof", "FAILED", error);
            try { report.put("ok", false).put("error", error.toString()); } catch (Exception ignored) { }
        } finally {
            // Keep fixture assets for independent hash/identity inspection, owned app data only.
            try {
                report.put("checks", checks).put("abi", android.os.Build.SUPPORTED_ABIS[0]).put("sdk", android.os.Build.VERSION.SDK_INT);
                if (assets != null) report.put("assetDirectory", assets.getAbsolutePath());
                String text = report.toString(2);
                try (FileOutputStream output = openFileOutput("proof.json", MODE_PRIVATE)) { output.write(utf8(text)); }
                Log.i("OriginalInferenceProof", "COMPLETE " + text);
                runOnUiThread(() -> status.setText("Fixture only. Sorting disabled.\n" + text));
            } catch (Throwable error) { Log.e("OriginalInferenceProof", "report", error); }
        }
    }
    @Override public void onDestroy() {
        synchronized (this) { destroyed = true; OriginalInference.cancel0(fixtureJob); }
        owner.close(); if (tests != null) tests.interrupt(); super.onDestroy();
    }
}
