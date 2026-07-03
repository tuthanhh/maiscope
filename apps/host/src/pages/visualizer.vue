<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import useI18n from "~/composables/useI18n";
import useGameData from "~/composables/useGameData";
import usePageTitle from "~/composables/usePageTitle";
import {
    ensureEngine,
    mountEngineCanvas,
    detachEngineCanvas,
    hasChart,
    loadChart,
    fetchChart,
    fetchSheetInfo,
    pause,
    resume,
    restart,
    setSpeed,
    setNoteSpeed,
} from "~/composables/useEngine";
import { resumeAudio } from "~/utils/audioUnlock";
import { GAME } from "~/app/game";
import type { Sheet } from "~/types";
import MvIcon from "~/components/ui/MvIcon.vue";
import MvCover from "~/components/ui/MvCover.vue";

defineOptions({ name: "GameVisualizerPage" });

const { t } = useI18n();
const { getTypeName, getDifficultyName, getSheetSearchLink } = useGameData();
const route = useRoute();

// Catalog metadata for the loaded sheet (null for a manually pasted chart).
const sheetInfo = ref<Sheet | null>(null);
const internalLevel = computed(() =>
    sheetInfo.value?.internalLevelValue != null
        ? `(${sheetInfo.value.internalLevelValue.toFixed(1)})`
        : "",
);
const noteCountRows = computed(() =>
    Object.entries(sheetInfo.value?.noteCounts ?? {})
        .filter(([key, v]) => v != null && key !== "total")
        .map(([key, value]) => ({ key, value: value as number })),
);
const totalNotes = computed(() => sheetInfo.value?.noteCounts?.total ?? null);

// maimai difficulty accent colors (engine-agnostic; just for the pill).
const DIFFICULTY_COLORS: Record<string, string> = {
    basic: "#3ba955",
    advanced: "#e0a000",
    expert: "#e0405a",
    master: "#9a4fd0",
    remaster: "#c6a8e8",
};
const difficultyColor = computed(
    () =>
        DIFFICULTY_COLORS[sheetInfo.value?.difficulty ?? ""] ?? "var(--mv-fg)",
);

type EngineStatus = "booting" | "ready" | "error";
const status = ref<EngineStatus>("booting");
const errorMsg = ref("");

const simai = ref("");
const loading = ref(false);

// Transport state. The engine owns the real clock; these just mirror intent.
const loaded = ref(false);
const playing = ref(false);

// Playback (chart) speed: 0.1–1.0. Note (scroll) speed: 3–9. Both stepped.
const speed = ref(1);
const noteSpeed = ref(8);
const SPEED_MIN = 0.1;
const SPEED_MAX = 1;
const SPEED_STEP = 0.1;
const NOTE_MIN = 3;
const NOTE_MAX = 9;
const NOTE_STEP = 0.5;

function clamp(v: number, lo: number, hi: number): number {
    return Math.min(hi, Math.max(lo, Math.round(v * 100) / 100));
}
function applySpeed(v: number): void {
    speed.value = clamp(v, SPEED_MIN, SPEED_MAX);
    setSpeed(speed.value);
}
function applyNoteSpeed(v: number): void {
    noteSpeed.value = clamp(v, NOTE_MIN, NOTE_MAX);
    setNoteSpeed(noteSpeed.value);
}

// Ruler ticks for a slider: one mark per step, labelled on the major ones.
type Tick = { v: number; major: boolean };
function rangeTicks(
    min: number,
    max: number,
    step: number,
    isMajor: (v: number) => boolean,
): Tick[] {
    const out: Tick[] = [];
    const n = Math.round((max - min) / step);
    for (let i = 0; i <= n; i += 1) {
        const v = Math.round((min + i * step) * 100) / 100;
        out.push({ v, major: isMajor(v) });
    }
    return out;
}
const noteTicks = computed(() =>
    rangeTicks(NOTE_MIN, NOTE_MAX, NOTE_STEP, (v) => Number.isInteger(v)),
);
const playTicks = computed(() =>
    rangeTicks(SPEED_MIN, SPEED_MAX, SPEED_STEP, () => true),
);

function togglePlay(): void {
    playing.value = !playing.value;
    if (playing.value) {
        resumeAudio(); // this click is a gesture — unlock sound too
        resume();
    } else {
        pause();
    }
}
function onRestart(): void {
    resumeAudio();
    restart();
    playing.value = true;
}
function searchOnYouTube(): void {
    if (!sheetInfo.value) return;
    const link = getSheetSearchLink(sheetInfo.value);
    if (link != null) window.open(link, "_blank", "noopener");
}

