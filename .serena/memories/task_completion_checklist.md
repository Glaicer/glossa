# Task completion checklist
- If the task changed Rust code, run formatting, clippy, and the relevant tests once the Cargo workspace exists.
- Prefer unit tests around the pure reducer/state machine first, then adapter-level tests for config parsing, provider DTO parsing, IPC protocol mapping, and command mapping.
- For behavior-level work, verify the invariants from `AGENTS.md`: one shared state machine, portal mode obeys `[input]`, `glossa ctl toggle` ignores `[input.mode]`, processing/pasting blocks new recordings, clipboard remains populated if paste fails, temp files are cleaned.
- For platform work, confirm graceful degradation paths: missing tray support, missing cue files, missing `wl-copy`/`dotool`, and absent portal backend when `backend = "none"`.
- Keep docs/examples/systemd config aligned with implementation whenever the scaffold starts to exist.
- Note current limitation explicitly in task summaries when relevant: the repository is still scaffold-only, so some verification commands may remain planned rather than executable until the workspace is created.
