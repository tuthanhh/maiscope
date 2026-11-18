import { ref, type Ref } from "vue";
import type { LocationQuery } from "vue-router";
import { fetchChart, fetchSheetInfo, loadChart, pause, setSpeed } from "~/composables/useEngine";
import type { Playback } from "~/composables/usePlayback";
import { resumeAudio } from "~/utils/audioUnlock";
import { sheetKey } from "~/utils/data";
import type { Sheet } from "~/types";

/**
 * The full deep link (`song` + `type` + `difficulty`), or null. `song` is the
 * minted id and `difficulty` the code (contract §0). v1's `?songId=<title>`
 * links do not resolve: a title is not an address in v2.
 */
function deepLink(query: LocationQuery): { song: string; type: string; difficulty: string } | null {
    const { song, type, difficulty } = query;
    return typeof song === "string" && typeof type === "string" && typeof difficulty === "string"
        ? { song, type, difficulty }
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
    dispose: () => void;
} {
    // Catalog metadata for the loaded sheet (null for a manually pasted chart).
    const sheetInfo = ref<Sheet | null>(null);
    const simai = ref("");
    const loading = ref(false);
    const errorMsg = ref("");

    // Last deep-linked sheet, so re-activating with the same query doesn't reload.
    let lastKey = "";
    // Set when the page is left (visualizer-majdata 03): a download still in
    // flight then must not reach the engine, or the chart would play behind
    // whatever page the user went to and reappear on the next visit.
    let disposed = false;
    function dispose(): void {
        disposed = true;
    }

    // Deep-link: /visualizer?song=..&type=..&difficulty=.. (e.g. from the sheet
    // dialog) auto-fetches that chart from the backend.
    async function autoLoad(query: LocationQuery): Promise<void> {
        const link = deepLink(query);
        if (!link || disposed) return;
        const { song, type, difficulty } = link;
        const key = sheetKey(song, type, difficulty);
        if (key === lastKey) return; // same chart already loaded
        lastKey = key;

        loading.value = true;
        errorMsg.value = "";
        // The previous sheet's info stays until the new one arrives, so the
        // title block does not unmount (and drop focus) between the two.

        // Info is best-effort and independent of chart availability — resolve and
        // display it on its own, so the info card still shows for songs with no
        // chart text (hasChart: false, the common case). Previously this was
        // awaited together with fetchChart via Promise.all, so a chart-fetch
        // failure (also common) rejected the whole thing and info never rendered.
        fetchSheetInfo(song, type, difficulty)
            .then((info) => {
                if (key === lastKey && !disposed) sheetInfo.value = info;
            })
            .catch(() => {
                if (key === lastKey && !disposed) sheetInfo.value = null;
            });

        try {
            const chart = await fetchChart(song, type, difficulty);
            // A later deep link superseded this one while it was in flight,
            // or the page was left.
            if (key !== lastKey || disposed) return;
            simai.value = chart;
            await loadChart(chart);

            // Sound can only start from a user gesture (e.g. arriving via Play in
            // the song modal). On a gesture-less hard reload, start paused — the
            // first transport click then both plays and unlocks audio.
            const active = navigator.userActivation?.isActive ?? false;
            if (active) {
                resumeAudio();
                playback.markLoaded(true); // engine auto-plays on parse
            } else {
                pause();
                playback.markLoaded(false);
            }
            if (playback.speed.value !== 1) setSpeed(playback.speed.value);
        } catch (err: unknown) {
            if (key === lastKey) errorMsg.value = err instanceof Error ? err.message : String(err);
        } finally {
            if (key === lastKey) loading.value = false;
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
            playback.markLoaded(true);
            // `lastKey` keeps the URL's deep link: the page reloads only when
            // the link itself changes, never over pasted text.
            sheetInfo.value = null; // pasted chart has no catalog metadata
            resumeAudio(); // this click is a gesture — unlock sound
            if (playback.speed.value !== 1) setSpeed(playback.speed.value);
        } catch (err: unknown) {
            errorMsg.value = err instanceof Error ? err.message : String(err);
        } finally {
            loading.value = false;
        }
    }

    return { sheetInfo, simai, loading, errorMsg, autoLoad, loadPasted, dispose };
}
