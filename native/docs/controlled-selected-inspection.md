# Controlled selected-job reconciliation

This additive backend slice is based on `61afbb4` plus the three sealed copied-Move readiness commits (`2bd0ae5`, `d590993`, `15f3f375`). It keeps the observed native compiler corrections and formatter changes. It does not activate restart UI, Fix descriptors, Album authority, or a new mutation route.

## Host and CLI contract

- `InspectionControl::new(real_cancel_arc, observer)` shares the UI owner's existing flag.
- `InspectionControl::borrowed(actual_cancel_flag, observer)` borrows the CLI/static Ctrl-C flag for synchronous inspection. It does not create a replacement flag or polling relay. The type is `InspectionControl<'a>`; an owned control may be retained as `InspectionControl<'static>`.
- `SelectedWindowsJob::inspect_recovery_controlled(&scope, &control)` reads that job's retained exact journal identities, strictly replays the current copied-Move/Copy/rename domain and returns the existing report shape.
- Cancellation escapes as `ExecutionError::Cancelled`, including journal/parser/history, metadata, source/destination, retired-cohort and main/named-stream reads. The caller must retain an unresolved verification state and existing effect receipts. Cancellation does not undo Apply, resume a job, delete anything, or justify repeating an effect.
- Progress snapshots count actual journal and media bytes, probes, checked records, original operation IDs, completed observations and elapsed milliseconds using checked arithmetic. The observer runs outside the control's mutex. The host owns terminal snapshot publication and must retain its worker/Busy lease until all read handles have dropped.
- Reusing a cursor in `recover_job_controlled` changes observation order only. Every required object is freshly re-read, report IDs remain original, and dependency evaluation remains in original order. No cached hash becomes current proof.

## Evidence and bounded work

The current route-specific metadata requirement and whole-cohort metadata prepass remain before any source/destination/main/named-stream content reads. Default-refusing full-stream hooks remain default-refusing. Retired copied Move still requires every required destination's exact identity, main digest, named inventory/digests and metadata receipt. A foreign/reappeared original remains unresolved.

Created directory identities are freshly checked through the selected roots and held through the file inspection. Replacement cannot be hidden by moving the original copied files back into the replacement. A mismatched directory makes controlled inspection fail without an all-clear report.

Content streams use a 64 KiB buffer, checked cumulative actual-byte accounting, expected-length bounds and a one-byte EOF/growth probe. There is no 64 MiB aggregate media ceiling. The controlled recorded-input limits are 64 MiB per journal, 4 MiB per record, 65,536 terminated records, 4,096 operations and 8,192 created directories. Exhaustion returns an error, never success. These parser ceilings bound in-memory replay; they do not silently skip oversized evidence.

Observations describe an inspection interval. File handles close between observations; this is not an atomic whole-library snapshot. Existing ordinary/quiescent NTFS assumptions remain, including no adversarial concurrent creation of new named streams.

Cancellation is cooperative between native probes and each bounded read. A synchronous OS call already in progress can delay acknowledgement; no hard wall-clock cancellation guarantee is claimed. Windows documents driver-dependent cancellation and races around thread-wide synchronous cancellation, so this slice does not add an unsafe cross-thread `CancelSynchronousIo` route. See [Windows cancellation considerations](https://learn.microsoft.com/en-us/windows/win32/fileio/canceling-pending-i-o-operations) and [ReadFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile).

## Acceptance status

Source fixtures cover actual selected completed and partial copied jobs: cancel from the first main chunk, destination ADS chunk and still-present-source ADS chunk; cancel identity-bound journal reads and actual strict replay; fresh full reinspection; mixed route metadata ordering; replaced directory with preserved file objects; and more than the old aggregate media limit. Owned snapshots compare root/directories, full identities/kinds/sizes, permissions/attributes, creation/write times, main content and every generated named stream, excluding access time. Native CI requires exactly 21 copied-Move job cases, retaining all original cases.

Portable test source additionally covers parser cancellation, cancellation escaping storage errors, original dependency order after a cursor and borrowed flag identity. `git diff --check` and source gate-count checks are local checks only. Rust compilation, formatter, all Rust tests, actual Windows fixture execution, host integration and package/renderer acceptance remain pending. There is no local native-runtime claim.

Copied-Move preparation now checks both exact serialized physical and directory
headers, including their hash envelopes and newline, against the same 4 MiB line
limit as controlled inspection. It also checks the physical-operation and maximum
record-count bounds before any journal/destination effect. Copied-domain writers
refuse an oversized subsequent record before appending it, preserving a readable
prefix and stopping on uncertainty. Existing Copy/rename admission is unchanged.
The size check occurs during fresh preparation; preview remains read-only but does
not yet reproduce the complete serialized execution binding. Three exact-boundary
unit cases are required, including the one-byte envelope overflow.
