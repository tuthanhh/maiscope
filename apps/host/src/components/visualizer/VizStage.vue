<script setup lang="ts">
import { ref } from "vue";
import useI18n from "~/composables/useI18n";

// The engine canvas's frame: the host the persistent `<canvas id="bevy">` is
// re-parented into, the boot/error overlay, and the `?debug` readout. The page
// mounts the canvas into `host` (exposed below); this component never touches
// the engine itself.
defineProps<{
    status: "booting" | "ready" | "error";
    errorMsg: string;
    debug: boolean;
    statsLines: string[];
}>();

const { t } = useI18n();
const host = ref<HTMLElement | null>(null);
defineExpose({ host });
</script>

<template>
    <div class="mv-canvas-wrap">
        <!-- The persistent <canvas id="bevy"> is re-parented here on mount. -->
        <div ref="host" class="mv-canvas-host" />
        <div v-if="status !== 'ready'" class="mv-canvas-overlay">
            <span v-if="status === 'booting'">{{ t("page.viz.booting") }}</span>
            <span v-else class="mv-canvas-err">{{ errorMsg }}</span>
        </div>
        <!-- ?debug — engine frame pacing, one line per second -->
        <pre v-if="debug" class="mv-canvas-stats">{{ statsLines.join("\n") }}</pre>
    </div>
</template>

<style scoped>
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
/* ?debug readout: top-left of the canvas, readable on a phone at arm's length. */
.mv-canvas-stats {
    position: absolute;
    top: 6px;
    left: 6px;
    margin: 0;
    padding: 6px 8px;
    font: 12px/1.4 var(--mv-mono);
    color: #fff;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 4px;
    pointer-events: none;
    white-space: pre;
}
</style>
