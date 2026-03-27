# Style and conventions
- Preserve the architecture described in `ARCHITECTURE.md`: keep pure domain/config/state types in `glossa-core`, orchestration + ports in `glossa-app`, Linux adapters in `glossa-platform-linux`, HTTP providers in `glossa-stt`, and wiring/CLI in `glossa-bin`.
- Treat the reducer/state machine as pure logic with no I/O side effects. Side effects execute from actions/use cases outside the reducer.
- Prefer explicit traits/ports for integrations (`CommandSource`, `AudioCapture`, `SttClient`, `ClipboardWriter`, `PasteBackend`, `TrayPort`, `TempStore`) so the app layer is testable without real devices or external binaries.
- Maintain one shared recording pipeline and one shared state machine. Do not split portal and CLI into separate recording services.
- Keep Linux- and GNOME-specific details isolated to `glossa-platform-linux`; `dotool` should appear only in the paste backend implementation and `wl-copy` only in the clipboard adapter.
- Config is TOML and supplied via `--config <path>`. `backend = "none"` disables portal registration only; CLI control remains available. `openai-compatible` requires `base_url`.
- Reliability rules: ignore new recording commands while in `Processing`/`Pasting`; return to `Idle` after failures; keep clipboard content even if paste fails when possible; always clean up temp audio files.
- Security rules: never log API secrets; log only secret sources like `env:GROQ_API_KEY`. Temp audio files must be cleaned after success or failure, and ideally on next launch after abnormal termination.
- Observability rules: use structured logging, include lifecycle events for startup/config/portal registration/recording/transcription/clipboard/paste/cleanup, and surface degraded modes (missing tray support, missing cue files, etc.) clearly.
