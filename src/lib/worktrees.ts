import type { Worktree, WorktreeOptIn } from "./api";

/**
 * Whether the row can be picked. `definitive` only unlocks after the user turns on the damage
 * switch: it is the level that can lose commits that exist nowhere else.
 */
export function canPick(w: Worktree, damage: boolean): boolean {
  if (w.allowance === "synced" || w.allowance === "recoverable") return true;
  return damage && w.allowance === "definitive";
}

/**
 * The least level that covers everything picked, so the removal never grants more than the user
 * asked for: one definitive item lifts the whole batch, one recoverable item lifts it to `local`.
 */
export function optInFor(chosen: Worktree[]): WorktreeOptIn {
  if (chosen.some((w) => w.allowance === "definitive")) return "definitive";
  if (chosen.some((w) => w.allowance === "recoverable")) return "local";
  return "synced";
}
