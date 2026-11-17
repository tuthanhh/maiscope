<script setup lang="ts">
import MvIcon from "~/components/ui/MvIcon.vue";

// Restart and play/pause. Disabled until a chart is loaded.
defineProps<{ loaded: boolean; playing: boolean }>();
defineEmits<{ restart: []; toggle: [] }>();
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
</style>
