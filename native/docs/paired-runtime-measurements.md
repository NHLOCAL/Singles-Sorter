# Measured runtime and package size

The existing actual-Windows paired-v14 lane records elapsed nanoseconds for each
native invocation and each original execution. It also queries Windows
[`K32GetProcessMemoryInfo`](https://learn.microsoft.com/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo)
using read-only handles for the owned process. The OS process-lifetime peak
working set and pagefile counters are distinguished from the maximum current
working set sampled every 10 ms. Sampling is a lower bound; child processes are
excluded. Failed queries are `unavailable`, never zero-byte measurements.

The 32 settings and four duet comparisons keep their existing execution model:
the original runs inside a shared Python harness with models/imports already
loaded; the native executable starts a fresh process. Original process-lifetime
peaks therefore include setup and prior cases. Those rows are not comparable
speed ratios. Native execution follows a preview, so disk caches can be warm.

One additional fresh-process pair reuses the already-covered Copy-all duet and
its authored media, aliases and real original models. Original code/model bytes
are verified and staged before timing. The exact original CLI executes in pinned
CPython inside a filesystem-mutation confinement wrapper; the native executable
uses its ordinary positional command. Both processes include startup/model load,
use closed stdin and remove the developer PATH. Raw outputs, final media trees,
functional-match status, elapsed times and memory observations are retained.
This single small fixture does not establish full-library performance. The
original wrapper overhead and uncontrolled OS cache remain explicit; no cold-disk
claim, speedup ratio or performance conclusion is automatically produced.

When exact final CLI package paths are supplied, the report verifies the package,
compares every ZIP file member and byte digest to the extracted directory, and
records archive SHA-256, compressed byte length, total unpacked file bytes,
executable size and file count. These are file-byte sizes, not allocated disk
blocks. They are not the desktop package size or an inferred original release
size. The final desktop artifact's separate hash inventory supplies its sizes.

All 36 functional cases are persisted before package sizing or the extra fresh-
process pair starts. Optional telemetry has at most 60 seconds and uses only the
remaining eight-minute lane budget, reserving 30 seconds for final report work.
Telemetry/setup failures remain visible without aborting functional evidence. Performance does not turn any `OPEN`, `MISMATCH` or missing
functional evidence into a parity pass. Authored tests and source review are not
actual measured Windows results; the next existing lane must produce them.
