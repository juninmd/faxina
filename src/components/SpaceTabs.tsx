import type { Page } from "../domain";
import type { SpaceDef } from "../lib/spaces";

export default function SpaceTabs({ space, page, onPage }: { space: SpaceDef; page: Page; onPage: (p: Page) => void }) {
  if (space.tabs.length < 2) return null;
  return (
    <div role="tablist" aria-label={space.label} className="seg">
      {space.tabs.map((t) => (
        <button
          key={t.page}
          type="button"
          role="tab"
          aria-selected={page === t.page}
          className="seg-item"
          onClick={() => onPage(t.page)}
        >
          {t.label}
          {page === t.page && <span className="seg-glow" aria-hidden="true" />}
        </button>
      ))}
    </div>
  );
}
