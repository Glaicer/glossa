mod evdev_monitor;
mod mapping;
mod shortcut_source;

pub use self::mapping::{map_portal_signal_to_command, PortalSignal};
pub use self::shortcut_source::PortalShortcutSource;
pub(crate) use self::shortcut_source::{
    portal_shortcut_description, PORTAL_APP_ID, PORTAL_SHORTCUT_ID,
};
