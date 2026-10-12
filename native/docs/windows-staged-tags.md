# Staged MP3 repair coverage

The byte transformer is separate from the read-only metadata ABI. It accepts only
unflagged ID3v2.3/v2.4 tags over an MP3 audio stream, the released five-field repair
order and exact original projected values. Unknown frames, artwork and the full
media tail are copied byte-for-byte. Repaired text retains UTF-16 endianness or
UTF-8 representation. Latin-1 repaired to Hebrew uses Unicode within the same
ID3 major version. The original tag size and audio offset remain fixed: Unicode
growth consumes existing zero padding; insufficient padding is rejected.

Unsynchronization, extended headers, footer flags, per-frame flags, invalid sizes,
nonzero padding, duplicate repaired frames, ID3v1/APE mirrors and unsupported text
representations reject before staging. Unchanged fields are a byte-preserving
no-op; physical write counts must remain distinct from released processed-file
counts. The transformer hashes the entire original, staged result and unchanged
media tail, checks exact source length and supports bounded-memory cancellation.

This module accepts streams only. It cannot open or publish a path, delete a
source or clean up a backup. Native publication must retain the original by an
identity-bound no-replace backup rename before installing the verified staged
file, with journal-before-effects and read-only recovery. The selected Windows driver supplies that native integration. The transformer
alone cannot enable product Fix or acquire filesystem authority.

Native FLAC uses the separate sealed FLAC transformer through shared product
dispatch. Other formats remain unsupported for physical tag repair. Local compilation and
native metadata tests are unrun until exact-source CI, due to unavailable tooling.
