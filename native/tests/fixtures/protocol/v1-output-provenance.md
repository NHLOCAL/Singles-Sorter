# Frozen v1 CLI stdout

The three `*.v1-output.json` files were captured on 2026-10-11 from the
unchanged supplied-facts CLI binary before adding the real-file v2 command. Each command consumed the adjacent
same-stem request JSON. `classify-alias` and `classify-fail-open` use
`classify --json`; `plan-collisions` uses `plan --json`. Capture included the
single trailing newline. This evidence pins **transport bytes**, separate from
release-source policy goldens. The v1 transport regression compares bytes,
not reordered JSON values.

SHA256:

- classify-alias.v1-output.json: `2b6e59265712216f56f75c19826ddfa89cecb73c66201b81afcc31bda17b4a3e`
- classify-fail-open.v1-output.json: `ded93c9505b0da71388dd00d0c8ddab5664c6dfdb734a1b3c224ced6c099f135`
- plan-collisions.v1-output.json: `e20aa2579466829a6b08bb401ebc1f27f727f33fab54b9f2703a4c6582691e1c`

No legacy release data, oracle output, models, or music files were changed.
