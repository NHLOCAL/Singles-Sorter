# Android execution diagnostic export

The developer Settings action opens a fixed, non-exported native Activity. It lists only UUID-named directories beneath the app's private provider-executions directory. The selected export contains exactly authority.json (16 MiB maximum), state.json (8 MiB), index.json (4 KiB), and a generated size/SHA-256 receipt. No renderer-supplied private path, file contents outside this journal, replay, repair, or mutation authority is accepted.

A process-owned diagnostic lease excludes new execution acceptance and refuses capture during an active execution. No file or provider I/O runs while holding the execution monitor. The immutable byte snapshot releases the lease before opening the user-selected output. A stale predecessor cannot release a successor's lease. Journal consistency is deliberately not asserted by this export: interrupted raw bytes are useful evidence and require independent reconciliation.

The Activity explains that names, references, settings and results are included. Export requires explicit ACTION_CREATE_DOCUMENT selection with document URI and returned read/write grants. It writes bounded ZIP entries, checks cancellation before output and each chunk, then rereads and hashes the output. Destruction cancels the owned worker; partial picker-created output is preserved. It does not change app debuggability, device security, permissions on private files, or the journals.

Local helper and strict Java compilation do not establish actual ZIP or cancellation behavior. Device validation is required on the exact rebuilt artifact.