// The page remounts on every route open (no <keep-alive>). The engine canvas is
// a JS-owned singleton (see useEngine): re-parent it into this page's stage on
// mount, park it off-screen on unmount — the wasm engine stays attached without
// re-instantiating. Boot the engine once; subsequent mounts just reattach.
const stage = ref<HTMLElement | null>(null);

onMounted(async () => {
    try {
        if (stage.value) mountEngineCanvas(stage.value);
        await ensureEngine();
        status.value = "ready";
        // Replay any already-loaded chart from the start on each open. A deep-link
        // (maybeAutoLoad) reloads its own chart and supersedes this.
        if (hasChart() && !hasDeepLink()) {
            restart();
            loaded.value = true;
            playing.value = true;
        }
        await maybeAutoLoad();
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
});

// Last deep-linked sheet, so re-activating with the same query doesn't reload.
let lastKey = "";

// True when the route carries a full deep-link (songId+type+difficulty).
function hasDeepLink(): boolean {
    const { songId, type, difficulty } = route.query;
    return (
        typeof songId === "string" &&
        typeof type === "string" &&
        typeof difficulty === "string"
    );
}

// Deep-link: /visualizer?songId=..&type=..&difficulty=.. (e.g. from the sheet
// dialog) auto-fetches that chart from the backend.
async function maybeAutoLoad(): Promise<void> {
    const { songId, type, difficulty } = route.query;
    if (
        typeof songId !== "string" ||
        typeof type !== "string" ||
        typeof difficulty !== "string"
    ) {
        return;
    }
    const key = `${songId}|${type}|${difficulty}`;
    if (key === lastKey) return; // same chart already loaded
    lastKey = key;

    loading.value = true;
    errorMsg.value = "";
    try {
        // Fetch the chart + its catalog metadata in parallel. Chart is required;
        // info is best-effort (don't fail the load if it's missing).
        const [chart, info] = await Promise.all([
            fetchChart(songId, type, difficulty),
            fetchSheetInfo(key).catch(() => null),
        ]);
        // Resolve the cover URL the same way preprocessData would (the API sends
        // only the raw imageName), so MvCover shows the real jacket.
        if (info && info.imageName && info.imageUrl == null) {
            info.imageUrl = new URL(
                info.imageName,
                `${GAME.dataSourceUrl}/img/cover/`,
            ).toString();
        }
        sheetInfo.value = info;
        simai.value = chart;
        await loadChart(chart);
        loaded.value = true;

        // Sound can only start from a user gesture (e.g. arriving via a click in
        // the sheet dialog). On a gesture-less hard reload, start paused — the
        // first transport click then both plays and unlocks audio.
        const active = navigator.userActivation?.isActive ?? false;
        if (active) {
            resumeAudio();
            playing.value = true; // engine auto-plays on parse
        } else {
            pause();
            playing.value = false;
        }
        if (speed.value !== 1) setSpeed(speed.value);
    } catch (err: unknown) {
        errorMsg.value = err instanceof Error ? err.message : String(err);
    } finally {
        loading.value = false;
    }
}

// Hand the pasted chart to the running engine. The chart clock is frame-driven,
// so it plays (silently) with no audio track.
async function onLoad(): Promise<void> {
    if (!simai.value.trim()) return;
    loading.value = true;
    errorMsg.value = "";
    try {
        await loadChart(simai.value);
        // Engine transitions to Playing on its own once the chart is parsed.
        loaded.value = true;
        playing.value = true;
        lastKey = ""; // manual load supersedes any deep-link key
        sheetInfo.value = null; // pasted chart has no catalog metadata
        resumeAudio(); // this click is a gesture — unlock sound
        if (speed.value !== 1) setSpeed(speed.value);
    } catch (err: unknown) {
        errorMsg.value = err instanceof Error ? err.message : String(err);
    } finally {
        loading.value = false;
    }
}

usePageTitle(() => ({ title: t("page-title.visualizer") as string }));
</script>

<template>
    <div class="mv-root mv-viz">
        <!-- engine canvas: the Bevy app renders the chart here -->
        <div class="mv-viz-stage">
            <div class="mv-canvas-wrap">
                <!-- The persistent <canvas id="bevy"> is re-parented here on mount. -->
                <div ref="stage" class="mv-canvas-host" />
                <div v-if="status !== 'ready'" class="mv-canvas-overlay">
                    <span v-if="status === 'booting'">{{
                        t("page.viz.booting")
                    }}</span>
                    <span v-else class="mv-canvas-err">{{ errorMsg }}</span>
                </div>
            </div>

            <!-- transport: drives the engine clock (disabled until a song is loaded) -->
            <div class="mv-transport" :class="{ 'mv-transport--off': !loaded }">
                <button
                    type="button"
                    class="mv-btn mv-ctl"
                    :disabled="!loaded"
                    @click="onRestart"
                >
                    <MvIcon name="restart" />
                </button>
                <button
                    type="button"
                    class="mv-btn mv-ctl mv-ctl--play"
                    :disabled="!loaded"
                    @click="togglePlay"
                >
                    <MvIcon :name="playing ? 'pause' : 'play'" filled />
                </button>
            </div>

            <!-- speed controls: stepped sliders with -/+ steppers -->
            <div class="mv-sliders" :class="{ 'mv-sliders--off': !loaded }">
                <div class="mv-slider-row">
                    <span class="mv-slider-label">{{
                        t("page.viz.noteSpeed")
                    }}</span>
                    <button
                        type="button"
                        class="mv-step"
                        :disabled="!loaded"
                        @click="applyNoteSpeed(noteSpeed - NOTE_STEP)"
                    >
                        <span class="mv-step-g">−</span>
                    </button>
                    <div class="mv-slider-track">
                        <input
                            type="range"
                            class="mv-range"
                            :min="NOTE_MIN"
                            :max="NOTE_MAX"
                            :step="NOTE_STEP"
                            :value="noteSpeed"
                            :disabled="!loaded"
                            @input="
                                applyNoteSpeed(
                                    +($event.target as HTMLInputElement).value,
                                )
                            "
                        />
                        <div class="mv-ticks">
                            <span
                                v-for="(tk, i) in noteTicks"
                                :key="i"
                                class="mv-tick"
                                :class="{ 'mv-tick--major': tk.major }"
                            >
                                <i class="mv-tick-mark" />
                                <em v-if="tk.major" class="mv-tick-lbl">{{
                                    tk.v
                                }}</em>
                            </span>
                        </div>
                    </div>
                    <button
                        type="button"
                        class="mv-step"
                        :disabled="!loaded"
                        @click="applyNoteSpeed(noteSpeed + NOTE_STEP)"
                    >
                        <span class="mv-step-g">+</span>
                    </button>
                    <span class="mv-slider-val">{{
                        noteSpeed.toFixed(1)
                    }}</span>
                </div>

                <div class="mv-slider-row">
                    <span class="mv-slider-label">{{
                        t("page.viz.playSpeed")
                    }}</span>
                    <button
                        type="button"
                        class="mv-step"
                        :disabled="!loaded"
                        @click="applySpeed(speed - SPEED_STEP)"
                    >
                        <span class="mv-step-g">−</span>
                    </button>
                    <div class="mv-slider-track">
                        <input
                            type="range"
                            class="mv-range"
                            :min="SPEED_MIN"
                            :max="SPEED_MAX"
                            :step="SPEED_STEP"
                            :value="speed"
                            :disabled="!loaded"
                            @input="
                                applySpeed(
                                    +($event.target as HTMLInputElement).value,
                                )
                            "
                        />
                        <div class="mv-ticks">
                            <span
                                v-for="(tk, i) in playTicks"
                                :key="i"
                                class="mv-tick"
                                :class="{ 'mv-tick--major': tk.major }"
                            >
                                <i class="mv-tick-mark" />
                                <em v-if="tk.major" class="mv-tick-lbl">{{
                                    tk.v.toFixed(1)
                                }}</em>
                            </span>
                        </div>
                    </div>
                    <button
                        type="button"
                        class="mv-step"
                        :disabled="!loaded"
                        @click="applySpeed(speed + SPEED_STEP)"
                    >
                        <span class="mv-step-g">+</span>
                    </button>
                    <span class="mv-slider-val">{{ speed.toFixed(1) }}×</span>
                </div>
            </div>
        </div>

        <!-- chart info + loader -->
        <div class="mv-viz-side">
            <div class="mv-label mv-side-label">
                {{ t("page.viz.chartInfo") }}
            </div>

            <!-- rich info when a catalog sheet is loaded -->
            <div v-if="sheetInfo" class="mv-card">
                <!-- header: jacket + identity -->
                <div class="mv-card-head">
                    <div class="mv-card-jacket">
                        <MvCover :item="sheetInfo" />
                    </div>
                    <div class="mv-card-id">
                        <span class="mv-card-cat">{{
                            sheetInfo.category
                        }}</span>
                        <div class="mv-card-title">{{ sheetInfo.title }}</div>
                        <div class="mv-card-artist">{{ sheetInfo.artist }}</div>
                    </div>
                </div>

                <!-- difficulty + level banner -->
                <div
                    class="mv-card-diff"
                    :style="{ '--diff': difficultyColor }"
                >
                    <span class="mv-card-diff-name">
                        {{ getDifficultyName(sheetInfo.difficulty) }}
                    </span>
                    <span class="mv-card-diff-level">
                        {{ sheetInfo.level }}
                        <span class="mv-card-diff-internal">{{
                            internalLevel
                        }}</span>
                    </span>
                </div>

                <!-- quick stats -->
                <div class="mv-card-stats">
                    <div class="mv-stat">
                        <span class="mv-stat-k">{{ t("term.type") }}</span>
                        <span class="mv-stat-v">{{
                            getTypeName(sheetInfo.type)
                        }}</span>
                    </div>
                    <div v-if="sheetInfo.bpm" class="mv-stat">
                        <span class="mv-stat-k">{{ t("term.bpm") }}</span>
                        <span class="mv-stat-v">{{ sheetInfo.bpm }}</span>
                    </div>
                    <div v-if="totalNotes != null" class="mv-stat">
                        <span class="mv-stat-k">{{
                            t("term.totalNotes")
                        }}</span>
                        <span class="mv-stat-v">{{ totalNotes }}</span>
                    </div>
                </div>

                <!-- note-count breakdown -->
                <div v-if="noteCountRows.length" class="mv-notes-grid">
                    <div
                        v-for="row in noteCountRows"
                        :key="row.key"
                        class="mv-note-cell"
                    >
                        <span class="mv-note-v">{{ row.value }}</span>
                        <span class="mv-note-k">{{ row.key }}</span>
                    </div>
                </div>

                <!-- meta rows -->
                <div class="mv-card-meta">
                    <div v-if="sheetInfo.noteDesigner" class="mv-meta">
                        <span class="mv-label">{{
                            t("term.noteDesigner")
                        }}</span>
                        <span>{{ sheetInfo.noteDesigner }}</span>
                    </div>
                    <div v-if="sheetInfo.version" class="mv-meta">
                        <span class="mv-label">{{ t("term.version") }}</span>
                        <span>{{ sheetInfo.version }}</span>
                    </div>
                    <div v-if="sheetInfo.releaseDate" class="mv-meta">
                        <span class="mv-label">{{
                            t("term.releaseDate")
                        }}</span>
                        <span>{{ sheetInfo.releaseDate }}</span>
                    </div>
                </div>

                <button
                    type="button"
                    class="mv-btn mv-yt"
                    @click="searchOnYouTube"
                >
                    <MvIcon name="youtube" :size="15" />
                    <span>{{ t("sfc.SheetDialog.searchOnYouTube") }}</span>
                </button>
            </div>

            <div v-else class="mv-awaiting">{{ t("page.viz.awaiting") }}</div>

            <div class="mv-paste">
                <div class="mv-label mv-side-label">
                    {{ t("page.viz.pasteTitle") }}
                </div>
                <textarea
                    v-model="simai"
                    class="mv-textarea"
                    rows="12"
                    :placeholder="t('page.viz.pastePlaceholder')"
                />

                <div class="mv-paste-row">
                    <span class="mv-paste-hint">
                        {{ t("page.viz.pasteHint") }}
                    </span>
                    <button
                        type="button"
                        class="mv-btn mv-parse"
                        :disabled="
                            loading || !simai.trim() || status !== 'ready'
                        "
                        @click="onLoad"
                    >
                        {{
                            loading ? t("page.viz.loading") : t("page.viz.load")
                        }}
                    </button>
                </div>
                <div v-if="errorMsg && status === 'ready'" class="mv-load-err">
                    {{ errorMsg }}
                </div>
            </div>
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
.mv-canvas-wrap {
    position: relative;
    width: 100%;
    aspect-ratio: 1000 / 800;
    background: #000;
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    overflow: hidden;
}
.mv-canvas-host {
    width: 100%;
    height: 100%;
}
/* :deep — the canvas is injected at runtime, so it lacks this component's scope. */
.mv-canvas-host :deep(.mv-canvas) {
    display: block;
    width: 100%;
    height: 100%;
}
.mv-canvas-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: var(--mv-mut);
    text-align: center;
    padding: 12px;
}
.mv-canvas-err {
    color: #e66;
    font-family: var(--mv-mono);
}

