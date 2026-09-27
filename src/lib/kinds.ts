import type { Kind } from "./api";

export const KINDS: Record<Kind, { label: string; color: string }> = {
  cache: { label: "Cache", color: "#c9a13b" },
  build: { label: "Build", color: "#c47a45" },
  git: { label: "Git", color: "#c4506a" },
  code: { label: "Código", color: "#4f7dd6" },
  media: { label: "Mídia", color: "#8b5cd6" },
  documents: { label: "Documentos", color: "#7d8594" },
  archives: { label: "Compactados", color: "#3fa5a8" },
  apps: { label: "Apps", color: "#3f9f68" },
  other: { label: "Outros", color: "#4b5566" },
};

export const KIND_ORDER = Object.keys(KINDS) as Kind[];

export function kindColor(kind: Kind): string {
  return KINDS[kind].color;
}
