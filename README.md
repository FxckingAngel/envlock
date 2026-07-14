# envlock

**Encrypt, decrypt, and inject `.env` files with [age](https://age-encryption.org/) encryption.**

`envlock` keeps your secrets out of version control by encrypting `.env` files into `.env.vault` artifacts that are safe to commit. Decrypted plaintext never touches disk when you use `envlock run` — secrets are injected directly into your subprocess's environment.

## Install

```bash
cargo install envlock
```

Or download a prebuilt binary from [GitHub Releases](https://github.com/FxckingAngel/envlock/releases).

## Quick start

```bash
envlock doctor       # first: verify your setup is safe (run this after git clone)
envlock init          # generate keypair, update .gitignore, install pre-commit hook
envlock encrypt       # .env → .env.vault
git add .env.vault    # safe to commit
git commit -m "add encrypted env"
envlock run -- python app.py   # inject secrets, no .env on disk
```

## Scope — what this actually is

envlock is a small, single-purpose tool for one problem: keeping `.env` files out of plaintext and out of git history for solo developers and small teams who don't have (or don't want) cloud infrastructure for secrets. It is **not** a replacement for a cloud KMS, a password manager, or your CI/CD provider's built-in secrets manager — if you have access to AWS Secrets Manager, GitHub Actions Secrets, or Bitwarden, those are more capable and better suited for anything beyond local `.env` hygiene. envlock's honest niche is: free, no account, no cloud dependency, works fully offline.

## Why not SOPS / git-crypt / dotenv-vault?

| Tool | Difference |
|---|---|
| **SOPS** | General-purpose structured encryption (YAML, JSON, Vault/KMS backends, key groups). envlock has a narrower scope — `.env` files only, no cloud KMS integration — which keeps it simpler to reason about, but it is **not** "zero config": you still manage `.envlock/identity.txt` and `.envlock/recipients.txt`, the same category of setup SOPS requires for its own key config. If you need structured file support or KMS integration, SOPS is the stronger tool. |
| **git-crypt** | Transparent encryption at the git level. Requires GPG, harder to rotate access per-person. envlock uses age keys and supports multi-recipient out of the box. |
| **dotenv-vault** | Node.js ecosystem, requires an account on their service. envlock is a standalone Rust binary — no server, no account, no telemetry. |

None of this makes envlock "better" than these tools in general — it's narrower, not superior. Pick SOPS or a cloud KMS if you have access to one and need more than `.env` files. Pick envlock if you want the smallest possible offline tool for exactly this one job.

## Commands

### `envlock doctor`

Diagnose your envlock setup — run this after `git clone` or any time you're unsure. Checks:

- Git repository present
- `.envlock/` directory and identity file exist with correct permissions
- Recipients file present
- `.gitignore` covers all sensitive files
- `.env.vault` exists
- Pre-commit hook is installed
- No secret files are currently tracked by git

Exits non-zero if issues are found. This is the first command you should run in any checkout.

### `envlock init`

- Generates an age x25519 keypair
- Writes private key to `.envlock/identity.txt` (mode 0600 on Unix)
- Writes public key (recipient) to `.envlock/recipients.txt`
- Auto-appends `.env`, `.envlock/identity.txt`, and `.env.edit.tmp` to `.gitignore`
- **Installs the pre-commit hook by default** (use `--no-hook` to skip)

### `envlock encrypt [path]`

Encrypts a plaintext file (default `.env`) to `<path>.vault` using the recipients in `.envlock/recipients.txt`. Warns if `.gitignore` isn't covering secrets in this checkout.

### `envlock decrypt [path]`

Decrypts a vault file (default `.env.vault`) using the identity in `.envlock/identity.txt` and writes the plaintext, stripping the `.vault` suffix. Output file permissions are set to 0600.

### `envlock run -- <command...>`

Decrypts `.env.vault` **in memory only**, parses key-value pairs, and spawns the given command with those variables injected into its environment. No `.env` file is ever written to disk. Stdout/stderr stream through; the subprocess exit code is forwarded.

**Vault values override existing environment variables.** This is intentional: the vault is the source of truth for secrets.

### `envlock edit [--vault <path>]`

Opens the vault in your `$EDITOR` (or `$VISUAL`, falling back to `vi`). When you save and close the editor, the vault is re-encrypted automatically.

- The plaintext is written to a temp file (`.env.edit.tmp`) — **not** `.env`
- When the editor closes, the temp file is scrubbed (overwritten with zeros then deleted) — even on panic
- The vault is only updated if the edited content parses as valid `KEY=VALUE` pairs
- If the editor exits non-zero, the vault is left unchanged
- Warns if `.gitignore` isn't covering secrets in this checkout

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

### `envlock check [--strict]`

Scans `.gitignore` and `git status` to verify that secrets won't be accidentally committed. Warns if `.env`, `.envlock/identity.txt`, or `.env.edit.tmp` would be trackable. Also checks whether the pre-commit hook is installed. Exits non-zero if problems are found.

With `--strict`, also flags secret-shaped files like `.env.local`, `.env.production`, `.env.staging` that are tracked by git. Use this in CI or manual audits.

### `envlock hook install`

Installs a git pre-commit hook that runs `envlock check` before every commit. If `check` finds security issues, the commit is blocked. If a pre-commit hook already exists, appends to it instead of overwriting.

### `envlock hook uninstall`

Removes the envlock section from the pre-commit hook. Deletes the hook entirely if it was only envlock-managed.

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

## CI/CD bridge

The most common critique of git-native secret tools is fair: your SCM/CI provider already has a secrets manager, and it's usually better. envlock doesn't try to replace that — instead, these commands bridge your local encrypted vault to whatever secrets store your CI already uses, so you're not manually copy-pasting values between the two.

### `envlock ci export [--format dotenv|json|github]`

Decrypts the vault and prints the contents to **stdout** in the requested format, for piping into your CI provider's secret-import tooling (e.g. `gh secret set --env-file -`).

> **Warning:** this prints decrypted secrets to stdout. If a CI step echoes or logs its own stdout, piping `ci export` through it will leak the values into build logs. Only use this to pipe directly into a secret-setting command, never into a step that prints its input.

### `envlock ci run --prefix <PREFIX> -- <command...>`

Reads environment variables matching `<PREFIX>` (as set by your CI provider from its own secrets store), strips the prefix, and injects them into the subprocess — mirroring the local `envlock run` interface so the same command works in both places.

### `envlock ci seal --prefix <PREFIX> [--overwrite]`

Reads environment variables matching `<PREFIX>` and encrypts them into a `.env.vault`, using **only the public recipients** already committed to the repo. This lets CI produce an updated vault for local developers without the private key ever being present in CI. Refuses to overwrite an existing vault unless `--overwrite` is passed.

### GitHub Action

A reusable `action.yml` is included for GitHub Actions:

```yaml
- uses: FxckingAngel/envlock/action@main
  with:
    prefix: 'ENVLOCK_'
    command: 'python app.py'
  env:
    ENVLOCK_DB_URL: ${{ secrets.DB_URL }}
    ENVLOCK_API_KEY: ${{ secrets.API_KEY }}
```

## File layout (per-project)

```
.envlock/
  identity.txt      # age private key — NEVER committed
  recipients.txt    # age public keys (one per line)
.env.vault           # encrypted, safe to commit
.env                 # decrypted plaintext, gitignored
.env.edit.tmp        # edit temp file, gitignored & scrubbed on exit
.gitignore           # auto-managed by envlock
.git/hooks/pre-commit  # auto-installed by envlock init
action.yml            # GitHub Action for CI/CD bridge
```

## Recommended workflow

```bash
# After git clone (or any time you're unsure)
envlock doctor          # verify setup is safe

# First time setup
envlock init            # generates keys + installs pre-commit hook

# Daily workflow
envlock edit            # decrypt → $EDITOR → re-encrypt (no .env on disk)
# or
envlock run -- python app.py   # inject secrets in memory only

# Adding a teammate
envlock recipients add age1teammate...
envlock encrypt         # re-encrypt to include new recipient
git add .env.vault .envlock/recipients.txt
git commit -m "add teammate"

# CI / manual audit
envlock check --strict  # catches .env.local, .env.production, etc.

# Populating CI secrets from your local vault (one-time or on rotation)
envlock ci export --format json | gh secret set --env-file -
```

## Team sharing workflow

```bash
# Alice generates a keypair and commits .env.vault + .envlock/recipients.txt
envlock init
envlock encrypt

# Bob clones the repo and runs doctor first
envlock doctor          # catches any setup issues

# Bob generates his own keypair
envlock init            # creates his own .envlock/identity.txt (gitignored)

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

- **Accidental commits** of `.env` files containing secrets — `.gitignore` is auto-managed, `envlock check` verifies it, and the pre-commit hook is installed by default on `init`
- **Plaintext-at-rest** on disk — `envlock run` and `envlock edit` decrypt in memory only; `.env` is gitignored and 0600-permissioned; edit temp files are zeroed before deletion
- **Fresh checkout exposure** — `envlock encrypt` and `envlock edit` warn if `.gitignore` isn't covering secrets; `envlock doctor` catches the full picture after `git clone`
- **Insider access creep** — `envlock recipients remove` + re-encrypt revokes future access
- **Shell history leaks** — `envlock rotate --prompt` reads secrets without echoing
- **Manual CI secret copy-paste** — `envlock ci export`/`ci seal` reduce the number of times a secret is manually retyped or pasted between local and CI

### What envlock does NOT protect against

- **Compromised machines** — if an attacker has access to `.envlock/identity.txt`, they can decrypt any vault encrypted to that key. Protect your private key. Note: this doesn't eliminate the "one file must never leak" problem that plaintext `.env` has — it consolidates it into a single file that's generated once and never needs to be copied between machines or people (only the encrypted vault travels). Whether that's a meaningful improvement over plaintext-in-git depends on your setup; if you have access to a proper secrets manager (Vault, cloud KMS), that provides stronger guarantees than envlock does.
- **Runtime memory attacks** — `envlock run` holds decrypted secrets in process memory. A debugger or `ptrace` can read them. This is the same threat model as any tool that loads `.env` files.
- **Git history exposure** — if you commit `.env` before running `envlock init`, the secret exists in git history. Use `git filter-repo` to scrub it.
- **Vault file tampering** — age provides authenticity, but envlock does not audit access logs. If you need audit trails, use a dedicated secrets manager.
- **SIGKILL during edit** — the temp file scrub uses a `Drop` guard, which fires on panics and normal exits but not on `SIGKILL`. An unclean kill could leave `.env.edit.tmp` on disk (it's gitignored and 0600-permissioned, but the plaintext content would survive).
- **Stdout leakage from `ci export`** — the decrypted output is only as safe as the pipeline it's piped into. See the warning under [CI/CD bridge](#cicd-bridge).

envlock is a **local secret hygiene tool**, not a secrets manager. For production infrastructure, pair it with a proper secrets management solution or your CI provider's built-in secrets store — envlock's `ci` commands are meant to bridge to those, not replace them.

## Security notes

- Private keys live in `.envlock/identity.txt` with 0600 permissions and are automatically added to `.gitignore`.
- `envlock init` installs the pre-commit hook by default — the safety net is on from the start. Use `--no-hook` only if you have an alternative.
- `envlock decrypt` writes `.env` with 0600 permissions.
- `envlock run` never writes decrypted content to disk — secrets exist only in the subprocess's memory.
- `envlock edit` writes to a temp file that is scrubbed (zeroed + deleted) on exit, even on panic. `.env` is never created.
- `envlock encrypt` and `envlock edit` warn if `.gitignore` isn't protecting secrets in the current checkout.
- `envlock diff` redacts values — only key names and change types are shown.
- Decryption failures produce a generic "Decryption failed" message to avoid oracle-style information leaks.
- `envlock rotate --prompt` reads secrets from stdin without echoing, keeping them out of shell history.
- `envlock ci seal` never requires the private key in CI — only public recipients are used.
- `envlock check` + pre-commit hook provide a safety net that's on by default after `envlock init`.
- `envlock doctor` gives a full diagnostic — run it after `git clone` or any time something feels off.
- **Before every `cargo publish`**, run `cargo package --list` and review it manually. This project's own history includes releases where secrets were nearly published to the crate registry due to an untracked file being caught late — always verify the package list before publishing, don't rely on gitignore alone.

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

## Development notes

This project was built by Korone with AI assistance (Claude) for planning, architecture specs, and debugging discussion. All code was written, tested, and verified by hand — every commit reflects actual running, tested behavior, not generated-and-assumed-correct output. Design tradeoffs (e.g. identity-key-on-disk vs. OS keychain, the in-memory `run` decision, and the CI bridge design) were made by the author.

This README, including this notice, was drafted with AI assistance and edited by the author.
