use anyhow::{anyhow, Context};
use tokio::task::JoinHandle;
use tracing::{error, info};

use glossa_app::ports::CommandSource;
use glossa_core::InputBackend;
use glossa_platform_linux::{ipc::IpcServer, portal::PortalShortcutSource};

use crate::bootstrap::{build_actor, init_tracing, load_config};

pub async fn run(config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let config_path =
        config_path.ok_or_else(|| anyhow!("`glossa daemon` requires --config <path>"))?;
    let config = load_config(&config_path).await?;
    init_tracing(&config)?;
    info!(path = %config_path.display(), "starting glossa daemon");

    let socket_path = config
        .control
        .socket_path
        .resolve()
        .context("failed to resolve daemon socket path")?;
    let input_backend = config.input.backend;
    let portal_config = config.input.clone();
    let cli_enabled = config.control.enable_cli;

    let (actor, handle) = build_actor(config)?;
    let mut tasks: Vec<JoinHandle<()>> = Vec::new();

    if cli_enabled {
        let server = IpcServer::new(socket_path, handle.clone());
        tasks.push(tokio::spawn(async move {
            if let Err(error) = server.run().await {
                error!(error = %error, "ipc server exited with an error");
            }
        }));
    }

    if input_backend == InputBackend::Portal {
        let tx = handle.command_sender();
        let source: Box<dyn CommandSource> = Box::new(PortalShortcutSource::new(portal_config));
        tasks.push(tokio::spawn(async move {
            if let Err(error) = source.run(tx).await {
                error!(error = %error, "portal shortcut source exited with an error");
            }
        }));
    }

    actor.run().await?;
    for task in tasks {
        task.abort();
    }
    Ok(())
}
