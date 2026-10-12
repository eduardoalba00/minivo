mod ffmpeg;

use ffmpeg::{Done, Job, Probe, Progress};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::spawn_blocking;
use tauri::ipc::Channel;
use tauri::State;

#[derive(Default)]
struct Current(Mutex<Option<Arc<Job>>>);

/// Tauri places sidecars next to the app's executable.
fn ffmpeg_path() -> PathBuf {
    let exe = std::env::current_exe().expect("no executable path");
    let dir = exe.parent().expect("executable has no directory");
    let name = format!("ffmpeg{}", std::env::consts::EXE_SUFFIX);
    dir.join(name)
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    let result = spawn_blocking(work).await;
    result.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn probe(path: PathBuf) -> Result<Probe, String> {
    blocking(move || {
        let ffmpeg = ffmpeg_path();
        ffmpeg::probe(&ffmpeg, &path)
    })
    .await
}

#[tauri::command]
async fn compress(
    path: PathBuf,
    target: u64,
    progress: Channel<Progress>,
    current: State<'_, Current>,
) -> Result<Done, String> {
    let job = Arc::new(Job::default());
    *current.0.lock().unwrap() = Some(job.clone());
    blocking(move || {
        let ffmpeg = ffmpeg_path();
        ffmpeg::compress(&ffmpeg, &path, target, &ffmpeg::X264, &job, |p| {
            let _ = progress.send(p);
        })
    })
    .await
}

#[tauri::command]
fn cancel(current: State<'_, Current>) {
    let job = current.0.lock().unwrap().take();
    if let Some(job) = job {
        job.cancel();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Current::default())
        .invoke_handler(tauri::generate_handler![probe, compress, cancel])
        .run(tauri::generate_context!())
        .expect("error while running Minivo");
}
