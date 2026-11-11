import type { IconName } from "../components/Icon";
import type { Page } from "../domain";

export type Space = "agora" | "explorar" | "limpar" | "ajustes";

export interface SpaceDef {
  id: Space;
  label: string;
  icon: IconName;
  /** Pages shown as tabs inside the space; the first one opens by default. */
  tabs: { page: Page; label: string }[];
}

/** Twelve pages folded into four spaces. Every page keeps a home here: none is dropped. */
export const SPACES: SpaceDef[] = [
  { id: "agora", label: "Agora", icon: "orbit", tabs: [{ page: "overview", label: "Visão geral" }] },
  {
    id: "explorar",
    label: "Explorar",
    icon: "grid",
    tabs: [
      { page: "map", label: "Mapa" },
      { page: "bigfiles", label: "Grandes" },
      { page: "growth", label: "Cresceu" },
    ],
  },
  {
    id: "limpar",
    label: "Limpar",
    icon: "spark",
    tabs: [
      { page: "review", label: "Caches" },
      { page: "dupes", label: "Duplicatas" },
      { page: "empty", label: "Vazios e links" },
      { page: "worktrees", label: "Repositórios" },
    ],
  },
  {
    id: "ajustes",
    label: "Ajustes",
    icon: "gear",
    tabs: [
      { page: "coverage", label: "Aplicativos" },
      { page: "routine", label: "Rotina" },
      { page: "report", label: "Histórico" },
      { page: "updates", label: "Atualizações" },
    ],
  },
];

export function spaceOf(page: Page): SpaceDef {
  return SPACES.find((s) => s.tabs.some((t) => t.page === page)) ?? SPACES[0];
}

export function pageLabel(page: Page): string {
  for (const s of SPACES) for (const t of s.tabs) if (t.page === page) return t.label;
  return page;
}

export interface Command {
  id: string;
  label: string;
  hint: string;
  keywords?: string;
}

/** Every page is reachable from the palette, named by its space and tab. */
export const PAGE_COMMANDS: (Command & { page: Page })[] = SPACES.flatMap((s) =>
  s.tabs.map((t) => ({
    id: `page:${t.page}`,
    page: t.page,
    label: s.tabs.length > 1 ? `${s.label} · ${t.label}` : s.label,
    hint: "ir para",
  })),
);

const fold = (s: string) =>
  s
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();

/** Accent- and case-insensitive match on every word typed, in any order. */
export function filterCommands<T extends Command>(commands: T[], query: string): T[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (!words.length) return commands;
  return commands.filter((c) => {
    const hay = fold(`${c.label} ${c.keywords ?? ""}`);
    return words.every((w) => hay.includes(w));
  });
}
