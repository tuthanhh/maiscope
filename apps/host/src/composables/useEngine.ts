// Lazy bridge to the Bevy/wasm chart viewer (built by engine/build-wasm.sh into
// ~/wasm). The wasm module auto-starts its Bevy App on init (#[wasm_bindgen(start)]
// in engine/src/lib.rs), grabbing the <canvas id="bevy"> that must already be in
// the DOM. So: mount the canvas first, THEN call ensureEngine().
//
// The 74 MB debug wasm is fetched lazily on first visit to the visualizer page,
// never as part of the main bundle. A release build (engine/build-wasm.sh release)
// shrinks it by orders of magnitude.

import { GAME } from "~/app/game";
import type { Sheet } from "~/types";

type EngineModule = typeof import("~/wasm/maiscope_viewer.js");

let enginePromise: Promise<EngineModule> | null = null;

// ── Persistent engine canvas ────────────────────────────────────────────────
// Bevy/wasm grabs <canvas id="bevy"> exactly once on init and can't re-attach to
// a new element. With <keep-alive> removed the visualizer page remounts on every
// route open, which would destroy a page-owned canvas. So we own ONE canvas in
// JS that outlives any component: it's re-parented into the page on mount and
// parked in an off-screen holder on unmount, keeping the engine attached without
// re-instantiating the (74MB) wasm.

let canvasEl: HTMLCanvasElement | null = null;
let holderEl: HTMLElement | null = null;

function holder(): HTMLElement {
  if (!holderEl) {
    holderEl = document.createElement("div");
    holderEl.setAttribute("aria-hidden", "true");
    holderEl.style.cssText =
      "position:absolute;left:-9999px;top:-9999px;width:0;height:0;overflow:hidden;";
    document.body.appendChild(holderEl);
  }
  return holderEl;
}

/// The singleton engine canvas, created once and kept alive for the session.
export function engineCanvas(): HTMLCanvasElement {
  if (!canvasEl) {
    canvasEl = document.createElement("canvas");
    canvasEl.id = "bevy";
    canvasEl.className = "mv-canvas";
    // Live in the DOM (off-screen) so `#bevy` exists when the engine inits.
    holder().appendChild(canvasEl);
  }
  return canvasEl;
}

/// Move the canvas into the page's stage slot (call on page mount).
export function mountEngineCanvas(parent: HTMLElement): void {
  parent.appendChild(engineCanvas());
}

/// Park the canvas off-screen so it survives the page unmount (call on unmount).
export function detachEngineCanvas(): void {
  if (canvasEl) holder().appendChild(canvasEl);
}

// Boot the wasm module exactly once per page session. Bevy's App starts itself
// on init; we just await readiness and hand back the module for load_song().
export function ensureEngine(): Promise<EngineModule> {
  if (!enginePromise) {
    enginePromise = (async () => {
      engineCanvas(); // ensure <canvas id="bevy"> is in the DOM before init
      const mod = await import("~/wasm/maiscope_viewer.js");
      // Default export is wasm-bindgen's init(); fetches + instantiates the .wasm
      // and runs the #[wasm_bindgen(start)] entry, which launches the Bevy App.
      await mod.default();
      return mod;
    })();
  }
  return enginePromise;
}

// Feed a chart + its audio into the running engine.
// Whether a chart has ever been handed to the engine this session. Module-level
// so it survives the visualizer page remounting on route changes — lets the page
// decide to replay-from-0 on each open.
let chartLoaded = false;
export function hasChart(): boolean {
  return chartLoaded;
}

export async function loadSong(
  chart: string,
  audio: Uint8Array,
): Promise<void> {
  const mod = await ensureEngine();
  mod.load_song(chart, audio);
  chartLoaded = true;
}

// Play a chart with no audio. The chart clock is frame-driven, so it runs
// silently at the correct rate.
export async function loadChart(chart: string): Promise<void> {
  const mod = await ensureEngine();
  mod.load_chart(chart);
  chartLoaded = true;
}

// Fetch a sheet's simai from the global backend (apps/server). sheetExpr parts:
// GET /sheets/{songId}/chart?type=..&difficulty=.. → raw simai text.
export async function fetchChart(
  songId: string,
  type: string,
  difficulty: string,
): Promise<string> {
  const url =
    `${GAME.apiBaseUrl}/sheets/${encodeURIComponent(songId)}/chart` +
    `?type=${encodeURIComponent(type)}&difficulty=${encodeURIComponent(difficulty)}`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`chart fetch failed (${res.status}): ${await res.text()}`);
  }
  return res.text();
}

// Fetch a sheet's catalog metadata (title/artist/level/noteCounts/…) for display.
// GET /sheets/{sheetExpr} where sheetExpr = songId|type|difficulty.
export async function fetchSheetInfo(sheetExpr: string): Promise<Sheet> {
  const url = `${GAME.apiBaseUrl}/sheets/${encodeURIComponent(sheetExpr)}`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`sheet fetch failed (${res.status}): ${await res.text()}`);
  }
  return res.json() as Promise<Sheet>;
}

// ── Transport ────────────────────────────────────────────────────────────────
// Each just enqueues an intent the engine drains next frame (apply_commands in
// engine/src/systems/visual/spawning.rs). Safe to call before a song is loaded —
// the command queues harmlessly.

export async function pause(): Promise<void> {
  (await ensureEngine()).pause();
}

export async function resume(): Promise<void> {
  (await ensureEngine()).resume();
}

export async function restart(): Promise<void> {
  (await ensureEngine()).restart();
}

// rate: 1.0 = normal. Affects both the audio rate and chart timing.
export async function setSpeed(rate: number): Promise<void> {
  (await ensureEngine()).set_song_speed(rate);
}

// Note (scroll) speed — how fast notes travel, independent of playback rate.
// The engine doesn't export this yet; guarded so it no-ops until it does.
// TODO(engine): add #[wasm_bindgen] set_note_speed(speed: f32) in engine/src/lib.rs.
export async function setNoteSpeed(speed: number): Promise<void> {
  const mod = (await ensureEngine()) as {
    set_note_speed?: (s: number) => void;
  };
  mod.set_note_speed?.(speed);
}
