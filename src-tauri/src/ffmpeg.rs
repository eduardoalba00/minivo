use serde::Serialize;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Share of the target spent on the streams; the rest covers the MP4 container and encoder overshoot.
const HEADROOM: f64 = 0.95;
const MIN_VIDEO_KBPS: f64 = 150.0;
const ATTEMPTS: u32 = 3;

pub struct Encoder {
    pub name: &'static str,
    pub label: &'static str,
    pub options: &'static [&'static str],
}

pub const X264: Encoder = Encoder {
    name: "libx264",
    label: "x264 (CPU)",
    options: &["-preset", "veryfast"],
};

/// Tried in order; ones the machine can't run fail within milliseconds.
static ENCODERS: [Encoder; 5] = [
    Encoder {
        name: "h264_videotoolbox",
        label: "Apple VideoToolbox",
        options: &["-constant_bit_rate", "1"],
    },
    Encoder {
        name: "h264_nvenc",
        label: "NVIDIA NVENC",
        options: &["-preset", "p4", "-rc", "cbr"],
    },
    Encoder {
        name: "h264_qsv",
        label: "Intel Quick Sync",
        options: &["-preset", "faster"],
    },
    Encoder {
        name: "h264_amf",
        label: "AMD AMF",
        options: &["-quality", "speed", "-rc", "cbr"],
    },
    X264,
];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub has_audio: bool,
    pub bytes: u64,
}

#[derive(Debug, PartialEq)]
pub struct Plan {
    pub video_kbps: u32,
    pub audio_kbps: u32,
    /// Longest the shorter side may be, so low bitrates don't spread over too many pixels.
    pub max_short_side: Option<u32>,
    pub cap_fps: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub output: PathBuf,
    pub bytes: u64,
    pub short_side: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub fraction: f64,
    pub speed: f64,
    pub attempt: u32,
}

/// A running compression that another thread can cancel.
#[derive(Default)]
pub struct Job {
    child: Mutex<Option<Child>>,
    cancelled: AtomicBool,
}

