#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod dependencies;
#[cfg(target_os = "macos")]
mod macos;
mod media;
mod model;
mod queue;

use model::{Job, Queue, Status, Video, VideoCodec, VideoContainer};
use queue::Engine;
use serde::Serialize;
use std::sync::{atomic::Ordering, Arc};
use tauri::{Emitter, Manager, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    queue: Queue,
    platform: String,
    version_label: String,
    first_launch: bool,
    dependencies: Option<Vec<dependencies::Dependency>>,
}

#[tauri::command]
fn bootstrap(app: tauri::AppHandle, state: State<Arc<Engine>>) -> Bootstrap {
    let version = &app.package_info().version;
    Bootstrap {
        queue: state.queue.lock().unwrap().clone(),
        platform: std::env::consts::OS.into(),
        version_label: format!("{}.{} ({})", version.major, version.minor, version.build),
        first_launch: state.first_launch,
        dependencies: state.dependencies.lock().unwrap().clone(),
    }
}

#[tauri::command]
async fn scan_dependencies(
    app: tauri::AppHandle,
    state: State<'_, Arc<Engine>>,
) -> Result<Vec<dependencies::Dependency>, String> {
    check_dependencies(&app, &state, false).await
}

async fn check_dependencies(
    app: &tauri::AppHandle,
    engine: &Engine,
    background: bool,
) -> Result<Vec<dependencies::Dependency>, String> {
    #[cfg(not(target_os = "macos"))]
    use tauri_plugin_notification::NotificationExt;
    let result = dependencies::scan().await;
    let missing: Vec<_> = result
        .iter()
        .filter(|d| d.required && d.version.is_none())
        .map(|d| d.name.as_str())
        .collect();
    if missing.is_empty() {
        let mut queue = engine.queue.lock().unwrap();
        if !queue.setup_complete {
            let previous = queue.clone();
            queue.setup_complete = true;
            if let Err(error) = engine.save(&queue) {
                *queue = previous;
                return Err(error);
            }
        }
    } else if background {
        let message = format!(
            "Missing tools: {}. Open YT Down → Settings to restore them.",
            missing.join(", ")
        );
        #[cfg(target_os = "macos")]
        macos::notify_missing(app, message.clone());
        #[cfg(not(target_os = "macos"))]
        if let Err(error) = app
            .notification()
            .builder()
            .title("YT Down needs a dependency")
            .body(&message)
            .show()
        {
            let _ = app.emit(
                "app-error",
                format!("{message} Notification delivery failed: {error}"),
            );
        }
    }
    *engine.dependencies.lock().unwrap() = Some(result.clone());
    engine.publish(app);
    let _ = app.emit("dependencies-checked", &result);
    Ok(result)
}

#[tauri::command]
fn resize_panel(app: tauri::AppHandle, height: f64) {
    #[cfg(target_os = "macos")]
    macos::resize(&app, height);
    #[cfg(not(target_os = "macos"))]
    let _ = (app, height);
}

#[tauri::command]
async fn inspect_video(url: String, state: State<'_, Arc<Engine>>) -> Result<Video, String> {
    let codec = state.queue.lock().unwrap().video_codec;
    let video = media::fetch(&url, codec).await?;
    let mut cache = state.metadata.lock().unwrap();
    if cache.len() >= 20 {
        cache.clear();
    }
    cache.insert(video.id.clone(), video.clone());
    Ok(video)
}

