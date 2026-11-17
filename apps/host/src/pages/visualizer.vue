<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useRoute } from "vue-router";
import useI18n from "~/composables/useI18n";
import usePageTitle from "~/composables/usePageTitle";
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
import VizSheetCard from "~/components/visualizer/VizSheetCard.vue";
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

usePageTitle(() => ({ title: t("page-title.visualizer") as string }));
</script>

<template>
    <div class="mv-root mv-viz">
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
                    :is-major="(v) => Number.isInteger(v)"
                    :format="(v) => String(v)"
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
                    :is-major="() => true"
                    :format="(v) => v.toFixed(1)"
                    :readout="`${speed.toFixed(1)}×`"
                    @change="playback.applySpeed"
                />
            </div>
        </div>

        <!-- chart info + loader -->
        <div class="mv-viz-side">
            <div class="mv-label mv-side-label">
                {{ t("page.viz.chartInfo") }}
            </div>

            <!-- rich info when a catalog sheet is loaded -->
            <VizSheetCard v-if="sheetInfo" :sheet="sheetInfo" />
            <div v-else class="mv-awaiting">{{ t("page.viz.awaiting") }}</div>

            <VizPastePanel
                v-model="simai"
                :loading="loading"
                :engine-ready="status === 'ready'"
                :error-msg="errorMsg"
                @load="loadPasted"
            />
        </div>
    </div>
</template>

<style scoped>
.mv-viz {
    display: flex;
    flex-wrap: wrap;
}
.mv-viz-stage {
    flex: 1 1 380px;
    padding: 20px;
    border-right: 1px solid var(--mv-bd);
}
.mv-viz-side {
    flex: 1 1 280px;
    min-width: 260px;
    padding: 20px;
}
.mv-side-label {
    margin-bottom: 6px;
}
.mv-awaiting {
    font-size: 13px;
    color: var(--mv-mut);
    padding: 20px 0;
}

/* --- Container & Layout --- */
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
