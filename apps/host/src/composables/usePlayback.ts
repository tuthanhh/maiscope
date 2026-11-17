import { ref, type Ref } from "vue";
import { pause, restart, resume, setNoteSpeed, setSpeed } from "~/composables/useEngine";
import { resumeAudio } from "~/utils/audioUnlock";

// Playback (chart) speed: 0.1–1.0. Note (scroll) speed: 3–9. Both stepped.
export const SPEED = { min: 0.1, max: 1, step: 0.1 } as const;
export const NOTE_SPEED = { min: 3, max: 9, step: 0.5 } as const;

function clamp(v: number, lo: number, hi: number): number {
    return Math.min(hi, Math.max(lo, Math.round(v * 100) / 100));
}

/** Transport state. The engine owns the real clock; these just mirror intent. */
export type Playback = {
    loaded: Ref<boolean>;
    playing: Ref<boolean>;
    speed: Ref<number>;
    noteSpeed: Ref<number>;
    applySpeed: (v: number) => void;
    applyNoteSpeed: (v: number) => void;
    togglePlay: () => void;
    restartFromTop: () => void;
};

export default function usePlayback(): Playback {
    const loaded = ref(false);
    const playing = ref(false);
    const speed = ref(1);
    const noteSpeed = ref(8);

    function applySpeed(v: number): void {
        speed.value = clamp(v, SPEED.min, SPEED.max);
        setSpeed(speed.value);
    }
    function applyNoteSpeed(v: number): void {
        noteSpeed.value = clamp(v, NOTE_SPEED.min, NOTE_SPEED.max);
        setNoteSpeed(noteSpeed.value);
    }
    function togglePlay(): void {
        playing.value = !playing.value;
        if (playing.value) {
            resumeAudio(); // this click is a gesture — unlock sound too
            resume();
        } else {
            pause();
        }
    }
    function restartFromTop(): void {
        resumeAudio();
        restart();
        playing.value = true;
    }

    return {
        loaded,
        playing,
        speed,
        noteSpeed,
        applySpeed,
        applyNoteSpeed,
        togglePlay,
        restartFromTop,
    };
}
