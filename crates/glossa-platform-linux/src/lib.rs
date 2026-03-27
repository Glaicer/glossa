//! Linux-specific adapters for IPC, clipboard, paste, diagnostics, and portals.

pub mod clipboard;
pub mod doctor;
pub mod ipc;
pub mod paste;
pub mod portal;
pub(crate) mod shortcut_capture;
pub mod temp;
pub mod tray;
