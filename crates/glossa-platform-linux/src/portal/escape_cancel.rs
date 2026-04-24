use std::time::Duration;

use tokio::{
    sync::{mpsc, oneshot, watch},
    time::sleep,
};
use tracing::{debug, warn};

use glossa_app::AppError;
use glossa_core::{AppCommand, AppStateKind, AppStatus, CommandOrigin};

use super::evdev_monitor::spawn_escape_press_monitor;

const ESCAPE_MONITOR_RETRY_DELAY: Duration = Duration::from_secs(2);

pub async fn run_escape_cancel_monitor(
    status_rx: watch::Receiver<AppStatus>,
    command_tx: mpsc::UnboundedSender<AppCommand>,
) -> Result<(), AppError> {
    run_escape_cancel_monitor_with(
        status_rx,
        command_tx,
        spawn_escape_press_monitor,
        ESCAPE_MONITOR_RETRY_DELAY,
    )
    .await
}

async fn run_escape_cancel_monitor_with<F>(
    mut status_rx: watch::Receiver<AppStatus>,
    command_tx: mpsc::UnboundedSender<AppCommand>,
    mut spawn_escape_monitor: F,
    retry_delay: Duration,
) -> Result<(), AppError>
where
    F: FnMut() -> oneshot::Receiver<()>,
{
    let _ = status_rx.borrow_and_update();
    let mut cancel_sent_for_current_recording = false;
    let mut escape_rx = spawn_escape_monitor();

    loop {
        tokio::select! {
            changed = status_rx.changed() => {
                if changed.is_err() {
                    return Ok(());
                }

                let _ = status_rx.borrow_and_update();
                cancel_sent_for_current_recording = false;
            }
            escape = &mut escape_rx => {
                match escape {
                    Ok(()) => {
                        if status_rx.has_changed().unwrap_or(false) {
                            let _ = status_rx.borrow_and_update();
                            cancel_sent_for_current_recording = false;
                        }
                        let state = status_rx.borrow_and_update().state;
                        if state == AppStateKind::Recording
                            && !cancel_sent_for_current_recording
                        {
                            debug!("escape key pressed; cancelling active recording");
                            command_tx.send(AppCommand::CancelRecording {
                                origin: CommandOrigin::EscapeKey,
                            }).map_err(|_| AppError::message("daemon command channel is closed"))?;
                            cancel_sent_for_current_recording = true;
                        } else if state != AppStateKind::Recording {
                            cancel_sent_for_current_recording = false;
                        }

                        escape_rx = spawn_escape_monitor();
                    }
                    Err(_) => {
                        warn!(
                            retry_delay_ms = retry_delay.as_millis(),
                            "escape key monitor stopped; retrying"
                        );
                        sleep(retry_delay).await;
                        escape_rx = spawn_escape_monitor();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;

    use tokio::time::timeout;

    use glossa_core::{AppCommand, AppStateKind, AppStatus, CommandOrigin, ProviderKind};

    use super::run_escape_cancel_monitor_with;

    fn status(state: AppStateKind) -> AppStatus {
        AppStatus {
            state,
            provider: ProviderKind::Groq,
            tray_available: true,
            portal_available: true,
        }
    }

    #[tokio::test]
    async fn escape_monitor_should_emit_cancel_when_recording_and_escape_is_pressed() {
        let (status_tx, status_rx) = tokio::sync::watch::channel(status(AppStateKind::Idle));
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (escape_tx, escape_rx) = tokio::sync::oneshot::channel();
        let mut receivers = vec![escape_rx].into_iter();

        let task = tokio::spawn(run_escape_cancel_monitor_with(
            status_rx,
            command_tx,
            move || receivers.next().expect("escape receiver should exist"),
            Duration::from_millis(5),
        ));

        status_tx
            .send(status(AppStateKind::Recording))
            .expect("recording status should be sent");
        escape_tx.send(()).expect("escape event should be sent");

        let command = timeout(Duration::from_millis(200), command_rx.recv())
            .await
            .expect("command should arrive")
            .expect("cancel command should be queued");

        assert_eq!(
            command,
            AppCommand::CancelRecording {
                origin: CommandOrigin::EscapeKey,
            }
        );

        task.abort();
    }

    #[tokio::test]
    async fn escape_monitor_should_emit_nothing_while_not_recording() {
        let (status_tx, status_rx) = tokio::sync::watch::channel(status(AppStateKind::Idle));
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let spawn_calls = Arc::new(AtomicUsize::new(0));
        let spawn_calls_for_task = Arc::clone(&spawn_calls);

        let task = tokio::spawn(run_escape_cancel_monitor_with(
            status_rx,
            command_tx,
            move || {
                spawn_calls_for_task.fetch_add(1, Ordering::SeqCst);
                let (_tx, rx) = tokio::sync::oneshot::channel();
                rx
            },
            Duration::from_millis(5),
        ));

        status_tx
            .send(status(AppStateKind::Processing))
            .expect("processing status should be sent");
        status_tx
            .send(status(AppStateKind::Pasting))
            .expect("pasting status should be sent");

        assert!(
            timeout(Duration::from_millis(100), command_rx.recv())
                .await
                .is_err(),
            "no cancel command should be sent"
        );
        assert!(
            spawn_calls.load(Ordering::SeqCst) > 0,
            "escape monitor should stay armed while idle"
        );

        task.abort();
    }

    #[tokio::test]
    async fn escape_monitor_should_arm_escape_source_before_recording_starts() {
        let (_status_tx, status_rx) = tokio::sync::watch::channel(status(AppStateKind::Idle));
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let spawn_calls = Arc::new(AtomicUsize::new(0));
        let spawn_calls_for_task = Arc::clone(&spawn_calls);

        let task = tokio::spawn(run_escape_cancel_monitor_with(
            status_rx,
            command_tx,
            move || {
                spawn_calls_for_task.fetch_add(1, Ordering::SeqCst);
                let (_tx, rx) = tokio::sync::oneshot::channel();
                rx
            },
            Duration::from_millis(5),
        ));

        tokio::task::yield_now().await;

        assert_eq!(spawn_calls.load(Ordering::SeqCst), 1);
        assert!(
            timeout(Duration::from_millis(100), command_rx.recv())
                .await
                .is_err(),
            "armed idle monitor should not send cancel commands"
        );

        task.abort();
    }

    #[tokio::test]
    async fn escape_monitor_should_not_emit_duplicate_cancel_until_recording_state_exits() {
        let (status_tx, status_rx) = tokio::sync::watch::channel(status(AppStateKind::Idle));
        let (command_tx, mut command_rx) = tokio::sync::mpsc::unbounded_channel();
        let (first_tx, first_rx) = tokio::sync::oneshot::channel();
        let (second_tx, second_rx) = tokio::sync::oneshot::channel();
        let (third_tx, third_rx) = tokio::sync::oneshot::channel();
        let (_fourth_tx, fourth_rx) = tokio::sync::oneshot::channel();
        let spawn_calls = Arc::new(AtomicUsize::new(0));
        let spawn_calls_for_task = Arc::clone(&spawn_calls);
        let mut receivers = vec![first_rx, second_rx, third_rx, fourth_rx].into_iter();

        let task = tokio::spawn(run_escape_cancel_monitor_with(
            status_rx,
            command_tx,
            move || {
                spawn_calls_for_task.fetch_add(1, Ordering::SeqCst);
                receivers.next().expect("escape receiver should exist")
            },
            Duration::from_millis(5),
        ));

        status_tx
            .send(status(AppStateKind::Recording))
            .expect("recording status should be sent");
        first_tx.send(()).expect("first escape should be sent");

        let first_command = timeout(Duration::from_millis(200), command_rx.recv())
            .await
            .expect("first command should arrive")
            .expect("first cancel command should be queued");
        assert_eq!(
            first_command,
            AppCommand::CancelRecording {
                origin: CommandOrigin::EscapeKey,
            }
        );
        second_tx.send(()).expect("second escape should be sent");
        assert!(
            timeout(Duration::from_millis(100), command_rx.recv())
                .await
                .is_err(),
            "monitor should not emit duplicate cancel while recording stays active"
        );

        status_tx
            .send(status(AppStateKind::Idle))
            .expect("idle status should be sent");
        status_tx
            .send(status(AppStateKind::Recording))
            .expect("recording status should be sent again");
        third_tx.send(()).expect("third escape should be sent");

        let second_command = timeout(Duration::from_millis(200), command_rx.recv())
            .await
            .expect("second command should arrive")
            .expect("second cancel command should be queued");
        assert_eq!(
            second_command,
            AppCommand::CancelRecording {
                origin: CommandOrigin::EscapeKey,
            }
        );

        task.abort();
    }
}
