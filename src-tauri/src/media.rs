use crate::{
    dependencies,
    model::{Job, Quality, Subtitle, Video},
};
use serde_json::Value;
use std::collections::BTreeMap;
use url::Url;

pub fn canonical_url(input: &str) -> Result<String, String> {
    let url = Url::parse(input.trim()).map_err(|_| "Paste a complete YouTube video link.")?;
    if url.scheme() != "https" && url.scheme() != "http" {
        return Err("Use a YouTube https link.".into());
    }
    let host = url.host_str().unwrap_or("");
    let id = match host {
        "youtu.be" => url
            .path_segments()
            .and_then(|mut s| s.next())
            .unwrap_or("")
            .to_string(),
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "music.youtube.com" => {
            if url.path() == "/watch" {
                url.query_pairs()
                    .find(|(key, _)| key == "v")
                    .map(|(_, value)| value.into_owned())
                    .unwrap_or_default()
            } else {
                let parts: Vec<_> = url.path_segments().map(|s| s.collect()).unwrap_or_default();
                if parts.len() >= 2 && matches!(parts[0], "shorts" | "live" | "embed") {
                    parts[1].into()
                } else {
                    String::new()
                }
            }
        }
        _ => return Err("This app accepts YouTube video links.".into()),
    };
    if id.len() != 11
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("The link needs a valid YouTube video ID.".into());
    }
    Ok(format!("https://www.youtube.com/watch?v={id}"))
}

pub async fn fetch(input: &str) -> Result<Video, String> {
    let url = canonical_url(input)?;
    let mut cmd = dependencies::downloader()?;
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        cmd.args(["--dump-single-json", "--skip-download", "--", &url])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "YouTube took too long to respond. Try the link again.")?
    .map_err(|e| format!("Could not start yt-dlp: {e}"))?;
    if !output.status.success() {
        return Err(readable_error(&String::from_utf8_lossy(&output.stderr)));
    }
    let data: Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| "yt-dlp returned invalid video information.")?;
    parse_metadata(&data, url)
}

pub fn parse_metadata(data: &Value, url: String) -> Result<Video, String> {
    if data["is_live"].as_bool() == Some(true) {
        return Err("Live broadcasts can be downloaded after they finish.".into());
    }
    let mut qualities = BTreeMap::new();
    for format in data["formats"].as_array().into_iter().flatten() {
        let height = format["height"].as_u64().unwrap_or(0);
        let fps = format["fps"].as_f64().unwrap_or(0.0).round() as u64;
        let protocol = format["protocol"].as_str().unwrap_or("");
        if height == 0
            || format["vcodec"] == "none"
            || !matches!(protocol, "https" | "http")
            || format["has_drm"].as_bool() == Some(true)
        {
            continue;
        }
        let Some(id) = format["format_id"].as_str() else {
            continue;
        };
        let suffix = if fps > 30 {
            format!(" · {fps} fps")
        } else {
            String::new()
        };
        let label = match height {
            2160 => format!("4K · 2160p{suffix}"),
            4320 => format!("8K · 4320p{suffix}"),
            _ => format!("{height}p{suffix}"),
        };
        qualities.insert(
            (height, fps),
            Quality {
                id: id.into(),
                label,
                height,
                fps,
                size: format["filesize"]
                    .as_f64()
                    .or_else(|| format["filesize_approx"].as_f64()),
            },
        );
    }
    if qualities.is_empty() {
        return Err(
            "No downloadable video formats were found. Update yt-dlp and try again.".into(),
        );
    }
    let mut subtitles = vec![];
    for (key, automatic) in [("subtitles", false), ("automatic_captions", true)] {
        for (language, tracks) in data[key].as_object().into_iter().flatten() {
            if language == "live_chat"
                || !tracks.as_array().is_some_and(|tracks| {
                    tracks
                        .iter()
                        .any(|t| t["ext"] == "vtt" || t["ext"] == "srt")
                })
            {
                continue;
            }
            let name = tracks
                .as_array()
                .and_then(|t| t.first())
                .and_then(|t| t["name"].as_str())
                .unwrap_or(language)
                .to_string();
            subtitles.push(Subtitle {
                id: format!("{}:{language}", if automatic { "a" } else { "m" }),
                language: language.clone(),
                name,
                automatic,
            });
        }
    }
    subtitles.sort_by_key(|s| {
        (
            s.automatic,
            !s.language.starts_with("en"),
            s.name.to_lowercase(),
        )
    });
    let thumbnail = data["thumbnail"]
        .as_str()
        .filter(|url| {
            url.starts_with("https://i.ytimg.com/") || url.starts_with("https://i9.ytimg.com/")
        })
        .unwrap_or("")
        .to_string();
    Ok(Video {
        id: data["id"].as_str().ok_or("Video ID missing.")?.into(),
        url,
        title: data["title"].as_str().unwrap_or("YouTube video").into(),
        channel: data["channel"]
            .as_str()
            .or_else(|| data["uploader"].as_str())
            .unwrap_or("YouTube")
            .into(),
        duration: data["duration"].as_f64().unwrap_or(0.0),
        thumbnail,
        qualities: qualities.into_values().rev().collect(),
        subtitles,
    })
}

