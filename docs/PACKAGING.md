# Local release packaging

The release is **1.0 (1)**. Source package versions use `1.0.0+1`, macOS uses `CFBundleShortVersionString=1.0` and `CFBundleVersion=1`, and the Windows installer uses file version `1.0.0.1`.

## macOS

Install Rust, Node.js, Xcode command-line tools, and Python 3. Add the Intel target when building from Apple Silicon.

```sh
npm ci
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run build -- --target aarch64-apple-darwin --bundles app
npm run build -- --target x86_64-apple-darwin --bundles app
python3 -m venv .packaging-venv
.packaging-venv/bin/pip install dmgbuild==2.0.0
```

Create the architecture-specific DMGs from their signed app bundles.

```sh
.packaging-venv/bin/python scripts/build-dmg.py \
  'src-tauri/target/aarch64-apple-darwin/release/bundle/macos/YT Down.app' \
  dist/YT-Down-1.0.0-1-macos-arm64.dmg
.packaging-venv/bin/python scripts/build-dmg.py \
  'src-tauri/target/x86_64-apple-darwin/release/bundle/macos/YT Down.app' \
  dist/YT-Down-1.0.0-1-macos-x64.dmg
```

The background has a plain neutral surface, one arrow, and one installation instruction. `dmgbuild` writes the Finder icon positions and view settings without scripting Finder. It automatically combines the 1× and 2× background PNGs.

Signing uses the local ad-hoc identity. Notarization and a Developer ID certificate are not configured.

## Windows

On a Windows build machine, install Visual Studio's Desktop development with C++ workload, Rust stable, Node.js, and npm. Run `npm ci` followed by `npm run build:windows` for Tauri's NSIS installer.

On macOS, install Homebrew's `mingw-w64` and `makensis`, and add Rust's GNU Windows target.

```sh
brew install mingw-w64 makensis
rustup target add x86_64-pc-windows-gnu
./scripts/build-windows.sh
```

The script builds release assets with Tauri's custom protocol enabled, links a Windows GUI executable, includes the matching x64 WebView2 loader, and creates a per-user NSIS installer. Windows platform configuration keeps the app visible, decorated, resizable, and in the taskbar. macOS AppKit code is compiled only for macOS.

## Publish

Build all assets locally, inspect architecture and version metadata, verify Mac app signatures and DMG integrity, and create SHA-256 checksums. Commit both platform branches before tagging the macOS release commit as `v1.0.0`.

Use `gh release create` with `docs/RELEASE-v1.0.0.md` as the notes file and attach the three installers plus `SHA256SUMS.txt`. GitHub Actions workflows are not used.
