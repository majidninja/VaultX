<div align="center">

# VaultX (`vx`)

**A local-first, encrypted secrets vault and SSH identity manager for your terminal.**

No server. No cloud account. No GUI. One encrypted file on your machine, written in Rust.

[![npm version](https://img.shields.io/npm/v/@vaultx-official/vaultx?color=cb3837&logo=npm)](https://www.npmjs.com/package/@vaultx-official/vaultx)
[![npm downloads](https://img.shields.io/npm/dy/@vaultx-official/vaultx?label=downloads%2Fyear)](https://www.npmjs.com/package/@vaultx-official/vaultx)
[![GitHub release](https://img.shields.io/github/v/release/skdas20/VaultX?logo=github)](https://github.com/skdas20/VaultX/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange?logo=rust)](https://www.rust-lang.org)
[![PRs welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

[Install](#install) · [Quick start](#quick-start) · [Commands](#commands) · [Security model](#security-model) · [Roadmap](#roadmap) · [Contributing](#contributing)

</div>

---

## Why VaultX?

`.env` files get committed, shared over chat and left in plaintext on laptops. Hosted secret managers fix that, but they need an account, a network connection and trust in a third party.

VaultX sits in between. It's a single binary that keeps your project secrets and SSH keys in **one encrypted file in your home directory**, unlocked with a master password that never leaves your machine.

```console
$ vx init payments-api
$ vx add payments-api STRIPE_SECRET_KEY          # prompted, never echoed, never in shell history
$ vx add payments-api TEMP_DEPLOY_TOKEN --ttl 6h  # expires automatically
$ vx get payments-api STRIPE_SECRET_KEY
$ vx audit                                        # flags expired, long-lived and risky secrets
```

## Features

- **Encrypted, project-based vault**: secrets are grouped by project and stored in `~/.vaultx/vault.vx`, encrypted with AES-256-GCM.
- **Strong key derivation**: your master password is stretched with Argon2id (64 MB memory, 3 iterations).
- **Secrets never touch your shell history**: values come from an interactive prompt, a file (`--file`) or an environment variable (`--env`), never from a command-line argument.
- **Expiring secrets**: give any secret a TTL (`6h`, `7d`, `2w`) for short-lived tokens.
- **Security audit**: `vx audit` reports expired, long-lived and high-risk secrets.
- **SSH identity manager**: generate ed25519 keys inside the vault, save server profiles, then `vx ssh <server>` or `vx scp`. Temporary key files are deleted after each use.
- **Small, fast, cross-platform**: a native binary of about 3 MB for Linux, macOS (Intel and Apple Silicon) and Windows.
- **Portable crypto core**: `vx-core` is plain Rust and also compiles to WebAssembly.

## Install

### npm (Linux, macOS, Windows)

```bash
npm install -g @vaultx-official/vaultx
vx --version
```

The package downloads the right prebuilt binary for your OS and CPU when it installs.

### Prebuilt binaries

Download from the [latest release](https://github.com/skdas20/VaultX/releases/latest), and check them against `checksums.txt`:

| Platform | File |
| --- | --- |
| Linux x86_64 | `vx-x86_64-unknown-linux-gnu` |
| macOS Apple Silicon | `vx-aarch64-apple-darwin` |
| macOS Intel | `vx-x86_64-apple-darwin` |
| Windows x86_64 | `vx-x86_64-pc-windows-msvc.exe` |

On Linux and macOS, make the file executable and put it on your `PATH`:

```bash
chmod +x vx-x86_64-unknown-linux-gnu
sudo mv vx-x86_64-unknown-linux-gnu /usr/local/bin/vx
```

### Build from source

Requires a recent stable [Rust toolchain](https://rustup.rs).

```bash
git clone https://github.com/skdas20/VaultX.git
cd VaultX
cargo build --release
./target/release/vx --version
```

See [BUILDING.md](BUILDING.md) for platform-specific notes.

## Quick start

```bash
# 1. Create a project (the first run also creates the vault and asks for a master password)
vx init my-app

# 2. Add secrets: interactively, from a file, or from an environment variable
vx add my-app DATABASE_URL
vx add my-app TLS_KEY --file ./server.key
vx add my-app GITHUB_TOKEN --env GITHUB_TOKEN --ttl 7d

# 3. Read them back
vx get my-app DATABASE_URL
vx get my-app                 # every secret in the project

# 4. Keep the vault healthy
vx audit
```

## Commands

| Command | What it does |
| --- | --- |
| `vx init <project>` | Create a project (and the vault, on first use) |
| `vx add <project> [key] [--file F \| --env VAR] [--ttl T]` | Add a secret; prompts for the value by default |
| `vx get <project> [key]` | Print one secret, or all secrets in a project |
| `vx edit <project> <key>` | Change a secret's value |
| `vx remove <project> [key]` | Remove a secret, or the whole project |
| `vx list` | List projects |
| `vx secrets <project>` | List secret names in a project (values hidden) |
| `vx audit` | Report expired, long-lived and risky secrets |
| `vx login` | Cache the master password for the current terminal session |
| `vx ssh init <name>` | Generate an ed25519 SSH identity inside the vault |
| `vx ssh add <name> <user@host>` | Save a server profile |
| `vx ssh <server>` / `vx ssh connect <server>` | Connect using the stored identity (extra args go to `ssh`) |
| `vx ssh list` / `vx ssh remove <name>` | Manage identities and servers |
| `vx scp <server> <src> <dest>` | Copy files; prefix remote paths with `:` |
| `vx update [-y]` | Update `vx` to the latest GitHub release |

Run `vx <command> --help` for every option.

## Security model

| Layer | Choice |
| --- | --- |
| Key derivation | Argon2id: 64 MB memory, 3 iterations, 32-byte random salt per vault |
| Encryption | AES-256-GCM with a fresh random 96-bit nonce for every write |
| SSH keys | ed25519, generated in-process; uses your system `ssh` and `scp` |
| Storage | Single file: `~/.vaultx/vault.vx` |
| Network | None, except `vx update` and the npm installer downloading release binaries |

The vault file layout:

```
+----------------+------------------+-----------------------+
| Header (16 B)  | Salt (32 B)      | Encrypted payload     |
| magic "VX01",  | Argon2id salt    | AES-256-GCM over the  |
| version (u32)  |                  | JSON vault data       |
+----------------+------------------+-----------------------+
```

**What VaultX protects against:** a stolen laptop disk, a leaked backup or a copied `vault.vx`. Without the master password, the file is just ciphertext.

**What it doesn't protect against:** malware running as your user while the vault is unlocked, or a weak master password.

**Known limitations** (contributions welcome, see [SECURITY.md](SECURITY.md)):

- The `vx login` session cache is a convenience, not a security boundary. Avoid it on shared machines.
- `vx update` does not yet verify `checksums.txt` before replacing the binary.
- Decrypted secrets are not yet wiped from memory after use.

To report a vulnerability privately, see [SECURITY.md](SECURITY.md).

## Architecture

```
vx (vx-cli)                   command parsing, prompts, vault file I/O, ssh/scp
  └── vx-core (Rust, WASM-ready)
        ├── crypto.rs         Argon2id key derivation, AES-256-GCM
        ├── vault.rs          projects, secrets, serialisation
        ├── ttl.rs            expiry parsing and checks
        └── ssh.rs            ed25519 identity generation
```

More detail is in [docs/architecture.md](docs/architecture.md). `vx-core` has property-based tests (`proptest`) for the crypto, vault, TTL, SSH and audit logic:

```bash
cargo test --workspace
```

## Design principles

- Never store a secret in plaintext, and never accept one as a command-line argument.
- Reuse system tools (`ssh`, `scp`) instead of reimplementing them.
- Local-first: no account, no server, no telemetry.
- Security before convenience; composability before features.

## Roadmap

- [ ] `vx run <project> -- <command>`: inject secrets as environment variables into a child process
- [ ] `vx export` / `vx import` for `.env` files
- [ ] OS keychain integration for `vx login` (macOS Keychain, Windows Credential Manager, Secret Service)
- [ ] Verify release checksums in `vx update`
- [ ] Zero decrypted secrets in memory (`zeroize`)
- [ ] Linux ARM64 binaries
- [ ] Shell completions (bash, zsh, fish, PowerShell)
- [ ] Homebrew, Scoop and `cargo install` distribution

Have an idea? [Open a feature request](https://github.com/skdas20/VaultX/issues/new/choose).

## Contributing

Contributions of every size are welcome, from typo fixes to new commands. Start with an issue labelled [`good first issue`](https://github.com/skdas20/VaultX/labels/good%20first%20issue), and read [CONTRIBUTING.md](CONTRIBUTING.md) for setup, tests and the PR checklist.

If VaultX is useful to you, **a star helps other developers find it**. ⭐

## License

[MIT](LICENSE) © Sumit Kumar Das
