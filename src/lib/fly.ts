import { shouldAnimate } from "./motion";

let lastPointer: { x: number; y: number } | null = null;

/** Remembers where the last click landed, so a mark made by keyboard or mouse can fly from there. */
export function trackPointer() {
  const onDown = (e: PointerEvent) => {
    lastPointer = { x: e.clientX, y: e.clientY };
  };
  window.addEventListener("pointerdown", onDown, { capture: true, passive: true });
  return () => window.removeEventListener("pointerdown", onDown, { capture: true });
}

/** A small comet travels from the last click to the Órbita, so a mark visibly goes somewhere. */
export function flyToOrbit(color: string) {
  if (!lastPointer || !shouldAnimate()) return;
  // The tray is a column on wide windows and a floating button on narrow ones: aim at whichever shows.
  const t = [...document.querySelectorAll<HTMLElement>("[data-orbit-target]")]
    .map((el) => el.getBoundingClientRect())
    .find((r) => r.width > 0 && r.height > 0);
  if (!t) return;
  const from = lastPointer;
  const dx = t.left + t.width / 2 - from.x;
  const dy = t.top + t.height / 2 - from.y;
  const dot = document.createElement("div");
  dot.className = "orbit-comet";
  dot.style.left = `${from.x}px`;
  dot.style.top = `${from.y}px`;
  dot.style.setProperty("--comet", color);
  document.body.appendChild(dot);
  const anim = dot.animate(
    [
      { transform: "translate(-50%,-50%) scale(1)", opacity: 1 },
      { transform: `translate(calc(${dx * 0.55}px - 50%), calc(${dy * 0.35 - 60}px - 50%)) scale(1.3)`, opacity: 1 },
      { transform: `translate(calc(${dx}px - 50%), calc(${dy}px - 50%)) scale(0.2)`, opacity: 0 },
    ],
    { duration: 650, easing: "cubic-bezier(.5,0,.2,1)" },
  );
  anim.onfinish = () => dot.remove();
  anim.oncancel = () => dot.remove();
}
