# envlock

**Encrypt, decrypt, and inject `.env` files with [age](https://age-encryption.org/) encryption.**

`envlock` keeps your secrets out of version control by encrypting `.env` files into `.env.vault` artifacts that are safe to commit. Decrypted plaintext never touches disk when you use `envlock run` — secrets are injected directly into your subprocess's environment.

## Install

```bash
cargo install envlock
```

Or download a prebuilt binary from [GitHub Releases](https://github.com/nicholasgasior/envlock/releases).

## Quick start

```bash
envlock init          # generate keypair, update .gitignore
envlock encrypt       # .env → .env.vault
git add .env.vault    # safe to commit
git commit -m "add encrypted env"
envlock run -- python app.py   # inject secrets, no .env on disk
```

## Why not SOPS / git-crypt / dotenv-vault?

| Tool | Difference |
|---|---|
| **SOPS** | General-purpose structured encryption (YAML, JSON, etc.). Heavier dependency, more complexity. envlock is single-purpose: `.env` files only, zero config. |
| **git-crypt** | Transparent encryption at the git level. Requires GPG, harder to rotate access per-person. envlock uses age keys and supports multi-recipient out of the box. |
| **dotenv-vault** | Node.js ecosystem, requires an account on their service. envlock is a standalone Rust binary — no server, no account, no telemetry. |

If you need structured file encryption or integration with cloud KMS, use SOPS. If you want the simplest possible `.env` encryption with no infrastructure, use envlock.

## Commands

### `envlock init`

- Generates an age x25519 keypair
- Writes private key to `.envlock/identity.txt` (mode 0600 on Unix)
- Writes public key (recipient) to `.envlock/recipients.txt`
- Auto-appends `.env` and `.envlock/identity.txt` to `.gitignore`

### `envlock encrypt [path]`

Encrypts a plaintext file (default `.env`) to `<path>.vault` using the recipients in `.envlock/recipients.txt`.

### `envlock decrypt [path]`

Decrypts a vault file (default `.env.vault`) using the identity in `.envlock/identity.txt` and writes the plaintext, stripping the `.vault` suffix. Output file permissions are set to 0600.

### `envlock run -- <command...>`

Decrypts `.env.vault` **in memory only**, parses key-value pairs, and spawns the given command with those variables injected into its environment. No `.env` file is ever written to disk. Stdout/stderr stream through; the subprocess exit code is forwarded.

**Vault values override existing environment variables.** This is intentional: the vault is the source of truth for secrets. If you need the parent env to win, set the variable after `envlock run` in your shell.

### `envlock diff <vault-a> <vault-b>`

Compares two vault files by decrypting both in memory and diffing their keys. Values are **redacted** — only key names and change type are shown.

Output format:
- `+ KEY` — only in vault-b (added)
- `- KEY` — only in vault-a (removed)
- `~ KEY (value differs)` — present in both but different values

### `envlock rotate KEY_NAME [--value <new-value>|--prompt]`

Decrypts the vault, replaces a single key's value, and re-encrypts with the same recipients.

- `--value <val>` — set the new value directly
- `--prompt` — read from stdin without echoing (secrets stay out of shell history)

### `envlock recipients add <public-key>`

Adds an age public key (age1...) to `.envlock/recipients.txt`. Validates the key before writing. Rejects duplicates.

### `envlock recipients list`

Prints all recipients from `.envlock/recipients.txt`.

### `envlock recipients remove <public-key>`

Removes a recipient. Refuses to remove the last one — at least one is required so vaults remain decryptable.

### `envlock check`

Scans `.gitignore` and `git status` to verify that secrets won't be accidentally committed. Warns if `.env` or `.envlock/identity.txt` would be trackable. Exits non-zero if problems are found.

### `envlock completions <shell>`

Generates shell completions to stdout. Supported shells: `bash`, `zsh`, `fish`, `powershell`.

```bash
# Bash
envlock completions bash > /etc/bash_completion.d/envlock

# Zsh
envlock completions zsh > ~/.zfunc/_envlock

# Fish
envlock completions fish > ~/.config/fish/completions/envlock.fish

# PowerShell
envlock completions powershell > _envlock.ps1
```

## File layout (per-project)

```
.envlock/
  identity.txt      # age private key — NEVER committed
  recipients.txt    # age public keys (one per line)
.env.vault           # encrypted, safe to commit
.env                 # decrypted plaintext, gitignored
.gitignore           # auto-managed by envlock
```

## Team sharing workflow

```bash
# Alice generates a keypair and commits .env.vault + .envlock/recipients.txt
envlock init
envlock encrypt

# Bob generates his own keypair in the same repo
envlock init  # creates his own .envlock/identity.txt (gitignored)

# Bob shares his public key with Alice
cat .envlock/recipients.txt
# → age1bob...

# Alice adds Bob's key and re-encrypts
envlock recipients add age1bob...
envlock encrypt

# Now both Alice and Bob can decrypt
envlock decrypt
envlock run -- python app.py
```

## Threat model

### What envlock protects against

- **Accidental commits** of `.env` files containing secrets — `.gitignore` is auto-managed and `envlock check` verifies it
- **Plaintext-at-rest** on disk — `envlock run` decrypts in memory only; `.env` is gitignored and 0600-permissioned
- **Insider access creep** — `envlock recipients remove` + re-encrypt revokes future access
- **Shell history leaks** — `envlock rotate --prompt` reads secrets without echoing

### What envlock does NOT protect against

- **Compromised machines** — if an attacker has access to `.envlock/identity.txt`, they can decrypt any vault encrypted to that key. Protect your private key.
- **Runtime memory attacks** — `envlock run` holds decrypted secrets in process memory. A debugger or `ptrace` can read them. This is the same threat model as any tool that loads `.env` files.
- **Git history exposure** — if you commit `.env` before running `envlock init`, the secret exists in git history. Use `git filter-repo` to scrub it.
- **Vault file tampering** — age provides authenticity, but envlock does not audit access logs. If you need audit trails, use a dedicated secrets manager.

envlock is a **local secret hygiene tool**, not a secrets manager. For production infrastructure, pair it with a proper secrets management solution.

## Security notes

- Private keys live in `.envlock/identity.txt` with 0600 permissions and are automatically added to `.gitignore`.
- `envlock decrypt` writes `.env` with 0600 permissions.
- `envlock run` never writes decrypted content to disk — secrets exist only in the subprocess's memory.
- `envlock diff` redacts values — only key names and change types are shown.
- Decryption failures produce a generic "Decryption failed" message to avoid oracle-style information leaks.
- `envlock rotate --prompt` reads secrets from stdin without echoing, keeping them out of shell history.
- `envlock check` provides a safety net after cloning repos that use envlock.

## Dependencies

| Crate | Purpose |
|---|---|
| [age](https://crates.io/crates/age) | File encryption |
| [clap](https://crates.io/crates/clap) | CLI argument parsing |
| [clap_complete](https://crates.io/crates/clap_complete) | Shell completions |
| [dotenvy](https://crates.io/crates/dotenvy) | `.env` file parsing |
| [anyhow](https://crates.io/crates/anyhow) | Error handling |
| [dirs](https://crates.io/crates/dirs) | Cross-platform config paths |
| [rpassword](https://crates.io/crates/rpassword) | Secure terminal input |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
