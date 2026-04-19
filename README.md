<p align="center">
  <img src="resources/glossa-logo.png" alt="glossa" height="80">
</p>

> Press a hotkey, speak, and drop the transcription straight into whatever field is focused.

Glossa is a free, open source speech-to-text daemon for Ubuntu on GNOME Wayland. It records audio from your microphone, sends it to a speech-to-text provider, copies the result to the clipboard, and pastes it into the active window.

It works with Groq, OpenAI, and other OpenAI-compatible speech-to-text providers, including self-hosted ones.

I personally recommend using [Groq](https://groq.com/) as it offers the following benefits:

- Generous free limits (2,000 requests per day) and low prices (starting at $0.04 per hour) when you need more
- Amazingly fast LPU, capable of processing minutes of audio in seconds
- Whisper V3 Large and Whisper V3 Large Turbo, which support multilingual input out of the box

## Features

- Starts automatically with your system and works as a background daemon. Pastes transcribed text to the active input inside any window.
- Supports both toggle and push-to-talk modes.
- Works with Groq, OpenAI, and other OpenAI-compatible STT providers (including self-hosted setups).
- Easily configurable via `config.toml`.
- Pastes using standard clipboard shortcuts such as `Ctrl+V`, `Ctrl+Shift+V`, or `Shift+Insert`, which helps with non-English input without switching keyboard layouts.

## Installation

Run the interactive installer:

```
bash <(wget -qO- https://raw.githubusercontent.com/Glaicer/glossa/main/install.sh)
```

The script will automatically install runtime dependencies if missing and configure Glossa via terminal interface.

You may need to log out and back in before paste works if `dotool` was installed during the script run.

## Updating

Update an existing installation in any of these ways:

- `bash <(wget -qO- https://raw.githubusercontent.com/Glaicer/glossa/main/update.sh)`
- `glossa update`
- tray menu: `Update`

The updater downloads the latest stable release, verifies its checksum, replaces the Glossa binary and bundled assets, and restarts `glossa.service`.

## Why I built this

I started Glossa because speech-to-text on GNOME Wayland still feels more awkward than it should.

Here are the issues I kept running into with other tools:

- On Wayland, apps usually cannot register true global hotkeys directly, so you often end up depending on desktop portal support.
- A popular library `wtype` fails in Wayland throwing: "Compositor does not support the virtual keyboard protocol".
- Another popular library `ydotool` throws `ydotoold backend unavailable (may have latency+delay issues)`.
- You can fix `ydotool` installation, but it still has issues: it doesn't support many non-English keyboard layouts and it has broken keybindings which prevents pasting from the clipboard via shortcuts.
- `dotool` has worked for me, however it is shipped as the source code and must be compiled manually.
- There are some very decent apps with local (gguf) STT models support, however these models are slower and less accurate on laptops than cloud APIs.

Glossa is my attempt to make this whole workflow simpler and more reliable on Ubuntu GNOME Wayland.

## Requirements

Glossa depends on:

- `wl-copy`
- `dotool`

The installer checks for both and installs them automatically.

## Roadmap

Planned features:

- settings GUI
- clipboard restore after paste
- direct input support
- FLAC recording
