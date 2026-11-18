// Lazy bridge to the Bevy/wasm chart viewer (built by scripts/build-wasm.sh into
// ~/wasm). The wasm module auto-starts its Bevy App on init (#[wasm_bindgen(start)]
// in engine/src/lib.rs), grabbing the <canvas id="bevy"> that must already be in
// the DOM. So: mount the canvas first, THEN call ensureEngine().
//
// The 74 MB debug wasm is fetched lazily on first visit to the visualizer page,
// never as part of the main bundle. A release build (scripts/build-wasm.sh release)
// shrinks it by orders of magnitude.

import { GAME } from "~/app/game";
import { attachSong } from "~/utils/data";
import { fetchJson } from "~/utils/api";
import type { Sheet, Song } from "~/types";
import type { Timeline } from "~/utils/timeline";

type EngineModule = typeof import("~/wasm/engine.js");

let enginePromise: Promise<EngineModule> | null = null;
// The booted module, for the synchronous snapshot reads (`readPlayback`).
let engineModule: EngineModule | null = null;
// The wasm instance's linear memory, kept from init so the mobile overlay can
// report it. `WebAssembly.Memory` only grows, so this is a high-water mark,
// not a live figure — which is the number the memory-ceiling question wants.
let wasmMemory: WebAssembly.Memory | null = null;

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
      const mod = await import("~/wasm/engine.js");
      // Default export is wasm-bindgen's init(); fetches + instantiates the .wasm
      // and runs the #[wasm_bindgen(start)] entry, which launches the Bevy App.
      wasmMemory = (await mod.default()).memory;
      engineModule = mod;
      return mod;
    })();
  }
  return enginePromise;
}

// Feed a chart + its audio into the running engine.
export async function loadSong(
  chart: string,
  audio: Uint8Array,
): Promise<void> {
  const mod = await ensureEngine();
  mod.load_song(chart, audio);
}

// Play a chart with no audio. The chart clock is frame-driven, so it runs
// silently at the correct rate.
export async function loadChart(chart: string): Promise<void> {
  const mod = await ensureEngine();
  mod.load_chart(chart);
}

// Leaving Play: the engine drops the chart, every note and the music
// (visualizer-majdata 03). The canvas and the engine itself stay alive.
export async function unloadChart(): Promise<void> {
  const mod = await ensureEngine();
  mod.unload();
}

/** A sheet's address on the v2 API: `/api/songs/{id}/sheets/{type}/{code}`. */
function sheetUrl(songId: string, type: string, difficulty: string): string {
  return (
    `${GAME.apiBaseUrl}/songs/${encodeURIComponent(songId)}` +
    `/sheets/${encodeURIComponent(type)}/${encodeURIComponent(difficulty)}`
  );
}

// Fetch a sheet's simai from the global backend (apps/server) → raw simai
// text. Errors are plain text too (contract §0).
export async function fetchChart(
  songId: string,
  type: string,
  difficulty: string,
): Promise<string> {
  const res = await fetch(`${sheetUrl(songId, type, difficulty)}/chart`);
  if (!res.ok) {
    // The route's own errors are plain text; one the middleware raises
    // first (a 429) is the JSON envelope. Show the message either way.
    const body = await res.text();
    let message = body;
    if (res.headers.get("content-type")?.includes("json")) {
      try {
        message = (JSON.parse(body) as { message?: string }).message ?? body;
      } catch {
        // Not JSON after all: the text is the message.
      }
    }
    throw new Error(`chart fetch failed (${res.status}): ${message}`);
  }
  return res.text();
}

