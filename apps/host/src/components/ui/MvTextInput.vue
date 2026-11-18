<script setup lang="ts">
import { inject } from "vue";
import { fieldKey } from "./fieldContext";
import MvIcon from "./MvIcon.vue";
import type { IconName } from "./icons";

defineProps<{
  modelValue: string;
  placeholder?: string;
  /** The accessible name when no Field labels this control. */
  label?: string;
  inputmode?: "text" | "decimal" | "numeric" | "search";
  width?: number;
  /** A leading icon in `--fg-muted`, e.g. search. */
  icon?: IconName;
  invalid?: boolean;
  disabled?: boolean;
}>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

function onInput(event: Event): void {
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

// Inside an MvField: take its id and describing note.
const field = inject(fieldKey, null);
</script>

<template>
  <div class="mv-input-wrap" :style="{ width: width != null ? `${width}px` : undefined }">
    <span v-if="icon" class="mv-input-icon"><MvIcon :name="icon" :size="15" /></span>
    <input
      :id="field?.id"
      :aria-label="field ? undefined : label"
      :aria-describedby="field?.describedBy()"
      class="mv-input"
      :class="{ 'mv-input--icon': icon }"
      :value="modelValue"
      :placeholder="placeholder"
      :inputmode="inputmode"
      :aria-invalid="invalid || field?.invalid() || undefined"
      :disabled="disabled"
      @input="onInput"
    />
  </div>
</template>

<style scoped lang="scss">
@use "./control";

.mv-input-wrap {
  position: relative;
  display: inline-block;
}
.mv-input {
  @include control.control;
  width: 100%;
}
.mv-input--icon {
  padding-left: calc(var(--control-px) + 15px + var(--s-2));
}
.mv-input-icon {
  position: absolute;
  left: var(--control-px);
  top: 50%;
  transform: translateY(-50%);
  color: var(--fg-muted);
  pointer-events: none;
}
</style>
