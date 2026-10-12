# Stream-only staged Ogg Vorbis/Opus FixNames

`sorter_io::staged_ogg::OggStagedPlan` inspects and transforms owned byte streams.
It cannot open a path, replace a source, publish a file, or acquire storage rights.
This checkpoint does not activate the product dispatcher or a platform driver.
The unchanged `StagedTagEvidence` is returned only after full input length, SHA256,
and EOF match. Staging failure can leave untrusted partial output, which the
caller must discard. Flush, independent readback/hash, native metadata recheck,
durable journal, exact retained-original backup, and publication are separate
platform obligations. There is no source deletion in this transformer.

## Released assignment evidence

The frozen released sorter remains commit
`78d001aee5ce49628bdffebd34540fa83ba32c12`, source blob
`0b67726e72e527556f47a8eeffeb44778c62809b`. The repair subset/order remains
artist, albumartist, title, album, genre, from `FIX_FIELDS`. Every supplied change
must equal the existing Hebrew `fix_jibrish` result, differ from the original,
and match the exact raw-comment scalar before any output exists.

Format-specific source inspection on 2026-10-12 confirmed:

- [music-tag vorbis.py](https://github.com/KristoforMaynard/music-tag/blob/43389a429a423392221e0786af486974be3c4411/music_tag/vorbis.py),
  blob `170e3cd5ded8fec3552eca7137ff09312e0064ec`: both `OggVorbisFile` and
  `OggOpusFile` inherit `OggFile`, including its five lowercase key mappings and
  one-element string-list assignment. This is not inferred from FLAC or suffix.
- [music-tag file.py](https://github.com/KristoforMaynard/music-tag/blob/43389a429a423392221e0786af486974be3c4411/music_tag/file.py),
  blob `e59a9e36069bd2db6cda0e7b278a2fb03f2ea02a`: title is an alias of tracktitle;
  ordered multiple strings project through comma-space joining.
- [Mutagen 1.47.0 VCommentDict](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/_vorbis.py)
  reads all case-insensitive matching values in physical order. Assignment removes
  all matches, then appends the assigned pair. Its
  [Vorbis wrapper](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/oggvorbis.py)
  uses framed comments; its
  [Opus wrapper](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/oggopus.py)
  uses unframed comments and retains flagged binary trailers.

The private `staged_xiph` helper now shares exactly these assignment/projection
semantics with FLAC. Its explicit container mode retains the FLAC no-trailer
rule and 24-bit block budget. Ogg keys use the Vorbis specification's ASCII
0x20–0x7D range, excluding equals; FLAC keeps its already reviewed RFC 9639
0x20–0x7E range. Every unassigned pair retains exact bytes/order/casing, including
pictures, custom keys, alternate spellings such as ALBUM ARTIST, and empty values.
Vendor bytes are unchanged. Invalid UTF-8/lengths/keys/counts are rejected rather
than normalized. Repaired NUL-containing values remain explicitly unsupported.
The native TagLib reader's existing loss of empty duplicate values can cause a
stale plan rejection; this writer does not silently repair a lossy observation.

## Container contract and ordinary supported domain

The primary references are [RFC 3533](https://www.rfc-editor.org/rfc/rfc3533.html),
[Vorbis I specification, Appendix A](https://xiph.org/vorbis/doc/Vorbis_I_spec.html),
[Vorbis comment specification](https://xiph.org/vorbis/doc/v-comment.html), and
[RFC 7845 sections 3–5](https://www.rfc-editor.org/rfc/rfc7845.html).

Supported input is offset-zero Ogg v0 containing one logical Vorbis I or Opus
stream with contiguous sequence numbers starting at zero, one identification
packet alone in its BOS page, comment/setup header pages ending before audio,
and a complete EOS audio page. Ordinary FFmpeg mono/stereo output is included.
Vorbis comments and setup may share pages and span pages. Opus version 1 mapping
families 0 and 1 are accepted after basic size/channel/map bounds checks; other
versions/families are explicitly refused pending proof. Identification and
Vorbis setup packet bytes are preserved exactly; setup/codebook decoding and
full codec semantic validation are not claimed.

Inspection reads the whole recording page by page before the caller creates an
output. Every incoming page CRC, serial, sequence, reserved flag, continuation,
length and packet boundary is checked. Nonfinal 255 laces continue a packet;
exact multiples of 255 require the terminating zero lace. A zero lace that
finishes an existing packet is supported. Truly empty audio/header packets and
nil pages are explicitly outside this initial domain. Audio pages without a
finished packet require granule -1; completed audio granules must be nonnegative
and nondecreasing. Exact codec timing equations are not newly validated.

Rejected before staging: chained/grouped/multiplexed streams anywhere in the
file, foreign/repeated BOS, missing EOS, sequence gaps, CRC failures, unfinished
EOS packets, mixed last-header/audio pages, unsupported codecs (including Ogg
FLAC/Speex/Theora), invalid comments/headers, recognized or arbitrary trailing
bytes, and budget excess. A later source change is rejected during staging and
never returns success evidence.

Only the comment packet content changes. Vorbis's framing byte (including its
unused bits) and all following bytes are retained. Opus's complete trailing
binary/padding suffix is retained whether its leading low bit is zero or one.
This satisfies preservation without guessing private trailer semantics.

For nonempty repairs, the identification page is copied exactly. Each remaining
codec header packet is separately paginated with the same stream serial. Header
CRC/sequence/continuation and zero versus unset granules are generated correctly.
Every original audio page then retains exact payload, lacing, serial, flags
(including continuation/EOS), and granule bytes. Only its sequence number and
CRC can change when header page count changes. Audio packet grouping and timing
are never reconstructed or repaginated. No-op repairs preserve the whole file
byte for byte, including original header pagination.

Header separation can add one 27-byte wrapper to Vorbis files whose comment and
setup previously shared a page. Repairs do not grow comment payloads under the
released two-byte-to-two-byte mapping; duplicate collapse may shrink them.
Opus has only one subsidiary header and needs no additional page. This is why
Ogg output cannot be assumed equal-length or universally shorter.

Both input and output header prefixes are limited to 64 MiB; original headers
also have a 4096-page budget and comments a 4096-entry budget. Audio packets are
counted without buffering, with a 64 MiB per-packet budget. Only one audio page
(maximum 65,307 bytes) is buffered at a time. Cancellation is checked around
reads/writes, pages, segments, comments, assignments and each 2048 CRC bytes.

`media_sha256`/`media_length` here cover the concatenated encoded audio page
payload bytes, in packet order, excluding Ogg wrapper/lacing bytes. Source/stage
hashes cover the complete original/replacement files. The page/packet tests
independently prove the stronger unchanged audio packet boundaries and timing.
Preserved custom text and binary trailers may have private application semantics;
no claim is made about unknown nonstandard absolute-offset references in them.

## Verification status

The new portable `staged_ogg` suite has 17 cases when the native metadata bridge
is present. It independently parses packets/pages and covers five assignments,
unknown comments/artwork, duplicate/empty values, no-op identity, continued
headers/audio, exact-255 terminating laces, changed page counts, malformed
comments, CRC/sequence/serial/continuation faults, chaining/mixed-page rejection,
stale hashes/source changes, bounded reads, I/O failure, and cancellation.

Two exact CI gates cannot silently disappear: real TagLib five-field roundtrip
and FFmpeg decode comparisons on actual Rust-staged bytes. Decoder tests use both
the owned frozen recordings and newly generated 1.2-second ordinary FFmpeg
Vorbis/Opus files. CI provisions FFmpeg from its normal package registry only
when missing and fails if it remains unavailable. FFmpeg is a test-only tool;
it is not a shipped dependency. Existing FLAC counts/gates are unchanged.

Local Rust/Cargo/rustfmt/native-bridge execution is unavailable. Previously denied
Rust installation was not retried or bypassed. Rust tests, Clippy and formatter
therefore remain **unrun here**, not passed. Required exact-source CI remains open.

The independent Python fixture/spec probe was run with installed FFmpeg 7.1.5:

    python native/tests/staged_ogg/verify_fixture_contract.py

All six tests passed on 2026-10-12, including frozen source identity/CRC checks,
new ordinary FFmpeg recordings, continued comment/header repagination, preserved
encoded packets, and equal decoded PCM. The existing FLAC probe also passed four
tests. These probes verify fixture/layout assumptions, **not the Rust writer**.
