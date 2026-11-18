<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import useI18n from "~/composables/useI18n";
import {
    formatClock,
    measureAt,
    measureStart,
    stepMeasure,
    timeMarks,
    type Timeline,
} from "~/utils/timeline";

// The chart at a glance, and a way to jump in it (visualizer-majdata 11): one
// bar per measure, its notes stacked by kind, time marks every 30 s and a
// playhead; under it a slider numbered in measures. Graph, marks, playhead and
// slider ticks share one axis — chart time — so a measure's bar sits over its
// tick even where the BPM changes.
const props = defineProps<{
    timeline: Timeline | null;
    position: number;
    disabled: boolean;
}>();
const emit = defineEmits<{ seek: [secs: number] }>();

const { t } = useI18n();

const off = computed(() => props.disabled || !props.timeline?.measures.length);
const length = computed(() => props.timeline?.length ?? 0);
const count = computed(() => props.timeline?.measures.length ?? 0);

// Fraction of the chart, as a CSS percentage.
function pct(secs: number): string {
    return length.value > 0 ? `${(Math.min(Math.max(secs / length.value, 0), 1) * 100).toFixed(3)}%` : "0%";
}

const KINDS = ["tap", "slide", "touch", "hold"] as const;

// One stacked bar per measure, in viewBox units: x and width in seconds,
// heights scaled to the busiest measure (100 = full height).
const bars = computed(() => {
    const tl = props.timeline;
    if (!tl) return [];
    const most = Math.max(1, ...tl.measures.map((m) => m.tap + m.slide + m.touch + m.hold));
    return tl.measures.map((m, i) => {
        const end = tl.measures[i + 1]?.start ?? tl.length;
        const width = Math.max((end - m.start) * 0.8, 0);
        let y = 100;
        const parts = KINDS.filter((k) => m[k] > 0).map((k) => {
            const h = (m[k] / most) * 100;
            y -= h;
            return { kind: k, y, h };
        });
        return { x: m.start, width, parts };
    });
});

// A mark in the last 5% would sit on the end of the chart, which the clock
// above already shows.
const marks = computed(() => timeMarks(length.value).filter((m) => m < length.value * 0.95));

// Slider ticks at each measure's start; numbered every 10.
const ticks = computed(() =>
    (props.timeline?.measures ?? []).map((m, i) => ({ at: m.start, label: i % 10 === 0 ? String(i) : null })),
);

const current = computed(() => (props.timeline ? measureAt(props.timeline, props.position) : 0));

// ── Seeking ─────────────────────────────────────────────────────────────────
// A drag asks for at most one seek per display frame: each one respawns the
// notes on screen.

let pending: number | null = null;
let frame: number | null = null;
function requestSeek(secs: number): void {
    pending = secs;
    if (frame !== null) return;
    frame = requestAnimationFrame(() => {
        frame = null;
        if (pending !== null) emit("seek", pending);
        pending = null;
    });
}
onUnmounted(() => {
    if (frame !== null) cancelAnimationFrame(frame);
});

function timeAt(e: PointerEvent): number {
    const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
    return (Math.min(Math.max((e.clientX - box.left) / box.width, 0), 1)) * length.value;
}

// On the graph a drag seeks to the pointer; on the slider, to the start of
// the measure under it.
function seekFrom(e: PointerEvent, snap: boolean): void {
    if (off.value || !props.timeline) return;
    const secs = timeAt(e);
    requestSeek(snap ? measureStart(props.timeline, measureAt(props.timeline, secs)) : secs);
}

const dragging = ref<"graph" | "slider" | null>(null);
function down(e: PointerEvent, part: "graph" | "slider"): void {
    if (off.value) return;
    dragging.value = part;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    seekFrom(e, part === "slider");
}
function move(e: PointerEvent, part: "graph" | "slider"): void {
    if (dragging.value === part) seekFrom(e, part === "slider");
}
function up(): void {
    dragging.value = null;
}

function key(e: KeyboardEvent): void {
    if (off.value || !props.timeline) return;
    const to = stepMeasure(e.key, current.value, count.value);
    if (to === null) return;
    e.preventDefault();
    requestSeek(measureStart(props.timeline, to));
}
</script>

