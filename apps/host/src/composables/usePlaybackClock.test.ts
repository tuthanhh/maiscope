import { beforeEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";

// The engine publishes a snapshot each frame; the clock copies it into the
// page once per animation frame. Both are stood in for here: `readPlayback`
// answers what the test says, and frames run when the test runs them.
const engine = vi.hoisted(() => ({
  readPlayback: vi.fn(),
  readTimeline: vi.fn(),
}));
vi.mock("~/composables/useEngine", () => engine);

import usePlaybackClock from "./usePlaybackClock";
import type { Playback } from "./usePlayback";

let frames: Map<number, FrameRequestCallback>;
let nextId: number;
function runFrame(): void {
  const due = [...frames.values()];
  frames.clear();
  for (const cb of due) cb(0);
}

function fakePlayback(playing: boolean): Playback {
  const p = { playing: ref(playing), markLoaded: vi.fn() };
  p.markLoaded.mockImplementation((v: boolean) => (p.playing.value = v));
  return p as unknown as Playback;
}

const snapshot = (position: number, playing: boolean, chartId: number) => ({
  position,
  playing,
  chartId,
});

beforeEach(() => {
  frames = new Map();
  nextId = 1;
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    frames.set(nextId, cb);
    return nextId++;
  });
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
  engine.readPlayback.mockReset();
  engine.readTimeline.mockReset().mockReturnValue(null);
});

describe("usePlaybackClock", () => {
  it("copies the position each frame", () => {
    engine.readPlayback.mockReturnValue(snapshot(12.5, true, 1));
    const clock = usePlaybackClock(fakePlayback(true));
    clock.start();
    runFrame();
    expect(clock.position.value).toBe(12.5);
    engine.readPlayback.mockReturnValue(snapshot(13, true, 1));
    runFrame();
    expect(clock.position.value).toBe(13);
  });

  // Review Focus 5: the chart ended, so the transport shows play.
  it("shows the chart stopping when the engine stops it", () => {
    const playback = fakePlayback(true);
    engine.readPlayback.mockReturnValue(snapshot(1, true, 1));
    const clock = usePlaybackClock(playback);
    clock.start();
    runFrame();
    engine.readPlayback.mockReturnValue(snapshot(90.2, false, 1));
    runFrame();
    expect(playback.playing.value).toBe(false);
  });

  it("does not undo a click the engine has not applied yet", () => {
    const playback = fakePlayback(false);
    engine.readPlayback.mockReturnValue(snapshot(1, false, 1));
    const clock = usePlaybackClock(playback);
    clock.start();
    runFrame();
    playback.playing.value = true; // the user pressed play
    runFrame(); // the engine still says paused this frame
    expect(playback.playing.value).toBe(true);
    expect(playback.markLoaded).not.toHaveBeenCalled();
  });

  it("reads the timeline again only when the chart changes", () => {
    const timeline = { length: 4, measures: [] };
    engine.readTimeline.mockReturnValue(timeline);
    engine.readPlayback.mockReturnValue(snapshot(0, false, 1));
    const clock = usePlaybackClock(fakePlayback(false));
    clock.start();
    runFrame();
    runFrame();
    expect(engine.readTimeline).toHaveBeenCalledTimes(1);
    expect(clock.timeline.value).toEqual(timeline);
    engine.readPlayback.mockReturnValue(snapshot(0, false, 2));
    engine.readTimeline.mockReturnValue(null);
    runFrame();
    expect(engine.readTimeline).toHaveBeenCalledTimes(2);
    expect(clock.timeline.value).toBeNull();
  });

  it("stops asking for frames when stopped", () => {
    engine.readPlayback.mockReturnValue(snapshot(5, true, 1));
    const clock = usePlaybackClock(fakePlayback(true));
    clock.start();
    clock.stop();
    expect(frames.size).toBe(0);
  });

  it("waits for the engine before it has booted", () => {
    engine.readPlayback.mockReturnValue(null);
    const clock = usePlaybackClock(fakePlayback(false));
    clock.start();
    runFrame();
    expect(clock.position.value).toBe(0);
    expect(frames.size).toBe(1);
  });
});
