# Stream-only staged FLAC FixNames

`sorter_io::staged_flac::FlacStagedPlan` is an isolated byte-stream transformer.
It does not open paths and cannot replace, rename, delete, or publish a user
file. The shared `staged_repair::StagedRepairPlan` product dispatcher reuses this
transformer and the unchanged MP3 transformer. The dispatcher is ready for
separately reviewed platform integration: each driver must supply its own
original/replacement evidence, retained-original publication and storage
authority. The stream API and dispatcher alone do not activate a platform.

## API and trust boundary

- `inspect(input: &mut (impl Read + Seek), source_length: u64,
  repairs: &[FieldRepair], cancel: &AtomicBool) -> Result<FlacStagedPlan,
  TagWriteError>` validates the original metadata, supplied length, ordered
  assignments, and initial native audio-frame header before output exists.
- `changes_bytes()` reports whether the metadata prefix differs.
- `staged_length()` is the checked full output size, including a shifted audio
  tail when duplicate comments are collapsed.
- `write_staged(input, output: &mut impl Write, expected_source_sha256: &str,
  cancel)` returns the existing `StagedTagEvidence`: source/staged/media SHA256,
  media/output byte counts, and changed fields in original policy order.
  SHA256 strings use lowercase hexadecimal, as in `Id3StagedPlan`.

The caller supplies an exclusively owned, empty staging stream and a previously
verified source digest. Before writing, the original metadata prefix must still
match the inspected snapshot. The full consumed source length/hash and EOF are
checked before successful evidence is returned. A changed tail, wrong digest,
appended bytes, truncation, cancellation, or I/O failure produces no success
evidence. Errors may leave partial or untrusted staging bytes; the caller must
discard them. Emitted-byte hashing cannot prove that a `Write` implementation
persisted those bytes. Stage flush, independent readback/hash, metadata
reverification, durable journaling, identity/path binding, publication and source
retirement remain the caller's separate responsibilities.

Metadata is limited to 64 MiB in each prefix, 4096 blocks, and 4096 comments.
Each FLAC metadata payload retains the format's 24-bit length limit. Audio is
streamed in 64 KiB chunks; cancellation is checked between metadata reads,
comments, assignments and output chunks. No whole recording is buffered.

## Released assignment semantics

The exact order is artist, albumartist, title, album, genre (`FIX_FIELDS`). A
request must be a strictly ordered subset, must actually change a value, and
must equal `fix_jibrish(original, Hebrew)`. Other fields and arbitrary edits are
rejected. Case-insensitive matching uses the five exact keys; for example,
`ALBUM ARTIST` is an untouched custom field, not an albumartist alias.

The source proof is:

- The released `src/core/singles_sorter_v5.py` and current Python sorter read
  each field's `.value`, repair it, and assign the resulting string sequentially.
