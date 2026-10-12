// The target size: three presets and a custom field. The active tab's pill glides between choices.

import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef, useState } from "react";

const PRESETS = [
  { mb: 10 },
  { mb: 20, note: "Discord" },
  { mb: 50 },
];

export function SizePicker({ mb, onChange, disabled }) {
  const isPreset = PRESETS.some((p) => p.mb === mb);
  const [custom, setCustom] = useState(!isPreset);
  const initialDraft = String(mb);
  const [draft, setDraft] = useState(initialDraft);
  const field = useRef(null);

  useEffect(() => {
    const input = field.current;
    if (!custom || !input) return;
    input.focus();
    input.setSelectionRange(input.value.length, input.value.length);
  }, [custom]);

  const pick = (value) => {
    setCustom(false);
    onChange(value);
  };
  const enterCustom = () => {
    const current = String(mb);
    setDraft(current);
    setCustom(true);
  };
  const edit = (text) => {
    setDraft(text);
    const value = Number(text);
    if (value >= 1 && value <= 4000) onChange(value);
  };

  return (
    <div className={`sizes${disabled ? " disabled" : ""}`} role="radiogroup" aria-label="Target size">
      {PRESETS.map((p) => {
        const active = !custom && mb === p.mb;
        return (
          <button key={p.mb} type="button" role="radio" aria-checked={active} className={active ? "active" : ""} onClick={() => pick(p.mb)} disabled={disabled}>
            {active && <motion.span layoutId="size-pill" className="size-pill" transition={{ type: "spring", stiffness: 500, damping: 38 }} />}
            <span className="size-label">
              {p.mb} MB
              {p.note && <small>{p.note}</small>}
            </span>
          </button>
        );
      })}
      <button type="button" role="radio" aria-checked={custom} className={custom ? "active" : ""} onClick={enterCustom} disabled={disabled}>
        {custom && <motion.span layoutId="size-pill" className="size-pill" transition={{ type: "spring", stiffness: 500, damping: 38 }} />}
        <span className="size-label">
          <AnimatePresence initial={false} mode="popLayout">
            {custom ? (
              <motion.span key="field" className="size-field" initial={{ opacity: 0, width: 0 }} animate={{ opacity: 1, width: "auto" }} exit={{ opacity: 0, width: 0 }}>
                <input
                  ref={field}
                  value={draft}
                  onChange={(e) => edit(e.target.value)}
                  inputMode="decimal"
                  style={{ width: `${draft.length || 1}ch` }}
                  aria-label="Custom size in MB"
                  disabled={disabled}
                />
                MB
              </motion.span>
            ) : (
              <motion.span key="label" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                Custom
              </motion.span>
            )}
          </AnimatePresence>
        </span>
      </button>
    </div>
  );
}
