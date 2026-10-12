// One video's trip through Minivo: reading it, compressing it (with live progress), and the result.

import { animate, motion, useMotionValue, useTransform } from "motion/react";
import { useEffect } from "react";
import { bytes, clock, remaining } from "./format.js";

const EASE = [0.2, 0.8, 0.2, 1];

/** Counts a byte size from `from` down (or up) to `to`. */
function CountBytes({ from, to }) {
  const value = useMotionValue(from);
  const text = useTransform(value, bytes);
  useEffect(() => {
    const controls = animate(value, to, { duration: 1.4, ease: [0.16, 1, 0.3, 1] });
    return () => controls.stop();
  }, [value, to]);
  return <motion.span>{text}</motion.span>;
}

function Meta({ info }) {
  if (!info) return <p className="job-meta">Reading video…</p>;
  const shortSide = Math.min(info.width, info.height);
  const length = clock(info.duration);
  const size = bytes(info.bytes);
  return <p className="job-meta">{size} · {shortSide}p · {length}</p>;
}

function Working({ job, onCancel }) {
  const { fraction, speed, attempt } = job.progress;
  const percent = Math.floor(fraction * 100);
  const left = speed > 0 ? ((1 - fraction) * job.info.duration) / speed : null;
  const stage = attempt > 1 ? "Fine-tuning the size" : "Compressing";
  const pace = speed.toFixed(1);
  const eta = left === null ? "" : remaining(left);
  return (
    <>
      <div className="percent" aria-live="polite">
        {percent}
        <span>%</span>
      </div>
      <div className="bar" aria-hidden="true">
        <motion.i animate={{ scaleX: fraction }} transition={{ type: "spring", stiffness: 90, damping: 22 }} />
      </div>
      <div className="job-stats">
        <span className="step">
          <span className="spinner" />
          {stage}
        </span>
        <span>{eta ? `${pace}× · ${eta}` : "Starting…"}</span>
      </div>
      <div className="actions">
        <button type="button" className="button" onClick={onCancel}>Cancel</button>
      </div>
    </>
  );
}

function Finished({ job, onReveal, onReset }) {
  const { result, info } = job;
  const saved = Math.round((1 - result.bytes / info.bytes) * 100);
  const before = bytes(info.bytes);
  return (
    <>
      <div className="outcome">
        <span className="from">{before}</span>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
        <span className="to"><CountBytes from={info.bytes} to={result.bytes} /></span>
      </div>
      <motion.p className="job-summary" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.5, duration: 0.6, ease: EASE }}>
        <b>{saved}% smaller</b> · {result.shortSide}p · saved next to the original
      </motion.p>
      <div className="actions">
        <button type="button" className="button primary" onClick={onReveal}>{SHOW_IN_FOLDER}</button>
        <button type="button" className="button" onClick={onReset}>Shrink another</button>
      </div>
    </>
  );
}

const SHOW_IN_FOLDER = navigator.userAgent.includes("Mac") ? "Show in Finder" : "Show in folder";

function Note({ title, children, onReset }) {
  return (
    <>
      <h2 className="note-title">{title}</h2>
      <p className="note-text">{children}</p>
      <div className="actions">
        <button type="button" className="button" onClick={onReset}>Choose another video</button>
      </div>
    </>
  );
}

export function JobCard({ job, targetMb, onCancel, onReveal, onReset }) {
  const fits = job.phase === "fits";
  const size = job.info ? bytes(job.info.bytes) : "";
  return (
    <motion.section
      className={`job ${job.phase}`}
      layout
      initial={{ opacity: 0, y: 16, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: -8, transition: { duration: 0.2 } }}
      transition={{ duration: 0.5, ease: EASE }}
    >
      <header className="job-head">
        <span className="file-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24"><rect x="3.5" y="5.5" width="17" height="13" rx="3" /><path d="m10.5 9.5 4 2.5-4 2.5Z" /></svg>
        </span>
        <div className="job-file">
          <p className="job-name" title={job.path}>{job.name}</p>
          <Meta info={job.info} />
        </div>
      </header>
      {job.phase === "working" && <Working job={job} onCancel={onCancel} />}
      {job.phase === "done" && <Finished job={job} onReveal={onReveal} onReset={onReset} />}
      {fits && (
        <Note title={`Already under ${targetMb} MB`} onReset={onReset}>
          This video is {size}, so there's nothing to shrink.
        </Note>
      )}
      {job.phase === "error" && (
        <Note title="Couldn't shrink this one" onReset={onReset}>{job.error}</Note>
      )}
      {job.phase === "probing" && <div className="bar idle" aria-hidden="true"><i /></div>}
    </motion.section>
  );
}
