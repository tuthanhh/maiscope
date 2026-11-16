// Browser autoplay policy starts every AudioContext suspended until a user
// gesture. The Bevy engine's audio (kira) creates its context internally, so we
// can't resume it directly — instead we wrap the AudioContext constructor to
// collect every instance the page makes, and resume them on user interaction.
//
// `installAudioUnlock()` must run BEFORE any audio code (call it at app boot in
// main.ts) so the wrapper is in place when kira constructs its context.

const contexts = new Set<AudioContext>();

export function installAudioUnlock(): void {
  const w = window as unknown as {
    AudioContext?: typeof AudioContext;
    webkitAudioContext?: typeof AudioContext;
    __audioUnlockInstalled?: boolean;
  };
  const Orig = w.AudioContext ?? w.webkitAudioContext;
  if (!Orig || w.__audioUnlockInstalled) return;
  w.__audioUnlockInstalled = true;

  class Tracked extends Orig {
    constructor(...args: ConstructorParameters<typeof AudioContext>) {
      super(...args);
      contexts.add(this);
    }
  }
  w.AudioContext = Tracked;
  w.webkitAudioContext = Tracked;

  // Backstop: any interaction anywhere resumes whatever is suspended.
  const ev = ["pointerdown", "keydown", "touchstart"] as const;
  ev.forEach((e) =>
    window.addEventListener(e, resumeAudio, { capture: true, passive: true }),
  );
}

// Resume every suspended context. Safe to call repeatedly. A user gesture's
// activation persists a few seconds, so calling this shortly after a click
// (even for a context created just after) still succeeds.
export function resumeAudio(): void {
  contexts.forEach((c) => {
    if (c.state === "suspended") c.resume().catch(() => {});
  });
}

/** What the browser reports about the path from the engine's mixer to the ear. */
export type AudioLatency = {
  /** `AudioContext.baseLatency`: the context's own processing buffer, ms. */
  baseMs: number;
  /**
   * `AudioContext.outputLatency`: from the context's output to the device,
   * ms. Chrome reports it (and it jumps when Bluetooth connects); Safari and
   * Firefox leave it undefined, read here as 0.
   */
  outputMs: number;
  /** The context's sample rate, which sets how long cpal's buffers last. */
  sampleRate: number;
  /** Whether the context is actually running — latencies read 0 until it is. */
  running: boolean;
};

// The engine's context is the one kira made through the wrapper above. Pick
// the running one if any; the engine creates exactly one, so this is it.
export function audioLatency(): AudioLatency | null {
  let picked: AudioContext | null = null;
  contexts.forEach((c) => {
    if (!picked || (picked.state !== "running" && c.state === "running")) {
      picked = c;
    }
  });
  if (!picked) return null;
  const ctx = picked as AudioContext & { outputLatency?: number };
  return {
    baseMs: (ctx.baseLatency ?? 0) * 1000,
    outputMs: (ctx.outputLatency ?? 0) * 1000,
    sampleRate: ctx.sampleRate,
    running: ctx.state === "running",
  };
}