.mv-transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-top: 16px;
}
.mv-transport--off {
    opacity: 0.5;
}
.mv-ctl {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: 1px solid var(--mv-bd);
    background: none;
    color: var(--mv-fg);
    display: flex;
    align-items: center;
    justify-content: center;
}
.mv-ctl:disabled {
    cursor: not-allowed;
}
.mv-ctl--play {
    width: 46px;
    height: 46px;
    background: var(--mv-invbg);
    border-color: var(--mv-fg);
    color: var(--mv-inv);
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

/* ── info card ─────────────────────────────────────────────────────────── */
.mv-card {
    margin-top: 12px;
    border: 1px solid var(--mv-bd);
    border-radius: 8px;
    overflow: hidden;
    background: var(--mv-panel);
}

.mv-card-head {
    display: flex;
    gap: 12px;
    padding: 14px;
}
.mv-card-jacket {
    flex: 0 0 84px;
    width: 84px;
    height: 84px;
    border-radius: 6px;
    overflow: hidden;
    background: var(--mv-jacket-bg);
    border: 1px solid var(--mv-bd);
}
.mv-card-id {
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
}
.mv-card-cat {
    align-self: flex-start;
    font-size: 9px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--mv-mut);
    border: 1px solid var(--mv-bd);
    border-radius: 3px;
    padding: 2px 6px;
    margin-bottom: 6px;
}
.mv-card-title {
    font-size: 17px;
    font-weight: 700;
    line-height: 1.2;
    word-break: break-word;
}
.mv-card-artist {
    font-size: 11px;
    color: var(--mv-mut);
    margin-top: 2px;
    word-break: break-word;
}

