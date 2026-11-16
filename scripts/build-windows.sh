#!/usr/bin/env bash
# Cross-compile on macOS with Homebrew mingw-w64 and NSIS, without CI.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export CARGO_TARGET_DIR="$PWD/src-tauri/target-windows"
cargo build --manifest-path src-tauri/Cargo.toml --release --locked --features custom-protocol --target x86_64-pc-windows-gnu
build="$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release"
loader=("$build"/build/webview2-com-sys-*/out/x64/WebView2Loader.dll)
if [[ ! -f "${loader[0]}" ]]; then
  printf 'The WebView2 loader was not found in the build output.\n' >&2
  exit 1
fi
mkdir -p dist packaging/.cache
bootstrapper="$PWD/packaging/.cache/MicrosoftEdgeWebview2Setup.exe"
if [[ ! -f "$bootstrapper" ]]; then
  curl --fail --location --proto '=https' --tlsv1.2 \
    'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -o "$bootstrapper"
fi
makensis "-DAPP_EXE=$build/yt-down.exe" "-DWEBVIEW2_LOADER=${loader[0]}" \
  "-DWEBVIEW2_BOOTSTRAPPER=$bootstrapper" \
  "-DOUTPUT_EXE=$PWD/dist/YT-Down-1.0.0-1-windows-x64-setup.exe" packaging/windows.nsi
