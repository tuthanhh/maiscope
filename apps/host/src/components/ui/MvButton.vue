<script setup lang="ts">
import type { RouteLocationRaw } from "vue-router";
import MvIcon from "./MvIcon.vue";
import type { IconName } from "./icons";

// design-direction "Button": four kinds, one height. A primary button is the
// one strong state on a screen; an icon button always carries a name.
const props = withDefaults(
  defineProps<{
    kind?: "primary" | "ghost" | "text" | "icon";
    icon?: IconName;
    /** Required for `kind="icon"`: the accessible name. */
    label?: string;
    /** Icon drawn filled (a state signal: bookmarked, playing). */
    filled?: boolean;
    disabled?: boolean;
    to?: RouteLocationRaw;
    type?: "button" | "submit";
  }>(),
  { kind: "ghost", filled: false, disabled: false, type: "button" },
);
defineEmits<{ click: [event: MouseEvent] }>();
</script>

<template>
  <RouterLink
    v-if="to && !disabled"
    :to="to"
    class="mv-button"
    :class="`mv-button--${kind}`"
    :aria-label="props.kind === 'icon' ? label : undefined"
    :title="props.kind === 'icon' ? label : undefined"
  >
    <MvIcon v-if="icon" :name="icon" :size="16" :filled="filled" />
    <span v-if="kind !== 'icon'" class="mv-button-text"><slot /></span>
  </RouterLink>
  <button
    v-else
    :type="type"
    class="mv-button"
    :class="`mv-button--${kind}`"
    :disabled="disabled"
    :aria-label="props.kind === 'icon' ? label : undefined"
    :title="props.kind === 'icon' ? label : undefined"
    @click="$emit('click', $event)"
  >
    <MvIcon v-if="icon" :name="icon" :size="16" :filled="filled" />
    <span v-if="kind !== 'icon'" class="mv-button-text"><slot /></span>
  </button>
</template>

<style scoped>
.mv-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--s-2);
  height: var(--control-h);
  padding: 0 var(--control-px);
  border-radius: var(--r);
  border: var(--line-w) solid transparent;
  background: transparent;
  color: var(--fg);
  font: 400 14px/21px var(--f-sans);
  text-decoration: none;
  cursor: pointer;
  white-space: nowrap;
  transition:
    background var(--dur),
    border-color var(--dur),
    color var(--dur),
    opacity var(--dur);
}
.mv-button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.mv-button--primary {
  background: var(--inverse-bg);
  color: var(--inverse-fg);
}
.mv-button--primary:not(:disabled):is(:hover, :active) {
  background: var(--inverse-hover);
}

.mv-button--ghost {
  border-color: var(--line);
}
.mv-button--ghost:not(:disabled):hover {
  border-color: var(--line-strong);
}
.mv-button--ghost:not(:disabled):active {
  background: var(--bg-panel);
}

.mv-button--text {
  padding: 0;
  height: auto;
  min-height: var(--control-h);
}
.mv-button--text:not(:disabled):hover .mv-button-text {
  text-decoration: underline;
  text-underline-offset: 3px;
}

.mv-button--icon {
  width: var(--control-h);
  padding: 0;
}
.mv-button--icon:not(:disabled):hover {
  opacity: 0.7;
}
.mv-button--icon:not(:disabled):active {
  background: var(--bg-panel);
}
</style>
