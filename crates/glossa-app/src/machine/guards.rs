use glossa_core::AppState;

/// Returns `true` when new recording commands should be ignored.
#[must_use]
pub fn should_ignore_recording_command(state: &AppState) -> bool {
    matches!(state, AppState::Processing(_) | AppState::Pasting(_))
}