impl Job {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let mut child = self.child.lock().unwrap();
        if let Some(child) = child.as_mut() {
            let _ = child.kill();
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub const CANCELLED: &str = "cancelled";

pub fn command(ffmpeg: &Path) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new(ffmpeg);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Times a short 1080p encode with each encoder and returns the fastest that works.
pub fn fastest(ffmpeg: &Path) -> &'static Encoder {
    let mut best = (&ENCODERS[4], Duration::MAX);
    for encoder in &ENCODERS {
        let elapsed = time_encode(ffmpeg, encoder);
        if let Some(elapsed) = elapsed.filter(|e| *e < best.1) {
            best = (encoder, elapsed);
        }
    }
    best.0
}

fn time_encode(ffmpeg: &Path, encoder: &Encoder) -> Option<Duration> {
    let start = Instant::now();
    let status = command(ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=s=1920x1080:r=30:d=4,noise=alls=12:allf=t",
        ])
        .args(["-c:v", encoder.name])
        .args(encoder.options)
        .args([
            "-b:v", "4000k", "-maxrate", "4000k", "-bufsize", "8000k", "-pix_fmt", "yuv420p", "-f", "null", "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    status.success().then(|| start.elapsed())
}

pub fn probe(ffmpeg: &Path, input: &Path) -> Result<Probe, String> {
    let metadata = std::fs::metadata(input).map_err(|e| e.to_string())?;
    // ffmpeg prints stream info to stderr and exits non-zero when given no output.
    let output = command(ffmpeg)
        .arg("-hide_banner")
        .arg("-i")
        .arg(input)
        .output()
        .map_err(|e| e.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let bytes = metadata.len();
    let probe = parse_probe(&stderr, bytes);
    probe.ok_or_else(|| "This doesn't look like a video Minivo can read.".into())
}

fn parse_probe(info: &str, bytes: u64) -> Option<Probe> {
    let duration = info.split("Duration: ").nth(1)?.split(',').next()?;
    let duration = parse_clock(duration)?;
    let video = info.lines().find(|l| l.contains("Stream #") && l.contains("Video:"))?;
    let (mut width, mut height) = video.split(", ").find_map(parse_size)?;
    let sideways = is_quarter_turn(info);
    if sideways {
        (width, height) = (height, width);
    }
    let has_audio = info.lines().any(|l| l.contains("Stream #") && l.contains("Audio:"));
    (duration > 0.0).then_some(Probe {
        duration,
        width,
        height,
        has_audio,
        bytes,
    })
}

fn parse_clock(clock: &str) -> Option<f64> {
    let mut parts = clock.trim().split(':');
    let hours: f64 = parts.next()?.parse().ok()?;
    let minutes: f64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// "1920x1080" or "1920x1080 [SAR 1:1 DAR 16:9]".
fn parse_size(field: &str) -> Option<(u32, u32)> {
    let size = field.split(' ').next()?;
    let (w, h) = size.split_once('x')?;
    let width = w.parse().ok()?;
    let height = h.parse().ok()?;
    Some((width, height))
}

/// Phone videos are often stored sideways with a rotation flag that ffmpeg applies when decoding.
fn is_quarter_turn(info: &str) -> bool {
    let Some(rest) = info.split("rotation of ").nth(1) else {
        return false;
    };
    let degrees = rest.split(' ').next().unwrap_or_default();
    let degrees: f64 = degrees.parse().unwrap_or(0.0);
    let turns = (degrees / 90.0).round() as i64;
    turns % 2 != 0
}

/// Bitrates for `target` bytes over `duration` seconds; `scale` shrinks them on a retry.
pub fn plan(target: u64, duration: f64, has_audio: bool, scale: f64) -> Result<Plan, String> {
    let total = target as f64 * 8.0 * HEADROOM * scale / duration / 1000.0;
    let audio = match (has_audio, total) {
        (false, _) => 0.0,
        (true, t) if t < 1000.0 => 64.0,
        (true, t) if t < 3000.0 => 96.0,
        _ => 128.0,
    };
    let video = total - audio;
    if video < MIN_VIDEO_KBPS {
        let min_bytes = (MIN_VIDEO_KBPS + audio) * 1000.0 * duration / 8.0 / HEADROOM;
        let min_mb = (min_bytes / 1e6).ceil();
        return Err(format!("Too long to fit in that size. Try {min_mb} MB or more."));
    }
    let max_short_side = match video {
        v if v < 400.0 => Some(360),
        v if v < 800.0 => Some(480),
        v if v < 1500.0 => Some(720),
        v if v < 3000.0 => Some(1080),
        v if v < 6000.0 => Some(1440),
        _ => None,
    };
    Ok(Plan {
        video_kbps: video as u32,
        audio_kbps: audio as u32,
        max_short_side,
        cap_fps: video < 4000.0,
    })
}

fn filters(plan: &Plan) -> Vec<String> {
    let mut filters = Vec::new();
    if let Some(m) = plan.max_short_side {
        // Scale the shorter side down to `m` (never up), whichever way the video is oriented.
        filters.push(format!(
            "scale=w='if(gte(iw,ih),-2,min(iw,{m}))':h='if(gte(iw,ih),min(ih,{m}),-2)'"
        ));
    }
    if plan.cap_fps {
        filters.push("fps='min(source_fps,30)'".into());
    }
    filters
}

fn encode_args(input: &Path, output: &Path, plan: &Plan, encoder: &Encoder) -> Vec<String> {
    let video = format!("{}k", plan.video_kbps);
    let buffer = format!("{}k", plan.video_kbps * 2);
    let input = input.to_string_lossy();
    let output = output.to_string_lossy();
    let filters = filters(plan).join(",");
    let mut args = vec![
        "-hide_banner",
        "-v",
        "error",
        "-y",
        "-i",
        &input,
        "-map",
        "0:v:0",
        "-map",
        "0:a:0?",
    ];
    if !filters.is_empty() {
        args.extend(["-vf", &filters]);
    }
    args.extend(["-c:v", encoder.name]);
    args.extend(encoder.options);
    args.extend([
        "-b:v", &video, "-maxrate", &video, "-bufsize", &buffer, "-pix_fmt", "yuv420p",
    ]);
    let audio = format!("{}k", plan.audio_kbps);
    if plan.audio_kbps > 0 {
        args.extend(["-c:a", "aac", "-b:a", &audio]);
    }
    args.extend(["-movflags", "+faststart", "-progress", "pipe:1", "-nostats", &output]);
    args.into_iter().map(String::from).collect()
}

/// `clip.mov` at 20 MB becomes `clip-20MB.mp4` beside it, or `clip-20MB-2.mp4` if that's taken.
pub fn output_path(input: &Path, target: u64) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let mb = target as f64 / 1e6;
    let dir = input.parent().unwrap_or(Path::new("."));
    let name = format!("{stem}-{mb}MB.mp4");
    let mut path = dir.join(name);
    let mut n = 2;
    let mut taken = path.exists();
    while taken {
        let name = format!("{stem}-{mb}MB-{n}.mp4");
        path = dir.join(name);
        taken = path.exists();
        n += 1;
    }
    path
}

/// Encodes `input` to fit in `target` bytes, retrying at a lower bitrate if the encoder overshoots.
pub fn compress(
    ffmpeg: &Path,
    input: &Path,
    target: u64,
    encoder: &Encoder,
    job: &Job,
    on_progress: impl Fn(Progress),
) -> Result<Done, String> {
    let info = probe(ffmpeg, input)?;
    let output = output_path(input, target);
    let mut scale = 1.0;
    for attempt in 1..=ATTEMPTS {
        let plan = plan(target, info.duration, info.has_audio, scale)?;
        let args = encode_args(input, &output, &plan, encoder);
        let result = run(ffmpeg, &args, job, info.duration, |fraction, speed| {
            on_progress(Progress {
                fraction,
                speed,
                attempt,
            })
        });
        if let Err(e) = result {
            let _ = std::fs::remove_file(&output);
            return Err(e);
        }
        let metadata = std::fs::metadata(&output).map_err(|e| e.to_string())?;
        let bytes = metadata.len();
        if bytes <= target {
            let short_side = info.width.min(info.height);
            let short_side = plan.max_short_side.map_or(short_side, |m| m.min(short_side));
            return Ok(Done {
                output,
                bytes,
                short_side,
            });
        }
        scale *= target as f64 / bytes as f64 * 0.97;
    }
    let _ = std::fs::remove_file(&output);
    Err("Couldn't get this video under the size. Try a larger size.".into())
}

fn run(ffmpeg: &Path, args: &[String], job: &Job, duration: f64, on_progress: impl Fn(f64, f64)) -> Result<(), String> {
    let mut child = command(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let errors = thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    *job.child.lock().unwrap() = Some(child);
    // A cancel that landed before the child was stored had nothing to kill.
    let cancelled = job.is_cancelled();
    if cancelled {
        job.cancel();
    }

    let (mut fraction, mut speed) = (0.0, 0.0);
    let lines = BufReader::new(stdout).lines();
    for line in lines.map_while(Result::ok) {
        if let Some(us) = line.strip_prefix("out_time_us=") {
            let seconds = us.parse::<f64>().unwrap_or(0.0) / 1e6;
            fraction = (seconds / duration).clamp(0.0, 1.0);
        } else if let Some(s) = line.strip_prefix("speed=") {
            speed = s.trim().trim_end_matches('x').parse().unwrap_or(speed);
        } else if line.starts_with("progress=") {
            on_progress(fraction, speed);
        }
    }

    let child = job.child.lock().unwrap().take();
    let status = child.unwrap().wait().map_err(|e| e.to_string())?;
    let errors = errors.join().unwrap_or_default();
    let cancelled = job.is_cancelled();
    if cancelled {
        return Err(CANCELLED.into());
    }
    if !status.success() {
        let last = errors.lines().last().unwrap_or("ffmpeg failed");
        return Err(format!("Compression failed: {last}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANDSCAPE: &str = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':
  Duration: 00:01:02.50, start: 0.000000, bitrate: 41072 kb/s
  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 41047 kb/s, 60 fps
  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 44100 Hz, stereo, fltp, 128 kb/s (default)";

    const PORTRAIT: &str = "  Duration: 00:00:08.01, start: 0.000000, bitrate: 41072 kb/s
  Stream #0:0[0x1](und): Video: hevc (Main 10) (hvc1 / 0x31637668), yuv420p10le(tv, bt2020nc/bt2020/arib-std-b67), 3840x2160, 41047 kb/s, 30 fps
      Side data:
        Display Matrix: rotation of -90.00 degrees";

    #[test]
    fn reads_landscape_with_audio() {
        let probe = parse_probe(LANDSCAPE, 1).unwrap();
        assert_eq!(
            probe,
            Probe {
                duration: 62.5,
                width: 1920,
                height: 1080,
                has_audio: true,
                bytes: 1
            }
        );
    }

    #[test]
    fn swaps_sides_for_rotated_video() {
        let probe = parse_probe(PORTRAIT, 1).unwrap();
        assert_eq!((probe.width, probe.height, probe.has_audio), (2160, 3840, false));
    }

    #[test]
    fn rejects_non_video() {
        assert!(parse_probe("Invalid data found when processing input", 1).is_none());
    }

    #[test]
    fn plans_bitrates_and_downscale() {
        // 20 MB over 20 s: 7600 kbps total, 128 for audio, full resolution and frame rate.
        let roomy = plan(20_000_000, 20.0, true, 1.0).unwrap();
        assert_eq!(
            roomy,
            Plan {
                video_kbps: 7472,
                audio_kbps: 128,
                max_short_side: None,
                cap_fps: false
            }
        );
        // 10 MB over 60 s: about 1267 kbps total, so 720p at 30 fps.
        let tight = plan(10_000_000, 60.0, true, 1.0).unwrap();
        assert_eq!(
            (tight.audio_kbps, tight.max_short_side, tight.cap_fps),
            (96, Some(720), true)
        );
    }

    fn bundled_ffmpeg() -> PathBuf {
        let name = format!("binaries/ffmpeg-{}{}", env!("TARGET"), std::env::consts::EXE_SUFFIX);
        Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    /// A noisy 1080p60 clip with audio, which is hard to compress.
    fn sample(name: &str, seconds: u32) -> PathBuf {
        let dir = std::env::temp_dir().join("minivo-tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let video = format!("testsrc2=s=1920x1080:r=60:d={seconds},noise=alls=25:allf=t");
        let audio = format!("sine=f=440:d={seconds}");
        let ffmpeg = bundled_ffmpeg();
        let status = command(&ffmpeg)
            .args([
                "-v", "error", "-y", "-f", "lavfi", "-i", &video, "-f", "lavfi", "-i", &audio,
            ])
            .args([
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-crf",
                "12",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        path
    }

    #[test]
    fn compresses_under_target_with_x264() {
        compresses_under_target(&X264, "x264.mp4");
    }

    #[test]
    fn compresses_under_target_with_fastest() {
        let ffmpeg = bundled_ffmpeg();
        let encoder = fastest(&ffmpeg);
        println!("fastest encoder: {}", encoder.label);
        compresses_under_target(encoder, "fastest.mp4");
    }

    fn compresses_under_target(encoder: &Encoder, name: &str) {
        let ffmpeg = bundled_ffmpeg();
        let input = sample(name, 6);
        let target = 2_000_000;
        let done = compress(&ffmpeg, &input, target, encoder, &Job::default(), |_| {}).unwrap();
        let out = probe(&ffmpeg, &done.output).unwrap();
        std::fs::remove_file(&done.output).unwrap();
        assert!(
            done.bytes <= target && done.bytes > target * 7 / 10,
            "{} bytes",
            done.bytes
        );
        assert_eq!((out.width.min(out.height), out.has_audio), (done.short_side, true));
        assert!((out.duration - 6.0).abs() < 0.2, "{}", out.duration);
    }

    #[test]
    fn cancels_and_cleans_up() {
        let ffmpeg = bundled_ffmpeg();
        let input = sample("cancel.mp4", 20);
        let job = Job::default();
        let output = output_path(&input, 5_000_000);
        let result = thread::scope(|s| {
            s.spawn(|| {
                thread::sleep(std::time::Duration::from_millis(500));
                job.cancel();
            });
            compress(&ffmpeg, &input, 5_000_000, &X264, &job, |_| {})
        });
        assert_eq!(result.err().as_deref(), Some(CANCELLED));
        assert!(!output.exists());
    }

    #[test]
    fn refuses_impossible_sizes() {
        let err = plan(10_000_000, 3600.0, true, 1.0).unwrap_err();
        assert!(err.contains("Try 102 MB"), "{err}");
    }
}
