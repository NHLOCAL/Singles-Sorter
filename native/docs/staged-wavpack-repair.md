# Exact-length WavPack APEv2 repair

This stream transformer patches only UTF-8 value bytes for the released five
fields in an existing terminal APEv2 tag. The entire contiguous WavPack block
sequence, tag headers/footer, keys, flags, item order, binary artwork and unknown
items remain byte-identical. Each repaired source/replacement UTF-8 string has
exactly the same byte length under the frozen 27-character Hebrew conversion.
No offset or size is recalculated, and no live file/path authority is present.

## Inspected legacy projection

Music-tag commit 43389a429a423392221e0786af486974be3c4411 maps Artist, Album Artist,
Title, Album and Genre. Its APE getter converts the Mutagen value to a string;
Mutagen 1.47.0 represents multiple text values with NUL separators, not comma-space.
The setter assigns that scalar string. The shared native APE projection already
reconstructs those NUL separators. The transformer checks that exact scalar and
canonical field order before patching. It does not reuse the Xiph/MP4 join rule.

- [Pinned music-tag APE mapping/getter/setter](https://github.com/KristoforMaynard/music-tag/blob/43389a429a423392221e0786af486974be3c4411/music_tag/apev2.py)
- [Pinned Mutagen APEv2 values](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/apev2.py)

Historical packaged wheel versions remain unverified. Mutagen's generic save
sorts items and can normalize assigned key casing; this preservation-oriented
implementation retains the original item order and case while producing the
same logical field values. It refuses an ID3v1 mirror or a tag at the beginning
rather than silently discarding/relocating it. Those layouts remain coverage gaps.

## Admission and evidence

The file must contain a bounded contiguous sequence of ordinary version-range
WavPack blocks ending exactly at the tag. Every block's declared extent is
checked, so the tag cannot overlap encoded payload or hide intervening bytes.
APEv2 version 2000 footer is required; a matching optional header is supported.
Tag/header reserved bytes and flags, exact item count/extent, valid ASCII keys
and case-insensitive uniqueness are checked. Writable UTF-8 text is required for
repaired fields. Read-only/binary/external repaired values are refused; unknown
binary/artwork values are preserved. No external URI is opened.

Bounds: 16 MiB tag, 4,096 items and text values, one million block headers.
Streaming uses 64 KiB buffers, controlled seek/read/write boundaries, full-source
SHA256/EOF and unchanged encoded-block SHA256. Cancellation or output error never
returns verified staging evidence. This is a quiescent input byte transform;
the existing native retained-original product layer separately provides binding,
publication and recovery guarantees.

An independent Python/FFmpeg fixture check passes NUL values, exact block bytes
and identical decoded PCM. It does not execute Rust. Ten authored stream/native/
decoder cases require actual CI. The separate product append dispatches .wv,
requires exact original replacement length during recovery and authors five
Windows cases for ordinary/multi-value repair, real native preview/replan,
unsupported representation, cancellation before/after backup and restart.
Actual Windows and host/package acceptance remain required.
