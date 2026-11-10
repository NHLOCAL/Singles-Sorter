# CLI comparison with the original sorter

`check_cli.py` runs the release `singles-sorter` executable and the original
version 14 Python routing code on the same authored audio files and requires
identical decisions. The original code and both models come from the public
baseline commit `78d001aee5ce49628bdffebd34540fa83ba32c12`; nothing is stored here.

The fixtures are silent PCM WAV files generated during the run, with authored
ID3 metadata. Cases cover aliases, the metadata classifier, filename and title
recognition, duet ordering and Unicode titles, each with AI on and off, plus
missing or damaged model bundles.

`.github/workflows/original-ai-cli.yml` runs this check before packaging the
Windows CLI. `test_harness.py` tests the checker itself.
