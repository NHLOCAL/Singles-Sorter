# Native Singles Sorter: Windows, Android and CLI

This Rust workspace is Singles Sorter 15. It replaces the Python/Flet
application while keeping the original sorting rules and the original local
recognition models. Product identity is pinned in [`release.json`](../release.json).

## Layout

- `crates/sorter-core`: sorting policy, request validation and planning. No I/O.
- `crates/sorter-io`: CSV resources, storage access and the TagLib metadata bridge.
- `crates/sorter-recognition`, `crates/sorter-classifier`: the original local AI models.
- `crates/sorter-executor`: journaled Copy, Move, album and tag-repair execution.
- `crates/sorter-cli`: the `singles-sorter` command line. See [CLI usage](docs/cli.md).
- `apps/singles-ui`: the shared Hebrew interface and the Windows desktop host.
- `apps/singles-android`: the Android host. See its [signing and update guide](apps/singles-android/docs/signing-and-v15-updates.md).
- `plugins/`: the native bridges used by the desktop and Android hosts.
- `packaging/`: Windows packages, the installer and third-party license collection.
- `probes/released-ner`: the [recognition engine](probes/released-ner/README.md) crates.
- `tests/`: shared fixtures and the CLI comparison with the original sorter.

## Build and verify

Install Rust 1.99.0 from the official Rust distribution. Cargo dependencies
come from crates.io and their exact versions/checksums are in Cargo.lock.
Install CMake 3.24+ and a C++17 toolchain for the metadata bridge. Its two native
source dependencies are automatically fetched and verified at build time; see
[native dependency cache/offline instructions](crates/sorter-io/dependencies/README.md).
Workspace tests additionally require the verified static PCRE2 archive described
in [the recognition engine notes](probes/released-ner/README.md), even when the
optional CLI feature is not selected. From this directory:

```sh
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo build --locked --release -p sorter-cli
```

`rust-toolchain.toml` pins Rust 1.99.0, edition 2024, rustfmt and clippy.
For a command run from the repository root, select the toolchain explicitly
with `cargo +1.99.0 ... --manifest-path native/Cargo.toml`.

## Continuous integration and release

`native-ci.yml` formats, lints and tests every pull request on Linux and
Windows. `release-v15.yml` builds, tests and packages the desktop application,
the CLI and the signed APK from one commit, then drafts the GitHub release.
See [RELEASING.md](../RELEASING.md).
