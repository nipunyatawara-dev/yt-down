# YT Down · macOS

A compact menu bar downloader with a Rust engine and a Tauri system webview. Click the download arrow in the menu bar to open a compact AppKit NSPopover, anchored to its NSStatusItem. AppKit owns the arrow, window chrome, opening animation, and click-away dismissal. The webview is hosted inside the popover while its Tauri backing window stays hidden. Downloads continue when the popover closes.

## Download

Paste a YouTube video, Shorts, or share link. The app loads its thumbnail, title, available resolutions, and subtitle languages. Choose a video quality, MP3 audio, or subtitles only. Choose a subtitle language or None, then download. The link clears immediately so the next video can be prepared while the queue runs.

New video downloads default to MP4. Settings can switch the container to MKV and select Automatic or AV1 for the video codec. The chosen resolution and source video codec are preserved without re-encoding. MP4 prefers AAC/M4A audio for compatibility. Selecting AV1 lists only source AV1 qualities, while audio-only and subtitle-only downloads remain available. Subtitles are saved as separate SRT files and also embedded in video downloads. MP3 conversion uses FFmpeg's highest-quality variable bitrate setting. Automatic captions and translations are labeled separately from creator-provided subtitles. YouTube must make a language available for it to appear.

The queue runs one download at a time. Move waiting items up or down, cancel, retry, pause upcoming downloads, and open finished files' folders. Pausing holds the next download and lets the current item finish. The queue and destination persist. Interrupted work returns paused on relaunch and can resume its partial files. Quit is available under Setup and settings after the active download is canceled or completed.

## First launch

The first-launch setup checks executable paths and versions, including Homebrew paths when launched from Finder. A successful scan is saved immediately, even if the user closes the popover before pressing Done. Later launches open to downloads and check tools in the background. Missing tools trigger a macOS UserNotifications alert and a warning inside the popover. Notification delivery follows the user’s macOS notification permissions. Settings shows cached tool status, with Check Again available explicitly.

```sh
brew install yt-dlp ffmpeg deno
```

FFmpeg includes ffprobe. Deno handles YouTube's JavaScript challenges. Tools are detected on the computer and are not bundled or silently installed. An existing Homebrew installation is needed to use the command above. Restart the app after installing new dependencies.

Choose the save folder in Settings or with the folder row below the composer. File format, codec, and folder preferences persist between launches. It defaults to `~/Downloads/YT Down`. Each queued item remembers its folder and container when added. Existing saved queue entries retain their original MKV format. File sizes and transfer speeds automatically use B, KB, MB, or GB, with decimal units matching macOS file-size conventions.

## Develop and build

Requires Rust stable, Node.js 22+, npm, and the Xcode command-line tools. The UI uses plain HTML/CSS/JavaScript without a frontend framework or production Node runtime.

```sh
npm ci
npm run dev
npm run build
```

Build for both Apple Silicon and Intel Macs with the installed Rust target.

```sh
rustup target add x86_64-apple-darwin
npm run build:mac
```

Bundles are written under `src-tauri/target/`. The local builds are ad-hoc signed, without Apple notarization. macOS may require Open Anyway in Privacy & Security when installed from a downloaded archive.

## Platform branches

- `macos` is the menu bar app, with an AppKit NSPopover, click-away dismissal, and accessory activation.
- `windows` is the desktop app, with a normal resizable window, taskbar entry, and NSIS installer. Its GitHub workflow builds on Windows.

Both branches keep the Rust engine and UI compatible with either platform. Platform-specific Tauri configuration and Rust conditionals provide the native behavior. The default configuration and build instructions on each branch target its platform.

## Behavior and limits

Public, non-live YouTube videos are supported. Videos requiring an account, DRM, or restricted access can fail with the error returned by yt-dlp. Keep yt-dlp current as YouTube changes. Dependency setup does not collect browser cookies. A queue item can select one subtitle language at a time. Video and audio transfers can show separate progress phases before merging or conversion finishes.

Motion follows the emil-design-eng principles with short, origin-aware entrances, press feedback, compositor-based progress, and reduced-motion support. Keyboard actions avoid entrance animation. Remote titles and errors render as text, links are normalized to YouTube watch URLs, and download commands use argument arrays.
