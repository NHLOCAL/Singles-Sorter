# Desktop and CLI selected-root execution candidate

This integration is opt-in (`selected-windows-execution`) and must pass the
actual Windows gates at its final commit before it is described as validated.
Default preview binaries and the separately packaged original-AI preview remain
read-only. A plan document alone is never an executable authorization token.

## Desktop flow

1. Select source and target, retain the released six settings, then plan.
2. The native planning adapter probes read/list/seek capabilities and hashes the
   observed source contents without changing the selected music trees. The host
   retains the exact roots, identities, ordered resources, settings and digest.
3. Only a supported whole plan gets the Hebrew “בצע את התוכנית” action. The review
   names source, target, operation count and Copy/Move before the user confirms.
4. The renderer sends only job ID and inspected plan digest. The worker rechecks
   aliases, models, directory identities, exact plan, metadata and content hashes.
   Selected source, target and private journal roots are pinned independently.
5. Per-file byte progress is actual callback evidence, not predicted completion.
   Cancel requests remain pending until the native worker reaches a terminal
   state. Settings, root changes and repeat execution stay blocked while busy.
6. Results distinguish complete, partial, cancelled and needs-recovery. Counters
   come from durable journal evidence. Row details show actual operation results.
   Read-only reconciliation checks retained journal identities and current files;
   no automatic replay, rollback or source deletion is inferred from a receipt.

The host owns the job through reload/close cancellation. A reload can restore the
retained execution result within the same running host. Persisted journals remain
available after process exit, but a process-restart recovery UI/import flow is
not implemented in this candidate; its absence must not be described as recovery
completion. The results expose the journal directory for inspection.

## CLI flow

Run `singles-sorter run-files --ndjson` from a Windows build with the explicit
feature. Unlike `plan-files --json`, this is a bidirectional single-process
session. The explicitly selected journal directory must already exist and be disjoint from
source and target. The host never recursively creates it from the supplied path.
Send one UTF-8 line:

```json
{"schema_version":1,"journal_directory":"C:\\AppData\\SinglesJobJournals","preview":{"schema_version":2,"mode":"Sort","source":"C:\\Music\\Incoming","target":"C:\\Music\\Sorted","settings":{"copy_mode":true,"main_folder_only":false,"singles_folder":true,"exist_only":false,"abc_sort":false,"duet_mode":false},"ai_profile":"Disabled","personal_aliases":null}}
```

Read the `planned` event, review the complete plan and paths, then send exactly
`{"action":"apply","plan_id":"<returned exact digest>"}`. Sending
`{"action":"cancel"}` before confirmation changes no music files. During apply,
the same cancel command sets the cooperative cancellation token; already
committed effects are reported and are not rolled back. Each line is bounded to
16 MiB. Only the first request and its confirmation enter the one-slot stdin queue.
After confirmation, only cancel is accepted; malformed or repeated commands
cancel safely without enqueuing undrained messages. Lost stdout cancels subsequent work rather
than silently proceeding without reporting it. EOF before confirmation is an
error. Closing stdin after confirmation does not itself request cancellation.

Events are `planned`, `started`, `progress`, `result`, `cancelled` and `error`.
Partial/cancelled/recovery results exit nonzero; only a complete result exits 0.
No standalone `apply plan.json` command exists. `plan-files` still changes no
music files and still returns a non-executable plan. An unsupported build rejects
`run-files` before consuming user paths. Ctrl-C OS process termination is not a
cooperative CLI handler; inspect the journal after forced termination.

## Evidence semantics and bounds

- Resource hashes are the actual ordered alias/model content hashes supplied by
  verified resource loaders. The four original model IDs are deterministically
  JSON-encoded and hashed for the model aggregate; this is not a fifth model file.
- Adapter hashes hash pinned UTF-8 build/coverage descriptors. They are not
  claimed to be executable binary hashes.
- Copy and same-volume Move with supported single-file operations are the current
  selected-root execution scope. Artist/ABC/singles directories are journaled.
  Full albums, FixNames/tag writes, cross-volume Move and Move duets requiring
  copy-plus-retirement remain unavailable unless a later verified backend adds
  them. Whole unsupported plans are not silently reduced to a successful subset.
- Linux remains a read-only testing adapter; no Linux mutation path is enabled.
- Sources, results and journal paths are rendered with textContent/value, never
  injected as HTML. The local main view has only named scoped IPC commands.

## Acceptance

- `tests/execution.test.ts`: retained binding, one-shot ownership, no implicit
  eligibility, partial counts, cancellation, stale/lost replies, recovery states.
- `tests/desktop_apply.py`: actual Windows WebView2/IPC on generated owned audio;
  review cancellation, multi-file Copy, same-volume Move, created folders,
  independent SHA-256 checks, stale-source rejection and screenshots.
- CLI `tests/run_files.rs`: actual subprocess inspect/apply handshake, Copy/Move,
  cancel and changed-source refusal. Unsupported builds fail closed.
- Strict formatter, Clippy and Windows native gates remain required. Renderer
  fixtures do not count as native filesystem or Windows acceptance.

Official Tauri command/background-worker references used for this integration:
https://v2.tauri.app/develop/calling-rust/
https://v2.tauri.app/develop/calling-frontend/
