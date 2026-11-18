// The chart as the timeline draws it (visualizer-majdata 10), as the engine
// publishes it: see engine/src/systems/timeline.rs. Times are chart seconds.

export type MeasureCounts = {
  start: number;
  tap: number;
  slide: number;
  touch: number;
  hold: number;
};

export type Timeline = { length: number; measures: MeasureCounts[] };

function clampIndex(i: number, count: number): number {
  return Math.min(Math.max(i, 0), Math.max(count - 1, 0));
}

// The measure containing `t`; before the chart, the first; after, the last.
export function measureAt(tl: Timeline, t: number): number {
  let i = 0;
  while (i + 1 < tl.measures.length && tl.measures[i + 1].start <= t) i++;
  return i;
}

export function measureStart(tl: Timeline, i: number): number {
  return tl.measures[clampIndex(i, tl.measures.length)]?.start ?? 0;
}

// Time labels across the graph: every 30 s, strictly inside the chart.
export function timeMarks(length: number, every = 30): number[] {
  const marks: number[] = [];
  for (let t = every; t < length; t += every) marks.push(t);
  return marks;
}

// The measure a key moves the timeline to, or null for a key it ignores:
// arrows one measure, Page Up/Down ten, Home/End the ends.
export function stepMeasure(key: string, current: number, count: number): number | null {
  const to = (i: number) => clampIndex(i, count);
  switch (key) {
    case 'ArrowRight':
    case 'ArrowUp':
      return to(current + 1);
    case 'ArrowLeft':
    case 'ArrowDown':
      return to(current - 1);
    case 'PageUp':
      return to(current + 10);
    case 'PageDown':
      return to(current - 10);
    case 'Home':
      return 0;
    case 'End':
      return to(count - 1);
    default:
      return null;
  }
}

// Where a timeline key seeks to (seconds), or null for a key it ignores. It
// steps from `inflight` — a seek sent but not yet reached — when there is one,
// so presses faster than the engine's round trip each move a measure.
export function keyTarget(
  tl: Timeline,
  key: string,
  position: number,
  inflight: number | null,
): number | null {
  const from = measureAt(tl, inflight ?? position);
  const to = stepMeasure(key, from, tl.measures.length);
  return to === null ? null : measureStart(tl, to);
}

// `1:05`.
export function formatClock(secs: number): string {
  const whole = Math.max(0, Math.floor(secs));
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`;
}
