# Original AI in the read-only desktop host

## Bounded integration decision

The existing Tauri 2 host calls `sorter-recognition` directly behind an explicit
`original-ai` Cargo feature. This reuses the same authenticated original NER,
secondary classifier, limits, cancellation signal and released routing as the
verified CLI. No model is trained or substituted, and no model assets enter Git.

Alternatives considered were spawning the CLI as a sidecar (adds executable
selection, JSON transport and process lifetime complexity) and implementing a
second UI inference adapter (duplicates already-verified model/routing logic).
Direct optional library integration is the smallest path that preserves the
existing host-owned root capabilities and read-only plan projection.

## Runtime and packaging contract

- Default `desktop` build does not compile AI. `Configured` fails closed.
- `desktop,original-ai` builds the native libraries. It loads only
  `<executable-directory>/models/original`, matching the CLI ZIP layout. The
  frontend cannot select, forge or transmit an alternate model path.
- This is an explicit portable Windows directory layout, not a Tauri installer.
  Bundle generation remains off. The packaging gate must stage all five exact
  authenticated files and their notices at that location before advertising AI.
  A future installer must explicitly map the same resources; simply enabling a
  Cargo feature is not evidence that assets were bundled.
- Startup status fully validates both models off the UI thread. Each configured
  job independently reloads and revalidates the assets. An availability display
  is a point-in-time observation, never authority to skip later validation.
- Missing/corrupt assets fail with `RECOGNITION_UNAVAILABLE`, no partial plan and
  no automatic change to Disabled. Explicit Disabled remains the initial UI
  choice, as before this integration.
- No runtime downloads, arbitrary shell commands, Python, model cache, or music
  writes are introduced. Existing profile/list edits keep their existing scope.

## Job ownership and cancellation

`start_preview` returns a job ID with a `running` snapshot immediately after
reserving the sole host worker. The busy guard stays owned by that worker until
it finishes. Worker panic and thread creation failures become terminal failures.
The renderer polls `job_snapshot`; only a matching terminal response releases
its busy state. Temporary polling errors never imply the worker has stopped. An explicit host
`UNKNOWN_JOB` response instead clears the lost job and exits processing with an
error; it never fabricates a completed/cancelled result. The host busy guard
still protects any attempted retry.

`cancel_preview({jobId})` is scoped to the retained job and idempotent for its
terminal state. A shared monotonic signal reaches planning and both original
inference components. Cancelling before the start reply is remembered by the
renderer and sent once the ID arrives. Back navigation requests cancellation;
new work and setting edits remain blocked until terminal acknowledgement.
After reload, initialization retries only transient host `BUSY` replies while
the previous cancelled worker drains; it does not permanently mark the host
disconnected. Other startup failures remain explicit errors.

Result acceptance and cancellation share the host state lock. A cancelled job
cannot publish a success or expose partial pages, and an obsolete worker cannot
overwrite a newer job. Pages require a completed job. The snapshot's
`inference_executed` is actual provider evidence for completed plans only;
failed, running and cancelled jobs use null, because their discarded inference
may have begun. Plan binding and physical-apply blockers remain unchanged.

Page reload, window close and window destruction request cancellation of both
planning and the startup model probe. A lifecycle generation check also rejects
a queued start or queued model probe from a closed/reloaded view before either
can launch. There are no detached child processes.
Cancellation is cooperative; it does not forcibly interrupt an operating-system
filesystem read. Model loading is authenticated and bounded and checks the token
between loading phases. The application makes no hard real-time shutdown claim.

## Verification

Headless host tests cover busy release/repeat, scoped cancellation, no partial
pages, stale-worker protection, feature-off/missing-model availability, existing
artist import/edit persistence, and unchanged read-only planning fixtures.
An opt-in release-profile test requires `ORIGINAL_AI_BUNDLE` and checks actual
original inference, cancellation, every asset missing and corrupt, authenticated
resource binding and unchanged source/target files:

```
cargo test --locked --no-default-features --features original-ai --release --lib -- --ignored
```

Run from `native/apps/singles-ui/src-tauri`, with the documented pinned native
build prerequisites. Frontend tests cover cancellation before ID, repeated click,
retry, stale responses and Back/Forward behavior. These are not actual Windows
WebView2 acceptance. The separate Windows workflow must build and test the final
commit and packaged files before the UI is described as usable.

## Official references consulted

- Tauri resource packaging and resource resolution:
  https://v2.tauri.app/develop/resources/
- Tauri background blocking work:
  https://docs.rs/tauri/latest/tauri/async_runtime/fn.spawn_blocking.html
- Webview builder page-load hooks and Windows setup considerations:
  https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html
- Tauri Windows WebView2 prerequisites/options:
  https://v2.tauri.app/distribute/windows-installer/
