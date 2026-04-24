use serde::{Deserialize, Serialize};

/// Origin of a command routed into the daemon state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandOrigin {
    PortalShortcut,
    CliControl,
    TrayMenu,
    EscapeKey,
    Internal,
}

/// Commands accepted by the daemon state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AppCommand {
    StartRecording { origin: CommandOrigin },
    StopRecording { origin: CommandOrigin },
    ToggleRecording { origin: CommandOrigin },
    CancelRecording { origin: CommandOrigin },
    Restart { origin: CommandOrigin },
    Shutdown { origin: CommandOrigin },
}
