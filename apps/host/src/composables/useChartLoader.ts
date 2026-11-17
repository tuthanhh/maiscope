import { ref, type Ref } from "vue";
import type { LocationQuery } from "vue-router";
import { fetchChart, fetchSheetInfo, loadChart, pause, setSpeed } from "~/composables/useEngine";
import type { Playback } from "~/composables/usePlayback";
import { resumeAudio } from "~/utils/audioUnlock";
import { GAME } from "~/app/game";
import type { Sheet } from "~/types";

/** The full deep link (songId + type + difficulty), or null. */
function deepLink(query: LocationQuery): { songId: string; type: string; difficulty: string } | null {
    const { songId, type, difficulty } = query;
    return typeof songId === "string" && typeof type === "string" && typeof difficulty === "string"
        ? { songId, type, difficulty }
        : null;
}

export function hasDeepLink(query: LocationQuery): boolean {
    return deepLink(query) !== null;
}

/**
 * Getting a chart into the engine: from a deep link (fetched from the API with
 * its Catalog metadata) or from pasted text. Owns the loading state and the
 * sheet metadata; drives the transport through `playback`.
 */
export default function useChartLoader(playback: Playback): {
    sheetInfo: Ref<Sheet | null>;
    simai: Ref<string>;
    loading: Ref<boolean>;
    errorMsg: Ref<string>;
    autoLoad: (query: LocationQuery) => Promise<void>;
    loadPasted: () => Promise<void>;
} {
    // Catalog metadata for the loaded sheet (null for a manually pasted chart).
    const sheetInfo = ref<Sheet | null>(null);
    const simai = ref("");
    const loading = ref(false);
    const errorMsg = ref("");

    // Last deep-linked sheet, so re-activating with the same query doesn't reload.
    let lastKey = "";

    // Deep-link: /visualizer?songId=..&type=..&difficulty=.. (e.g. from the sheet
    // dialog) auto-fetches that chart from the backend.
    async function autoLoad(query: LocationQuery): Promise<void> {
        const link = deepLink(query);
        if (!link) return;
        const { songId, type, difficulty } = link;
        const key = `${songId}|${type}|${difficulty}`;
        if (key === lastKey) return; // same chart already loaded
        lastKey = key;

        loading.value = true;
        errorMsg.value = "";
        sheetInfo.value = null;

        // Info is best-effort and independent of chart availability — resolve and
        // display it on its own, so the info card still shows for songs with no
        // chart text (hasChart: false, the common case). Previously this was
        // awaited together with fetchChart via Promise.all, so a chart-fetch
        // failure (also common) rejected the whole thing and info never rendered.
        fetchSheetInfo(key)
            .then((info) => {
                // Resolve the cover URL the same way preprocessData would (the API
                // sends only the raw imageName), so MvCover shows the real jacket.
                if (info.imageName && info.imageUrl == null) {
                    info.imageUrl = new URL(
                        info.imageName,
                        `${GAME.dataSourceUrl}/img/cover/`,
                    ).toString();
                }
                if (key === lastKey) sheetInfo.value = info;
            })
            .catch(() => {});

        try {
            const chart = await fetchChart(songId, type, difficulty);
            simai.value = chart;
            await loadChart(chart);
            playback.loaded.value = true;

            // Sound can only start from a user gesture (e.g. arriving via a click in
            // the sheet dialog). On a gesture-less hard reload, start paused — the
            // first transport click then both plays and unlocks audio.
            const active = navigator.userActivation?.isActive ?? false;
            if (active) {
                resumeAudio();
                playback.playing.value = true; // engine auto-plays on parse
            } else {
                pause();
                playback.playing.value = false;
            }
            if (playback.speed.value !== 1) setSpeed(playback.speed.value);
        } catch (err: unknown) {
            errorMsg.value = err instanceof Error ? err.message : String(err);
        } finally {
            loading.value = false;
        }
    }

    // Hand the pasted chart to the running engine. The chart clock is frame-driven,
    // so it plays (silently) with no audio track.
    async function loadPasted(): Promise<void> {
        if (!simai.value.trim()) return;
        loading.value = true;
        errorMsg.value = "";
        try {
            await loadChart(simai.value);
            // Engine transitions to Playing on its own once the chart is parsed.
            playback.loaded.value = true;
            playback.playing.value = true;
            lastKey = ""; // manual load supersedes any deep-link key
            sheetInfo.value = null; // pasted chart has no catalog metadata
            resumeAudio(); // this click is a gesture — unlock sound
            if (playback.speed.value !== 1) setSpeed(playback.speed.value);
        } catch (err: unknown) {
            errorMsg.value = err instanceof Error ? err.message : String(err);
        } finally {
            loading.value = false;
        }
    }

    return { sheetInfo, simai, loading, errorMsg, autoLoad, loadPasted };
}
