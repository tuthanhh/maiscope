<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute } from "vue-router";
import useI18n from "~/composables/useI18n";
import usePageTitle from "~/composables/usePageTitle";
import useSelectedSheets from "~/composables/useSelectedSheets";
import { useDataStore } from "~/stores/data";
import {
    ensureEngine,
    mountEngineCanvas,
    detachEngineCanvas,
    hasChart,
    pause,
    restart,
    setMsaa,
    setRenderScale,
    setAudioLeadMs,
} from "~/composables/useEngine";
import useAutoLead from "~/composables/useAutoLead";
import useDebugOverlay from "~/composables/useDebugOverlay";
import usePlayback, { NOTE_SPEED, SPEED } from "~/composables/usePlayback";
import useChartLoader, { hasDeepLink } from "~/composables/useChartLoader";
import VizStage from "~/components/visualizer/VizStage.vue";
import VizTransport from "~/components/visualizer/VizTransport.vue";
import VizSlider from "~/components/visualizer/VizSlider.vue";
import VizTitleBlock from "~/components/visualizer/VizTitleBlock.vue";
import VizPastePanel from "~/components/visualizer/VizPastePanel.vue";

defineOptions({ name: "GameVisualizerPage" });

const { t } = useI18n();
const route = useRoute();

type EngineStatus = "booting" | "ready" | "error";
const status = ref<EngineStatus>("booting");

const lead = useAutoLead(route);
const overlay = useDebugOverlay(route, lead);
const playback = usePlayback();
const { loaded, playing, speed, noteSpeed } = playback;
const { sheetInfo, simai, loading, errorMsg, autoLoad, loadPasted } = useChartLoader(playback);

// A/B toggles, query-only and applied once the engine is up:
//   ?msaa=4   MSAA back on (the engine now defaults to off)
//   ?scale=1  render at CSS pixels, ignoring dpr   ?scale=0 back to dpr
//   ?lead=120 fixed audio lead, overriding the automatic one
// Not sticky on purpose: an experiment should not outlive its URL. The overlay
// shows the values the engine actually applied.
function applyRenderToggles(): void {
    const msaa = Number(route.query.msaa);
    if (Number.isInteger(msaa) && msaa >= 1) setMsaa(msaa);
    const scale = Number(route.query.scale);
    if (Number.isFinite(scale) && scale >= 0 && route.query.scale !== undefined) {
        setRenderScale(scale);
    }
    if (lead.manualLead.value !== null) setAudioLeadMs(lead.manualLead.value);
}

// The page remounts on every route open (no <keep-alive>). The engine canvas is
// a JS-owned singleton (see useEngine): re-parent it into this page's stage on
// mount, park it off-screen on unmount — the wasm engine stays attached without
// re-instantiating. Boot the engine once; subsequent mounts just reattach.
const stage = ref<InstanceType<typeof VizStage> | null>(null);

onMounted(async () => {
    try {
        if (stage.value?.host) mountEngineCanvas(stage.value.host);
        await ensureEngine();
        status.value = "ready";
        overlay.start();
        applyRenderToggles();
        lead.start();
        // Replay any already-loaded chart from the start on each open. A deep-link
        // (autoLoad) reloads its own chart and supersedes this.
        if (hasChart() && !hasDeepLink(route.query)) {
            restart();
            loaded.value = true;
            playing.value = true;
        }
        await autoLoad(route.query);
    } catch (err: unknown) {
        status.value = "error";
        errorMsg.value = err instanceof Error ? err.message : String(err);
    }
});

// Leaving the route: pause audio/visuals and park the canvas so it survives the
// unmount (the engine keeps running on the detached canvas).
onUnmounted(() => {
    if (loaded.value) pause();
    detachEngineCanvas();
    overlay.stop();
    lead.stop();
});

// Another difficulty of the song is a link to this same route: the page does
// not remount, so a changed deep link loads here.
watch(
    () => route.query,
    (query) => {
        if (status.value === "ready") autoLoad(query);
    },
);

// The title block's siblings come from the Catalog: the single-sheet fetch
// carries no other sheets.
const dataStore = useDataStore();
const siblings = computed(() => {
    const sheet = sheetInfo.value;
    if (!sheet) return [];
    const song = dataStore.currentData.songs.find((s) => s.id === sheet.song.id);
    return song?.sheets.filter((s) => s.key !== sheet.key) ?? [];
});

const { isBookmarked, toggleSheetSelection } = useSelectedSheets();
const bookmarked = computed(() => !!sheetInfo.value && isBookmarked(sheetInfo.value.key));

