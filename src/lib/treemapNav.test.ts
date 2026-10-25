import { describe, expect, test } from "bun:test";
import { nextTile, ringPoint } from "./treemapNav";

// Two columns: top-left, top-right, bottom-left.
const grid = [
  { x: 10, y: 10 },
  { x: 90, y: 10 },
  { x: 10, y: 90 },
];

describe("nextTile", () => {
  test("moves to the nearest tile in the arrow's direction", () => {
    expect(nextTile(grid, 0, "ArrowRight")).toBe(1);
    expect(nextTile(grid, 0, "ArrowDown")).toBe(2);
    expect(nextTile(grid, 1, "ArrowLeft")).toBe(0);
    expect(nextTile(grid, 2, "ArrowUp")).toBe(0);
  });

  test("returns null at an edge, so focus stays put", () => {
    expect(nextTile(grid, 0, "ArrowLeft")).toBeNull();
    expect(nextTile(grid, 0, "ArrowUp")).toBeNull();
  });

  test("ignores keys that are not arrows", () => {
    expect(nextTile(grid, 0, "Tab")).toBeNull();
    expect(nextTile(grid, 0, "Enter")).toBeNull();
  });

  test("prefers a tile on the same row over a nearer one off the axis", () => {
    const row = [
      { x: 0, y: 0 },
      { x: 100, y: 80 },
      { x: 200, y: 0 },
    ];
    expect(nextTile(row, 0, "ArrowRight")).toBe(2);
  });

  test("a missing current tile yields null instead of throwing", () => {
    expect(nextTile([], 0, "ArrowRight")).toBeNull();
  });
});

describe("ringPoint", () => {
  test("angle 0 is straight up and angles grow clockwise", () => {
    const top = ringPoint(0, 100);
    expect(top.x).toBeCloseTo(0);
    expect(top.y).toBeCloseTo(-100);
    const right = ringPoint(Math.PI / 2, 100);
    expect(right.x).toBeCloseTo(100);
    expect(right.y).toBeCloseTo(0);
  });
});
