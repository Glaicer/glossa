use glossa_core::{
    AppCommand, AppState, CoreError, PastingState, ProcessingState, RecordingState, SessionId,
};

use super::{should_ignore_recording_command, Action};

/// Result of reducing one command against the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub next_state: AppState,
    pub actions: Vec<Action>,
}

/// Applies a command to the current state and returns the next state plus side effects.
pub fn reduce(state: &AppState, command: &AppCommand) -> Result<Decision, CoreError> {
    use AppCommand::{Restart, Shutdown, StartRecording, StopRecording, ToggleRecording};

    let decision = match (state, command) {
        (_, Restart { .. }) => Decision {
            next_state: AppState::ShuttingDown,
            actions: vec![Action::Restart],
        },
        (_, Shutdown { .. }) => Decision {
            next_state: AppState::ShuttingDown,
            actions: vec![Action::Shutdown],
        },
        (AppState::Idle, StartRecording { .. } | ToggleRecording { .. }) => {
            let session_id = SessionId::new();
            Decision {
                next_state: AppState::Recording(RecordingState { session_id }),
                actions: vec![
                    Action::SetTrayRecording,
                    Action::PlayStartCue,
                    Action::StartRecording { session_id },
                ],
            }
        }
        (AppState::Idle, StopRecording { .. }) => Decision {
            next_state: AppState::Idle,
            actions: vec![Action::Ignore {
                reason: "recording is not active",
            }],
        },
        (
            AppState::Recording(RecordingState { session_id }),
            StopRecording { .. } | ToggleRecording { .. },
        ) => Decision {
            next_state: AppState::Processing(ProcessingState {
                session_id: *session_id,
            }),
            actions: vec![
                Action::PlayStopCue,
                Action::SetTrayProcessing,
                Action::StopRecording {
                    session_id: *session_id,
                },
            ],
        },
        (AppState::Recording(_), StartRecording { .. }) => Decision {
            next_state: state.clone(),
            actions: vec![Action::Ignore {
                reason: "recording is already active",
            }],
        },
        (busy_state, StartRecording { .. } | StopRecording { .. } | ToggleRecording { .. })
            if should_ignore_recording_command(busy_state) =>
        {
            Decision {
                next_state: busy_state.clone(),
                actions: vec![Action::Ignore {
                    reason: "recording command ignored while the previous cycle is busy",
                }],
            }
        }
        (AppState::ShuttingDown, _) => Decision {
            next_state: AppState::ShuttingDown,
            actions: vec![Action::Ignore {
                reason: "daemon is shutting down",
            }],
        },
        (AppState::Pasting(PastingState { .. }), _)
        | (AppState::Processing(ProcessingState { .. }), _) => Decision {
            next_state: state.clone(),
            actions: vec![Action::Ignore {
                reason: "unexpected command for busy state",
            }],
        },
    };

    if matches!(decision.next_state, AppState::Idle) && !matches!(state, AppState::Idle) {
        return Ok(Decision {
            actions: [decision.actions, vec![Action::SetTrayIdle]].concat(),
            ..decision
        });
    }

    Ok(decision)
}

#[cfg(test)]
mod tests {
    use glossa_core::{AppCommand, AppState, CommandOrigin, RecordingState};

    use super::reduce;

    #[test]
    fn idle_toggle_should_start_recording() {
        let decision = reduce(
            &AppState::Idle,
            &AppCommand::ToggleRecording {
                origin: CommandOrigin::PortalShortcut,
            },
        )
        .expect("reducer should succeed");

        assert!(matches!(decision.next_state, AppState::Recording(_)));
        assert_eq!(decision.actions.len(), 3);
    }

    #[test]
    fn recording_toggle_should_stop_recording() {
        let decision = reduce(
            &AppState::Recording(RecordingState {
                session_id: Default::default(),
            }),
            &AppCommand::ToggleRecording {
                origin: CommandOrigin::CliControl,
            },
        )
        .expect("reducer should succeed");

        assert!(matches!(decision.next_state, AppState::Processing(_)));
    }

    #[test]
    fn processing_commands_should_be_ignored() {
        let decision = reduce(
            &AppState::Processing(glossa_core::ProcessingState {
                session_id: Default::default(),
            }),
            &AppCommand::ToggleRecording {
                origin: CommandOrigin::CliControl,
            },
        )
        .expect("reducer should succeed");

        assert!(matches!(decision.next_state, AppState::Processing(_)));
        assert_eq!(decision.actions.len(), 1);
    }

    #[test]
    fn restart_should_request_shutdown_from_any_state() {
        let decision = reduce(
            &AppState::Recording(RecordingState {
                session_id: Default::default(),
            }),
            &AppCommand::Restart {
                origin: CommandOrigin::TrayMenu,
            },
        )
        .expect("reducer should succeed");

        assert!(matches!(decision.next_state, AppState::ShuttingDown));
        assert_eq!(decision.actions, vec![super::Action::Restart]);
    }
}
