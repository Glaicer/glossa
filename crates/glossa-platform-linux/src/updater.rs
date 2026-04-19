use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Output},
};

use glossa_app::AppError;

const LOCAL_BIN_DIR: &str = ".local/bin";
const UPDATER_SCRIPT_NAME: &str = "update.sh";
const STATUS_PREFIX: &str = "GLOSSA_UPDATE_STATUS=";
const VERSION_PREFIX: &str = "GLOSSA_UPDATE_VERSION=";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdaterStatus {
    UpToDate,
    Available,
    Updated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdaterResult {
    pub status: UpdaterStatus,
    pub version: String,
}

#[must_use]
fn updater_script_candidates(current_exe: &Path, home_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(parent) = current_exe.parent() {
        candidates.push(parent.join(UPDATER_SCRIPT_NAME));
    }

    if let Some(home_dir) = home_dir {
        let fallback = home_dir.join(LOCAL_BIN_DIR).join(UPDATER_SCRIPT_NAME);
        if !candidates.iter().any(|candidate| candidate == &fallback) {
            candidates.push(fallback);
        }
    }

    candidates
}

fn find_local_updater_script_with<F>(
    current_exe: &Path,
    home_dir: Option<&Path>,
    exists: F,
) -> Result<PathBuf, AppError>
where
    F: Fn(&Path) -> bool,
{
    updater_script_candidates(current_exe, home_dir)
        .into_iter()
        .find(|candidate| exists(candidate))
        .ok_or_else(|| {
            AppError::message(
                "could not find update.sh next to the glossa binary or in ~/.local/bin",
            )
        })
}

pub fn find_local_updater_script() -> Result<PathBuf, AppError> {
    let current_exe = env::current_exe()
        .map_err(|error| AppError::io("failed to resolve current executable", error))?;
    let home_dir = env::var_os("HOME").map(PathBuf::from);

    find_local_updater_script_with(&current_exe, home_dir.as_deref(), Path::is_file)
}

pub fn run_local_updater() -> Result<ExitStatus, AppError> {
    let updater_path = find_local_updater_script()?;

    Command::new(&updater_path)
        .status()
        .map_err(|error| AppError::io("failed to run local updater", error))
}

pub fn spawn_local_updater() -> Result<(), AppError> {
    let updater_path = find_local_updater_script()?;

    Command::new(&updater_path)
        .spawn()
        .map_err(|error| AppError::io("failed to launch local updater", error))?;

    Ok(())
}

fn run_local_updater_with_args(args: &[&str]) -> Result<Output, AppError> {
    let updater_path = find_local_updater_script()?;

    Command::new(&updater_path)
        .args(args)
        .output()
        .map_err(|error| AppError::io("failed to run local updater", error))
}

fn parse_updater_result(stdout: &str) -> Option<UpdaterResult> {
    let mut status = None;
    let mut version = None;

    for line in stdout.lines() {
        if let Some(raw_status) = line.strip_prefix(STATUS_PREFIX) {
            status = match raw_status {
                "up-to-date" => Some(UpdaterStatus::UpToDate),
                "available" => Some(UpdaterStatus::Available),
                "updated" => Some(UpdaterStatus::Updated),
                _ => None,
            };
            continue;
        }

        if let Some(raw_version) = line.strip_prefix(VERSION_PREFIX) {
            version = Some(raw_version.to_owned());
        }
    }

    Some(UpdaterResult {
        status: status?,
        version: version?,
    })
}

fn command_failed_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();

    if !stderr.is_empty() {
        return stderr;
    }

    if !stdout.is_empty() {
        return stdout;
    }

    format!("updater exited with status {}", output.status)
}

fn run_updater_command(args: &[&str]) -> Result<UpdaterResult, AppError> {
    let output = run_local_updater_with_args(args)?;

    if !output.status.success() {
        return Err(AppError::message(command_failed_message(&output)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_updater_result(&stdout)
        .ok_or_else(|| AppError::message("updater output did not contain a valid result"))
}

pub fn check_for_update() -> Result<UpdaterResult, AppError> {
    run_updater_command(&["check"])
}

pub fn install_update() -> Result<UpdaterResult, AppError> {
    run_updater_command(&["install"])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{parse_updater_result, updater_script_candidates, UpdaterResult, UpdaterStatus};

    #[test]
    fn updater_candidates_should_prefer_current_executable_directory() {
        let candidates = updater_script_candidates(
            Path::new("/opt/glossa/bin/glossa"),
            Some(Path::new("/home/tester")),
        );

        assert_eq!(
            candidates,
            vec![
                Path::new("/opt/glossa/bin/update.sh").to_path_buf(),
                Path::new("/home/tester/.local/bin/update.sh").to_path_buf(),
            ]
        );
    }

    #[test]
    fn updater_candidates_should_deduplicate_home_fallback() {
        let candidates = updater_script_candidates(
            Path::new("/home/tester/.local/bin/glossa"),
            Some(Path::new("/home/tester")),
        );

        assert_eq!(
            candidates,
            vec![Path::new("/home/tester/.local/bin/update.sh").to_path_buf()]
        );
    }

    #[test]
    fn parse_updater_result_should_extract_up_to_date_status() {
        let result = parse_updater_result(
            "Downloading glossa-update.sh\nGLOSSA_UPDATE_STATUS=up-to-date\nGLOSSA_UPDATE_VERSION=0.3.0\n",
        );

        assert_eq!(
            result,
            Some(UpdaterResult {
                status: UpdaterStatus::UpToDate,
                version: "0.3.0".into(),
            })
        );
    }

    #[test]
    fn parse_updater_result_should_extract_updated_status() {
        let result = parse_updater_result(
            "Updating Glossa from 0.2.0 to 0.3.0\nGLOSSA_UPDATE_STATUS=updated\nGLOSSA_UPDATE_VERSION=0.3.0\n",
        );

        assert_eq!(
            result,
            Some(UpdaterResult {
                status: UpdaterStatus::Updated,
                version: "0.3.0".into(),
            })
        );
    }
}