pub fn download_args(job: &Job) -> Vec<String> {
    let mut args: Vec<String> = [
        "--newline",
        "--progress",
        "--progress-delta",
        "0.3",
        "--no-simulate",
        "--windows-filenames",
        "--continue",
        "--no-overwrites",
        "--socket-timeout",
        "30",
        "--retries",
        "5",
        "--fragment-retries",
        "5",
        "--progress-template",
        "download:__PROGRESS__%(progress)j",
        "--print",
        "after_move:__FILE__%(filepath)s",
        "-P",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    args.push(job.folder.clone());
    args.extend(["-o".into(), "%(title).160B [%(id)s].%(ext)s".into()]);
    match job.format.as_str() {
        "mp3" => args.extend(
            [
                "-f",
                "bestaudio/best",
                "--extract-audio",
                "--audio-format",
                "mp3",
                "--audio-quality",
                "0",
            ]
            .map(String::from),
        ),
        "subtitles" => args.push("--skip-download".into()),
        format => args.extend([
            "-f".into(),
            format!("{format}+bestaudio/{format}"),
            "--merge-output-format".into(),
            "mkv".into(),
        ]),
    }
    if let Some(subtitle) = &job.subtitle {
        args.extend(
            [
                if subtitle.automatic {
                    "--write-auto-subs"
                } else {
                    "--write-subs"
                },
                "--sub-langs",
            ]
            .map(String::from),
        );
        // yt-dlp treats --sub-langs as regex. Quote the language key literally.
        args.push(format!(
            "^{}$",
            subtitle
                .language
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c.to_string()
                } else {
                    format!("\\{c}")
                })
                .collect::<String>()
        ));
        args.extend(["--sub-format", "srt/vtt/best", "--convert-subs", "srt"].map(String::from));
        if !matches!(job.format.as_str(), "subtitles" | "mp3") {
            args.push("--embed-subs".into());
        }
    }
    args.extend(["--".into(), job.video.url.clone()]);
    args
}

pub fn readable_error(stderr: &str) -> String {
    let message = stderr
        .lines()
        .rev()
        .find(|l| l.contains("ERROR:"))
        .or_else(|| stderr.lines().last())
        .unwrap_or("Download failed. Check your connection and try again.");
    message.chars().take(800).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires YouTube access and installed yt-dlp/FFmpeg/Deno"]
    async fn live_downloads_video_mp3_and_subtitles() {
        use crate::model::{Status, Subtitle};
        let video = fetch("https://www.youtube.com/watch?v=BaW_jenozKc")
            .await
            .unwrap();
        let folder = std::env::temp_dir().join(format!("yt-down-smoke-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut job = Job {
            id: "smoke".into(),
            video: video.clone(),
            format: video.qualities.last().unwrap().id.clone(),
            quality_label: "Test".into(),
            subtitle: None,
            folder: folder.to_string_lossy().into(),
            status: Status::Queued,
            progress: 0.0,
            speed: String::new(),
            eta: String::new(),
            error: None,
            output: None,
        };
        for format in [job.format.clone(), "mp3".into()] {
            job.format = format;
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(180),
                dependencies::downloader()
                    .unwrap()
                    .args(download_args(&job))
                    .kill_on_drop(true)
                    .output(),
            )
            .await
            .unwrap()
            .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                stdout.contains("__PROGRESS__"),
                "Missing progress: {stdout}"
            );
            assert!(stdout.contains("__FILE__"), "Missing final file: {stdout}");
        }
        assert!(std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .any(|f| f.path().extension().is_some_and(|e| e == "mp3")));
        let captions = fetch("https://www.youtube.com/watch?v=fwqEB2HdY88")
            .await
            .unwrap();
        let sub: Subtitle = captions
            .subtitles
            .iter()
            .find(|s| s.language == "en-orig")
            .unwrap()
            .clone();
        job.video = captions;
        job.format = "subtitles".into();
        job.subtitle = Some(sub);
        let output = dependencies::downloader()
            .unwrap()
            .args(download_args(&job))
            .kill_on_drop(true)
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .any(|f| f.path().extension().is_some_and(|e| e == "srt")));
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn accepts_video_urls_and_rejects_other_hosts() {
        for input in [
            "https://youtu.be/fwqEB2HdY88?t=2",
            "https://www.youtube.com/watch?v=fwqEB2HdY88&list=abc",
            "https://youtube.com/shorts/fwqEB2HdY88",
        ] {
            assert_eq!(
                canonical_url(input).unwrap(),
                "https://www.youtube.com/watch?v=fwqEB2HdY88"
            );
        }
        assert!(canonical_url("https://youtube.com.evil.com/watch?v=fwqEB2HdY88").is_err());
        assert!(canonical_url("file:///etc/passwd").is_err());
        assert!(canonical_url("https://youtube.com/playlist?list=abc").is_err());
    }
    #[test]
    fn metadata_groups_formats_and_keeps_subtitle_sources() {
        let data = serde_json::json!({"id":"fwqEB2HdY88", "title":"Example", "formats":[
            {"format_id":"a","height":2160,"fps":25,"vcodec":"vp9","protocol":"https"},
            {"format_id":"b","height":2160,"fps":25,"vcodec":"av1","protocol":"https"},
            {"format_id":"c","height":1080,"fps":60,"vcodec":"avc1","protocol":"https"}
        ], "subtitles":{"en":[{"ext":"vtt","name":"English"}]},"automatic_captions":{"en":[{"ext":"vtt","name":"English"}],"ta":[{"ext":"vtt","name":"Tamil"}]}});
        let video = parse_metadata(&data, "test".into()).unwrap();
        assert_eq!(video.qualities.len(), 2);
        assert_eq!(video.qualities[0].id, "b");
        assert_eq!(video.subtitles.len(), 3);
        assert!(!video.subtitles[0].automatic);
    }
}
