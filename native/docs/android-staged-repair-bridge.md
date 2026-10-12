# Android private staged repair boundary

This inactive JNI endpoint reuses `sorter_io::staged_repair::StagedRepairPlan`; it does not derive repairs, publish provider effects, rewrite inputs or change sorting rules. The caller must derive ordered FieldRepair values from the private original FixFile plan and bind the current source hash before any provider effect.

A caller owns a private cache directory and two different ASCII leaf names. Input is a regular no-follow file up to 64 MiB. Output is created exclusively with private permissions and bounded to 72 MiB. The shared writer verifies original content, preserves its supported audio representation, returns actual replacement length/hash and rejects unsupported formats/flags. Failed output remains in the owned cache for caller cleanup. No path, field or hash arrives directly from the WebView.

The existing numeric native job registry owns execution. Cancellation sets the original monotonic recognition signal and a separate monotonic atomic flag used by the shared staged writer. Registry ownership remains held until the JNI call unwinds, including panic. No raw pointer or JNI local reference escapes the call. Java and Rust both enforce request/path byte limits; Java additionally bounds the small response.

This is not Android Fix activation. Provider backup/publication, original-plan field binding, returned-URI rebinding, journal/recovery, actual ABI compilation and device tests remain required. The source-reused MP3/FLAC transformers retain their documented narrow format limits. No model or runtime dependency is added.
