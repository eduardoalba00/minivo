<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" height="128" alt="Minivo icon">
</p>

<h1 align="center">Minivo</h1>

<p align="center">Make any video fit. Drop a video, pick a size, done.</p>

---

Minivo shrinks videos to fit under a file size limit, like Discord's 20 MB cap or an email attachment limit. It's a small, fast desktop app for macOS, Windows and Linux, with ffmpeg built in, so there's nothing else to install.

## Features

- **Hits the size you pick.** Choose 10, 20 or 50 MB, or type any size. Minivo plans the bitrate for your video's length, checks the result, and retries in the rare case the encoder overshoots.
- **Fast on every machine.** At startup Minivo benchmarks the encoders your machine has (Apple VideoToolbox, NVIDIA NVENC, Intel Quick Sync, AMD AMF, and x264 on the CPU) and uses whichever is fastest.
- **Looks as good as the size allows.** When the budget is tight, Minivo lowers the resolution and caps the frame rate at 30 fps instead of smearing a 4K, 60 fps video into mush. Phone videos keep their orientation.
- **Plays everywhere.** Output is H.264 and AAC in an MP4, which previews inline in Discord, iMessage, Slack, WhatsApp and email.
- **Private.** Everything happens on your computer. Nothing is uploaded.

The result is saved next to the original as `name-20MB.mp4`.

## Install

Download the latest installer from [Releases](../../releases).

Minivo isn't signed with an Apple Developer ID yet, so macOS blocks it the first time you open it. Open it once, then go to **System Settings → Privacy & Security** and click **Open Anyway**.

## Build from source

You need [Node.js](https://nodejs.org) 24+, [Rust](https://rustup.rs), and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

```sh
npm install          # also downloads a pinned, checksummed ffmpeg for your platform
npm run tauri dev    # run the app
npm run tauri build  # build an installer
```

Run the tests, including end-to-end compressions with the bundled ffmpeg:

```sh
cd src-tauri && cargo test
```

## How it works

Minivo is a [Tauri](https://tauri.app) app. The interface is React, and the Rust side drives ffmpeg as a sidecar process.

1. **Probe:** read the video's duration, resolution, rotation and audio streams.
2. **Plan:** turn the target size into a bitrate (`target × 8 × 0.95 ÷ duration`, minus audio), then pick a resolution and frame rate that bitrate can carry.
3. **Encode:** run ffmpeg with constant bitrate on the fastest encoder, reporting progress as it goes.
4. **Verify:** check the output size, and re-encode at a slightly lower bitrate if it came out too big.

The core logic is in [`src-tauri/src/ffmpeg.rs`](src-tauri/src/ffmpeg.rs).

## Credits

Minivo bundles [FFmpeg](https://ffmpeg.org) 9.0.2 as a separate program. FFmpeg is licensed under the [GPL](https://ffmpeg.org/legal.html), and its source code is available at [ffmpeg.org](https://ffmpeg.org/download.html). The macOS and Linux builds come from [Martin Riedl](https://ffmpeg.martin-riedl.de) and the Windows builds from [Gyan Doshi](https://www.gyan.dev/ffmpeg/builds/).

## License

[MIT](LICENSE)