// Fetch a sheet with its song (title/artist/level/noteCounts/…) for display,
// as the client shape: the song carries its cover URL and the sheet points at
// it, as in the Catalog.
export async function fetchSheetInfo(
  songId: string,
  type: string,
  difficulty: string,
): Promise<Sheet> {
  const { song, ...sheet } = await fetchJson<Omit<Sheet, "song" | "key"> & {
    song: Omit<Song, "sheets">;
  }>(sheetUrl(songId, type, difficulty));
  return attachSong({ ...song, sheets: [sheet as Sheet] }, GAME.dataSourceUrl).sheets[0];
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

// Jump to a chart time (seconds), playing or paused (visualizer-majdata 08).
export async function seek(secs: number): Promise<void> {
  (await ensureEngine()).seek(secs);
}

// The engine's latest playback snapshot (visualizer-majdata 09): position in
// chart seconds, whether it plays, and an id that changes with every load and
// unload. Synchronous, for a display-frame loop; null before the engine boots.
export function readPlayback(): { position: number; playing: boolean; chartId: number } | null {
  if (!engineModule) return null;
  return {
    position: engineModule.playback_position(),
    playing: engineModule.playback_playing(),
    chartId: engineModule.chart_id(),
  };
}

// The loaded chart's timeline, or null with no chart (or before boot).
export function readTimeline(): Timeline | null {
  const raw = engineModule?.timeline();
  return raw ? (JSON.parse(raw) as Timeline) : null;
}

// rate: 1.0 = normal. Affects both the audio rate and chart timing.
export async function setSpeed(rate: number): Promise<void> {
  (await ensureEngine()).set_song_speed(rate);
}

// ── Diagnostics ──────────────────────────────────────────────────────────────

/** One second of engine frame pacing. See engine/src/systems/frame_stats.rs. */
export type FrameStats = {
  fps: number;
  avg_ms: number;
  max_ms: number;
  frames: number;
  /** Frames over 33ms — visibly late at 30 FPS. */
  slow_frames: number;
  /** Frames over 100ms — long enough to starve the web-audio buffer. */
  stalled_frames: number;
  /** Physical canvas size, i.e. CSS size × devicePixelRatio. */
  canvas: string;
  scale_factor: number;
  /** MSAA samples on the main camera; 1 means off. */
  msaa: number;
  /** How far ahead of the visual hit guide sounds are requested. */
  audio_lead_ms: number;
};

// Read the engine's latest snapshot. `null` until the first second has
// elapsed. Polling, not an event: the engine publishes into a mailbox that JS
// reads, so the boundary stays one-directional per call (never JS←Bevy calls).
export async function frameStats(): Promise<FrameStats | null> {
  const raw = (await ensureEngine()).frame_stats();
  return raw ? (JSON.parse(raw) as FrameStats) : null;
}

/** Bytes the wasm instance has claimed so far, or null before init. */
export function wasmMemoryBytes(): number | null {
  return wasmMemory ? wasmMemory.buffer.byteLength : null;
}

// Render toggles for the mobile investigation (mobile-webview-spike 02). Each
// enqueues a command the engine applies next frame, like transport.
// samples: 1 = MSAA off, 2, 4 (Bevy default), 8.
export async function setMsaa(samples: number): Promise<void> {
  (await ensureEngine()).set_msaa(samples);
}
// scale: 1 renders at CSS pixels regardless of devicePixelRatio; 0 restores
// the browser's own ratio.
export async function setRenderScale(scale: number): Promise<void> {
  (await ensureEngine()).set_render_scale(scale);
}
// ms: guide sounds and metronome clicks are requested this far ahead of the
// visual hit, to cancel audio output latency.
export async function setAudioLeadMs(ms: number): Promise<void> {
  (await ensureEngine()).set_audio_lead_ms(ms);
}

// ── Automatic audio lead ─────────────────────────────────────────────────────
//
// The delay between "engine asks for a sound" and "ear hears it" on the web is
// three parts, and the browser can report two of them:
//
//   cpal's webaudio backend  — two AudioBuffers of 2048 frames, refilled from
//                              the main thread; a sound requested now starts
//                              at the earliest after the buffer in flight and
//                              at the latest after both. Not observable from
//                              JS, so the midpoint is assumed.
//   AudioContext.baseLatency — the context's own processing buffer.
//   AudioContext.outputLatency — context output to the hardware. The big one on
//                              Android, and it changes when Bluetooth connects.
//
// The sum is what the engine should run ahead by (mobile-webview-spike 02,
// P2 fault a). Desktop reports a few ms for the context parts, so the lead
// there is mostly the buffer term — which was always being paid, just unseen.

/** Frames per cpal webaudio buffer. cpal 0.15 `DEFAULT_BUFFER_SIZE`. */
const CPAL_WEB_BUFFER_FRAMES = 2048;
/** How many of the two buffers a request waits through, on average. */
const CPAL_WEB_BUFFERS_IN_FLIGHT = 1.5;

export type AutoLead = {
  leadMs: number;
  bufferMs: number;
  baseMs: number;
  outputMs: number;
  running: boolean;
};

/** The lead the browser's own numbers imply, or null before any context exists. */
export function computeAutoLead(
  latency: {
    baseMs: number;
    outputMs: number;
    sampleRate: number;
    running: boolean;
  } | null,
): AutoLead | null {
  if (!latency) return null;
  const bufferMs =
    (CPAL_WEB_BUFFER_FRAMES / latency.sampleRate) *
    1000 *
    CPAL_WEB_BUFFERS_IN_FLIGHT;
  return {
    leadMs: Math.round(bufferMs + latency.baseMs + latency.outputMs),
    bufferMs: Math.round(bufferMs),
    baseMs: Math.round(latency.baseMs),
    outputMs: Math.round(latency.outputMs),
    running: latency.running,
  };
}

// Note (scroll) speed — how fast notes travel, independent of playback rate.
// `7.0` is the engine's default (engine/src/wasm_bridge.rs:set_note_speed).
export async function setNoteSpeed(speed: number): Promise<void> {
  (await ensureEngine()).set_note_speed(speed);
}
