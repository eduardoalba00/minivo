import { motion } from "motion/react";

const EASE = [0.2, 0.8, 0.2, 1];

export function DropZone({ dragging, onBrowse }) {
  return (
    <motion.button
      type="button"
      className={`drop${dragging ? " dragging" : ""}`}
      onClick={onBrowse}
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0, scale: dragging ? 1.015 : 1 }}
      exit={{ opacity: 0, y: -8, transition: { duration: 0.2 } }}
      transition={{ duration: 0.6, ease: EASE, scale: { type: "spring", stiffness: 400, damping: 30 } }}
    >
      <motion.span className="drop-icon" animate={{ y: dragging ? 4 : 0 }} transition={{ type: "spring", stiffness: 400, damping: 22 }}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 4v11M7.5 10.5 12 15l4.5-4.5" />
          <path d="M4.5 15.5v2a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-2" />
        </svg>
      </motion.span>
      <span className="drop-title">{dragging ? "Let go to shrink it" : "Drop a video here"}</span>
      <span className="drop-hint">
        or <u>browse your files</u>
      </span>
    </motion.button>
  );
}
