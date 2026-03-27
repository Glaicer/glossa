use anyhow::anyhow;

use glossa_platform_linux::doctor::Doctor;

use crate::bootstrap::{init_tracing, load_config_or_default};

pub async fn run(config_path: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    let config = load_config_or_default(config_path).await?;
    init_tracing(&config).map_err(|error| anyhow!("failed to initialize tracing: {error}"))?;
    let report = Doctor::run(&config).await?;
    print!("{report}");
    Ok(())
}
