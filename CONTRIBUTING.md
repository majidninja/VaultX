# Contributing to VaultX

Thanks for helping make VaultX better! Bug reports, docs fixes, tests and new features are all welcome.

## Ways to help

- **Pick an issue.** Issues labelled [`good first issue`](https://github.com/skdas20/VaultX/labels/good%20first%20issue) are small and well scoped; [`help wanted`](https://github.com/skdas20/VaultX/labels/help%20wanted) ones are bigger. Comment on an issue before you start, so two people don't work on the same thing.
- **Report a bug** with the [bug report template](https://github.com/skdas20/VaultX/issues/new/choose). Include your OS, how you installed `vx`, and `vx --version`.
- **Suggest a feature** with the feature request template. Explain the problem first, then your idea.
- **Security issues** must not be filed as public issues. See [SECURITY.md](SECURITY.md).

## Development setup

You need a recent stable [Rust toolchain](https://rustup.rs).

```bash
git clone https://github.com/<your-username>/VaultX.git
cd VaultX
cargo build                # builds vx-core and the vx binary
cargo test --workspace     # unit and property-based tests
cargo run -p vx-cli -- --help
```

> **Tip:** `vx` stores its vault in `~/.vaultx/`. While developing, point `HOME` at a scratch directory so you don't touch your real vault:
> ```bash
> HOME=$(mktemp -d) cargo run -p vx-cli -- init demo
> ```

## Project layout

| Path | What lives there |
| --- | --- |
| `vx-core/` | Crypto, vault model, TTL and SSH key logic. No I/O, no CLI code; compiles to WASM. |
| `vx-core/tests/` | Property-based tests (`proptest`) |
| `vx-cli/src/commands/` | One file per `vx` subcommand |
| `vx-cli/src/storage.rs` | Reading and writing `~/.vaultx/vault.vx` |
| `npm/` | The npm wrapper that downloads release binaries |
| `.github/workflows/` | CI and the release build matrix |

## Pull request checklist

- [ ] One focused change per PR, linked to an issue (`Fixes #123`).
- [ ] `cargo fmt --all` and `cargo clippy --workspace` produce no new warnings.
- [ ] `cargo test --workspace` passes, and new behaviour has a test.
- [ ] User-visible changes are reflected in `README.md`.
- [ ] Crypto or storage changes explain the security reasoning in the PR description.

## Commit messages

Use short, imperative subjects, optionally with a [Conventional Commits](https://www.conventionalcommits.org) prefix:

```
feat(cli): add `vx run` to inject secrets into a child process
fix(storage): write the vault atomically on Windows
docs: document TTL formats
```

## Ground rules for security-sensitive code

- Never log, print or put a secret value in an error message.
- Never accept a secret as a command-line argument (it ends up in shell history and `ps`).
- Prefer well-reviewed crates (RustCrypto) over hand-rolled cryptography.
- Changes to the vault file format must stay backward compatible, or bump the format version and migrate.

## Code of conduct

By taking part you agree to follow our [Code of Conduct](CODE_OF_CONDUCT.md).
