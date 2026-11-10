# Ordinary ID3 read-for-Sort compatibility

This narrow correction retains the existing TagLib reader and strict fresh-input
binding. It extends bounded ID3 preflight for ordinary fixed-size ID3v1 fallback, unflagged v2.2, v2.2/v2.3 tag-level
unsynchronisation and the minimal no-CRC v2.3 extended header. Frame extents,
component/byte budgets, raw-byte snapshots and structured-frame refusals remain.
No metadata writer, sorting rule, collision rule or global guard is relaxed.

The owned fixture helper uses the frozen synthetic tagged.mp3 audio payload.
Its UTF-16 BOMs exercise actual FF-byte insertion/removal, not only an unsync flag.
Independent music-tag 0.4.3 / Mutagen 1.47.0 observations verified the supported
fixture values against the frozen v14 implementation. The combined v2.3 unsync
plus extended-header fixture has different original values and remains refused.
Genre's preexisting parenthesized numeric expansion gap is outside this Sort
correction; these tests do not establish universal six-field repair parity.

A verified MP3 shorter than four bytes cannot contain a MPEG frame header. After
successful bounded reads and the existing ASF/ADTS classifications, that exact
class becomes the original ordinary open failure, preserving filename fallback.
Other parser failures, unsupported layouts, storage/identity errors, budgets and
native/platform failures keep their existing classification and guards.

The mixed library contains 103 real files: two 40-track same-target album
candidates and 23 loose singles, including one empty filename-recognizable MP3 and one metadata-only ID3v1 file.
The actual frozen Python Copy and Move oracle gives 23 songs, one album, zero
new artist-folder credits, 63 destinations and respectively 103/40 retained
source files. Native Windows tests bind full destination byte maps and retained
source paths to the actual plan's selected album, without assuming enumeration
order. A changed source after preview must refuse before any destination or
journal effects. A separate three-file album vote retains the unreadable file
in its denominator, so two good files do not satisfy the original 70% rule.

Required new gates: sixteen actual native-reader/production-provider cases and three
actual Windows execution cases. Local cross-target checks establish compilation,
not Windows execution. User-library representation coverage requires actual
per-file diagnostics; these owned fixtures do not identify an unseen user's tag.

ID3v1 uses the pinned original last-131-byte first-TAG discovery boundary and
refuses ambiguous APE/competing/truncated representations. Its standard 128-byte
payload is captured before TagLib validates MPEG. Text is NUL-first with ASCII
byte stripping, not TagLib Unicode whitespace normalization. The original track
padding rule, all 256 genre-byte outcomes and missing-versus-present-empty v2
precedence are tested. ID3v1 does not acquire any tag-writing capability.
