export function bytes(n) {
  if (n >= 1e9) return `${(n / 1e9).toFixed(2)} GB`;
  if (n >= 1e8) return `${Math.round(n / 1e6)} MB`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
  const kb = Math.round(n / 1e3);
  return `${Math.max(1, kb)} KB`;
}

export function clock(seconds) {
  const s = Math.round(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  if (!h) return `${m}:${ss}`;
  const mm = String(m).padStart(2, "0");
  return `${h}:${mm}:${ss}`;
}

export function remaining(seconds) {
  const rounded = Math.round(seconds);
  if (seconds < 60) return `${Math.max(1, rounded)}s left`;
  const minutes = Math.round(seconds / 60);
  return `${minutes} min left`;
}

export function basename(path) {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1];
}
