# Original-plan Android duet cohort boundary

This candidate extends the reviewed Single/Fix/Album QA source only. It is not
normal-product activation or device acceptance.

- The immutable `JobPlanV2` remains the decision authority. The exact ordered
  `DuetDeleteIntention.after_operations` cohort must equal all original eligible
  `SingleTransfer` operations for its structurally equal source observation.
  Ineligible destinations are absent from that cohort, not failed required writes.
- Copies use the existing provider transfer implementation. New duet copies add
  a start and parent-component observation certificate; the new protocol checks
  the complete create/write/publish/verified sequence and exact source/stage/parent
  bindings. Legacy non-duet journal traces retain their original parser. Only its checked,
  durable `verified` receipt reaches the live retirement barrier.
- The barrier runs at the final required operation boundary, before any later
  operation. It re-resolves each complete planned destination path without writes,
  binds the currently observed document ID to its exact verified receipt, and
  checks its planned name and original bytes/hash. It then freshly checks original
  source name, content, ancestry and explicit grants. The fixed system local
  provider and quiescence assumptions still apply; these checks are not CAS.
- Retirement records prepared evidence, checked deletion intent, return, complete
  parent-list absence of both original name and ID, and a separate verified
  retirement. A failed/uncertain required copy or pre-retirement cancellation
  prevents deletion. Loss of the deletion return remains unresolved even if later
  inspection observes absence. No automatic retry or resume is added.
- Zero-required cohorts take a fresh read-only source-retained no-op path. Their
  source hashes are retained after original planning even though there are no
  native operation tokens. There is no invented transfer operation.
- The whole job and each cohort record are bounded before its first effect.
  All known partial outputs/journals survive failure; no rollback deletion occurs.
- Existing copy acknowledgments remain unchanged. UI and recovery separately
  report retirement/no-op/unresolved cohorts. New current planned-path/source
  observations never rewrite historical success. Old journals retain their old
  parser routes unless an explicit new `duet_protocol: 1` header is present.

Source checks: strict Android SDK Java compilation and portable ownership suite;
frontend typecheck and 181 tests. Fixed no-argument Android org.json runners are
`NativeDuetPlanJsonTest` and `DuetJournalJsonTest`; real-runner and provider/device
results must be recorded separately against an exact commit and artifact hash.
The new runner covers zero/one/multiple required destinations, incomplete copies,
changed required targets, cancellation before deletion, failed checked intent,
false/lost delete returns, every event prefix, malformed cohort/source/hash/name
receipts and unchanged pre-protocol historical copy observations.
