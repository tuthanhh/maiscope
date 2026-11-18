import { computed, ref, type ComputedRef, type Ref } from "vue";
import type { RouteLocationNormalizedLoaded } from "vue-router";
import { computeAutoLead, setAudioLeadMs, type AutoLead } from "~/composables/useEngine";
import { audioLatency } from "~/utils/audioUnlock";

// ── Automatic audio lead ─────────────────────────────────────────────────────
// Re-derived every two seconds while the page is open, not once: the context
// reports 0 until a gesture starts it, and `outputLatency` changes when a
// Bluetooth device connects. Only pushed to the engine when the number moves.
// `?lead=<ms>` in the URL overrides it for A/B against the ear.
export default function useAutoLead(route: RouteLocationNormalizedLoaded): {
    autoLead: Ref<AutoLead | null>;
    manualLead: ComputedRef<number | null>;
    start: () => void;
    stop: () => void;
} {
    const autoLead = ref<AutoLead | null>(null);
    const manualLead = computed(() => {
        const v = Number(route.query.lead);
        return route.query.lead !== undefined && Number.isFinite(v) && v >= 0 ? v : null;
    });
    let timer: ReturnType<typeof setInterval> | null = null;
    let appliedLeadMs = -1;

    function refresh(): void {
        autoLead.value = computeAutoLead(audioLatency());
        if (manualLead.value !== null) return;
        const next = autoLead.value?.leadMs ?? 0;
        if (next !== appliedLeadMs) {
            appliedLeadMs = next;
            setAudioLeadMs(next);
        }
    }
    function start(): void {
        if (timer !== null) return;
        refresh();
        timer = setInterval(refresh, 2000);
    }
    function stop(): void {
        if (timer !== null) clearInterval(timer);
        timer = null;
    }

    return { autoLead, manualLead, start, stop };
}
