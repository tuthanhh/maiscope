import { ref, type Ref } from "vue";
import { readPlayback, readTimeline } from "~/composables/useEngine";
import type { Playback } from "~/composables/usePlayback";
import type { Timeline } from "~/utils/timeline";

// Display frames a press may take to reach the engine before the transport
// gives up on it and shows the engine's state (a press of play at the end of
// the chart, which the engine does not take).
const SETTLE_FRAMES = 6;

// The engine's position and timeline, copied into the page once per display
// frame while the page is open (visualizer-majdata 10). The engine publishes
// them; this only reads, so the JS→Bevy boundary stays the mailbox.
export default function usePlaybackClock(playback: Playback): {
    position: Ref<number>;
    timeline: Ref<Timeline | null>;
    start: () => void;
    stop: () => void;
} {
    const position = ref(0);
    const timeline = ref<Timeline | null>(null);
    let handle: number | null = null;
    let lastPlaying: boolean | null = null;
    let lastChart: number | null = null;
    let disagreeing = 0;

    function frame(): void {
        handle = requestAnimationFrame(frame);
        const snapshot = readPlayback();
        if (!snapshot) return; // the engine is still booting
        position.value = snapshot.position;
        // Follow the engine when *it* changes — the chart reaching its end —
        // or when it has disagreed for a few frames; not at the first frame
        // it differs, which would undo a click it has not applied yet.
        disagreeing = playback.playing.value !== snapshot.playing ? disagreeing + 1 : 0;
        const changed = lastPlaying !== null && snapshot.playing !== lastPlaying;
        if (disagreeing > 0 && (changed || disagreeing >= SETTLE_FRAMES)) {
            playback.markLoaded(snapshot.playing);
            disagreeing = 0;
        }
        lastPlaying = snapshot.playing;
        if (snapshot.chartId !== lastChart) {
            lastChart = snapshot.chartId;
            timeline.value = readTimeline();
        }
    }

    function start(): void {
        if (handle === null) handle = requestAnimationFrame(frame);
    }

    function stop(): void {
        if (handle !== null) cancelAnimationFrame(handle);
        handle = null;
    }

    return { position, timeline, start, stop };
}
