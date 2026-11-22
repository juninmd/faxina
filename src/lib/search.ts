import type { ExportFormat, Kind, SearchQuery } from "./api";

const MB = 1024 * 1024;
const DAY = 24 * 3600;

export const SIZE_STEPS = [
  { value: 0, label: "Qualquer tamanho" },
  { value: 10 * MB, label: "≥ 10 MB" },
  { value: 100 * MB, label: "≥ 100 MB" },
  { value: 1024 * MB, label: "≥ 1 GB" },
];

export const AGE_STEPS = [
  { months: 0, label: "Qualquer idade" },
  { months: 6, label: "Parados há 6+ meses" },
  { months: 12, label: "Parados há 1+ ano" },
  { months: 24, label: "Parados há 2+ anos" },
];

export const EXPORT_FORMATS: { value: ExportFormat; label: string }[] = [
  { value: "csv", label: "CSV" },
  { value: "json", label: "JSON" },
  { value: "html", label: "HTML" },
];

export interface SearchForm {
  text: string;
  minSize: number;
  ageMonths: number;
  kind: Kind | null;
  includeDirs: boolean;
}

export const EMPTY_FORM: SearchForm = { text: "", minSize: 0, ageMonths: 0, kind: null, includeDirs: false };

/** Turns the form into the backend query. Months are 30 days, as in the CLI. */
export function toQuery(f: SearchForm, limit = 500): SearchQuery {
  return {
    text: f.text.trim(),
    minSize: f.minSize,
    olderThanSecs: f.ageMonths * 30 * DAY,
    kind: f.kind,
    // A folder has no age of its own, so asking for old items means files only.
    includeDirs: f.includeDirs && f.ageMonths === 0,
    limit,
  };
}

/** True when the form would match every file: the UI then asks for a filter first. */
export function isBlank(f: SearchForm): boolean {
  return !f.text.trim() && f.minSize === 0 && f.ageMonths === 0 && f.kind === null;
}

/** File name for an export, dated by the user's local day (toISOString would give the UTC one). */
export function exportName(format: ExportFormat, now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  const day = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
  return `faxina-busca-${day}.${format}`;
}
