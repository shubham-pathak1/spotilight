# <img src="src-tauri/icons/128x128.png" width="48" height="48" align="left" style="margin-right: 15px;"/> Spotilight

> A lightweight, fast Spotify desktop client built with Tauri — because the official app shouldn't need 500MB of RAM to play a song.

---

## What is Spotilight?

Spotilight is a thin Tauri wrapper around Spotify's web player (`open.spotify.com`). It uses your OS's native webview instead of bundling a full Chromium engine like the official Electron app does, making it significantly lighter, faster to launch, and easier on your system resources.

No custom API. No reverse engineering. No grey areas. Just Spotify — lighter.

---

## Why?

The official Spotify desktop app is built with Electron. That means it ships with an entire browser engine, eating ~300–500MB of RAM just sitting idle. Spotilight uses your system's native webview (WebKit on macOS/Linux, WebView2 on Windows), so the overhead is a fraction of that.

If you've ever thought *"why does a music player need this much RAM"* — this is for you.

---

## Features

- ✅ Full Spotify web player — login, playback, playlists, search, podcasts, everything
- ✅ System tray with minimize-to-tray support
- ✅ Media key support (play/pause, next, previous)
- ✅ Remembers window size and position
- ✅ Starts minimized to tray (optional)
- ✅ **Monochrome mode** — inject a grayscale filter for a clean, distraction-free look
- ✅ Tiny binary size compared to Electron
- ✅ Cross-platform: Windows, macOS, Linux

---

## What Won't Work

Spotilight wraps the Spotify **web player**, so anything the web player doesn't support won't work here either:

- ❌ Local files playback
- ❌ Offline / downloaded music
- ❌ A small number of obscure desktop-only features

---

## Monochrome Mode

Spotilight includes an optional monochrome filter that applies a grayscale effect to the entire player.

Toggle it from the system tray menu or with a keyboard shortcut. (ctrl+shift+m)
---

## Tech Stack

| Layer | Technology |
|---|---|
| App framework | [Tauri v2](https://tauri.app) |
| Language (backend) | Rust |
| Webview | OS native (WKWebView / WebView2 / WebKitGTK) |
| Package Manager | Bun |
| Frontend shell | Minimal HTML/JS |

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/)
- [Bun](https://bun.sh/)
- Platform-specific Tauri prerequisites: [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)

### Run in development

```bash
git clone https://github.com/yourusername/spotilight
cd spotilight
bun install
bun run tauri dev
```

### Build for production

```bash
bun run tauri build
```

Output binaries will be in `src-tauri/target/release/bundle/`.

---

## Contributing

PRs welcome. Keep it simple — the whole point of this project is to stay lean. Features that require touching Spotify's internals or adding heavy dependencies are out of scope.

---

## Disclaimer

Spotilight is an unofficial third-party app and is not affiliated with, endorsed by, or connected to Spotify. It simply loads Spotify's own web player in a native window. You still need a Spotify account (Free or Premium) to use it.

---

## License

MIT
