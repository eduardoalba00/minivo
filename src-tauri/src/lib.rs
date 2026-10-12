mod ffmpeg;

use ffmpeg::{Done, Encoder, Job, Probe, Progress};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
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

/// The fastest encoder on this machine, measured once; callers block until the benchmark finishes.
fn encoder() -> &'static Encoder {
    static FASTEST: OnceLock<&'static Encoder> = OnceLock::new();
    FASTEST.get_or_init(|| {
        let ffmpeg = ffmpeg_path();
        ffmpeg::fastest(&ffmpeg)
    })
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
        let encoder = encoder();
        let on_progress = |p| {
            let _ = progress.send(p);
        };
        let result = ffmpeg::compress(&ffmpeg, &path, target, encoder, &job, on_progress);
        // Hardware encoders reject some inputs (odd sizes, very high resolutions); x264 takes anything.
        match result {
            Err(e) if e != ffmpeg::CANCELLED && encoder.name != ffmpeg::X264.name => {
                ffmpeg::compress(&ffmpeg, &path, target, &ffmpeg::X264, &job, on_progress)
            }
            result => result,
        }
    })
    .await
}

#[tauri::command]
async fn encoder_label() -> Result<&'static str, String> {
    blocking(|| Ok(encoder().label)).await
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
        .setup(|_| {
            std::thread::spawn(encoder);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![probe, compress, cancel, encoder_label])
        .run(tauri::generate_context!())
        .expect("error while running Minivo");
}
