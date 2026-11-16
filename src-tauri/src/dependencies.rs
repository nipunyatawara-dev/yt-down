use serde::Serialize;
use std::{
    env,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub required: bool,
    pub purpose: String,
}

pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW for GUI launches.
    cmd
}

pub fn find(name: &str) -> Option<PathBuf> {
    let executable = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.into()
    };
    let mut folders: Vec<PathBuf> = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();
    folders.extend([
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ]);
    if let Some(home) = dirs::home_dir() {
        folders.extend([
            home.join(".local/bin"),
            home.join(".deno/bin"),
            home.join("scoop/shims"),
        ]);
    }
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        folders.extend([
            local.join("Microsoft/WinGet/Links"),
            local.join("Programs/yt-dlp"),
            local.join(
                "Microsoft/WinGet/Packages/DenoLand.Deno_Microsoft.Winget.Source_8wekyb3d8bbwe",
            ),
        ]);
        // WinGet portable packages do not always create a Links entry.
        if let Ok(packages) = std::fs::read_dir(local.join("Microsoft/WinGet/Packages")) {
            for package in packages.flatten() {
                if let Some(found) = find_in_package(&package.path(), &executable, 3) {
                    return Some(found);
                }
            }
        }
    }
    if let Some(data) = env::var_os("ProgramData") {
        folders.push(PathBuf::from(data).join("chocolatey/bin"));
    }
    folders
        .into_iter()
        .map(|p| p.join(&executable))
        .find(|p| p.is_file())
}

fn find_in_package(folder: &Path, executable: &str, depth: u8) -> Option<PathBuf> {
    if folder.join(executable).is_file() {
        return Some(folder.join(executable));
    }
    if depth == 0 {
        return None;
    }
    for item in std::fs::read_dir(folder).ok()?.flatten() {
        if item.file_type().ok()?.is_dir() {
            if let Some(found) = find_in_package(&item.path(), executable, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

pub async fn scan() -> Vec<Dependency> {
    let specs = [
        (
            "yt-dlp",
            "--version",
            true,
            "Video information and downloads",
        ),
        (
            "ffmpeg",
            "-version",
            true,
            "4K audio merging, MP3, and subtitles",
        ),
        (
            "ffprobe",
            "-version",
            true,
            "Media inspection for conversion",
        ),
        ("deno", "--version", true, "YouTube JavaScript challenges"),
    ];
    let mut results = vec![];
    for (name, flag, required, purpose) in specs {
        let path = find(name);
        let version = if let Some(path) = &path {
            match tokio::time::timeout(
                std::time::Duration::from_secs(8),
                command(path).arg(flag).kill_on_drop(true).output(),
            )
            .await
            {
                Ok(Ok(output)) if output.status.success() => Some(
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .next()
                        .unwrap_or("Installed")
                        .to_string(),
                ),
                _ => None,
            }
        } else {
            None
        };
        results.push(Dependency {
            name: name.into(),
            version,
            path: path.map(|p| p.to_string_lossy().into()),
            required,
            purpose: purpose.into(),
        });
    }
    results
}

pub fn downloader() -> Result<Command, String> {
    let path = find("yt-dlp").ok_or("yt-dlp is missing. Open Setup to check dependencies.")?;
    let mut cmd = command(path);
    // Ignore machine-wide configuration so a shell user's settings cannot change app behavior.
    cmd.args([
        "--ignore-config",
        "--no-playlist",
        "--encoding",
        "utf-8",
        "--no-color",
    ]);
    if let Some(ffmpeg) = find("ffmpeg") {
        if let Some(parent) = ffmpeg.parent() {
            cmd.arg("--ffmpeg-location").arg(parent);
        }
    }
    if let Some(deno) = find("deno") {
        cmd.arg("--js-runtimes")
            .arg(format!("deno:{}", deno.display()));
    }
    Ok(cmd)
}
