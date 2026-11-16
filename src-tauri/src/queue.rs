use crate::{
    dependencies, media,
    model::{Job, Queue, Status, Video},
};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::Notify,
};

pub struct Engine {
    pub queue: Mutex<Queue>,
    pub metadata: Mutex<HashMap<String, Video>>,
    pub wake: Notify,
    pub cancel: Mutex<Option<String>>,
    pub path: PathBuf,
    pub dialog_open: std::sync::atomic::AtomicBool,
}

impl Engine {
    pub fn load(path: PathBuf) -> Result<Arc<Self>, String> {
        let mut queue: Queue = match std::fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| format!("Could not read the saved queue: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Queue::default(),
            Err(e) => return Err(format!("Could not open the saved queue: {e}")),
        };
        queue.recover();
        Ok(Arc::new(Self {
            queue: Mutex::new(queue),
            metadata: Mutex::new(HashMap::new()),
            wake: Notify::new(),
            cancel: Mutex::new(None),
            path,
            dialog_open: false.into(),
        }))
    }

    pub fn save(&self, queue: &Queue) -> Result<(), String> {
        let parent = self.path.parent().ok_or("Invalid settings path.")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("tmp");
        let data = serde_json::to_vec_pretty(queue).map_err(|e| e.to_string())?;
        use std::io::Write;
        let mut file = std::fs::File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        std::fs::rename(temp, &self.path).map_err(|e| e.to_string())
    }

    pub fn publish(&self, app: &AppHandle) {
        if let Ok(queue) = self.queue.lock() {
            let _ = app.emit("queue-changed", &*queue);
        }
    }

    fn update(&self, app: &AppHandle, id: &str, save: bool, f: impl FnOnce(&mut Job)) {
        if let Ok(mut queue) = self.queue.lock() {
            if let Some(job) = queue.jobs.iter_mut().find(|j| j.id == id) {
                f(job);
            }
            if save {
                if let Err(error) = self.save(&queue) {
                    let _ = app.emit("app-error", error);
                }
            }
            let _ = app.emit("queue-changed", &*queue);
        }
    }

    pub async fn run(self: Arc<Self>, app: AppHandle) {
        loop {
            let next = {
                let mut queue = self.queue.lock().unwrap();
                if queue.paused {
                    None
                } else {
                    let job = queue.jobs.iter_mut().find(|j| j.status == Status::Queued);
                    let result = job.map(|job| {
                        job.status = Status::Downloading;
                        job.error = None;
                        job.clone()
                    });
                    if result.is_some() {
                        if let Err(error) = self.save(&queue) {
                            let _ = app.emit("app-error", error);
                        }
                        let _ = app.emit("queue-changed", &*queue);
                    }
                    result
                }
            };
            if let Some(job) = next {
                let result = self.download(&app, &job).await;
                self.update(&app, &job.id, true, |item| {
                    item.speed.clear();
                    item.eta.clear();
                    match result {
                        Ok(()) => {
                            item.status = Status::Completed;
                            item.progress = 100.0;
                        }
                        Err(error) if error == "Canceled" => {
                            item.status = Status::Canceled;
                        }
                        Err(error) => {
                            item.status = Status::Failed;
                            item.error = Some(error);
                        }
                    }
                });
                *self.cancel.lock().unwrap() = None;
            } else {
                self.wake.notified().await;
            }
        }
    }