<template>
    <div class="mv-tl" :class="{ 'mv-tl--off': off }">
        <div class="mv-tl-clock t-meta">
            <span>{{ formatClock(position) }}</span>
            <span>{{ formatClock(length) }}</span>
        </div>

        <div
            class="mv-tl-graph"
            aria-hidden="true"
            @pointerdown="down($event, 'graph')"
            @pointermove="move($event, 'graph')"
            @pointerup="up"
            @pointercancel="up"
        >
            <span v-for="m in marks" :key="m" class="mv-tl-mark" :style="{ left: pct(m) }">
                {{ formatClock(m) }}
            </span>
            <svg
                v-if="length > 0"
                class="mv-tl-bars"
                :viewBox="`0 0 ${length} 100`"
                preserveAspectRatio="none"
            >
                <g v-for="(bar, i) in bars" :key="i">
                    <rect
                        v-for="part in bar.parts"
                        :key="part.kind"
                        :class="`mv-tl-${part.kind}`"
                        :x="bar.x"
                        :y="part.y"
                        :width="bar.width"
                        :height="part.h"
                    />
                </g>
            </svg>
            <i v-if="!off" class="mv-tl-head" :style="{ left: pct(position) }" />
        </div>

        <div
            class="mv-tl-slider"
            role="slider"
            :tabindex="off ? -1 : 0"
            :aria-label="t('page.viz.timeline') as string"
            :aria-disabled="off"
            aria-valuemin="0"
            :aria-valuemax="Math.max(count - 1, 0)"
            :aria-valuenow="current"
            :aria-valuetext="t('page.viz.measure', { n: current }) as string"
            @keydown="key"
            @pointerdown="down($event, 'slider')"
            @pointermove="move($event, 'slider')"
            @pointerup="up"
            @pointercancel="up"
        >
            <div class="mv-tl-track">
                <i class="mv-tl-fill" :style="{ width: off ? '0%' : pct(position) }" />
                <i v-if="!off" class="mv-tl-thumb" :style="{ left: pct(position) }" />
            </div>
            <div class="mv-tl-ticks">
                <span
                    v-for="(tk, i) in ticks"
                    :key="i"
                    class="mv-tl-tick"
                    :class="{ 'mv-tl-tick--major': tk.label !== null }"
                    :style="{ left: pct(tk.at) }"
                >
                    <em v-if="tk.label !== null">{{ tk.label }}</em>
                </span>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* The spec's four colours: pink tap, blue slide, yellow touch, orange hold. */
.mv-tl {
    --tl-tap: #f0609a;
    --tl-slide: #3fa9e6;
    --tl-touch: #e8b923;
    --tl-hold: #f28a2e;
    margin-top: 16px;
    /* room for the slider thumb at either end */
    padding: 0 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    user-select: none;
    touch-action: none;
}
:global(:root.dark) .mv-tl {
    --tl-tap: #ff79b0;
    --tl-slide: #6cc8ff;
    --tl-touch: #ffd24d;
    --tl-hold: #ffa04d;
}
.mv-tl--off {
    opacity: 0.5;
}

.mv-tl-clock {
    display: flex;
    justify-content: space-between;
    font-variant-numeric: tabular-nums;
}

.mv-tl-graph {
    position: relative;
    height: 72px;
    padding-top: 16px; /* the time marks */
    border-bottom: 1px solid var(--line-strong);
    cursor: pointer;
}
.mv-tl--off .mv-tl-graph,
.mv-tl--off .mv-tl-slider {
    cursor: not-allowed;
}
.mv-tl-bars {
    display: block;
    width: 100%;
    height: 100%;
}
.mv-tl-tap {
    fill: var(--tl-tap);
}
.mv-tl-slide {
    fill: var(--tl-slide);
}
.mv-tl-touch {
    fill: var(--tl-touch);
}
.mv-tl-hold {
    fill: var(--tl-hold);
}
.mv-tl-mark {
    position: absolute;
    top: 0;
    transform: translateX(-50%);
    font-size: 11px;
    line-height: 14px;
    color: var(--fg-muted);
    pointer-events: none;
}
.mv-tl-mark::after {
    content: "";
    position: absolute;
    left: 50%;
    top: 14px;
    height: 58px;
    border-left: 1px dashed var(--line);
}
.mv-tl-head {
    position: absolute;
    top: 14px;
    bottom: -1px;
    width: 2px;
    margin-left: -1px;
    background: var(--fg);
    pointer-events: none;
}

.mv-tl-slider {
    position: relative;
    padding: 9px 0 0;
    cursor: pointer;
    outline: none;
}
.mv-tl-slider:focus-visible .mv-tl-thumb {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
}
.mv-tl-track {
    position: relative;
    height: 6px;
    border-radius: 3px;
    background: var(--bg-sunken);
    border: 1px solid var(--line);
}
.mv-tl-fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: 3px;
    background: var(--fg-muted);
}
.mv-tl-thumb {
    position: absolute;
    top: 50%;
    width: 16px;
    height: 16px;
    margin: -8px 0 0 -8px;
    border-radius: 50%;
    background: var(--inverse-bg);
    border: 2px solid var(--bg);
    box-shadow: 0 0 0 1px var(--fg);
}
.mv-tl-ticks {
    position: relative;
    height: 22px;
}
.mv-tl-tick {
    position: absolute;
    top: 2px;
    height: 4px;
    border-left: 1px solid var(--line-strong);
}
.mv-tl-tick--major {
    height: 7px;
    border-left-color: var(--fg-muted);
}
.mv-tl-tick em {
    position: absolute;
    top: 8px;
    transform: translateX(-50%);
    font-style: normal;
    font-size: 11px;
    color: var(--fg-muted);
}
</style>