#[tauri::command]
fn enqueue(
    app: tauri::AppHandle,
    state: State<Arc<Engine>>,
    video_id: String,
    format: String,
    subtitle_id: Option<String>,
) -> Result<Queue, String> {
    let video = state
        .metadata
        .lock()
        .unwrap()
        .get(&video_id)
        .cloned()
        .ok_or("Load the video again before downloading.")?;
    let quality_label = match format.as_str() {
        "mp3" => "MP3 audio".into(),
        "subtitles" => "Subtitles only".into(),
        id => video
            .qualities
            .iter()
            .find(|q| q.id == id)
            .map(|q| q.label.clone())
            .ok_or("Select an available quality.")?,
    };
    let subtitle = match subtitle_id {
        Some(id) => Some(
            video
                .subtitles
                .iter()
                .find(|s| s.id == id)
                .cloned()
                .ok_or("Select an available subtitle language.")?,
        ),
        None => None,
    };
    if format == "subtitles" && subtitle.is_none() {
        return Err("Select a subtitle language to download.".into());
    }
    let mut queue = state.queue.lock().unwrap();
    if !matches!(format.as_str(), "mp3" | "subtitles") && video.video_codec != queue.video_codec {
        return Err("Video codec settings changed. Load the link again before downloading.".into());
    }
    let previous = queue.clone();
    let job = Job {
        id: uuid::Uuid::new_v4().to_string(),
        video,
        format,
        quality_label,
        container: queue.video_container,
        subtitle,
        folder: queue.folder.clone(),
        status: Status::Queued,
        progress: 0.0,
        speed: String::new(),
        eta: String::new(),
        error: None,
        output: None,
    };
    queue.jobs.push(job);
    if let Err(error) = state.save(&queue) {
        *queue = previous;
        return Err(format!("Could not save the queue: {error}"));
    }
    let result = queue.clone();
    drop(queue);
    state.publish(&app);
    state.wake.notify_one();
    Ok(result)
}

#[tauri::command]
fn queue_action(
    app: tauri::AppHandle,
    state: State<Arc<Engine>>,
    action: String,
    id: Option<String>,
) -> Result<Queue, String> {
    let mut queue = state.queue.lock().unwrap();
    let previous = queue.clone();
    let mut cancel = None;
    match action.as_str() {
        "pause" => queue.paused = true,
        "resume" => queue.paused = false,
        "clear" => queue.jobs.retain(|j| {
            matches!(
                j.status,
                Status::Queued | Status::Downloading | Status::Processing
            )
        }),
        _ => {
            let index = queue
                .jobs
                .iter()
                .position(|j| Some(&j.id) == id.as_ref())
                .ok_or("Queue item no longer exists.")?;
            match action.as_str() {
                "cancel" => {
                    if matches!(
                        queue.jobs[index].status,
                        Status::Downloading | Status::Processing
                    ) {
                        cancel = Some(queue.jobs[index].id.clone());
                    } else if queue.jobs[index].status == Status::Queued {
                        queue.jobs[index].status = Status::Canceled;
                    }
                }
                "retry"
                    if matches!(queue.jobs[index].status, Status::Failed | Status::Canceled) =>
                {
                    let job = &mut queue.jobs[index];
                    job.status = Status::Queued;
                    job.progress = 0.0;
                    job.error = None;
                    job.output = None;
                }
                "up" | "down" if queue.jobs[index].status == Status::Queued => {
                    let other = if action == "up" {
                        (0..index)
                            .rev()
                            .find(|i| queue.jobs[*i].status == Status::Queued)
                    } else {
                        (index + 1..queue.jobs.len())
                            .find(|i| queue.jobs[*i].status == Status::Queued)
                    };
                    if let Some(other) = other {
                        queue.jobs.swap(index, other);
                    }
                }
                _ => return Err("That action is unavailable for this item.".into()),
            }
        }
    }
    if let Err(error) = state.save(&queue) {
        *queue = previous;
        return Err(error);
    }
    if let Some(id) = cancel {
        *state.cancel.lock().unwrap() = Some(id);
    }
    let result = queue.clone();
    drop(queue);
    state.publish(&app);
    state.wake.notify_one();
    Ok(result)
}

#[tauri::command]
fn save_preferences(
    app: tauri::AppHandle,
    state: State<Arc<Engine>>,
    container: VideoContainer,
    codec: VideoCodec,
) -> Result<Queue, String> {
    let mut queue = state.queue.lock().unwrap();
    let previous = queue.clone();
    queue.video_container = container;
    queue.video_codec = codec;
    if let Err(error) = state.save(&queue) {
        *queue = previous;
        return Err(error);
    }
    let result = queue.clone();
    drop(queue);
    state.publish(&app);
    Ok(result)
}

#[tauri::command]
async fn complete_setup(
    app: tauri::AppHandle,
    state: State<'_, Arc<Engine>>,
) -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    use tauri_plugin_notification::NotificationExt;
    {
        let mut queue = state.queue.lock().unwrap();
        let previous = queue.clone();
        queue.setup_complete = true;
        if let Err(error) = state.save(&queue) {
            *queue = previous;
            return Err(error);
        }
    }
    // Ask once during setup so later missing-tool alerts can be delivered by macOS.
    #[cfg(target_os = "macos")]
    macos::request_notification_permission(&app);
    #[cfg(not(target_os = "macos"))]
    let _ = app.notification().request_permission();
    Ok(())
}

