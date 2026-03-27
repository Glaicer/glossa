use std::sync::atomic::{AtomicUsize, Ordering};

static ACTIVE_SHORTCUT_CAPTURE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// RAII guard that marks a tray shortcut-capture dialog as active.
pub(crate) struct ShortcutCaptureGuard;

impl Drop for ShortcutCaptureGuard {
    fn drop(&mut self) {
        ACTIVE_SHORTCUT_CAPTURE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) fn begin_shortcut_capture() -> ShortcutCaptureGuard {
    ACTIVE_SHORTCUT_CAPTURE_COUNT.fetch_add(1, Ordering::SeqCst);
    ShortcutCaptureGuard
}

pub(crate) fn is_shortcut_capture_active() -> bool {
    ACTIVE_SHORTCUT_CAPTURE_COUNT.load(Ordering::SeqCst) > 0
}

#[cfg(test)]
mod tests {
    use super::{begin_shortcut_capture, is_shortcut_capture_active};

    #[test]
    fn capture_guard_should_track_active_state() {
        assert!(!is_shortcut_capture_active());
        let guard = begin_shortcut_capture();
        assert!(is_shortcut_capture_active());
        drop(guard);
        assert!(!is_shortcut_capture_active());
    }
}
