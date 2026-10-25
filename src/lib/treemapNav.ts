export interface Point {
  x: number;
  y: number;
}

/**
 * Index of the tile an arrow key moves focus to, or null when nothing lies that way.
 * The nearest tile along the arrow's axis wins; drifting off that axis costs more.
 */
export function nextTile(points: Point[], current: number, key: string): number | null {
  const from = points[current];
  if (!from) return null;
  let best: number | null = null;
  let bestScore = Number.POSITIVE_INFINITY;
  points.forEach((p, i) => {
    if (i === current) return;
    const dx = p.x - from.x;
    const dy = p.y - from.y;
    let along: number;
    let across: number;
    if (key === "ArrowLeft") [along, across] = [-dx, dy];
    else if (key === "ArrowRight") [along, across] = [dx, dy];
    else if (key === "ArrowUp") [along, across] = [-dy, dx];
    else if (key === "ArrowDown") [along, across] = [dy, dx];
    else return;
    if (along <= 0) return;
    const score = along + 2 * Math.abs(across);
    if (score < bestScore) {
      bestScore = score;
      best = i;
    }
  });
  return best;
}
