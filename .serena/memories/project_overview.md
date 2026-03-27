# glossa overview
- Purpose: headless Rust daemon for Ubuntu + GNOME + Wayland that records microphone audio from a global shortcut, transcribes it through an STT provider, writes the result to the Wayland clipboard via `wl-copy`, and pastes it into the active app strictly via `dotool`.
- Command sources: XDG Desktop Portal GlobalShortcuts is the primary backend; `glossa ctl toggle` over Unix socket IPC is the secondary independent control path.
- Product model: one unified recording state machine shared by portal and CLI control. Portal supports `toggle` and `push-to-talk`; CLI control always maps to toggle and never reads or mutates `[input.mode]`.
- Platform target: Ubuntu, GNOME, Wayland, user-session daemon, autostart through `systemd --user` tied to the graphical session.
- Runtime constraints: tray support is best-effort and depends on GNOME AppIndicator/KStatusNotifierItem support; failure to provide tray support must not break the daemon.
- Current repo state: specification-first repository. Tracked files are currently docs (`AGENTS.md`, `ARCHITECTURE.md`, `README.md`, `LICENSE`) plus empty crate directories under `crates/`. There is not yet a buildable Cargo workspace or source implementation.
- Intended crate layout from `ARCHITECTURE.md`: `glossa-core` (pure config/domain types), `glossa-app` (state machine/orchestration/ports), `glossa-audio` (capture/WAV/trim/cues), `glossa-platform-linux` (portal/tray/clipboard/paste/ipc/doctor), `glossa-stt` (Groq/OpenAI/OpenAI-compatible clients), `glossa-bin` (CLI + composition root).
- Planned core libraries: `serde`, `toml`, `thiserror`, `camino`, `uuid`, `tokio`, `tracing`, `async-trait`, `cpal`, `hound`, `rodio`, `ashpd`, `tray-icon`, `reqwest`, `clap`, `tracing-subscriber`.
