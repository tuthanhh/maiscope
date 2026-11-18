import { ref, type Ref } from "vue";
import { readPlayback, readTimeline } from "~/composables/useEngine";
import type { Playback } from "~/composables/usePlayback";
import type { Timeline } from "~/utils/timeline";

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

    function frame(): void {
        handle = requestAnimationFrame(frame);
        const snapshot = readPlayback();
        if (!snapshot) return; // the engine is still booting
        position.value = snapshot.position;
        // Follow the engine when *it* changes — the chart reaching its end —
        // not whenever it differs: a click it has not applied yet would be
        // undone for a frame.
        if (
            lastPlaying !== null &&
            snapshot.playing !== lastPlaying &&
            playback.playing.value !== snapshot.playing
        ) {
            playback.markLoaded(snapshot.playing);
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
