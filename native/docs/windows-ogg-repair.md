# Selected Windows Ogg Vorbis/Opus FixNames

The product dispatcher now routes released `.ogg` and `.opus` names to the
shared `OggStagedPlan`, inspecting the actual codec headers rather than claiming
all Ogg codecs. This adds no sorter policy or discovery extension. MP3 and FLAC
remain on their existing transformers. The shared stream contract and its real
layout restrictions are documented in [staged-ogg-repair.md](staged-ogg-repair.md).
Ogg FLAC, Speex, Theora, chained and multiplexed streams remain unsupported.

The selected Windows driver uses the unchanged preview → fresh prepare/replan
→ apply sequence. Original filename repair still precedes tag inspection. A
refused tag layout retains that filename result and the exact original content.
Actual publication requires an owned empty stage, the planned complete input and
output digests, independent output readback/hash, native metadata re-verification,
Windows metadata preservation, and exact retained-original backup.

## Length and recovery binding

Existing schema-v2 evidence already separates original and replacement length
and SHA256. The Ogg path uses that same evidence; it does not alter `RepairResult`
or introduce a journal schema. `TempPrepared` records the exact length/digests
computed by the stream dispatcher before a stage exists. The actual stage must
return identical complete evidence. Every staged/published record remains bound
to those immutable planned replacement fields, while source/backup records keep
their original identity, length, and digest.

The only new recovery size allowance is original length plus at most 27 bytes,
using checked addition. This is a container-derived upper bound: all currently
allowed repair characters preserve their two-byte UTF-8 length, duplicates can
only shrink a comment packet, and separating Vorbis comment/setup can require
one additional 27-byte page wrapper. Lacing count cannot increase when the
comment packet does not grow. Opus needs no additional header page, but journal
recovery uses the shared Ogg upper bound because an extension is not codec proof.
This ceiling is not a substitute for the exact planned length/hash comparisons.
Overflow, zero sizes, over-ceiling replacements, or inconsistent later evidence
are rejected. No source cleanup, deletion or automatic replay is exposed.

## Mandatory owned fixtures and status

There are three portable dispatcher cases and eleven additional Windows product
cases. Normal CI requires all 37 Windows repair cases: the unchanged 13 original
and 13 FLAC cases plus eleven Ogg cases. It requires each new exact test name,
including both genuine TagLib `StorageMetadataProvider` preview/prepare/apply
cases. One publishes a Vorbis recording that grows exactly 27 bytes; the other
collapses duplicate Opus fields and shrinks. Each checks all five assignments,
unknown/case-sensitive pairs, artwork/vendor/trailer, encoded packet and page
preservation, original-object backup, reopened evidence, and equal FFmpeg PCM
from the actual published replacement.

Other cases cover no-op identity, corrupt CRC/truncation/chaining/multiplexing
with filename-only outcome, cancellation during stage and after backup, lost
backup/publication responses with distinct lengths, exact original retention,
foreign collision, rehashed oversized/inconsistent length/hash evidence, schema
v1 downgrade refusal for a grown replacement, and a coherent rehashed wrong
length that cannot verify the current artifact. No test touches a user library.

Local Rust, rustfmt, Clippy, native TagLib and Windows execution remain unavailable
in the authoring container; no denied installer was retried. These new Rust and
Windows tests are authored but **not locally executed**. The independent Python
Ogg fixture/model probe passes six cases; it is not execution of the Rust writer
or product driver. Independent source review and exact-source Windows CI are
required before claiming a platform acceptance result.
