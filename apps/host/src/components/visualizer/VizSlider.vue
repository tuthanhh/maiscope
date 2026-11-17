<script setup lang="ts">
import { computed } from "vue";

// One stepped slider with -/+ steppers and a tick ruler: note speed and play
// speed are both this.
const props = defineProps<{
    label: string;
    value: number;
    min: number;
    max: number;
    step: number;
    disabled: boolean;
    /** Ticks that get a mark and a label; the rest get a short mark. */
    isMajor: (v: number) => boolean;
    format: (v: number) => string;
    /** The trailing readout, e.g. `0.5×`. */
    readout: string;
}>();
const emit = defineEmits<{ change: [value: number] }>();

// Ruler ticks: one mark per step, labelled on the major ones.
const ticks = computed(() => {
    const out: { v: number; major: boolean }[] = [];
    const n = Math.round((props.max - props.min) / props.step);
    for (let i = 0; i <= n; i += 1) {
        const v = Math.round((props.min + i * props.step) * 100) / 100;
        out.push({ v, major: props.isMajor(v) });
    }
    return out;
});
</script>

<template>
    <div class="mv-slider-row">
        <span class="mv-slider-label">{{ label }}</span>
        <button
            type="button"
            class="mv-step"
            :disabled="disabled"
            @click="emit('change', value - step)"
        >
            <span class="mv-step-g">−</span>
        </button>
        <div class="mv-slider-track">
            <input
                type="range"
                class="mv-range"
                :min="min"
                :max="max"
                :step="step"
                :value="value"
                :disabled="disabled"
                @input="emit('change', +($event.target as HTMLInputElement).value)"
            />
            <div class="mv-ticks">
                <span
                    v-for="(tk, i) in ticks"
                    :key="i"
                    class="mv-tick"
                    :class="{ 'mv-tick--major': tk.major }"
                >
                    <i class="mv-tick-mark" />
                    <em v-if="tk.major" class="mv-tick-lbl">{{ format(tk.v) }}</em>
                </span>
            </div>
        </div>
        <button
            type="button"
            class="mv-step"
            :disabled="disabled"
            @click="emit('change', value + step)"
        >
            <span class="mv-step-g">+</span>
        </button>
        <span class="mv-slider-val">{{ readout }}</span>
    </div>
</template>

<style scoped>
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
