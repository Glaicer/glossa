use anyhow::{anyhow, Context};
use tokio::process::Command;

use crate::cli::ServiceCommand;

pub async fn run(command: ServiceCommand) -> anyhow::Result<()> {
    let args = build_systemctl_args(command);
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .await
        .context("failed to invoke systemctl")?;

    ensure_systemctl_succeeded(command, output.status.success(), &output.stderr)
}

fn build_systemctl_args(command: ServiceCommand) -> [&'static str; 3] {
    ["--user", systemctl_action(command), "glossa"]
}

fn systemctl_action(command: ServiceCommand) -> &'static str {
    match command {
        ServiceCommand::Start => "start",
        ServiceCommand::Stop => "stop",
        ServiceCommand::Restart => "restart",
    }
}

fn ensure_systemctl_succeeded(
    command: ServiceCommand,
    success: bool,
    stderr: &[u8],
) -> anyhow::Result<()> {
    if success {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    if stderr.is_empty() {
        return Err(anyhow!(
            "`systemctl --user {} glossa` failed",
            systemctl_action(command)
        ));
    }

    Err(anyhow!(
        "`systemctl --user {} glossa` failed: {}",
        systemctl_action(command),
        stderr
    ))
}

#[cfg(test)]
mod tests {
    use crate::cli::ServiceCommand;

    use super::{build_systemctl_args, ensure_systemctl_succeeded};

    #[test]
    fn build_systemctl_args_should_target_user_glossa_service_for_restart() {
        assert_eq!(
            build_systemctl_args(ServiceCommand::Restart),
            ["--user", "restart", "glossa"]
        );
    }

    #[test]
    fn ensure_systemctl_succeeded_should_include_stderr_on_failure() {
        let error = ensure_systemctl_succeeded(
            ServiceCommand::Start,
            false,
            b"Unit glossa.service not found",
        )
        .expect_err("failed systemctl call should return an error");

        assert_eq!(
            error.to_string(),
            "`systemctl --user start glossa` failed: Unit glossa.service not found"
        );
    }

    #[test]
    fn ensure_systemctl_succeeded_should_return_generic_error_without_stderr() {
        let error = ensure_systemctl_succeeded(ServiceCommand::Stop, false, b"")
            .expect_err("failed systemctl call without stderr should return an error");

        assert_eq!(error.to_string(), "`systemctl --user stop glossa` failed");
    }
}
