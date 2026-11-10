# Frozen synthetic read-only metadata corpus

These 60 owned synthetic recordings contain silence/constant DSD, invented
metadata and one locally generated 1x1 black PNG. No user library or third-party
recording is included. Original fixture material is distributed with this
repository under its MIT license (see ../../../../license.md). TagLib and other
third-party source code retain their own licenses.

Frozen bytes are authoritative: `fixtures.sha256` was copied from the acquisition
evidence. Its `fixtures/` prefix refers to this directory. The acquisition
`reference-observations.json` retains actual container/codec, exact source
open/get/value observations, hashes, generator commands and known limitations.
The acquisition directory's frozen bytes were reused, not regenerated.

`metadata-coverage.json` describes tested classes, blockers and platform status;
it is deliberately **not** a general file-format support matrix. In particular,
ASF and ADTS AAC remain legacy open errors; MP4 `.alac` means MP4/ALAC rather than
raw ALAC. The labels `tagged.aac` and `malformed_track.aac` do not imply successful
tagging; the MP4 malformed-track labels are ordinary valid tuples.

`route-observations.json` invokes the unchanged v14.0 `artists_from_song` on all
60 files under both neutral and alias-bearing filenames (120 routes), with AI
intentionally disabled. It tests only existing single-song decisions. Complete
album/job-v2 sequential planning remains a separate Task 4 gate.

Re-observe without tag saves:

    python native/tests/reference/export_metadata.py --baseline /path/to/unchanged-v14 --media native/tests/fixtures/media --output /tmp/routes.json

The exporter verifies pinned release sources and fixture hashes, rechecks each
field/error result, records new reference routes, and verifies source/copy bytes
before and after. Error messages containing the source path can differ across
locations; error kind/stage must not. The original oracle is never rewritten.

Acquisition: 2026-10-10, Python 3.12.14/Unicode 15, music-tag 0.4.3, Mutagen
1.47.0, FFmpeg Debian 7.1.5 (test-only GPL generator, not shipped). Historical
Windows dependency versions remain unverified. No native Windows Unicode-path
or real Android FD/seek acceptance is inferred from Linux or synthetic input.

Public provenance command paths use `<acquisition-root>` in place of the original
absolute acquisition location. Recording bytes, hashes and observation values
are unchanged.
