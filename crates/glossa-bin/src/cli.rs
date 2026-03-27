use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Command-line interface for the Glossa daemon and helper tools.
#[derive(Debug, Parser)]
#[command(name = "glossa", about = "Speech-to-text daemon for GNOME Wayland")]
pub struct Cli {
    /// Path to the TOML configuration file.
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

/// Top-level commands supported by the binary.
#[derive(Debug, Subcommand)]
pub enum Command {
    Daemon,
    Ctl {
        #[command(subcommand)]
        ctl: CtlCommand,
    },
    Doctor,
    Status,
}

/// Subcommands for the external control CLI.
#[derive(Debug, Subcommand)]
pub enum CtlCommand {
    Toggle,
    Shutdown,
}
