import type { KeyboardEvent } from "react";
import type { Page } from "../domain";
import type { SpaceDef } from "../lib/spaces";

export default function SpaceTabs({ space, page, onPage }: { space: SpaceDef; page: Page; onPage: (p: Page) => void }) {
  if (space.tabs.length < 2) return null;
  const move = (e: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const n = space.tabs.length;
    let next = -1;
    if (e.key === "ArrowRight") next = (index + 1) % n;
    else if (e.key === "ArrowLeft") next = (index - 1 + n) % n;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = n - 1;
    if (next < 0) return;
    e.preventDefault();
    const target = space.tabs[next];
    onPage(target.page);
    e.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
  };
  return (
    <div role="tablist" aria-label={space.label} className="seg">
      {space.tabs.map((t, i) => (
        <button
          key={t.page}
          type="button"
          role="tab"
          tabIndex={page === t.page ? 0 : -1}
          aria-selected={page === t.page}
          className="seg-item"
          onClick={() => onPage(t.page)}
          onKeyDown={(e) => move(e, i)}
        >
          {t.label}
          {page === t.page && <span className="seg-glow" aria-hidden="true" />}
        </button>
      ))}
    </div>
  );
}
