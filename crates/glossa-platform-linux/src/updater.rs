use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use glossa_app::AppError;

const LOCAL_BIN_DIR: &str = ".local/bin";
const UPDATER_SCRIPT_NAME: &str = "update.sh";

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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::updater_script_candidates;

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
}
