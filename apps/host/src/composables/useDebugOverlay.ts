import { computed, ref, type ComputedRef, type Ref } from "vue";
import type { RouteLocationNormalizedLoaded } from "vue-router";
import { frameStats, wasmMemoryBytes, type AutoLead, type FrameStats } from "~/composables/useEngine";

// ── Frame-pacing readout ─────────────────────────────────────────────────────
// With the debug flag on, the page overlays the engine's one-second frame
// snapshot on the canvas. This is how a phone reports numbers: no devtools, no
// console, just read the corner. See mobile-webview-spike 02.
//
// The flag is sticky. An installed PWA has no address bar, so `?debug` in a
// URL cannot reach it — but it shares origin storage with the browser it was
// installed from. Visit `?debug` once in a Chrome tab and the installed app
// shows the overlay from then on; `?debug=0` turns it off again. Wrapped in
// try/catch because storage can be blocked or evicted, and a readout must
// never be the thing that breaks the page.
const DEBUG_KEY = "mv:debug";

function readDebugFlag(route: RouteLocationNormalizedLoaded): boolean {
    const q = route.query.debug;
    if (q !== undefined) {
        const on = q !== "0" && q !== "false";
        try {
            if (on) localStorage.setItem(DEBUG_KEY, "1");
            else localStorage.removeItem(DEBUG_KEY);
        } catch {
            /* storage unavailable: the query still works for this visit */
        }
        return on;
    }
    try {
        return localStorage.getItem(DEBUG_KEY) === "1";
    } catch {
        return false;
    }
}

// Wasm linear memory is a high-water mark (it never shrinks), which is the
// ceiling mobile-webview-spike 01 asks for. JS heap is Chrome-only and shown
// when the browser offers it; both numbers are MiB.
function memoryLine(): string {
    const wasm = wasmMemoryBytes();
    const perf = performance as Performance & {
        memory?: { usedJSHeapSize: number };
    };
    const heap = perf.memory?.usedJSHeapSize;
    const mib = (b: number) => (b / 1048576).toFixed(0);
    return (
        `wasm mem ${wasm === null ? "?" : mib(wasm)}MiB` +
        (heap === undefined ? "" : `  js heap ${mib(heap)}MiB`)
    );
}

export default function useDebugOverlay(
    route: RouteLocationNormalizedLoaded,
    lead: { autoLead: Ref<AutoLead | null>; manualLead: ComputedRef<number | null> },
): {
    debug: Ref<boolean>;
    statsLines: ComputedRef<string[]>;
    start: () => void;
    stop: () => void;
} {
    const debug = ref(readDebugFlag(route));
    const stats = ref<FrameStats | null>(null);
    let timer: ReturnType<typeof setInterval> | null = null;

    function start(): void {
        if (!debug.value || timer !== null) return;
        timer = setInterval(async () => {
            try {
                stats.value = await frameStats();
            } catch {
                stats.value = null;
            }
        }, 1000);
    }
    function stop(): void {
        if (timer !== null) clearInterval(timer);
        timer = null;
    }

    function leadLine(appliedMs: number): string {
        const a = lead.autoLead.value;
        const parts = a
            ? `buf ${a.bufferMs} + base ${a.baseMs} + out ${a.outputMs} = ${a.leadMs}${a.running ? "" : " (ctx not running)"}`
            : "no AudioContext yet";
        const mode = lead.manualLead.value !== null ? `manual ${lead.manualLead.value}` : "auto";
        return `lead ${appliedMs}ms (${mode})  ${parts}`;
    }

    const statsLines = computed(() => {
        const s = stats.value;
        if (!s) return ["waiting for first second…"];
        return [
            `${s.fps.toFixed(1)} fps  avg ${s.avg_ms.toFixed(1)}ms  max ${s.max_ms.toFixed(0)}ms`,
            `slow >33ms: ${s.slow_frames}/${s.frames}  stalled >100ms: ${s.stalled_frames}`,
            `canvas ${s.canvas} @ ${s.scale_factor.toFixed(2)}x  dpr ${window.devicePixelRatio}  msaa ${s.msaa}`,
            leadLine(s.audio_lead_ms),
            memoryLine(),
        ];
    });

    return { debug, statsLines, start, stop };
}
