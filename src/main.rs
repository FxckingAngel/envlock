use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, shells::Bash, shells::Zsh, shells::Fish, shells::PowerShell};

/// envlock — encrypt, decrypt, and inject .env files with age encryption.
#[derive(Parser, Debug)]
#[command(name = "envlock", version, about)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate an age keypair for this project.
    /// Installs the pre-commit hook by default.
    Init {
        /// Skip installing the pre-commit hook.
        #[arg(long)]
        no_hook: bool,
    },

    /// Encrypt a .env file to .env.vault.
    Encrypt {
        /// Path to the plaintext .env file. Defaults to .env.
        path: Option<std::path::PathBuf>,
    },

    /// Decrypt a .env.vault file back to .env.
    Decrypt {
        /// Path to the vault file. Defaults to .env.vault.
        path: Option<std::path::PathBuf>,
    },

    /// Decrypt in memory and run a command with env vars injected.
    ///
    /// Vault values override existing environment variables — the vault
    /// is the source of truth for secrets.
    Run {
        /// Path to the vault file. Defaults to .env.vault.
        #[arg(long)]
        vault: Option<std::path::PathBuf>,

        /// The command to run (after --).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Open the vault in $EDITOR, re-encrypt on save.
    ///
    /// The plaintext is written to a temp file (.env.edit.tmp) that is
    /// scrubbed (zeroed + deleted) on exit, even on panic. The .env file
    /// is never written to disk.
    Edit {
        /// Path to the vault file. Defaults to .env.vault.
        #[arg(long)]
        vault: Option<std::path::PathBuf>,
    },

    /// Compare keys between two vault files (values redacted).
    Diff {
        /// First vault file.
        vault_a: std::path::PathBuf,

        /// Second vault file.
        vault_b: std::path::PathBuf,
    },

    /// Rotate a single key's value in an encrypted vault.
    Rotate {
        /// Name of the key to rotate.
        key_name: String,

        /// New value for the key (mutually exclusive with --prompt).
        #[arg(long, group = "value_source")]
        value: Option<String>,

        /// Read the new value from stdin without echoing (for secrets).
        #[arg(long, group = "value_source")]
        prompt: bool,

        /// Path to the vault file. Defaults to .env.vault.
        #[arg(long)]
        vault: Option<std::path::PathBuf>,
    },

    /// Manage recipients who can decrypt vault files.
    Recipients {
        #[command(subcommand)]
        action: RecipientAction,
    },

    /// Check that secrets won't be accidentally committed to git.
    Check {
        /// Strict mode: also flag secret-shaped files like .env.local, .env.production.
        #[arg(long)]
        strict: bool,
    },

    /// Install or uninstall the envlock pre-commit hook.
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Diagnose your envlock setup — run after git clone.
    Doctor,

    /// Generate shell completions to stdout.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    /// PowerShell
    #[value(name = "powershell")]
    PowerShell,
}

#[derive(Subcommand, Debug)]
enum RecipientAction {
    /// Add a public key to the recipients list.
    Add {
        /// Age public key (age1...).
        public_key: String,
    },

    /// List all recipients.
    List,

    /// Remove a public key from the recipients list.
    Remove {
        /// Age public key to remove (age1...).
        public_key: String,
    },
}

#[derive(Subcommand, Debug)]
enum HookAction {
    /// Install the pre-commit hook that runs envlock check.
    Install,

    /// Uninstall the envlock pre-commit hook.
    Uninstall,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { no_hook } => envlock::commands::init::execute(no_hook),
        Commands::Encrypt { path } => envlock::commands::encrypt::execute(path),
        Commands::Decrypt { path } => envlock::commands::decrypt::execute(path),
        Commands::Run { vault, command } => envlock::commands::run::execute(command, vault),
        Commands::Edit { vault } => envlock::commands::edit::execute(vault),
        Commands::Diff { vault_a, vault_b } => envlock::commands::diff::execute(vault_a, vault_b),
        Commands::Rotate {
            key_name,
            value,
            prompt,
            vault,
        } => envlock::commands::rotate::execute(&key_name, value, prompt, vault),
        Commands::Recipients { action } => match action {
            RecipientAction::Add { public_key } => envlock::commands::recipients::add(&public_key),
            RecipientAction::List => envlock::commands::recipients::list(),
            RecipientAction::Remove { public_key } => {
                envlock::commands::recipients::remove(&public_key)
            }
        },
        Commands::Check { strict } => envlock::commands::check::execute(strict),
        Commands::Hook { action } => match action {
            HookAction::Install => envlock::commands::hook::install(),
            HookAction::Uninstall => envlock::commands::hook::uninstall(),
        },
        Commands::Doctor => envlock::commands::doctor::execute(),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = "envlock".to_string();
            match shell {
                Shell::Bash => generate(Bash, &mut cmd, &name, &mut std::io::stdout()),
                Shell::Zsh => generate(Zsh, &mut cmd, &name, &mut std::io::stdout()),
                Shell::Fish => generate(Fish, &mut cmd, &name, &mut std::io::stdout()),
                Shell::PowerShell => generate(PowerShell, &mut cmd, &name, &mut std::io::stdout()),
            }
            Ok(())
        }
    }
}
