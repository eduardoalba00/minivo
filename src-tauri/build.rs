fn main() {
    // Lets tests find the sidecar at binaries/ffmpeg-<target>.
    let target = std::env::var("TARGET").unwrap();
    println!("cargo:rustc-env=TARGET={target}");
    tauri_build::build()
}
