# Existing ID3 repair in WAVE, AIFF and DSF

This independent stream slice extracts the existing ID3v2.3/v2.4 text rewrite
into a private byte-block helper. MP3 still performs its original MPEG header
and mirror checks before calling the helper; its existing ten cases remain a
mandatory regression gate. Container adapters establish their own actual media
layout. No synthetic MPEG header or file-extension relabeling is used.

The five frozen fields and ordering are unchanged. Original scalar values,
frame flags/encoding, preserved unknown frames/artwork, canonical Hebrew
replacement and sufficient existing padding are checked by the shared helper.
No output is produced before inspection succeeds. File/tag size and every
non-tag byte remain unchanged. Full-source hash/EOF, staged hash and complete
encoded-audio hash are returned only after successful streaming, with 64 KiB
buffers and cancellation around I/O. A partial output is never publication
permission. Native retained-original staging remains a separate product layer.

## Exact inspected reader semantics

The pinned music-tag ID3 family reads and writes the existing ID3 tag for these
containers. Mutagen 1.47.0 WAVE selects the ID3 chunk, AIFF selects its ID3 chunk,
and DSF uses the metadata pointer in the DSD header. Its DSF save truncates after
that metadata; this implementation deliberately refuses a nonterminal or
inconsistent pointer rather than risk discarding unexplained data.

The current native bridge explicitly calls WAV::ID3v2Tag(), AIFF::tag() and
DSF::tag(); it does not use WAV's generic INFO-fallback TagUnion. Accordingly,
RIFF INFO data is preserved without being substituted for ID3. The native test
contains conflicting INFO title bytes and requires the five ID3 values. Duplicate
ID3 chunks are refused before writing rather than selecting an ambiguous tag.

- [Mutagen WAVE](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/wave.py)
- [Mutagen AIFF](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/aiff.py)
- [Mutagen DSF](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/dsf.py)
- [TagLib WAV reader](https://github.com/taglib/taglib/blob/v2.3.2/taglib/riff/wav/wavfile.cpp#L239-L280)

These are inspected pinned versions; the historical packaged wheel versions
remain unverified. No source code from those libraries is copied into the writer.

## Current admission

- RIFF/WAVE, one 16-byte PCM/IEEE-float fmt chunk, one data chunk, exact sample
  alignment and byte rate. RF64/BW64/extensible or compressed formats are explicit
  remaining representations.
- FORM/AIFF, one 18-byte COMM and one SSND, checked sample count/channel/bit-size
  extent and sound offset, no block interleaving. AIFC is not yet admitted.
- DSF, exact 28/52/12-byte DSD/fmt/data headers, ordinary raw DSD/channel mapping,
  checked block-padded sample extent and a terminal metadata pointer exactly
  following audio. Encoded payload and pointer/file-size bytes remain unchanged.
- One existing unflagged ID3v2.3/v2.4 block whose internal size exactly fills the
  container's tag region. Unknown chunks and odd alignment padding are retained.
  Tag/frame budgets remain 16 MiB/4,096; container chunk count is at most 65,536.

ID3 unsynchronization/footer/extended headers, ambiguous duplicates, insufficient
padding and new tag insertion remain explicit limitations. A missing tag does
not itself create an original five-field repair, while filename repair remains
independent. Future growth support must preserve unknown offset-bearing frames
and container sizes rather than relaxing this bound silently.

Three independent Python/FFmpeg fixture cases pass (WAVE, AIFF and DSF decoded
PCM equality). They do not execute the Rust writer. Ten authored Rust/native/
FFmpeg cases and the unchanged MP3 gate require actual CI. The separate product
dispatch admits .wav/.aiff/.dsf and binds replacement length exactly equal to
original length in both staging and strict recovery. Six authored Windows cases
cover all three formats, real native preview/replan, unsupported representation,
cancellation before/after backup, and read-only restart evidence. Their native
execution and host/package acceptance remain required.
