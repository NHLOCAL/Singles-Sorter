# Exact-extent MP4 metadata repair

This stream transformer repairs the released five fields in ordinary MP4
AAC/ALAC audio (`m4a` / `alac` product extensions). It has no filesystem authority.
The separate product composition selects m4a/alac, retains the original object,
and binds replacement length exactly equal to original length during recovery.
Ten Windows product/restart cases are authored; package acceptance remains
required. Twenty Rust/native-reader/FFmpeg cases are authored and unrun locally.

## Why a bounded metadata patch

The product already pins TagLib 2.3.2. Its stock MP4 writer does not satisfy the
opaque-item preservation contract: a non-text opaque item that fails
`ItemFactory::parseItem` is invalid, the tag constructor retains only valid
items, and save rebuilds the item list. Valid unknown UTF-8 text names do have
a generic Text fallback; the finding does not apply to all unknown names. This is a source finding, not a local runtime measurement.
A general TagLib save would therefore require custom preservation logic anyway.
The existing read-only bridge and its ABI remain unchanged.

- [Pinned item factory](https://github.com/taglib/taglib/blob/v2.3.2/taglib/mp4/mp4itemfactory.cpp#L92-L127)
- [Unknown-name text fallback](https://github.com/taglib/taglib/blob/v2.3.2/taglib/mp4/mp4itemfactory.cpp#L336-L346)
- [Pinned tag loading and saving](https://github.com/taglib/taglib/blob/v2.3.2/taglib/mp4/mp4tag.cpp#L56-L129)

## Exact-length proof and semantics

Frozen sorter blob `0b67726e72e527556f47a8eeffeb44778c62809b`, line 215, defines
artist, albumartist, title, album, genre in that order. The pinned music-tag MP4
mapping uses `©ART`, `aART`, `©nam`, `©alb`, `©gen`. Each scalar is the ordered
comma-space join of the original values; assignment replaces that field with
one text value. The transformer checks the existing shared original projection,
field order and exact frozen Hebrew conversion before accepting a repair.

All 27 convertible source characters and their Hebrew replacements occupy two
UTF-8 bytes. Other characters are unchanged. Joining k values adds 2(k−1) bytes;
removing k−1 data headers releases 16(k−1) bytes. Duplicate outer fields are
aggregated by the bounded raw preview correction and consolidated by the writer.
With r total outer items using an eight-byte header baseline and n total data children, the released
space is 8(r−1) + 14(n−1). Every extended outer header adds eight more released
bytes. An assigned scalar needs at least one data child. The admitted rewrite
therefore has either zero slack or at least eight bytes, enough for a valid
`free` atom.
The implementation still checks the exact calculated size and refuses growth or
1–7-byte slack. It does not rely on a guessed output-size allowance.

The smaller `ilst`, followed by a new adjacent `free` atom, occupies exactly its
previous extent. `moov` length, top-level order, fast-start layout, chunk offsets,
encoded packets and every non-metadata byte remain unchanged. Item positions
inside the resized ilst can change; private offset/authentication semantics in
opaque metadata are not claimed. Unknown `ilst`
atoms and artwork are copied verbatim. No duplicate movie/artwork is appended.

- [Frozen conversion](https://github.com/NHLOCAL/Singles-Sorter/blob/78d001aee5ce49628bdffebd34540fa83ba32c12/src/core/singles_sorter_v5.py#L164-L169)
- [Frozen five-field order](https://github.com/NHLOCAL/Singles-Sorter/blob/78d001aee5ce49628bdffebd34540fa83ba32c12/src/core/singles_sorter_v5.py#L215)
- [Inspected music-tag mapping](https://github.com/KristoforMaynard/music-tag/blob/43389a429a423392221e0786af486974be3c4411/music_tag/mp4.py)
- [Inspected Mutagen MP4 text semantics](https://github.com/quodlibet/mutagen/blob/release-1.47.0/mutagen/mp4/__init__.py#L736-L775)

Historical packaged dependency wheel versions remain unverified; these are exact
inspected source versions, not a claim about hidden installer contents.

## Supported boundary

Only implicit/explicit UTF-8 iTunes text data, one or more ordered atoms per repaired field, one ordinary movie, local
unencrypted AAC/ALAC sample descriptions and the documented structural allowlist
are admitted. UTF-16/other text types, fragmented/encrypted/external-reference
files, private/integrity structural atoms, sample groups other than roll/prol, numeric genre mirrors when repairing
genre, unusual sample descriptions and malformed/budget-exceeding atoms refuse
before output. No generic metadata insertion or arbitrary text assignment is
introduced. Filename-only repair remains a separate original-policy operation.

Inspection bounds movie metadata to 16 MiB, atoms to 65,536 per list and structural
walk, text values to 4,096 and structural depth to 12. Each track requires its exact local data reference,
sample description, stsc/stsz/stco or co64 and stts tables. Counts, description
indices, arithmetic and every complete chunk extent are checked against local
mdat payloads. Compact stz2 is explicitly refused in this first slice. The total
sample budget is one million. Streaming uses 64 KiB buffers,
checks cancellation around every I/O chunk, binds the whole original SHA256 and
EOF, and reports unchanged encoded-media SHA256 plus exact replacement length.
An output error/cancel returns no verified staging evidence. The Windows driver
must retain the complete original object and its established preservation checks.

Independent Python/FFmpeg fixture checks currently pass three cases (both AAC
and ALAC, fast-start, and multi-value padding); these do not execute the Rust
writer. Actual native-reader and Rust/Windows product gates remain required.

The duplicate-field representation check covers all five keys when a writer is inspected,
including fields without a requested assignment. The read-only metadata adapter
now independently scans the already-open MP4 handle (bounded 16 MiB movie,
65,536 top-level atoms, 4,096 values per field). For duplicate occurrences of the
five text keys it concatenates UTF-8 data values in original atom order before
the existing music-tag scalar projection. Thus a later duplicate that alone
needs repair cannot be hidden by TagLib's first-item map. Unsupported duplicate
text encodings return an explicit field error. Other native fields retain their
existing projection. Physical repair now consolidates supported duplicate outer items at the first
original occurrence, using the exact joined scalar. Removing each extra outer
header frees another eight bytes on top of the fourteen bytes released per
merged data child. For an assigned nonempty scalar the released space is 8(r−1) + 14(n−1),
where r is the number of all original outer items using an eight-byte baseline and n their total
data children; each extended outer header releases eight additional bytes.
Thus every nonzero remainder is at least eight bytes,
sufficient for a valid free atom; unknown non-target atoms remain byte-identical.
All duplicate target representations are validated before output, including
unchanged fields. Unsupported encodings remain explicit partial/refusal.
The preview correction is read-only and shared by platform metadata consumers; native
acceptance requires the new later-value-only and unsupported-encoding fixtures.
