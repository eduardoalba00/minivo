import { motion } from "motion/react";

export const NAME = "Minivo";
export const HEADLINE = "Make any video fit.";

// A frame with a play button in it. Each new `squish` value squishes the triangle once (a video just finished).
export function Logo({ size = 22, squish = 0 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true" className="logo">
      <rect x="2" y="2" width="28" height="28" rx="8" fill="currentColor" />
      <motion.path
        key={squish}
        d="M13 10.6c0-.8.9-1.3 1.6-.9l6.6 4.3c.6.4.6 1.3 0 1.7l-6.6 4.3c-.7.4-1.6-.1-1.6-.9Z"
        style={{ fill: "var(--bg)", transformBox: "fill-box", transformOrigin: "center" }}
        initial={squish ? { scale: 1 } : false}
        animate={squish ? { scaleX: [1, 0.6, 1.12, 1], scaleY: [1, 1.2, 0.94, 1] } : {}}
        transition={{ duration: 0.7, ease: [0.2, 0.8, 0.2, 1] }}
      />
    </svg>
  );
}
