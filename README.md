<p align="center">
  <img src="src-tauri/icons/128x128.png" width="80" height="80" alt="YT Down icon">
</p>

<h1 align="center">YT Down</h1>
<p align="center">Save YouTube videos, audio, and subtitles.</p>
<p align="center">
  <a href="https://github.com/nipunyatawara-dev/yt-down/releases/tag/v1.0.0">Download 1.0 (1)</a>
  · <a href="#getting-started">Getting started</a>
  · <a href="https://github.com/nipunyatawara-dev/yt-down/issues">Report an issue</a>
</p>

YT Down is a small desktop downloader built with Rust. On **macOS**, it lives in your menu bar and opens as a native popover. On **Windows**, it opens as a regular desktop window. Both versions share the same download controls, settings, and queue.

## Download

Choose the installer for your computer.

| Your computer | Download |
| --- | --- |
| Mac with Apple Silicon · M1 or newer | [Apple Silicon DMG](https://github.com/nipunyatawara-dev/yt-down/releases/download/v1.0.0/YT-Down-1.0.0-1-macos-arm64.dmg) |
| Mac with an Intel processor | [Intel DMG](https://github.com/nipunyatawara-dev/yt-down/releases/download/v1.0.0/YT-Down-1.0.0-1-macos-x64.dmg) |
| Windows · 64-bit | [Windows installer](https://github.com/nipunyatawara-dev/yt-down/releases/download/v1.0.0/YT-Down-1.0.0-1-windows-x64-setup.exe) |

Requires **macOS 12 or later**, or **64-bit Windows 10/11**. Checksums are included on the [release page](https://github.com/nipunyatawara-dev/yt-down/releases/tag/v1.0.0).

### Install on macOS

1. Open the DMG for your Mac.
2. Drag **YT Down** into **Applications**.
3. Open it from Applications, then click the download-arrow icon in the menu bar.

The app is ad-hoc signed and has not been notarized by Apple. If macOS blocks the first launch, use **System Settings → Privacy & Security → Open Anyway** after reviewing the download.

### Install on Windows

1. Open the Windows installer.
2. Choose an installation folder and finish setup.
3. Open **YT Down** from Start.

The installer includes the WebView2 loader and can install Microsoft's WebView2 runtime when it is missing. This runtime installation needs an internet connection. The installer is unsigned, so Windows may display an unknown-publisher warning.

## What you can save

- **Video at your chosen quality.** Load a link to see the resolutions that YouTube offers, including 4K when available. File sizes adjust between KB, MB, and GB.
- **MP4 or MKV.** MP4 is the default. Choose the container in Settings and use Automatic or AV1 for the video codec. AV1 options appear when the source provides them.
- **MP3 audio and SRT subtitles.** Choose MP3 to save audio only, or subtitles only to save one selected language. Creator-provided and automatic captions are labeled separately. Select None when you do not need subtitles.
- **An ordered download queue.** Prepare another link while a download runs. Move waiting items earlier or later, cancel, retry, or pause upcoming downloads. Progress shows transfer speed and remaining time.
- **Settings that stay saved.** Choose your download folder, default container, and codec once. The queue persists across launches, and interrupted work returns paused so you can resume it.

Video downloads preserve the chosen source video quality without re-encoding. MP4 prefers AAC/M4A audio when available. Selected subtitles are saved separately as SRT and embedded in video downloads. MP3 conversion uses FFmpeg's highest-quality variable bitrate setting.

## Getting started

1. **Install the download tools** using the command for your platform below. YT Down checks for them on first launch.
2. **Paste a YouTube link.** Review the title and thumbnail, then choose a quality, MP3, or subtitles only.
3. **Click Download.** The link field clears, ready for the next item. Files go to your chosen folder, with `Downloads/YT Down` as the initial destination.

### Download tools

YT Down uses **yt-dlp**, **FFmpeg/ffprobe**, and **Deno**. These tools are detected on your computer and are installed separately from the app.

**macOS · Homebrew**

```sh
brew install yt-dlp ffmpeg deno
```

**Windows · WinGet**

```powershell
winget install --id yt-dlp.yt-dlp -e
winget install --id Gyan.FFmpeg -e
winget install --id DenoLand.Deno -e
```

Restart YT Down after installation. FFmpeg supplies ffprobe. If Homebrew or WinGet is unavailable, install the tools from their official project distributions and add their executable folders to PATH.

After setup, dependency checks run in the background. Missing tools produce an in-app warning and a system notification, subject to your OS notification settings. Open **Settings → Download Tools → Check Again** to recheck them yourself.

### Using the queue

Pausing the queue holds the next download while the current item finishes. On macOS, downloads continue when you close the popover. On Windows, keep the app open or minimized while downloading. Finish or cancel the active download before quitting.

Each queue item remembers the folder and video container chosen when it was added. Preference changes apply to new queue items.

## A few things to know

- Available qualities, subtitle languages, and codecs depend on the video. Automatic captions can contain errors.
- MP4 and MKV are containers. AV1 is a video codec, and playback depends on your player's codec support.
- Video and audio can download in separate phases before merging or conversion finishes.
- Public, finished YouTube videos are supported. Account-restricted videos, DRM, and live broadcasts can fail. The app does not import browser cookies.
- Keep yt-dlp current as YouTube changes. Error messages from failed downloads are shown in the queue.

## Build from source

The app uses Rust, Tauri, and a plain HTML/CSS/JavaScript interface. It uses the operating system's webview and does not ship a Node.js runtime.

```sh
npm ci
npm run dev
```

For a production build, run `npm run build` with Rust stable and your platform's native build tools installed. macOS requires Xcode command-line tools. Windows requires Visual Studio's C++ build tools and WebView2.

Architecture-specific Mac builds and local packaging commands are documented in [Packaging](docs/PACKAGING.md). Windows cross-compilation and installer creation are provided by `scripts/build-windows.sh`. Release packages are built locally.

### Source branches

| Branch | Platform default |
| --- | --- |
| [`macos`](https://github.com/nipunyatawara-dev/yt-down/tree/macos) | Menu bar popover |
| [`windows`](https://github.com/nipunyatawara-dev/yt-down/tree/windows) | Resizable desktop window with a taskbar entry |

Both branches contain the shared Rust download engine and interface. Platform configuration controls how the app opens.
