import { describe, expect, it } from "vitest";
import {
  formatClock,
  measureAt,
  measureStart,
  stepMeasure,
  timeMarks,
  type Timeline,
} from "./timeline";

const measure = (start: number) => ({ start, tap: 0, slide: 0, touch: 0, hold: 0 });
const tl: Timeline = { length: 6, measures: [measure(0), measure(2), measure(4)] };

describe("measureAt", () => {
  it("is the measure containing the time, clamped to the chart", () => {
    expect(measureAt(tl, 0)).toBe(0);
    expect(measureAt(tl, 1.999)).toBe(0);
    expect(measureAt(tl, 2)).toBe(1);
    expect(measureAt(tl, -1)).toBe(0);
    expect(measureAt(tl, 50)).toBe(2);
    expect(measureAt({ length: 0, measures: [] }, 3)).toBe(0);
  });
});

describe("measureStart", () => {
  it("clamps the index", () => {
    expect(measureStart(tl, 1)).toBe(2);
    expect(measureStart(tl, -3)).toBe(0);
    expect(measureStart(tl, 9)).toBe(4);
    expect(measureStart({ length: 0, measures: [] }, 2)).toBe(0);
  });
});

describe("timeMarks", () => {
  it("marks every 30 s strictly inside the chart", () => {
    expect(timeMarks(95)).toEqual([30, 60, 90]);
    expect(timeMarks(90)).toEqual([30, 60]);
    expect(timeMarks(10)).toEqual([]);
  });
});

describe("stepMeasure", () => {
  it.each([
    ["ArrowRight", 3, 4],
    ["ArrowUp", 3, 4],
    ["ArrowLeft", 3, 2],
    ["ArrowDown", 3, 2],
    ["PageUp", 3, 9],
    ["PageDown", 5, 0],
    ["PageUp", 0, 9],
    ["Home", 5, 0],
    ["End", 5, 9],
    ["ArrowRight", 9, 9],
    ["ArrowLeft", 0, 0],
  ])("%s from %i of 10 goes to %i", (key, from, to) => {
    expect(stepMeasure(key, from, 10)).toBe(to);
  });

  it("ignores other keys", () => {
    expect(stepMeasure("a", 3, 10)).toBeNull();
  });
});

describe("formatClock", () => {
  it("is m:ss", () => {
    expect(formatClock(65.4)).toBe("1:05");
    expect(formatClock(0)).toBe("0:00");
    expect(formatClock(600)).toBe("10:00");
  });
});