// Share: the system sheet where there is one (phones), else the clipboard.
const shareNote = ref("");
async function share(): Promise<void> {
    const url = window.location.href;
    try {
        if (navigator.share) {
            await navigator.share({ title: sheetInfo.value?.song.title, url });
            return;
        }
        await navigator.clipboard.writeText(url);
        shareNote.value = t("description.copied") as string;
        window.setTimeout(() => (shareNote.value = ""), 2000);
    } catch {
        // Dismissed or denied: nothing to report.
    }
}

// Slider tick rules, hoisted so the sliders' computed ticks are not rebuilt
// on every page render (the `?debug` readout re-renders once a second).
const isWholeNumber = (v: number) => Number.isInteger(v);
const everyTick = () => true;
const plain = (v: number) => String(v);
const oneDecimal = (v: number) => v.toFixed(1);

usePageTitle(() => ({ title: t("page-title.visualizer") as string }));
</script>

<template>
    <div class="mv-viz">
        <div class="mv-viz-stage">
            <!-- engine canvas: the Bevy app renders the chart here -->
            <VizStage
                ref="stage"
                :status="status"
                :error-msg="errorMsg"
                :debug="overlay.debug.value"
                :stats-lines="overlay.statsLines.value"
            />

            <!-- transport: drives the engine clock (disabled until a song is loaded) -->
            <VizTransport
                :loaded="loaded"
                :playing="playing"
                @restart="playback.restartFromTop"
                @toggle="playback.togglePlay"
            />

            <!-- speed controls: stepped sliders with -/+ steppers -->
            <div class="mv-sliders" :class="{ 'mv-sliders--off': !loaded }">
                <VizSlider
                    :label="t('page.viz.noteSpeed') as string"
                    :value="noteSpeed"
                    :min="NOTE_SPEED.min"
                    :max="NOTE_SPEED.max"
                    :step="NOTE_SPEED.step"
                    :disabled="!loaded"
                    :is-major="isWholeNumber"
                    :format="plain"
                    :readout="noteSpeed.toFixed(1)"
                    @change="playback.applyNoteSpeed"
                />
                <VizSlider
                    :label="t('page.viz.playSpeed') as string"
                    :value="speed"
                    :min="SPEED.min"
                    :max="SPEED.max"
                    :step="SPEED.step"
                    :disabled="!loaded"
                    :is-major="everyTick"
                    :format="oneDecimal"
                    :readout="`${speed.toFixed(1)}×`"
                    @change="playback.applySpeed"
                />
            </div>
        </div>

        <div class="mv-viz-info">
            <VizTitleBlock
                v-if="sheetInfo"
                :sheet="sheetInfo"
                :siblings="siblings"
                :bookmarked="bookmarked"
                :share-note="shareNote"
                @bookmark="toggleSheetSelection(sheetInfo)"
                @share="share"
            />
            <p v-else class="t-body t-muted mv-viz-awaiting">{{ t("page.viz.awaiting") }}</p>
        </div>

        <VizPastePanel
            v-model="simai"
            class="mv-viz-side"
            :loading="loading"
            :engine-ready="status === 'ready'"
            :error-msg="errorMsg"
            @load="loadPasted"
        />
    </div>
</template>

<style scoped lang="scss">
@use "~/assets/styles/breakpoints" as bp;

// design-direction "Chart page": the playback surface, the title block under
// it, and the chart text — a side column the height of the stage from
// `--bp-lg`, stacked below the title block under it.
.mv-viz {
    display: grid;
    grid-template-areas:
        "stage"
        "info"
        "side";
    gap: var(--s-8);
    max-width: var(--max-w);
    margin: 0 auto;
    padding: var(--s-6);
}
.mv-viz-stage {
    grid-area: stage;
    min-width: 0;
}
.mv-viz-info {
    grid-area: info;
    min-width: 0;
}
.mv-viz-side {
    grid-area: side;
    min-width: 0;
}
.mv-viz-awaiting {
    margin: 0;
}
@media (min-width: bp.$bp-lg) {
    .mv-viz {
        grid-template-columns: minmax(0, 1fr) var(--drawer-w);
        grid-template-areas:
            "stage side"
            "info .";
    }
}
@media (max-width: #{bp.$bp-md - 1px}) {
    .mv-viz {
        padding: var(--s-4);
        gap: var(--s-6);
    }
}

/* --- playback surface, unchanged --- */
.mv-sliders {
    display: flex;
    flex-direction: column;
    gap: 24px; /* Space between the two slider rows */
    font-family: var(--mv-mono);
    width: 100%;
}

.mv-sliders--off {
    opacity: 0.5;
    pointer-events: none;
}
</style>
