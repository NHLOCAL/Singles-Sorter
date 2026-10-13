# Original checkout-byte preservation repair

Second hosted run: https://github.com/NHLOCAL/Singles-Sorter/actions/runs/38117708482

Linux passed the complete proof. Windows installed Python 3.12.10 and all 43
locked oracle packages and passed imports, then correctly rejected a changed
`config.cfg` hash before native inference. No Windows parity is claimed.

## Proven conversion mechanism

Original commit `78d001aee5ce49628bdffebd34540fa83ba32c12` has root
`.gitattributes` containing `* text=auto`, without an explicit eol attribute.
The workflow set `core.autocrlf=false`, but did not set `core.eol`. Git documents
that text/auto attributes enable checkout conversion, and native Windows line
endings can remain CRLF despite disabling automatic text detection.

Using the real original index/blob, an isolated `checkout-index` reproduction
with `core.autocrlf=false` produced:

- Original blob: 2,266 bytes, 129 LF, zero CRLF;
  SHA256 `582dc457c5832ac03f3f03a44fbe361c1f90f52312c28396e34cd85d807ffea8`.
- Explicit Windows-style `core.eol=crlf`: 2,395 bytes, 129 CRLF;
  SHA256 `235d882ca14164e630660c8ea707453408a27410c9f6086b4d7aa4d1e2ddb6f2`.
- Explicit `core.eol=lf`: exactly the original 2,266 bytes and pinned SHA256.

The failed Windows log did not include its actual config hash or byte counts;
the CRLF hash above is the controlled reproduction, not an invented hosted
measurement. The original source checkout was not modified by these checks.

## Narrow correction and unchanged gates

Set `core.eol=lf` alongside `core.autocrlf=false` **before either checkout**.
The pinned checkout action copies the global config into its temporary HOME,
as confirmed in the actual log and its source. This preserves Git's original
blob bytes instead of modifying the original model or normalizing bytes inside
the exporter. The original attributes, model, all source/tensor hashes, exporter,
golden references and inference code remain unchanged.

Two tiny owned-synthetic Git-index tests reproduce text conversion and verify
that explicit LF preserves both text and binary bytes. They run before export.
The workflow also logs original model index/worktree EOL status and the actual
config length/SHA256, so the next hosted result exposes the byte evidence.
The same strict exporter and all original/native parity gates must still pass.

Official sources:
- https://git-scm.com/docs/gitattributes (text/auto and eol conversion)
- https://git-scm.com/docs/git-config#Documentation/git-config.txt-coreeol
- https://github.com/actions/checkout/blob/3d3c42e5aac5ba805825da76410c181273ba90b1/src/git-auth-helper.ts
