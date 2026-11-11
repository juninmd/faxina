import { describe, expect, test } from "bun:test";
import { groupBySource, type OrbitItem, planOrbit } from "./orbit";

const item = (path: string, route: OrbitItem["route"], source = "Grandes"): OrbitItem => ({
  path,
  name: path,
  size: 1,
  color: "#fff",
  route,
  source,
});

describe("planOrbit", () => {
  test("each route goes to its own command", () => {
    const plan = planOrbit(
      ["/map/a"],
      [item("/big/b", "path"), item("/x/link", "link", "Vazios"), item("/repo/wt", "worktree", "Repositórios")],
    );
    expect(plan.paths).toEqual(["/map/a", "/big/b"]);
    expect(plan.links).toEqual(["/x/link"]);
    expect(plan.worktrees).toEqual(["/repo/wt"]);
  });

  test("a path marked in the map and queued by a panel is removed once", () => {
    expect(planOrbit(["/same"], [item("/same", "path")]).paths).toEqual(["/same"]);
  });

  test("links and worktrees never reach delete_paths", () => {
    const plan = planOrbit([], [item("/l", "link"), item("/w", "worktree")]);
    expect(plan.paths).toEqual([]);
  });
});

test("groupBySource keeps first-seen order", () => {
  const g = groupBySource([
    item("1", "path", "Duplicatas"),
    item("2", "path", "Grandes"),
    item("3", "path", "Duplicatas"),
  ]);
  expect(g.map(([s, l]) => [s, l.length])).toEqual([
    ["Duplicatas", 2],
    ["Grandes", 1],
  ]);
});
