use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Command-line interface for the Glossa daemon and helper tools.
#[derive(Debug, Parser)]
#[command(name = "glossa", about = "Speech-to-text daemon for GNOME Wayland")]
pub struct Cli {
    /// Path to the TOML configuration file.
    #[arg(long, global = true)]
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
    Update,
}

/// Subcommands for the external control CLI.
#[derive(Debug, Subcommand)]
pub enum CtlCommand {
    Toggle,
    Shutdown,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;

    #[test]
    fn config_flag_after_subcommand_should_parse() {
        let cli = Cli::try_parse_from(["glossa", "daemon", "--config", "/tmp/glossa.toml"])
            .expect("config flag after daemon subcommand should parse");

        assert_eq!(
            cli.config.as_deref(),
            Some(std::path::Path::new("/tmp/glossa.toml"))
        );
    }

    #[test]
    fn config_flag_before_subcommand_should_parse() {
        let cli = Cli::try_parse_from(["glossa", "--config", "/tmp/glossa.toml", "daemon"])
            .expect("config flag before daemon subcommand should parse");

        assert_eq!(
            cli.config.as_deref(),
            Some(std::path::Path::new("/tmp/glossa.toml"))
        );
    }

    #[test]
    fn update_subcommand_should_parse() {
        let cli =
            Cli::try_parse_from(["glossa", "update"]).expect("update subcommand should parse");

        assert!(matches!(cli.command, super::Command::Update));
    }
}