- [music-tag's inspected 0.4.3 source](https://github.com/KristoforMaynard/music-tag/blob/43389a429a423392221e0786af486974be3c4411/music_tag/flac.py)
  maps those fields to lowercase Vorbis keys and assigns a one-element list.
  Its [MetadataItem projection](https://github.com/KristoforMaynard/music-tag/blob/43389a429a423392221e0786af486974be3c4411/music_tag/file.py)
  joins ordered string values with comma-space. The inspected repository
  `__init__.py` declares 0.4.3; this is source inspection, not an executed
  historical wheel comparison.
- [Mutagen 1.47.0 VCommentDict](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/_vorbis.py)
  retrieves every matching value in physical order. Assignment deletes all
  case-insensitive matches, then appends the new pair.

Consequently the transformer retains all unassigned pairs exactly, removes all
pairs for repaired keys, and appends one lowercase assignment per repaired field
in policy order. Vendor bytes and untouched comment order/casing are preserved.
Duplicate repaired values are projected through the existing `project(Xiph, …)`
function and compared with the planner's original scalar before transformation.

### Current native-reader boundary

The owned bridge reads TagLib 2.3.2 `fieldListMap()` before applying Rust's Xiph
projection. [TagLib's parser/addField](https://github.com/taglib/taglib/blob/v2.3.2/taglib/ogg/xiphcomment.cpp)
ignores empty values. The transformer instead retains empty components in the
released projection. A repair computed from a lossy empty-duplicate observation
fails `Stale`; it is never silently applied. An exact manually supplied released
projection does work. This change does not fix the native reader. Repaired values
containing NUL are explicitly unsupported pending native interoperability proof;
unassigned values containing NUL remain untouched. Invalid UTF-8 is rejected,
not replaced with replacement characters.

## Native FLAC layout and preservation proof

Primary references are [RFC 9639](https://www.rfc-editor.org/rfc/rfc9639.html)
(sections 8.1, 8.3–8.8 and 9.1) and the
[Xiph Vorbis comment specification](https://xiph.org/vorbis/doc/v-comment.html).
FLAC comment payloads omit Vorbis's framing bit and packet prefix. Length/count
words inside comments are little-endian; FLAC block lengths are 24-bit big-endian.
Field names are printable ASCII excluding `=`, case-insensitive; values and the
vendor are UTF-8. RFC 9639 permits `~` in names, so unassigned such names are
preserved even though the inspected legacy libraries have a narrower validator.

Only the VORBIS_COMMENT block payload and its length can change. Its last-block
bit and block position remain the same. STREAMINFO, PADDING, SEEKTABLE, CUESHEET,
PICTURE, APPLICATION and reserved block header/body bytes are preserved. All
bytes after the final metadata block are copied exactly and independently hashed.

There is no padding requirement. RFC 9639 section 8.3 explicitly contemplates
rewriting a file when inserting metadata. SEEKTABLE byte offsets are relative to
the first audio-frame header, so moving the complete audio tail by a common
amount does not alter them. CUESHEET track/index offsets are measured in samples;
STREAMINFO's PCM MD5 and frame CRCs do not cover the preceding metadata prefix.
Picture payloads have no FLAC-file byte offset to adjust. Opaque application and
reserved payloads are preserved without interpreting or guaranteeing private,
nonstandard references inside them. The stream-only API has no blanket unknown-block rejection. The product dispatcher
rejects APPLICATION and reserved block types when metadata length changes, because
private absolute offsets cannot be proved safe. Equal-length product repair
preserves those bytes without claiming to validate their private semantics.

In the current Hebrew repair policy, every substituted character occupies two
UTF-8 bytes before and after repair. A single-value assignment is length-neutral;
collapsing duplicates shortens the prefix. The implementation calculates and
encodes the actual output length without a fixed-offset/padding gate, but a
policy-valid growth case is not claimed or fabricated.

## Rejection/validation scope

Rejected before output: non-offset-zero native FLAC (including Ogg and leading
ID3 wrappers), missing or duplicate STREAMINFO, invalid STREAMINFO audio basics,
forbidden block type, over-budget/truncated metadata, malformed/duplicate comment
blocks, bad UTF-8/names/counts/lengths/trailing comment bytes, nonzero padding,
truncated APPLICATION ID, malformed-size/duplicate SEEKTABLE, missing/invalid
initial frame header, and recognized ID3v1/APE trailing tag mirrors.

This is not a full FLAC decoder or a semantic validator for every metadata type.
It does not verify every audio CRC, SEEKTABLE entry, CUESHEET rule, picture codec,
private application payload, or arbitrary unrecognized trailing data. Their
unchanged bytes, rather than newly manufactured semantic validity, are the
preservation guarantee. Malformed comment data is never normalized into success.

## Verification and outstanding gates

`crates/sorter-io/tests/staged_flac.rs` contains portable stream tests using the
frozen owned `blank.flac` STREAMINFO and two actual silence frames. Tests cover
all five assignments, mixed-case duplicates/empty values, exact comment order,
no-padding prefix shrink, preserved blocks/artwork/media, last-block flags,
malformed inputs, budgets, stale hashes/bytes, short reads, failures, cancellation,
and no-op identity. The native-reader roundtrip is explicitly
`#[cfg(native_metadata_bridge)]`; a build without the bridge does not claim it.

From `native`, the required Rust gates are:

    cargo +1.99.0 test --locked -p sorter-io --test staged_flac
    cargo +1.99.0 fmt --all -- --check
    cargo +1.99.0 clippy --locked -p sorter-io --all-targets -- -D warnings

Rust/Cargo/rustfmt and the native bridge are unavailable in the authoring
container. These gates have **not run**. No installation retry or alternate
installation route was attempted. This shared checkpoint does not prove Windows or Android product integration,
publication or user-library acceptance. Those require separate platform changes
and exact-source validation. Product dispatch cases
add three portable tests to the original eleven-case suite including its native
reader roundtrip. Normal CI must discover and execute the exact native roundtrip,
not silently omit it when the metadata bridge is unavailable.

An independent fixture/spec sanity probe is available from the repository root:

    python native/tests/staged_flac/verify_fixture_contract.py

On 2026-10-11 its four tests passed with installed FFmpeg 7.1.5. The probe checks
the frozen source identity, actual seek-point target byte positions, metadata
shift preservation, policy UTF-8 length invariance, and equal decoded PCM for
before/after model fixtures. Both decoded to 5292 mono 16-bit samples at 44100 Hz.
This verifies the fixture/layout assumptions, **not execution of the Rust writer**.
FFmpeg's input `-ss` seeking on this 120 ms fixture yielded no frames on both the
unchanged baseline and shifted fixture; successful decoder seeking is not
claimed. The target frame bytes and relative offsets are checked directly.