.mv-card-diff {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    color: #fff;
    background: var(--diff);
}
.mv-card-diff-name {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
}
.mv-card-diff-level {
    font-size: 20px;
    font-weight: 800;
    font-family: var(--mv-mono);
}
.mv-card-diff-internal {
    font-size: 12px;
    font-weight: 500;
    opacity: 0.85;
}

.mv-card-stats {
    display: flex;
    border-bottom: 1px solid var(--mv-bd);
}
.mv-stat {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
    align-items: center;
    padding: 10px 4px;
    border-right: 1px solid var(--mv-bd);
}
.mv-stat:last-child {
    border-right: none;
}
.mv-stat-k {
    font-size: 9px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--mv-mut);
}
.mv-stat-v {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--mv-mono);
    text-align: center;
}

.mv-notes-grid {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 1px;
    background: var(--mv-bd);
    border-bottom: 1px solid var(--mv-bd);
}
.mv-note-cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 9px 2px;
    background: var(--mv-panel);
}
.mv-note-v {
    font-size: 14px;
    font-weight: 700;
    font-family: var(--mv-mono);
}
.mv-note-k {
    font-size: 8px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--mv-mut);
}

.mv-card-meta {
    padding: 6px 14px;
}
.mv-meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 7px 0;
    font-size: 12px;
    border-bottom: 1px solid var(--mv-bd2);
}
.mv-meta:last-child {
    border-bottom: none;
}
.mv-meta > span:last-child {
    text-align: right;
    word-break: break-word;
}

