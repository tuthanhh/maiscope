<script setup lang="ts">
import MvIcon from "~/components/ui/MvIcon.vue";

import useI18n from "~/composables/useI18n";

// Restart and play/pause, disabled until a chart is loaded; and the
// fullscreen toggle (visualizer-majdata 04), shown only where the browser
// supports it and usable with or without a chart.
defineProps<{
    loaded: boolean;
    playing: boolean;
    fullscreenSupported: boolean;
    fullscreen: boolean;
}>();
defineEmits<{ restart: []; toggle: []; fullscreen: [] }>();

const { t } = useI18n();
</script>

<template>
    <div class="mv-transport" :class="{ 'mv-transport--off': !loaded }">
        <button
            type="button"
            class="mv-btn mv-ctl"
            :disabled="!loaded"
            @click="$emit('restart')"
        >
            <MvIcon name="restart" />
        </button>
        <button
            type="button"
            class="mv-btn mv-ctl mv-ctl--play"
            :disabled="!loaded"
            @click="$emit('toggle')"
        >
            <MvIcon :name="playing ? 'pause' : 'play'" filled />
        </button>
        <button
            v-if="fullscreenSupported"
            type="button"
            class="mv-btn mv-ctl mv-ctl--fullscreen"
            :aria-label="(fullscreen ? t('page.viz.exitFullscreen') : t('page.viz.fullscreen')) as string"
            :title="(fullscreen ? t('page.viz.exitFullscreen') : t('page.viz.fullscreen')) as string"
            :aria-pressed="fullscreen"
            @click="$emit('fullscreen')"
        >
            <MvIcon :name="fullscreen ? 'minimize' : 'maximize'" />
        </button>
    </div>
</template>

<style scoped>
.mv-transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-top: 16px;
}
.mv-transport--off .mv-ctl:not(.mv-ctl--fullscreen) {
    opacity: 0.5;
}
.mv-ctl {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    border: 1px solid var(--line);
    background: none;
    color: var(--fg);
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
    background: var(--inverse-bg);
    border-color: var(--fg);
    color: var(--inverse-fg);
}
</style>
