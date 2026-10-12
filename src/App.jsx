import { AnimatePresence, MotionConfig, motion } from "motion/react";
import { useEffect, useEffectEvent, useRef, useState } from "react";
import { browse, CANCELLED, cancel, compress, encoderLabel, onFileDrop, probe, reveal } from "./api.js";
import { HEADLINE, Logo } from "./brand.jsx";
import { DropZone } from "./DropZone.jsx";
import { basename } from "./format.js";
import { JobCard } from "./JobCard.jsx";
import { SizePicker } from "./SizePicker.jsx";

const EASE = [0.2, 0.8, 0.2, 1];
const TARGET_KEY = "minivo.target";
const DEFAULT_MB = 20;

function savedTarget() {
  try {
    const saved = localStorage.getItem(TARGET_KEY);
    const mb = Number(saved);
    return saved && mb >= 1 ? mb : DEFAULT_MB;
  } catch {
    return DEFAULT_MB;
  }
}

function EncoderPill({ label }) {
  return (
    <motion.div className="pill" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.8, duration: 0.8 }}>
      <span className={`pill-dot${label ? "" : " pulsing"}`} />
      <AnimatePresence mode="wait" initial={false}>
        <motion.span key={label} initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -4 }} transition={{ duration: 0.25 }}>
          {label ? `Encoding with ${label}` : "Measuring your hardware…"}
        </motion.span>
      </AnimatePresence>
    </motion.div>
  );
}

export function App() {
  const [targetMb, setTargetMb] = useState(savedTarget);
  const [job, setJob] = useState(null);
  const [dragging, setDragging] = useState(false);
  const [encoder, setEncoder] = useState("");
  const [squish, setSquish] = useState(0);
  // Bumped for every new video (and on cancel), so a superseded run can't touch the screen.
  const run = useRef(0);
  const busy = job?.phase === "probing" || job?.phase === "working";

  useEffect(() => {
    try {
      const value = String(targetMb);
      localStorage.setItem(TARGET_KEY, value);
    } catch {
      // Storage can be unavailable; the size just won't be remembered.
    }
  }, [targetMb]);

  useEffect(() => {
    encoderLabel().then(setEncoder);
  }, []);

  async function start(path) {
    const id = ++run.current;
    const target = Math.round(targetMb * 1e6);
    const update = (patch) => {
      if (run.current === id) setJob((j) => ({ ...j, ...patch }));
    };
    const name = basename(path);
    setJob({ id, path, name, phase: "probing" });
    try {
      const info = await probe(path);
      if (info.bytes <= target) return update({ info, phase: "fits" });
      update({ info, phase: "working", progress: { fraction: 0, speed: 0, attempt: 1 } });
      const result = await compress(path, target, (progress) => update({ progress }));
      update({ phase: "done", result });
      if (run.current === id) setSquish((n) => n + 1);
    } catch (e) {
      if (e !== CANCELLED) update({ phase: "error", error: String(e) });
    }
  }

  const drop = useEffectEvent((path) => {
    if (!busy) start(path);
  });
  useEffect(() => {
    const listening = onFileDrop(setDragging, drop);
    return () => {
      listening.then((stop) => stop());
    };
  }, []);

  const pickFile = async () => {
    const path = await browse();
    if (path) start(path);
  };
  const stop = () => {
    run.current++;
    setJob(null);
    cancel();
  };
  const showResult = () => reveal(job.result.output);
  const reset = () => setJob(null);

  return (
    <MotionConfig reducedMotion="user">
      <div className={`app${dragging && job && !busy ? " drop-anywhere" : ""}`}>
        <div className="titlebar" data-tauri-drag-region />
        <main className="main">
          <section className="hero">
            <motion.div className="mark" initial={{ opacity: 0, scale: 0.8 }} animate={{ opacity: 1, scale: 1 }} transition={{ duration: 0.8, ease: EASE }}>
              <Logo size={44} squish={squish} />
            </motion.div>
            <h1 aria-label={HEADLINE}>
              {HEADLINE.split(" ").map((word, i) => (
                <motion.span
                  key={i}
                  aria-hidden="true"
                  style={{ "--i": i }}
                  initial={{ opacity: 0, y: 14, filter: "blur(8px)" }}
                  animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
                  transition={{ delay: 0.1 + 0.06 * i, duration: 0.8, ease: EASE }}
                >
                  {word}{" "}
                </motion.span>
              ))}
            </h1>
            <motion.p className="lede" initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.35, duration: 0.8, ease: EASE }}>
              Drop a video, pick a size. Minivo does the rest.
            </motion.p>
          </section>
          <motion.div className="picker" initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.45, duration: 0.8, ease: EASE }}>
            <SizePicker mb={targetMb} onChange={setTargetMb} disabled={busy} />
          </motion.div>
          <div className="stage">
            <AnimatePresence mode="wait">
              {job ? (
                <JobCard key={job.id} job={job} targetMb={targetMb} onCancel={stop} onReveal={showResult} onReset={reset} />
              ) : (
                <DropZone key="drop" dragging={dragging} onBrowse={pickFile} />
              )}
            </AnimatePresence>
          </div>
        </main>
        <footer className="footer">
          <EncoderPill label={encoder} />
        </footer>
      </div>
    </MotionConfig>
  );
}
