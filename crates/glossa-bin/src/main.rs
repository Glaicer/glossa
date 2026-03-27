mod bootstrap;
mod cli;
mod cmd;

use anyhow::Context;
use clap::Parser;

use crate::cli::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Daemon => cmd::daemon::run(cli.config).await,
        Command::Ctl { ctl } => cmd::ctl::run(cli.config, ctl).await,
        Command::Doctor => cmd::doctor::run(cli.config).await,
        Command::Status => cmd::status::run(cli.config).await,
    }
    .context("command failed")
}
