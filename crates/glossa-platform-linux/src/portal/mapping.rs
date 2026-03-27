use glossa_core::{AppCommand, CommandOrigin, InputMode};

/// Portal input signal emitted by GlobalShortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalSignal {
    Activated,
    Deactivated,
}

/// Maps a portal signal to the appropriate state-machine command.
#[must_use]
pub fn map_portal_signal_to_command(mode: InputMode, signal: PortalSignal) -> Option<AppCommand> {
    match (mode, signal) {
        (InputMode::Toggle, PortalSignal::Activated) => Some(AppCommand::ToggleRecording {
            origin: CommandOrigin::PortalShortcut,
        }),
        (InputMode::Toggle, PortalSignal::Deactivated) => None,
        (InputMode::PushToTalk, PortalSignal::Activated) => Some(AppCommand::StartRecording {
            origin: CommandOrigin::PortalShortcut,
        }),
        (InputMode::PushToTalk, PortalSignal::Deactivated) => Some(AppCommand::StopRecording {
            origin: CommandOrigin::PortalShortcut,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_should_start_on_press() {
        let command = map_portal_signal_to_command(InputMode::Toggle, PortalSignal::Activated);
        assert!(matches!(
            command,
            Some(AppCommand::ToggleRecording {
                origin: CommandOrigin::PortalShortcut,
            })
        ));
    }

    #[test]
    fn toggle_should_ignore_release() {
        let command = map_portal_signal_to_command(InputMode::Toggle, PortalSignal::Deactivated);
        assert!(command.is_none());
    }

    #[test]
    fn push_to_talk_should_start_on_press() {
        let command = map_portal_signal_to_command(InputMode::PushToTalk, PortalSignal::Activated);
        assert!(matches!(
            command,
            Some(AppCommand::StartRecording {
                origin: CommandOrigin::PortalShortcut,
            })
        ));
    }

    #[test]
    fn push_to_talk_should_stop_on_release() {
        let command =
            map_portal_signal_to_command(InputMode::PushToTalk, PortalSignal::Deactivated);
        assert!(matches!(
            command,
            Some(AppCommand::StopRecording {
                origin: CommandOrigin::PortalShortcut,
            })
        ));
    }
}
