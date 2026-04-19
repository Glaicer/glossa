use std::process::Command;

use glossa_app::AppError;

const GLOSSA_SERVICE_NAME: &str = "glossa";

pub(crate) fn restart_glossa_service() -> Result<(), AppError> {
    let output = Command::new("systemctl")
        .args(build_restart_args())
        .output()
        .map_err(|error| AppError::io("failed to invoke systemctl", error))?;

    ensure_restart_succeeded(output.status.success(), &output.stderr)
}

fn build_restart_args() -> [&'static str; 3] {
    ["--user", "restart", GLOSSA_SERVICE_NAME]
}

fn ensure_restart_succeeded(success: bool, stderr: &[u8]) -> Result<(), AppError> {
    if success {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    if stderr.is_empty() {
        return Err(AppError::message(
            "`systemctl --user restart glossa` failed",
        ));
    }

    Err(AppError::message(format!(
        "`systemctl --user restart glossa` failed: {stderr}"
    )))
}

#[cfg(test)]
mod tests {
    use super::{build_restart_args, ensure_restart_succeeded};

    #[test]
    fn build_restart_args_should_target_the_user_glossa_service() {
        assert_eq!(build_restart_args(), ["--user", "restart", "glossa"]);
    }

    #[test]
    fn ensure_restart_succeeded_should_include_stderr_on_failure() {
        let error = ensure_restart_succeeded(false, b"Unit glossa.service not found")
            .expect_err("failed restart should return an error");

        assert_eq!(
            error.to_string(),
            "`systemctl --user restart glossa` failed: Unit glossa.service not found"
        );
    }

    #[test]
    fn ensure_restart_succeeded_should_return_generic_error_when_stderr_is_empty() {
        let error = ensure_restart_succeeded(false, b"")
            .expect_err("failed restart without stderr should return an error");

        assert_eq!(
            error.to_string(),
            "`systemctl --user restart glossa` failed"
        );
    }
}
