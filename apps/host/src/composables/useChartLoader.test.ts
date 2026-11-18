import { beforeEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";

// The engine is a wasm module the test cannot load; the loader's contract
// with it is which calls it makes, and when. `fetchChart` answers when the
// test says so, to stand for a download still in flight.
let resolveChart: (text: string) => void;
const engine = vi.hoisted(() => ({
  fetchChart: vi.fn(),
  fetchSheetInfo: vi.fn(),
  loadChart: vi.fn(),
  pause: vi.fn(),
  setSpeed: vi.fn(),
  setNoteSpeed: vi.fn(),
}));
vi.mock("~/composables/useEngine", () => engine);
vi.mock("~/utils/audioUnlock", () => ({ resumeAudio: vi.fn() }));

import useChartLoader from "./useChartLoader";
import type { Playback } from "./usePlayback";

function fakePlayback(): Playback {
  return {
    speed: ref(1),
    noteSpeed: ref(8),
    markLoaded: vi.fn(),
  } as unknown as Playback;
}

beforeEach(() => {
  engine.fetchChart.mockImplementation(
    () => new Promise<string>((resolve) => (resolveChart = resolve)),
  );
  engine.fetchSheetInfo.mockResolvedValue({ song: { id: "s" } });
  engine.loadChart.mockResolvedValue(undefined);
});

describe("useChartLoader", () => {
  it("loads a deep-linked chart into the engine", async () => {
    const loader = useChartLoader(fakePlayback());
    const pending = loader.autoLoad({ song: "s", type: "dx", difficulty: "master" });
    resolveChart("(120){4}1,E");
    await pending;
    expect(engine.loadChart).toHaveBeenCalledWith("(120){4}1,E");
  });

  // The engine keeps its speeds across charts and pages; the sliders start
  // fresh with each page. Whatever they show is what the new chart plays at.
  it("hands the engine the speeds the sliders show", async () => {
    const loader = useChartLoader(fakePlayback());
    const pending = loader.autoLoad({ song: "s", type: "dx", difficulty: "master" });
    resolveChart("(120){4}1,E");
    await pending;
    expect(engine.setSpeed).toHaveBeenCalledWith(1);
    expect(engine.setNoteSpeed).toHaveBeenCalledWith(8);
  });

  it("drops a chart that arrives after the page has gone", async () => {
    const loader = useChartLoader(fakePlayback());
    const pending = loader.autoLoad({ song: "s", type: "dx", difficulty: "master" });
    loader.dispose();
    resolveChart("(120){4}1,E");
    await pending;
    expect(engine.loadChart).not.toHaveBeenCalled();
    expect(loader.simai.value).toBe("");
  });
});
