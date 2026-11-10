//! CLI argument definitions.

use clap::{Parser, Subcommand, ValueEnum};

use crate::output::format::OutputFormat;

/// Safely initialize, inspect, and maintain Git repositories with large files and Git LFS.
#[derive(Debug, Parser)]
#[command(name = "repo-guard", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output format for reports (`text` or `json`).
    #[arg(long, global = true, value_name = "FORMAT", default_value = "text")]
    pub format: FormatArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FormatArg {
    Text,
    Json,
}

impl From<FormatArg> for OutputFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Text => OutputFormat::Text,
            FormatArg::Json => OutputFormat::Json,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Validate Git, Git LFS, repository, hooks, and configuration readiness.
    Doctor,
    /// Detect hook managers and recommend a safe operating mode (read-only).
    Detect,
    /// Initialize `.repo-guard.toml` and shared hooks safely.
    Init {
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long, conflicts_with = "dry_run")]
        apply: bool,
    },
    /// Install hooks and Git LFS local configuration into an initialized repository.
    Install {
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long, conflicts_with = "dry_run")]
        apply: bool,
    },
    /// Validate staged Git index content against policy.
    Check,
    /// Inspect the working tree for large files while respecting Git ignore rules.
    Scan,
    /// Validate Git LFS configuration and staged pointer consistency.
    Lfs {
        #[command(subcommand)]
        command: LfsCommands,
    },
    /// Validate configured ignored paths and staged ignore conflicts.
    Ignore {
        #[command(subcommand)]
        command: IgnoreCommands,
    },
    /// Opt-in `.gitignore` preset management.
    Preset {
        #[command(subcommand)]
        command: PresetCommands,
    },
    /// Non-destructive remote validation.
    Remote {
        #[command(subcommand)]
        command: RemoteCommands,
    },
    /// Git hook entry points (non-interactive).
    Hook {
        #[command(subcommand)]
        command: HookCommands,
    },
    /// Validate outgoing commit history and push objects.
    History {
        #[command(subcommand)]
        command: HistoryCommands,
    },
    /// Detect hook managers and print integration snippets.
    Integrate {
        #[command(subcommand)]
        command: IntegrateCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum LfsCommands {
    /// Validate `.gitattributes`, Git LFS installation, and staged pointers.
    Check,
}

#[derive(Debug, Subcommand)]
pub enum IgnoreCommands {
    /// Validate configured ignored paths.
    Check,
}

#[derive(Debug, Subcommand)]
pub enum PresetCommands {
    /// List available opt-in presets.
    List,
    /// Preview or apply a preset to `.gitignore`.
    Apply {
        name: String,
        #[arg(long, conflicts_with = "apply")]
        dry_run: bool,
        #[arg(long, conflicts_with = "dry_run")]
        apply: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum RemoteCommands {
    /// Show remotes and optionally verify origin connectivity.
    Check {
        #[arg(long)]
        require_origin: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum HookCommands {
    /// Pre-commit hook validation (runs staged-index check).
    PreCommit,
    /// Pre-push hook validation (reads stdin ref list, validates outgoing objects).
    PrePush,
}

#[derive(Debug, Subcommand)]
pub enum IntegrateCommands {
    /// Detect hook managers and recommend an operating mode.
    Detect,
    /// Print a reviewable pre-commit integration snippet.
    PreCommit {
        #[arg(long)]
        print: bool,
    },
    /// Print a reviewable Husky integration snippet.
    Husky {
        #[arg(long)]
        print: bool,
    },
    /// Print a reviewable Lefthook integration snippet.
    Lefthook {
        #[arg(long)]
        print: bool,
    },
    /// Print CI range-check guidance.
    Ci {
        #[arg(long)]
        print: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum HistoryCommands {
    /// Validate blobs introduced between two refs or across all reachable history.
    Check {
        #[arg(long, conflicts_with = "all")]
        range: Option<String>,
        #[arg(long, conflicts_with = "range")]
        all: bool,
        /// Allow push/CI to continue when a remote baseline cannot be resolved locally.
        #[arg(long)]
        advisory: bool,
    },
}