.mv-yt {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    width: 100%;
    box-sizing: border-box;
    background: none;
    color: var(--mv-fg);
    border: none;
    border-top: 1px solid var(--mv-bd);
    font-family: var(--mv-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    padding: 11px;
}
.mv-yt:hover {
    background: var(--mv-bg);
    color: #e0405a;
}
.mv-paste {
    margin-top: 22px;
    border-top: 1px solid var(--mv-bd);
    padding-top: 16px;
}
.mv-textarea {
    width: 100%;
    box-sizing: border-box;
    min-height: 240px;
    background: var(--mv-panel);
    border: 1px solid var(--mv-bd);
    border-radius: 4px;
    color: var(--mv-fg);
    font-family: var(--mv-mono);
    font-size: 13px;
    line-height: 1.5;
    padding: 12px;
    resize: vertical;
}
.mv-paste-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 12px;
}
.mv-paste-hint {
    font-size: 10px;
    color: var(--mv-mut);
}
.mv-parse {
    background: var(--mv-invbg);
    color: var(--mv-inv);
    border: none;
    border-radius: 3px;
    font-family: var(--mv-mono);
    font-size: 11px;
    padding: 6px 14px;
    font-weight: 700;
}
.mv-parse:disabled {
    opacity: 0.45;
    cursor: not-allowed;
}
.mv-load-err {
    margin-top: 10px;
    font-size: 10px;
    color: #e66;
    font-family: var(--mv-mono);
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

.mv-slider-row {
    display: flex;
    align-items: center;
    gap: 12px;
}

.mv-slider-label {
    font-size: 14px;
    font-weight: 700;
    color: var(--mv-fg);
    width: 80px; /* Fixed width to align sliders nicely */
    flex-shrink: 0;
}

.mv-slider-val {
    font-size: 14px;
    font-weight: 700;
    color: var(--mv-fg);
    width: 44px; /* Fixed width for the trailing value */
    text-align: right;
    flex-shrink: 0;
}

/* --- Control Buttons (-/+) --- */
.mv-step {
    background: var(--mv-invbg);
    border: none;
    border-radius: 9px;
    width: 42px;
    height: 34px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--mv-inv);
    cursor: pointer;
    transition:
        transform 0.12s ease,
        filter 0.12s ease;
    flex-shrink: 0;
    padding: 0;
}

