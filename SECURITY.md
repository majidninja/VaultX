# Security Policy

VaultX stores secrets, so security reports are taken seriously and handled first.

## Reporting a vulnerability

**Please do not open a public issue for security problems.**

Report privately through [GitHub Security Advisories](https://github.com/skdas20/VaultX/security/advisories/new). Include:

- the affected version (`vx --version`) and platform,
- steps to reproduce or a proof of concept,
- the impact you expect (e.g. secret disclosure, privilege escalation).

You can expect an acknowledgement within 72 hours and a fix or mitigation plan within 14 days for confirmed issues. Reporters are credited in the release notes unless they ask otherwise.

## Supported versions

Only the latest release receives security fixes. Upgrade with `vx update` or `npm install -g @vaultx-official/vaultx@latest`.

## Threat model

VaultX is designed to keep secrets safe **at rest**: if someone copies `~/.vaultx/vault.vx`, they get only ciphertext protected by Argon2id and AES-256-GCM.

Out of scope: malware already running as your user, keyloggers, a compromised OS, and weak master passwords.

## Known limitations

These are tracked publicly and contributions are welcome:

- **Session cache (`vx login`).** The cached master password is stored in the system temp directory. The file is encrypted, but the key needed to decrypt it is kept alongside it, so the cache should be treated as plaintext-equivalent. Don't use `vx login` on shared or multi-user machines. The planned fix is OS keychain integration.
- **Self-update.** `vx update` downloads the latest release from GitHub over HTTPS but does not yet verify it against `checksums.txt`.
- **Memory hygiene.** Decrypted secrets are not yet zeroised in memory after use.
