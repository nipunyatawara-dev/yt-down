The first release of YT Down brings video, audio, and subtitle downloads into a small desktop app.

**macOS** opens from the menu bar as a native popover. **Windows** opens as a regular resizable window. Both versions use the same download controls and queue.

## Choose your download

| Platform | File |
| --- | --- |
| Apple Silicon Mac · M1 or newer | `YT-Down-1.0.0-1-macos-arm64.dmg` |
| Intel Mac | `YT-Down-1.0.0-1-macos-x64.dmg` |
| Windows · 64-bit | `YT-Down-1.0.0-1-windows-x64-setup.exe` |

Mac users can open the DMG and drag YT Down to Applications. Windows users can run the installer and launch the app from Start. The footer shows **1.0 (1)**.

## Included in this release

- **Choose the quality.** Preview the video title and thumbnail, then select an available resolution. MP4 is the default video container, with MKV and an Automatic/AV1 codec choice in Settings.
- **Save audio or subtitles.** Download MP3 audio, save subtitles as SRT, or include a selected subtitle track with a video. Creator-provided and automatic captions are labeled separately.
- **Queue the next video.** The link field clears after adding an item. Downloads run one at a time with progress, transfer speed, remaining time, reordering, cancellation, retry, and queue pause controls.
- **Keep your preferences.** The save folder, container, codec, and queue persist between launches. Interrupted work returns paused and can resume its partial files.
- **Stay informed.** First-launch tool detection is followed by background checks. Missing tools produce system notifications and an in-app warning. File sizes and speeds use automatically selected units.

## Before your first download

Install **yt-dlp**, **FFmpeg/ffprobe**, and **Deno**. These are separate dependencies. The [README](https://github.com/nipunyatawara-dev/yt-down#download-tools) includes Homebrew and WinGet commands.

Requires macOS 12+ or 64-bit Windows 10/11. The Windows installer includes the WebView2 loader and installs Microsoft's runtime when needed.

## Release notes

The Mac apps are ad-hoc signed and are not Apple-notarized. The Windows installer is unsigned. Your operating system may show a first-launch or publisher warning.

Packages were built locally without GitHub Actions. Native Windows UI and Intel Mac runtime behavior have not been tested on those platforms. SHA-256 checksums are included in `SHA256SUMS.txt`.

Video qualities, subtitle tracks, and AV1 availability depend on the source. MP4 preserves the downloaded source video codec, so playback support depends on the player.