.mv-step:hover:not(:disabled) {
    filter: brightness(1.12);
}

.mv-step:active:not(:disabled) {
    transform: translateY(1px) scale(0.96);
}
.mv-step:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}
.mv-step-g {
    line-height: 1;
    font-size: 20px;
    font-weight: 700;
}

/* --- Slider Track Area --- */
.mv-slider-track {
    flex: 1;
    position: relative;
    display: flex;
    flex-direction: column;
    padding-top: 9px; /* room for the top half of the thumb */
    min-width: 150px;
}

/* The pill background track */
.mv-slider-track::before {
    content: "";
    position: absolute;
    top: 16px;
    left: 0;
    right: 0;
    height: 5px;
    background: var(--mv-bd2);
    border-radius: 999px;
    z-index: 1;
}

/* --- Native Input Restyling --- */
.mv-range {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    background: transparent;
    margin: 0;
    outline: none;
    z-index: 2;
    position: relative;
    cursor: pointer;
}

/* Webkit (Chrome/Safari/Edge) */
.mv-range::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    height: 20px;
    width: 20px;
    border-radius: 50%;
    background: var(--mv-invbg);
    border: 3px solid var(--mv-bg, #fff);
    margin-top: -7px; /* (20 thumb - 5 track) / 2, minus border */
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
    transition: transform 0.1s ease;
}

.mv-range::-webkit-slider-thumb:hover {
    transform: scale(1.12);
}

/* Firefox */
.mv-range::-moz-range-thumb {
    height: 20px;
    width: 20px;
    border-radius: 50%;
    background: var(--mv-invbg);
    border: 3px solid var(--mv-bg, #fff);
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.25);
    transition: transform 0.1s ease;
    cursor: pointer;
}

.mv-range::-moz-range-thumb:hover {
    transform: scale(1.12);
}

.mv-range::-moz-range-track {
    background: transparent;
    border: none;
}

/* --- Tick ruler --- */
.mv-ticks {
    display: flex;
    justify-content: space-between;
    margin-top: 9px;
    padding: 0 10px; /* align first/last marks with thumb center at min/max */
}
.mv-tick {
    position: relative;
    width: 0; /* distribute strictly by center */
    display: flex;
    flex-direction: column;
    align-items: center;
}
/* minor mark */
.mv-tick-mark {
    width: 1px;
    height: 6px;
    background: var(--mv-bd);
    border-radius: 1px;
}
/* major mark: taller + stronger */
.mv-tick--major .mv-tick-mark {
    width: 2px;
    height: 10px;
    background: var(--mv-mut);
}
.mv-tick-lbl {
    margin-top: 4px;
    font-size: 10px;
    font-style: normal;
    color: var(--mv-mut);
    white-space: nowrap;
}
</style>
