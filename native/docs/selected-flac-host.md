# Native FLAC desktop acceptance

This candidate composes the source-reviewed FLAC repair dispatcher with the same
retained-plan source-only desktop and CLI driver used for MP3. No new authority,
file picker, fallback writer or plan mutation is introduced. The five frozen
fields remain artist, albumartist, title, album and genre, in their original order.

The accepted domain is native, offset-zero FLAC with bounded, valid UTF-8 Vorbis
comments. See [the stream contract](staged-flac-repair.md) and
[Windows product contract](windows-selected-repair.md) for all bounds. A duplicate
assignment can shrink the metadata area; the v2 journal binds replacement length
and digest separately from the retained original. Opaque offset-bearing blocks
prevent a size-changing publication. Filename-only effects remain visible as
partial when the tags cannot be repaired safely.

## Required actual consumer checks

`tests/desktop_flac.py` uses the real source picker contract, retained preview,
confirmation, TagLib and executor. Each case owns a fresh temporary source tree.
It neither injects IPC nor mocks inference, metadata or writes. The silent FLAC
media and generated 1x1 artwork are repository-owned fixtures.

Both staged and consumer-extracted EXEs must produce all seven FLAC receipts,
bound to the same executable and package inventory as the other desktop checks:

- Equal-size five-field repair and duplicate-collapse repair that shifts audio.
- Exact original backup, compressed audio, picture, vendor, unknown pair order,
  seek table and cue sheet preservation.
- Opaque private metadata that refuses a size-changing tag write while reporting
  an actual successful filename repair as partial.
- Unchanged FLAC stays byte-identical and is hidden until explicitly requested.
- Preview/dismiss and stale-plan refusal leave complete source snapshots intact,
  including directories, attributes and mtimes rather than only file content.

The independent Python parser checks raw bytes, not a writer-produced expected
receipt. Its portable unit tests validate the fixture and receipt contract only.
They do not establish Rust compilation, physical repair or application rendering.
The prior 690194e Windows checkpoint established MP3 repair only. This FLAC
composition requires fresh native, host and staged/extracted Windows runs before
it can be described as runtime-verified.
