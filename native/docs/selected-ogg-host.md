# Consumer Ogg Vorbis/Opus repair checks

The existing source-only Fix flow composes the same source-reviewed staged Ogg
backend. Its public result API, preview authority and cancellation/backup protocol
are unchanged. Scope is bounded ordinary single-logical-stream Vorbis/Opus;
[the stream contract](staged-ogg-repair.md) and [Windows driver contract](windows-ogg-repair.md)
list the refused layouts. No generic Ogg-codec claim is made.

Both desktop and positional CLI consumer gates now require real Vorbis five-field
repair with possible page-wrapper growth, duplicate Opus assignment/shrink,
exact original backup and independent page CRC, encoded-packet and timing checks.
Generated artwork, vendor, suffix bytes and unknown case-sensitive pairs must
remain unchanged. Chained input must keep original content after any permitted
filename-only repair, with an explicit partial result and no false tag success.

Desktop checks additionally retain unchanged files, show them only through the
explicit inactive toggle, reset that persistent preference through the actual
control between cases, and capture ordinary completed/partial results. Preview
and confirmation dismissal compare complete owned source trees including
metadata. No mock IPC or alternate tag writer supplies a success result.

The same mandatory receipts bind the staged and extracted executable/package.
Portable fixture/parser and packaging tests establish only the test contract.
Fresh native, actual Windows and consumer runs are required before this candidate
can be called runtime-verified. Original audio/artwork are owned synthetic assets;
no production library is read or modified.
