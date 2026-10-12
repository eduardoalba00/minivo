import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

export const CANCELLED = "cancelled";
const VIDEO_EXTENSIONS = ["mp4", "mov", "m4v", "mkv", "webm", "avi", "wmv", "flv", "mpg", "mpeg", "ts", "mts", "m2ts", "3gp"];

export const probe = (path) => invoke("probe", { path });
export const cancel = () => invoke("cancel");
export const encoderLabel = () => invoke("encoder_label");
export const reveal = (path) => revealItemInDir(path);

export function compress(path, target, onProgress) {
  const progress = new Channel();
  progress.onmessage = onProgress;
  return invoke("compress", { path, target, progress });
}

export function browse() {
  const filters = [{ name: "Videos", extensions: VIDEO_EXTENSIONS }];
  return open({ multiple: false, directory: false, filters });
}

/** Calls `onChange(dragging)` as files move over the window and `onDrop(path)` with the first dropped file. */
export function onFileDrop(onChange, onDrop) {
  const webview = getCurrentWebview();
  return webview.onDragDropEvent(({ payload }) => {
    const dragging = payload.type === "enter" || payload.type === "over";
    onChange(dragging);
    if (payload.type === "drop" && payload.paths.length) onDrop(payload.paths[0]);
  });
}
