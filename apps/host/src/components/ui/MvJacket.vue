<script setup lang="ts">
import { computed } from "vue";

// Deterministic monochrome generative jacket — seeded abstract art per song,
// so cover art never breaks the two-tone palette (real CDN covers would).
// Ported from the React prototype's seedRnd + Jacket.

type Shape =
  | { kind: "rect"; x: number; y: number; w: number; h: number; op: number; rot: number }
  | { kind: "circle"; cx: number; cy: number; r: number; sw: number; op: number }
  | { kind: "line"; x1: number; y1: number; x2: number; y2: number; sw: number; op: number };

const props = withDefaults(
  defineProps<{ seed: string; size?: string; round?: number }>(),
  { size: "100%", round: 0 },
);

function makeRng(seed: string): () => number {
  let s = 0;
  for (const ch of seed) s = (s * 31 + ch.charCodeAt(0)) >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

const shapes = computed<Shape[]>(() => {
  const r = makeRng(props.seed);
  const result: Shape[] = [];
  const n = 5 + Math.floor(r() * 4);
  for (let i = 0; i < n; i += 1) {
    const op = 0.12 + r() * 0.5;
    const kind = Math.floor(r() * 3);
    if (kind === 0) {
      result.push({
        kind: "rect",
        x: r() * 100,
        y: r() * 100,
        w: 20 + r() * 60,
        h: 4 + r() * 20,
        op,
        rot: r() * 360,
      });
    } else if (kind === 1) {
      result.push({
        kind: "circle",
        cx: r() * 100,
        cy: r() * 100,
        r: 6 + r() * 34,
        sw: 0.6 + r() * 2,
        op,
      });
    } else {
      result.push({
        kind: "line",
        x1: r() * 100,
        y1: r() * 100,
        x2: r() * 100,
        y2: r() * 100,
        sw: 0.6 + r() * 2.4,
        op,
      });
    }
  }
  return result;
});
</script>

<template>
  <svg
    viewBox="0 0 100 100"
    preserveAspectRatio="xMidYMid slice"
    :style="{
      width: size,
      height: size,
      display: 'block',
      background: 'var(--mv-jacket-bg)',
      color: 'var(--mv-fg)',
      borderRadius: `${round}px`,
    }"
  >
    <template v-for="(s, i) in shapes" :key="i">
      <rect
        v-if="s.kind === 'rect'"
        :x="s.x"
        :y="s.y"
        :width="s.w"
        :height="s.h"
        fill="currentColor"
        :opacity="s.op"
        :transform="`rotate(${s.rot} 50 50)`"
      />
      <circle
        v-else-if="s.kind === 'circle'"
        :cx="s.cx"
        :cy="s.cy"
        :r="s.r"
        fill="none"
        stroke="currentColor"
        :stroke-width="s.sw"
        :opacity="s.op"
      />
      <line
        v-else
        :x1="s.x1"
        :y1="s.y1"
        :x2="s.x2"
        :y2="s.y2"
        stroke="currentColor"
        :stroke-width="s.sw"
        :opacity="s.op"
      />
    </template>
  </svg>
</template>
