use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quality {
    pub id: String,
    pub label: String,
    pub height: u64,
    pub fps: u64,
    pub size: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subtitle {
    pub id: String,
    pub language: String,
    pub name: String,
    pub automatic: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub url: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub thumbnail: String,
    pub qualities: Vec<Quality>,
    pub subtitles: Vec<Subtitle>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Downloading,
    Processing,
    Completed,
    Failed,
    Canceled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub video: Video,
    pub format: String,
    pub quality_label: String,
    pub subtitle: Option<Subtitle>,
    pub folder: String,
    pub status: Status,
    pub progress: f64,
    pub speed: String,
    pub eta: String,
    pub error: Option<String>,
    pub output: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub jobs: Vec<Job>,
    pub paused: bool,
    pub folder: String,
    pub setup_complete: bool,
}

impl Default for Queue {
    fn default() -> Self {
        let folder = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_default()
            .join("YT Down");
        Self {
            jobs: vec![],
            paused: false,
            folder: folder.to_string_lossy().into(),
            setup_complete: false,
        }
    }
}

impl Queue {
    pub fn recover(&mut self) {
        for job in &mut self.jobs {
            if matches!(job.status, Status::Downloading | Status::Processing) {
                job.status = Status::Queued;
                job.speed.clear();
                job.eta.clear();
                self.paused = true;
            }
        }
    }
}