    async fn download(&self, app: &AppHandle, job: &Job) -> Result<(), String> {
        std::fs::create_dir_all(&job.folder)
            .map_err(|e| format!("Cannot write to the download folder: {e}"))?;
        let mut cmd = dependencies::downloader()?;
        cmd.args(media::download_args(job))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Could not start yt-dlp: {e}"))?;
        let mut stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or("Could not read download progress.")?,
        )
        .lines();
        let mut stderr = BufReader::new(
            child
                .stderr
                .take()
                .ok_or("Could not read download errors.")?,
        )
        .lines();
        let mut out_done = false;
        let mut err_done = false;
        let mut errors = std::collections::VecDeque::new();
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(150));
        loop {
            tokio::select! {
                line = stdout.next_line(), if !out_done => match line {
                    Ok(Some(line)) => self.progress_line(app, job, &line),
                    Ok(None) => out_done = true,
                    Err(error) => { errors.push_back(error.to_string()); out_done = true; }
                },
                line = stderr.next_line(), if !err_done => match line {
                    Ok(Some(line)) => { if errors.len() >= 30 { errors.pop_front(); } errors.push_back(line); },
                    Ok(None) => err_done = true,
                    Err(error) => { errors.push_back(error.to_string()); err_done = true; }
                },
                _ = tick.tick() => {
                    if self.cancel.lock().unwrap().as_deref() == Some(&job.id) {
                        terminate_tree(&mut child).await;
                        return Err("Canceled".into());
                    }
                    if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                        // Drain the final filepath/progress lines after process exit.
                        while let Ok(Some(line)) = stdout.next_line().await { self.progress_line(app, job, &line); }
                        while let Ok(Some(line)) = stderr.next_line().await { if errors.len() >= 30 { errors.pop_front(); } errors.push_back(line); }
                        if status.success() { return Ok(()); }
                        return Err(media::readable_error(&errors.into_iter().collect::<Vec<_>>().join("\n")));
                    }
                }
            }
        }
    }

    fn progress_line(&self, app: &AppHandle, job: &Job, line: &str) {
        if let Some(json) = line.strip_prefix("__PROGRESS__") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
                self.update(app, &job.id, false, |item| {
                    let downloaded = value["downloaded_bytes"].as_f64().unwrap_or(0.0);
                    let total = value["total_bytes"]
                        .as_f64()
                        .or_else(|| value["total_bytes_estimate"].as_f64())
                        .unwrap_or(0.0);
                    item.progress = if total > 0.0 {
                        (downloaded / total * 100.0).clamp(0.0, 100.0)
                    } else {
                        0.0
                    };
                    item.speed = value["speed"]
                        .as_f64()
                        .map(|n| format!("{:.1} MB/s", n / 1_000_000.0))
                        .unwrap_or_default();
                    item.eta = value["eta"]
                        .as_f64()
                        .map(|s| format!("{}:{:02}", s as u64 / 60, s as u64 % 60))
                        .unwrap_or_default();
                    if value["status"] == "finished" {
                        item.status = Status::Processing;
                    } else {
                        item.status = Status::Downloading;
                    }
                });
            }
        } else if let Some(path) = line.strip_prefix("__FILE__") {
            self.update(app, &job.id, true, |item| item.output = Some(path.into()));
        } else if line.starts_with("[Merger]")
            || line.starts_with("[ExtractAudio]")
            || line.starts_with("[EmbedSubtitle]")
            || line.starts_with("[SubtitlesConvertor]")
        {
            self.update(app, &job.id, false, |item| {
                item.status = Status::Processing;
                item.speed.clear();
                item.eta.clear();
            });
        }
    }
}

async fn terminate_tree(child: &mut tokio::process::Child) {
    if let Some(pid) = child.id() {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
        #[cfg(windows)]
        {
            let _ = dependencies::command("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .output()
                .await;
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_survives_restart_and_pauses_interrupted_work() {
        let mut q = Queue::default();
        let v = Video {
            id: "test".into(),
            url: "url".into(),
            title: "title".into(),
            channel: "channel".into(),
            duration: 1.0,
            thumbnail: "".into(),
            qualities: vec![],
            subtitles: vec![],
        };
        q.jobs.push(Job {
            id: "1".into(),
            video: v,
            format: "mp3".into(),
            quality_label: "MP3".into(),
            subtitle: None,
            folder: "test".into(),
            status: Status::Processing,
            progress: 70.0,
            speed: "fast".into(),
            eta: "1".into(),
            error: None,
            output: None,
        });
        let mut restored: Queue =
            serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        restored.recover();
        assert!(restored.paused);
        assert_eq!(restored.jobs[0].status, Status::Queued);
        assert!(restored.jobs[0].speed.is_empty());
    }
}