#[tauri::command]
async fn choose_folder(
    app: tauri::AppHandle,
    state: State<'_, Arc<Engine>>,
) -> Result<Option<String>, String> {
    state.dialog_open.store(true, Ordering::Relaxed);
    let current = state.queue.lock().unwrap().folder.clone();
    #[cfg(target_os = "macos")]
    macos::close(&app);
    let result = rfd::AsyncFileDialog::new()
        .set_title("Save downloads to")
        .set_directory(current)
        .pick_folder()
        .await;
    state.dialog_open.store(false, Ordering::Relaxed);
    #[cfg(target_os = "macos")]
    macos::restore(&app);
    #[cfg(not(target_os = "macos"))]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let Some(folder) = result else {
        return Ok(None);
    };
    let folder = folder.path().to_string_lossy().to_string();
    let mut queue = state.queue.lock().unwrap();
    let previous = queue.folder.clone();
    queue.folder = folder.clone();
    if let Err(error) = state.save(&queue) {
        queue.folder = previous;
        return Err(error);
    }
    drop(queue);
    state.publish(&app);
    Ok(Some(folder))
}

#[tauri::command]
async fn reveal_folder(state: State<'_, Arc<Engine>>, id: Option<String>) -> Result<(), String> {
    let folder = {
        let queue = state.queue.lock().unwrap();
        match id {
            Some(id) => queue
                .jobs
                .iter()
                .find(|j| j.id == id)
                .map(|j| j.folder.clone())
                .ok_or("Queue item no longer exists.")?,
            None => queue.folder.clone(),
        }
    };
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    #[cfg(target_os = "macos")]
    let mut cmd = dependencies::command("open");
    #[cfg(target_os = "windows")]
    let mut cmd = dependencies::command("explorer.exe");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut cmd = dependencies::command("xdg-open");
    cmd.arg(folder).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn dismiss(app: tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    macos::close(&app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle, state: State<Arc<Engine>>) -> Result<(), String> {
    let mut queue = state.queue.lock().unwrap();
    if queue
        .jobs
        .iter()
        .any(|j| matches!(j.status, Status::Downloading | Status::Processing))
    {
        return Err("Cancel the current download before quitting. You can close the panel while it finishes.".into());
    }
    queue.paused = queue.jobs.iter().any(|job| job.status == Status::Queued);
    state.save(&queue)?;
    app.exit(0);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            #[cfg(target_os = "macos")]
            macos::show(app);
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let state = Engine::load(app.path().app_data_dir()?.join("queue.json"))
                .map_err(std::io::Error::other)?;
            app.manage(state.clone());
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                macos::install(app.handle())?;
            }
            let dependency_handle = app.handle().clone();
            let dependency_state = state.clone();
            tauri::async_runtime::spawn(async move {
                let background = !dependency_state.first_launch;
                if let Err(error) =
                    check_dependencies(&dependency_handle, &dependency_state, background).await
                {
                    let _ = dependency_handle.emit("app-error", error);
                }
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(state.run(handle));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                #[cfg(target_os = "macos")]
                {
                    api.prevent_close();
                    macos::close(window.app_handle());
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let engine = window.state::<Arc<Engine>>();
                    if engine
                        .queue
                        .lock()
                        .unwrap()
                        .jobs
                        .iter()
                        .any(|j| matches!(j.status, Status::Downloading | Status::Processing))
                    {
                        api.prevent_close();
                        let _ = window.emit(
                            "app-error",
                            "A download is running. Cancel it before closing the app.",
                        );
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            resize_panel,
            scan_dependencies,
            inspect_video,
            enqueue,
            queue_action,
            complete_setup,
            save_preferences,
            choose_folder,
            reveal_folder,
            dismiss,
            quit_app
        ])
        .build(tauri::generate_context!())
        .expect("Could not launch YT Down")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let engine = app.state::<Arc<Engine>>();
                if engine
                    .queue
                    .lock()
                    .unwrap()
                    .jobs
                    .iter()
                    .any(|job| matches!(job.status, Status::Downloading | Status::Processing))
                {
                    api.prevent_exit();
                    let _ = app.emit("app-error", "Cancel the current download before quitting.");
                }
            }
        });
}
