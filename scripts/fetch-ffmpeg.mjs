// Downloads a pinned, checksummed static ffmpeg build for the target platform into
// src-tauri/binaries/, named the way Tauri expects sidecars (ffmpeg-<target triple>).
// Usage: node scripts/fetch-ffmpeg.mjs [target-triple]   (defaults to the host)

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, copyFileSync, mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const RIEDL = "https://ffmpeg.martin-riedl.de/download";
const BUILDS = {
  "aarch64-apple-darwin": {
    url: `${RIEDL}/macos/arm64/1789931890_9.0.2/ffmpeg.zip`,
    sha256: "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
    entry: "ffmpeg",
  },
  "x86_64-apple-darwin": {
    url: `${RIEDL}/macos/amd64/1789931006_9.0.2/ffmpeg.zip`,
    sha256: "7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4",
    entry: "ffmpeg",
  },
  "x86_64-unknown-linux-gnu": {
    url: `${RIEDL}/linux/amd64/1789931100_9.0.2/ffmpeg.zip`,
    sha256: "fa8ecf4abbd290d98f7d188b8649cc6b391ae209a98452be955a15aab1909d7f",
    entry: "ffmpeg",
  },
  "x86_64-pc-windows-msvc": {
    url: "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip",
    sha256: "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba",
    entry: "ffmpeg-9.0.2-essentials_build/bin/ffmpeg.exe",
  },
};

const hostTriple = () => {
  const info = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  return info.match(/^host: (.+)$/m)[1];
};

const target = process.argv[2] ?? process.env.MINIVO_TARGET ?? hostTriple();
const build = BUILDS[target];
if (!build) {
  console.error(`No pinned ffmpeg build for ${target}. Supported: ${Object.keys(BUILDS).join(", ")}`);
  process.exit(1);
}

const dir = join(import.meta.dirname, "..", "src-tauri", "binaries");
const isWindows = target.includes("windows");
const exe = isWindows ? ".exe" : "";
const dest = join(dir, `ffmpeg-${target}${exe}`);
const present = existsSync(dest);
if (present) process.exit(0);

console.log(`Downloading ffmpeg for ${target}…`);
const res = await fetch(build.url);
if (!res.ok) throw new Error(`Download failed: ${res.status} ${build.url}`);
const body = await res.arrayBuffer();
const zip = Buffer.from(body);
const sha = createHash("sha256").update(zip).digest("hex");
if (sha !== build.sha256) throw new Error(`Checksum mismatch for ${build.url}: got ${sha}`);

const tmp = tmpdir();
const prefix = join(tmp, "minivo-ffmpeg-");
const work = mkdtempSync(prefix);
try {
  const archive = join(work, "ffmpeg.zip");
  writeFileSync(archive, zip);
  // Windows ships bsdtar, which reads zips; macOS and Linux have unzip.
  if (process.platform === "win32") execFileSync("tar", ["-xf", archive, "-C", work, build.entry]);
  else execFileSync("unzip", ["-q", archive, build.entry, "-d", work]);
  mkdirSync(dir, { recursive: true });
  const extracted = join(work, build.entry);
  copyFileSync(extracted, dest);
  chmodSync(dest, 0o755);
} finally {
  rmSync(work, { recursive: true, force: true });
}
const { size } = statSync(dest);
console.log(`ffmpeg ready: ${dest} (${Math.round(size / 1e6)} MB)`);
