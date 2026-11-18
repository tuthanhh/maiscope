import { afterEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import useFullscreen from "./useFullscreen";

// The Fullscreen API is the browser's; the composable's job is to start it
// on the stage and to keep `active` true to what the browser did — including
// an exit made with Escape, which never goes through `toggle`.

let fullscreenElement: Element | null = null;

// happy-dom has no Fullscreen API, so the properties are defined here.
function stubDocument(enabled: boolean): void {
  Object.defineProperty(document, "fullscreenEnabled", { configurable: true, get: () => enabled });
  Object.defineProperty(document, "fullscreenElement", {
    configurable: true,
    get: () => fullscreenElement,
  });
  document.exitFullscreen = vi.fn(async () => {
    fullscreenElement = null;
    document.dispatchEvent(new Event("fullscreenchange"));
  });
}

function stage(): HTMLElement {
  const el = document.createElement("div");
  el.requestFullscreen = vi.fn(async () => {
    fullscreenElement = el;
    document.dispatchEvent(new Event("fullscreenchange"));
  });
  return el;
}

afterEach(() => {
  fullscreenElement = null;
});

describe("useFullscreen", () => {
  it("enters on toggle and leaves on the next", async () => {
    stubDocument(true);
    const el = ref(stage());
    const fs = useFullscreen(el);
    await fs.toggle();
    expect(el.value.requestFullscreen).toHaveBeenCalled();
    expect(fs.active.value).toBe(true);
    await fs.toggle();
    expect(document.exitFullscreen).toHaveBeenCalled();
    expect(fs.active.value).toBe(false);
  });

  it("follows an exit made with Escape", async () => {
    stubDocument(true);
    const el = ref(stage());
    const fs = useFullscreen(el);
    await fs.toggle();
    fullscreenElement = null; // the browser left fullscreen on Escape
    document.dispatchEvent(new Event("fullscreenchange"));
    expect(fs.active.value).toBe(false);
  });

  it("is unsupported where the API is missing (iPhone Safari)", () => {
    stubDocument(false);
    expect(useFullscreen(ref(null)).supported).toBe(false);
  });
});
