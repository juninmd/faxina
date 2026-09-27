export type MotionPref = "system" | "always" | "never";

const KEY = "faxina.motion";
export const MOTION_LABELS: Record<MotionPref, string> = { system: "sistema", always: "sempre", never: "nunca" };

export function getMotionPref(): MotionPref {
  try {
    const v = localStorage.getItem(KEY);
    return v === "always" || v === "never" ? v : "system";
  } catch {
    return "system";
  }
}

export function setMotionPref(p: MotionPref) {
  try {
    localStorage.setItem(KEY, p);
  } catch {
    // Storage can be unavailable; the choice just won't persist.
  }
}

/** Windows' "Animation effects: off" maps to reduced motion, so users can opt back in. */
export function shouldAnimate(pref = getMotionPref()): boolean {
  if (pref !== "system") return pref === "always";
  return !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
