/** Most names the duplicate search may skip; the backend enforces the same limit. */
export const MAX_EXCLUDED_NAMES = 50;

const FORBIDDEN = /[/\\*?:\0]/;

/**
 * Turns the text typed in the "ignorar" field (names separated by commas or line breaks) into a
 * clean list. Exact names only: a path, a wildcard or a drive would silently widen the exclusion.
 */
export function parseExcludedNames(text: string): { names: string[]; error: string | null } {
  const names = [
    ...new Set(
      text
        .split(/[,\n]/)
        .map((n) => n.trim())
        .filter(Boolean),
    ),
  ];
  const bad = names.find((n) => FORBIDDEN.test(n) || n === "." || n === ".." || n.length > 255);
  if (bad) return { names: [], error: `"${bad}" não é um nome válido: use só nomes, sem caminhos nem curingas.` };
  if (names.length > MAX_EXCLUDED_NAMES) {
    return { names: [], error: `Use no máximo ${MAX_EXCLUDED_NAMES} nomes.` };
  }
  return { names, error: null };
}
